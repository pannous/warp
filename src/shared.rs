//! Shared arrays natively (user decision P33 step 6, lowering/shared_arrays.rs): the host holds them for the whole run,
//! every instance (the program and its tasks) reaches them through the host words, each cell an atomic Int.

use crate::host::{HostState, HOST_LIBRARY, SHARED_FLOAT_WORDS, SHARED_WORDS};
use anyhow::Result;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use wasmtime::{Linker, Module};

/// The arrays of one run, numbered from 1 in the order they are made
#[derive(Default)]
pub struct SharedArrays {
	arrays: Mutex<Vec<Arc<[AtomicI64]>>>,
}

/// Does the module use shared arrays
pub fn imports_shared(module: &Module) -> bool {
	module.imports().any(|import| import.module() == HOST_LIBRARY && (SHARED_WORDS.contains(&import.name()) || SHARED_FLOAT_WORDS.contains(&import.name())))
}

impl SharedArrays {
	fn array(&self, id: i64) -> wasmtime::Result<Arc<[AtomicI64]>> {
		let arrays = self.arrays.lock().expect("shared arrays");
		usize::try_from(id - 1).ok().and_then(|index| arrays.get(index).cloned()).ok_or_else(|| wasmtime::Error::msg(format!("no shared array {id}")))
	}

	/// The cell of 1-based `index`, or the runtime error an index out of range is
	fn cell(&self, id: i64, index: i64) -> wasmtime::Result<(Arc<[AtomicI64]>, usize)> {
		let array = self.array(id)?;
		match usize::try_from(index - 1).ok().filter(|cell| *cell < array.len()) {
			Some(cell) => Ok((array, cell)),
			None => Err(wasmtime::Error::new(crate::tasks::TaskFailure("index out of range".to_string()))),
		}
	}

	/// Link the shared words into an instance of the run
	pub fn link_into(self: &Arc<Self>, linker: &mut Linker<HostState>) -> Result<()> {
		let [new, get, set, add, count] = SHARED_WORDS;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, new, move |length: i64| -> i64 {
			let mut all = arrays.arrays.lock().expect("shared arrays");
			all.push((0..length.max(0)).map(|_| AtomicI64::new(0)).collect());
			all.len() as i64
		})?;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, get, move |id: i64, index: i64| -> wasmtime::Result<i64> {
			let (array, cell) = arrays.cell(id, index)?;
			Ok(array[cell].load(Ordering::SeqCst))
		})?;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, set, move |id: i64, index: i64, value: i64| -> wasmtime::Result<i64> {
			let (array, cell) = arrays.cell(id, index)?;
			array[cell].store(value, Ordering::SeqCst);
			Ok(value)
		})?;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, add, move |id: i64, index: i64, value: i64| -> wasmtime::Result<i64> {
			let (array, cell) = arrays.cell(id, index)?;
			Ok(array[cell].fetch_add(value, Ordering::SeqCst) + value)
		})?;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, count, move |id: i64| -> wasmtime::Result<i64> { Ok(arrays.array(id)?.len() as i64) })?;
		// an array of floats: the cells hold the bits; an add swaps until no other task came between
		let [get_float, set_float, add_float] = SHARED_FLOAT_WORDS;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, get_float, move |id: i64, index: i64| -> wasmtime::Result<f64> {
			let (array, cell) = arrays.cell(id, index)?;
			Ok(f64::from_bits(array[cell].load(Ordering::SeqCst) as u64))
		})?;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, set_float, move |id: i64, index: i64, value: f64| -> wasmtime::Result<f64> {
			let (array, cell) = arrays.cell(id, index)?;
			let value = warp_runtime::floats::canonical_nan(value);
			array[cell].store(value.to_bits() as i64, Ordering::SeqCst);
			Ok(value)
		})?;
		let arrays = self.clone();
		linker.func_wrap(HOST_LIBRARY, add_float, move |id: i64, index: i64, value: f64| -> wasmtime::Result<f64> {
			let (array, cell) = arrays.cell(id, index)?;
			let mut old = array[cell].load(Ordering::SeqCst);
			loop {
				let sum = warp_runtime::floats::canonical_nan(f64::from_bits(old as u64) + value);
				match array[cell].compare_exchange(old, sum.to_bits() as i64, Ordering::SeqCst, Ordering::SeqCst) {
					Ok(_) => return Ok(sum),
					Err(current) => old = current,
				}
			}
		})?;
		Ok(())
	}
}
