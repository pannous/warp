//! Arrays in linear memory (`linear xs = int[n]`, notes/linear_arrays.md): the module's own functions over one
//! contiguous block `[count: i64][n cells of i64 or f64]`, 8-byte aligned, taken from the bump heap texts use too.
//! The program calls them like C functions (their signatures in crate::ffi, library LINEAR_LIBRARY, never imported):
//! an array is the Int of its address, so a host or GPU backend can later take the block as it is, without copying.

use super::WasmGcEmitter;
use wasm_encoder::{Function, Instruction as I, MemArg, ValType};
use ValType::{F64, I32, I64};

/// The library the linear words belong to: emitted by the module, not imported (import_manager skips it)
pub const LINEAR_LIBRARY: &str = "linear";
pub const LINEAR_NEW: &str = "linear_new";
pub const LINEAR_COUNT: &str = "linear_count";
/// get, set, add of an Int cell and of a float cell (the order of shared_arrays' element words)
pub const LINEAR_INT_WORDS: [&str; 3] = ["linear_get", "linear_set", "linear_add"];
pub const LINEAR_FLOAT_WORDS: [&str; 3] = ["linear_getf", "linear_setf", "linear_addf"];
/// The registry names crate::wasm_emitter's ffi_func_index looks the words up by
const FUNCTION_NAMES: [(&str, &str); 8] = [
	(LINEAR_NEW, "ffi_linear_new"), (LINEAR_COUNT, "ffi_linear_count"),
	("linear_get", "ffi_linear_get"), ("linear_set", "ffi_linear_set"), ("linear_add", "ffi_linear_add"),
	("linear_getf", "ffi_linear_getf"), ("linear_setf", "ffi_linear_setf"), ("linear_addf", "ffi_linear_addf"),
];

/// The block's header (its count) and each cell are 8 bytes
const CELL_BYTES: i64 = 8;
const CELL: MemArg = MemArg { offset: 0, align: 3, memory_index: 0 };

/// name, parameters, results of the linear words
pub fn linear_word_signatures() -> [(&'static str, Vec<ValType>, Vec<ValType>); 8] {
	let [get, set, add] = LINEAR_INT_WORDS;
	let [getf, setf, addf] = LINEAR_FLOAT_WORDS;
	[
		(LINEAR_NEW, vec![I64], vec![I64]), (LINEAR_COUNT, vec![I64], vec![I64]),
		(get, vec![I64, I64], vec![I64]), (set, vec![I64, I64, I64], vec![I64]), (add, vec![I64, I64, I64], vec![I64]),
		(getf, vec![I64, I64], vec![F64]), (setf, vec![I64, I64, F64], vec![F64]), (addf, vec![I64, I64, F64], vec![F64]),
	]
}

pub fn is_linear_word(name: &str) -> bool {
	FUNCTION_NAMES.iter().any(|(word, _)| *word == name)
}

impl WasmGcEmitter {
	/// The linear words the program calls
	pub(super) fn emit_linear_arrays(&mut self) {
		let called: Vec<_> = linear_word_signatures().into_iter().filter(|(word, _, _)| self.ctx.ffi_imports.contains_key(*word)).collect();
		if called.is_empty() {
			return;
		}
		self.emit_text_heap_global();
		for (word, params, results) in called {
			let name = FUNCTION_NAMES.iter().find(|(known, _)| *known == word).expect("every linear word is named").1;
			let float = results == [F64];
			match word {
				LINEAR_NEW => self.runtime_function(name, params, results, vec![I32, I32], |s, f| s.emit_linear_new(f)),
				LINEAR_COUNT => self.runtime_function(name, params, results, vec![], |_, f| {
					f.instruction(&I::LocalGet(0));
					f.instruction(&I::I32WrapI64);
					f.instruction(&I::I64Load(CELL));
				}),
				_ if word == LINEAR_INT_WORDS[0] || word == LINEAR_FLOAT_WORDS[0] => self.runtime_function(name, params, results, vec![], |s, f| {
					s.emit_linear_cell(f);
					f.instruction(&if float { I::F64Load(CELL) } else { I::I64Load(CELL) });
				}),
				_ => {
					let adds = word == LINEAR_INT_WORDS[2] || word == LINEAR_FLOAT_WORDS[2];
					self.runtime_function(name, params, results, vec![I32], |s, f| s.emit_linear_store(f, float, adds))
				}
			};
		}
	}

	/// linear_new(count) -> address: a zeroed block (fresh pages, never reused) with its count in the header
	fn emit_linear_new(&self, func: &mut Function) {
		let (length, address) = (1, 2);
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::I64Const(0));
		func.instruction(&I::I64LtS);
		self.emit_fail_if(func, "index_out_of_range");
		// header + cells + room to align the start to 8
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::I64Const(CELL_BYTES));
		func.instruction(&I::I64Mul);
		func.instruction(&I::I64Const(2 * CELL_BYTES - 1));
		func.instruction(&I::I64Add);
		func.instruction(&I::I32WrapI64);
		func.instruction(&I::LocalSet(length));
		self.emit_text_allocation(func, length, address);
		func.instruction(&I::LocalGet(address));
		func.instruction(&I::I32Const(CELL_BYTES as i32 - 1));
		func.instruction(&I::I32Add);
		func.instruction(&I::I32Const(-(CELL_BYTES as i32)));
		func.instruction(&I::I32And);
		func.instruction(&I::LocalTee(address));
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::I64Store(CELL));
		func.instruction(&I::LocalGet(address));
		func.instruction(&I::I64ExtendI32U);
	}

	/// The i32 address of cell `index` (param 1, from 1) of the block at param 0; index_out_of_range outside 1..count
	fn emit_linear_cell(&self, func: &mut Function) {
		func.instruction(&I::LocalGet(1));
		func.instruction(&I::I64Const(1));
		func.instruction(&I::I64Sub);
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::I32WrapI64);
		func.instruction(&I::I64Load(CELL));
		func.instruction(&I::I64GeU);
		self.emit_fail_if(func, "index_out_of_range");
		func.instruction(&I::LocalGet(0));
		func.instruction(&I::LocalGet(1));
		func.instruction(&I::I64Const(CELL_BYTES));
		func.instruction(&I::I64Mul);
		func.instruction(&I::I64Add);
		func.instruction(&I::I32WrapI64);
	}

	/// linear_set(address, index, value) -> value; linear_add(address, index, value) -> the cell's new value
	fn emit_linear_store(&self, func: &mut Function, float: bool, adds: bool) {
		let cell = 3;
		self.emit_linear_cell(func);
		func.instruction(&I::LocalSet(cell));
		func.instruction(&I::LocalGet(cell));
		if adds {
			func.instruction(&I::LocalGet(cell));
			func.instruction(&if float { I::F64Load(CELL) } else { I::I64Load(CELL) });
			func.instruction(&I::LocalGet(2));
			func.instruction(&if float { I::F64Add } else { I::I64Add });
			func.instruction(&I::LocalSet(2));
		}
		func.instruction(&I::LocalGet(2));
		func.instruction(&if float { I::F64Store(CELL) } else { I::I64Store(CELL) });
		func.instruction(&I::LocalGet(2));
	}
}
