//! warp-runtime as a C library, libwarp_runtime (`cargo rustc --release -p warp-runtime --lib --crate-type cdylib
//! --features library`): one function, run_wasm, declared in include/warp_runtime.h. It compiles and runs a module with
//! the stub's host words, libm, libc and print, so only bytes and a text cross C: no structs, no callbacks. A warp
//! program calls it as any C library: `use warp_runtime; run_wasm(bytes)`
use crate::standalone::{run_module, running_engine, Ran};
use std::cell::RefCell;
use std::ffi::{c_char, CString};
use wasmtime::{AsContextMut, Module, Result, Val};

thread_local! {
	/// The text run_wasm gave last on this thread: it stays until the next call
	static LAST_RESULT: RefCell<CString> = RefCell::default();
}

/// Run the WebAssembly module of `length` bytes at `module`: its main's result as a text (a number's digits, a warp
/// value's text), "" for none, the exit code after `exit(code)`; NULL when it fails, the reason on stderr. What the
/// module prints goes to stdout
///
/// # Safety
/// `module` points to `length` readable bytes (or is NULL)
#[no_mangle]
pub unsafe extern "C" fn run_wasm(module: *const u8, length: usize) -> *const c_char {
	let bytes = if module.is_null() { &[][..] } else { unsafe { std::slice::from_raw_parts(module, length) } };
	let ran = std::panic::catch_unwind(|| run_bytes(bytes)).unwrap_or_else(|_| Err(wasmtime::Error::msg("panicked")));
	match ran {
		Ok(text) => LAST_RESULT.with(|last| {
			*last.borrow_mut() = CString::new(text.replace('\0', "")).unwrap_or_default();
			last.borrow().as_ptr()
		}),
		Err(failure) => {
			eprintln!("run_wasm: {failure:#}");
			std::ptr::null()
		}
	}
}

fn run_bytes(bytes: &[u8]) -> Result<String> {
	let time_limit = crate::fuel::time_limit_from_environment();
	let engine = running_engine(time_limit)?;
	let module = Module::new(&engine, bytes)?;
	match run_module(&engine, &module, &[], time_limit)? {
		Ran::Exited => Ok(crate::system_signals::take_exit_code().unwrap_or(0).to_string()),
		Ran::Returned { mut store, instance, results } => {
			let memory = instance.get_memory(&mut store, crate::gc_values::MEMORY);
			match results.first() {
				None => Ok(String::new()),
				Some(Val::I32(number)) => Ok(number.to_string()),
				Some(Val::I64(number)) => Ok(number.to_string()),
				Some(Val::F32(bits)) => Ok(f32::from_bits(*bits).to_string()),
				Some(Val::F64(bits)) => Ok(f64::from_bits(*bits).to_string()),
				Some(value) => {
					let memory = memory
						.ok_or_else(|| wasmtime::Error::msg("main gives a value, and the module exports no memory"))?;
					Ok(match crate::gc_values::json_of(value, &mut store.as_context_mut(), memory)? {
						serde_json::Value::String(text) => text,
						other => other.to_string(),
					})
				}
			}
		}
	}
}
