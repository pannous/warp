//! Arrays in linear memory (`linear xs = int[n]`, notes/linear_arrays.md): the module's own functions over one
//! contiguous block `[count: i64][n cells of i64 or f64]`, 8-byte aligned, taken from the bump heap texts use too.
//! The program calls them like C functions (their signatures in crate::ffi, library LINEAR_LIBRARY, never imported):
//! an array is the Int of its address, so a host or GPU backend can later take the block as it is, without copying.

use super::WasmGcEmitter;
use crate::node::Node;
use crate::operators::Op;
use wasm_encoder::{Function, Instruction as I, MemArg, ValType};
use ValType::{F64, I32, I64, V128};

/// The library the linear words belong to: emitted by the module, not imported (import_manager skips it)
pub const LINEAR_LIBRARY: &str = "linear";
pub const LINEAR_NEW: &str = "linear_new";
pub const LINEAR_COUNT: &str = "linear_count";
/// get, set, add of an Int cell and of a float cell (the order of shared_arrays' element words)
pub const LINEAR_INT_WORDS: [&str; 3] = ["linear_get", "linear_set", "linear_add"];
pub const LINEAR_FLOAT_WORDS: [&str; 3] = ["linear_getf", "linear_setf", "linear_addf"];
/// linear_dotf(xs, ys) -> the sum of xs#i * ys#i over xs's count of cells, two pairs at a time in f64x2 lanes: `dot` and
/// `sum(xs .* ys)` of two linear float arrays of equal length (shared_arrays.rs checks it)
pub const LINEAR_DOT: &str = "linear_dotf";
/// The registry names crate::wasm_emitter's ffi_func_index looks the words up by
const FUNCTION_NAMES: [(&str, &str); 9] = [
	(LINEAR_NEW, "ffi_linear_new"), (LINEAR_COUNT, "ffi_linear_count"),
	("linear_get", "ffi_linear_get"), ("linear_set", "ffi_linear_set"), ("linear_add", "ffi_linear_add"),
	("linear_getf", "ffi_linear_getf"), ("linear_setf", "ffi_linear_setf"), ("linear_addf", "ffi_linear_addf"),
	(LINEAR_DOT, "ffi_linear_dotf"),
];

/// `ys = xs.map(x => x * 0.5 + 1)` over a linear float array: the kernel `linear_mapf·x·x*0.5+1`, its parameter and
/// body in its name, a new block of the results computed two cells at a time with f64x2 (notes/simd.md)
pub const LINEAR_MAP_PREFIX: &str = "linear_mapf·";
const KERNEL_NAME_SEPARATOR: char = '·';
/// The operators of a float kernel, scalar and two lanes at a time
const KERNEL_BINARY: [(Op, I<'static>, I<'static>); 4] = [
	(Op::Add, I::F64Add, I::F64x2Add), (Op::Sub, I::F64Sub, I::F64x2Sub), (Op::Mul, I::F64Mul, I::F64x2Mul), (Op::Div, I::F64Div, I::F64x2Div),
];
const KERNEL_PREFIX: [(Op, I<'static>, I<'static>); 3] = [
	(Op::Neg, I::F64Neg, I::F64x2Neg), (Op::Sqrt, I::F64Sqrt, I::F64x2Sqrt), (Op::Abs, I::F64Abs, I::F64x2Abs),
];
/// A pair of cells
const PAIR_BYTES: i32 = 16;
const PAIR_ALIGNED: MemArg = MemArg { offset: 0, align: 3, memory_index: 0 };

/// Whether `body` is float arithmetic of the parameter and number literals, which a kernel computes lane by lane
pub fn is_float_kernel(parameter: &str, body: &Node) -> bool {
	match body.drop_meta() {
		Node::Symbol(name) => name == parameter,
		Node::Number(number) => !matches!(number, crate::extensions::numbers::Number::Complex(..)),
		Node::Key(left, op, right) if KERNEL_BINARY.iter().any(|(known, _, _)| known == op) => is_float_kernel(parameter, left) && is_float_kernel(parameter, right),
		Node::Key(empty, op, operand) if matches!(empty.drop_meta(), Node::Empty) && KERNEL_PREFIX.iter().any(|(known, _, _)| known == op) => is_float_kernel(parameter, operand),
		_ => false,
	}
}

/// The kernel's name: its parameter and body after LINEAR_MAP_PREFIX
pub fn kernel_name(parameter: &str, body: &Node) -> String {
	format!("{LINEAR_MAP_PREFIX}{parameter}{KERNEL_NAME_SEPARATOR}{}", body.serialize().trim())
}

/// The parameter and body a kernel's name holds
fn kernel_of(name: &str) -> Option<(String, Node)> {
	let (parameter, body) = name.strip_prefix(LINEAR_MAP_PREFIX)?.split_once(KERNEL_NAME_SEPARATOR)?;
	Some((parameter.to_string(), crate::warp_parser::parse(body)))
}

/// The block's header (its count) and each cell are 8 bytes
const CELL_BYTES: i64 = 8;
const CELL: MemArg = MemArg { offset: 0, align: 3, memory_index: 0 };

/// name, parameters, results of the linear words
pub fn linear_word_signatures() -> [(&'static str, Vec<ValType>, Vec<ValType>); 9] {
	let [get, set, add] = LINEAR_INT_WORDS;
	let [getf, setf, addf] = LINEAR_FLOAT_WORDS;
	[
		(LINEAR_NEW, vec![I64], vec![I64]), (LINEAR_COUNT, vec![I64], vec![I64]),
		(get, vec![I64, I64], vec![I64]), (set, vec![I64, I64, I64], vec![I64]), (add, vec![I64, I64, I64], vec![I64]),
		(getf, vec![I64, I64], vec![F64]), (setf, vec![I64, I64, F64], vec![F64]), (addf, vec![I64, I64, F64], vec![F64]),
		(LINEAR_DOT, vec![I64, I64], vec![F64]),
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
		let mut kernels: Vec<String> = self.ctx.ffi_imports.keys().filter(|name| name.starts_with(LINEAR_MAP_PREFIX)).cloned().collect();
		kernels.sort();
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
				LINEAR_DOT => self.runtime_function(name, params, results, vec![I32, I32, I32, V128, F64], |_, f| Self::emit_float_dot(f)),
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
		for kernel in kernels {
			let Some((parameter, body)) = kernel_of(&kernel) else { continue };
			let name: &'static str = Box::leak(format!("ffi_{kernel}").into_boxed_str());
			self.runtime_function(name, vec![I64], vec![I64], vec![I64, I32, I32, I32, F64, V128], |s, f| s.emit_float_map(f, &parameter, &body));
		}
	}

	/// linear_mapf·x·body(source) -> a new block of body(x) for each cell x: two cells at a time in f64x2 lanes, an odd last
	/// cell alone
	fn emit_float_map(&self, func: &mut Function, parameter: &str, body: &Node) {
		let (source, result, reading, writing, pairs_end, cell, lanes) = (0, 1, 2, 3, 4, 5, 6);
		let new = self.ffi_func_index(LINEAR_NEW).expect("a kernel imports linear_new");
		let count = [I::LocalGet(source), I::I32WrapI64, I::I64Load(CELL)];
		Self::emit_list(func, &count);
		Self::emit_list(func, &[I::Call(new), I::LocalSet(result)]);
		for (block, cursor) in [(source, reading), (result, writing)] {
			Self::emit_list(func, &[I::LocalGet(block), I::I32WrapI64, I::I32Const(CELL_BYTES as i32), I::I32Add, I::LocalSet(cursor)]);
		}
		// the end of the whole pairs: reading + (count & -2) * 8
		func.instruction(&I::LocalGet(reading));
		Self::emit_list(func, &count);
		Self::emit_list(func, &[I::I32WrapI64, I::I32Const(-2), I::I32And, I::I32Const(3), I::I32Shl, I::I32Add, I::LocalSet(pairs_end)]);
		Self::emit_list(func, &[I::Block(wasm_encoder::BlockType::Empty), I::Loop(wasm_encoder::BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(reading), I::LocalGet(pairs_end), I::I32GeU, I::BrIf(1)]);
		Self::emit_list(func, &[I::LocalGet(reading), I::V128Load(PAIR_ALIGNED), I::LocalSet(lanes), I::LocalGet(writing)]);
		emit_kernel(func, parameter, body, lanes, true);
		func.instruction(&I::V128Store(PAIR_ALIGNED));
		for cursor in [reading, writing] {
			Self::emit_list(func, &[I::LocalGet(cursor), I::I32Const(PAIR_BYTES), I::I32Add, I::LocalSet(cursor)]);
		}
		Self::emit_list(func, &[I::Br(0), I::End, I::End]);
		// an odd count: the last cell
		Self::emit_list(func, &count);
		Self::emit_list(func, &[I::I64Const(1), I::I64And, I::I32WrapI64, I::If(wasm_encoder::BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(reading), I::F64Load(CELL), I::LocalSet(cell), I::LocalGet(writing)]);
		emit_kernel(func, parameter, body, cell, false);
		Self::emit_list(func, &[I::F64Store(CELL), I::End, I::LocalGet(result)]);
	}

	/// linear_dotf(xs, ys) -> the sum of the products of their cells: two pairs at a time in f64x2 lanes, an odd last
	/// pair alone
	fn emit_float_dot(func: &mut Function) {
		let (left, right, reading_left, reading_right, pairs_end, lanes, sum) = (0, 1, 2, 3, 4, 5, 6);
		let count = [I::LocalGet(left), I::I32WrapI64, I::I64Load(CELL)];
		for (block, cursor) in [(left, reading_left), (right, reading_right)] {
			Self::emit_list(func, &[I::LocalGet(block), I::I32WrapI64, I::I32Const(CELL_BYTES as i32), I::I32Add, I::LocalSet(cursor)]);
		}
		func.instruction(&I::LocalGet(reading_left));
		Self::emit_list(func, &count);
		Self::emit_list(func, &[I::I32WrapI64, I::I32Const(-2), I::I32And, I::I32Const(3), I::I32Shl, I::I32Add, I::LocalSet(pairs_end)]);
		Self::emit_list(func, &[I::Block(wasm_encoder::BlockType::Empty), I::Loop(wasm_encoder::BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(reading_left), I::LocalGet(pairs_end), I::I32GeU, I::BrIf(1)]);
		Self::emit_list(func, &[I::LocalGet(lanes), I::LocalGet(reading_left), I::V128Load(PAIR_ALIGNED), I::LocalGet(reading_right), I::V128Load(PAIR_ALIGNED), I::F64x2Mul, I::F64x2Add, I::LocalSet(lanes)]);
		for cursor in [reading_left, reading_right] {
			Self::emit_list(func, &[I::LocalGet(cursor), I::I32Const(PAIR_BYTES), I::I32Add, I::LocalSet(cursor)]);
		}
		Self::emit_list(func, &[I::Br(0), I::End, I::End]);
		Self::emit_list(func, &[I::LocalGet(lanes), I::F64x2ExtractLane(0), I::LocalGet(lanes), I::F64x2ExtractLane(1), I::F64Add, I::LocalSet(sum)]);
		// an odd count: the last pair
		Self::emit_list(func, &count);
		Self::emit_list(func, &[I::I64Const(1), I::I64And, I::I32WrapI64, I::If(wasm_encoder::BlockType::Empty)]);
		Self::emit_list(func, &[I::LocalGet(sum), I::LocalGet(reading_left), I::F64Load(CELL), I::LocalGet(reading_right), I::F64Load(CELL), I::F64Mul, I::F64Add, I::LocalSet(sum), I::End, I::LocalGet(sum)]);
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

/// The kernel's body on the parameter in local `value`: an f64, or two in f64x2 lanes
fn emit_kernel(func: &mut Function, parameter: &str, body: &Node, value: u32, lanes: bool) {
	let pick = |(_, scalar, vector): &(Op, I<'static>, I<'static>)| if lanes { vector.clone() } else { scalar.clone() };
	match body.drop_meta() {
		Node::Symbol(name) if name == parameter => { func.instruction(&I::LocalGet(value)); }
		Node::Number(number) => {
			func.instruction(&I::F64Const(f64::from(*number).into()));
			if lanes {
				func.instruction(&I::F64x2Splat);
			}
		}
		Node::Key(empty, op, operand) if matches!(empty.drop_meta(), Node::Empty) && let Some(known) = KERNEL_PREFIX.iter().find(|(known, _, _)| known == op) => {
			emit_kernel(func, parameter, operand, value, lanes);
			func.instruction(&pick(known));
		}
		Node::Key(left, op, right) if let Some(known) = KERNEL_BINARY.iter().find(|(known, _, _)| known == op) => {
			emit_kernel(func, parameter, left, value, lanes);
			emit_kernel(func, parameter, right, value, lanes);
			func.instruction(&pick(known));
		}
		other => unreachable!("is_float_kernel admits no {}", other.serialize()),
	}
}
