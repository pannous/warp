//! The canonical-ABI side of a component (`warp build --component`, src/component_builder.rs): for each function its
//! world exports, an adapter `api#add` taking and giving the core types of the WIT signature and converting them to the
//! program's own: s32 widened to its i64, a string (pointer and length in linear memory) a text node, a text result
//! written to a return area as pointer and length. `cabi_realloc` lets the host place arguments in the text heap.
use super::WasmGcEmitter;
use crate::analyzer::param_kind;
use crate::component_worlds::{Signature, WIT_STRING};
use wasm_encoder::*;
use Instruction as I;

/// The core type of a WIT scalar, and whether a narrower integer widens with its sign
const CORE_TYPES: [(&str, ValType, bool); 12] = [
	("s8", ValType::I32, true), ("s16", ValType::I32, true), ("s32", ValType::I32, true), ("u8", ValType::I32, false),
	("u16", ValType::I32, false), ("u32", ValType::I32, false), ("bool", ValType::I32, false), ("char", ValType::I32, false),
	("s64", ValType::I64, true), ("u64", ValType::I64, false), ("f32", ValType::F32, true), ("f64", ValType::F64, true),
];
pub(super) const REALLOC: &str = "cabi_realloc";
/// A text result's return area: its pointer and its length, two i32
const RETURN_AREA_BYTES: i32 = 8;
const RETURN_AREA_ALIGNMENT: i32 = 4;
const LENGTH_OFFSET: u64 = 4;
const WORD: MemArg = MemArg { offset: 0, align: 2, memory_index: 0 };
/// The runtime functions the adapters call
const ADAPTER_NEEDS: [&str; 5] = ["new_text", "new_int", "new_float", "get_int_value", super::text_builtins::TEXT_OF];

/// The runtime functions a component's adapters need, when it has any
pub(super) fn add_dependencies(required: &mut std::collections::HashSet<&'static str>) {
	if !crate::pipeline::component_exports().is_empty() {
		required.extend(ADAPTER_NEEDS);
	}
}

fn core_scalar(wit: &str) -> Option<(ValType, bool)> {
	CORE_TYPES.iter().find(|(scalar, _, _)| *scalar == wit).map(|(_, core, signed)| (*core, *signed))
}

impl WasmGcEmitter {
	/// An exported function emitted after the program's functions, which took their indices without the registry
	fn component_function(&mut self, export: &str, params: Vec<ValType>, results: Vec<ValType>, locals: Vec<ValType>, body: impl FnOnce(&mut Self, &mut Function)) {
		let func_type = self.type_manager.types().len();
		self.type_manager.types_mut().ty().function(params, results);
		let mut func = Function::new(locals.into_iter().map(|local| (1, local)).collect::<Vec<_>>());
		body(self, &mut func);
		func.instruction(&I::End);
		self.functions.function(func_type);
		self.code.function(&func);
		self.exports.export(export, ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	/// The adapters of the component being compiled, after the program's functions
	pub(super) fn emit_component_adapters(&mut self) {
		let exports = crate::pipeline::component_exports();
		if exports.is_empty() {
			return;
		}
		self.emit_text_heap_global();
		self.emit_component_realloc();
		for (interface, function) in exports {
			if let Err(failure) = self.emit_adapter(&interface, &function) {
				self.type_errors.push(failure);
			}
		}
	}

	/// cabi_realloc(old, old_size, align, size): `size` fresh bytes of the text heap, aligned, holding the old ones
	fn emit_component_realloc(&mut self) {
		let int = ValType::I32;
		let (old, old_size, align, size, address, padded) = (0, 1, 2, 3, 4, 5);
		self.component_function(REALLOC, vec![int; 4], vec![int], vec![int; 2], |s, f| {
			Self::emit_list(f, &[I::LocalGet(size), I::LocalGet(align), I::I32Add, I::LocalSet(padded)]);
			s.emit_text_allocation(f, padded, address);
			Self::emit_aligned_up(f, address, I::LocalGet(align));
			Self::emit_list(f, &[
				I::LocalGet(old_size), I::If(BlockType::Empty),
				I::LocalGet(address), I::LocalGet(old), I::LocalGet(old_size), I::MemoryCopy { src_mem: 0, dst_mem: 0 },
				I::End, I::LocalGet(address),
			]);
		});
	}

	/// The address in local `address` rounded up to the alignment `align` pushes: (address + align - 1) & -align
	fn emit_aligned_up(f: &mut Function, address: u32, align: Instruction) {
		Self::emit_list(f, &[I::LocalGet(address), align.clone(), I::I32Add, I::I32Const(1), I::I32Sub, I::I32Const(0), align, I::I32Sub, I::I32And, I::LocalSet(address)]);
	}

	/// `interface#name`, calling the program's function `name`
	fn emit_adapter(&mut self, interface: &str, function: &Signature) -> Result<(), String> {
		let name = &function.name;
		let user_fn = self.ctx.user_functions.get(name).cloned().ok_or_else(|| format!("export {interface}: the program has no `export def {name}`"))?;
		if user_fn.params.len() != function.parameters.len() {
			return Err(format!("{name} takes {} parameters, its interface says {}", user_fn.params.len(), function.parameters.len()));
		}
		let target = user_fn.func_index.ok_or_else(|| format!("{name} was not compiled"))?;
		let mut parameters = vec![];
		for wit in &function.parameters {
			match core_scalar(wit) {
				Some((core, _)) => parameters.push(core),
				None if wit == WIT_STRING => parameters.extend([ValType::I32, ValType::I32]),
				None => return Err(format!("{name}: {wit} cannot cross the component boundary yet")),
			}
		}
		let returns_text = function.result.as_deref() == Some(WIT_STRING);
		let result = match function.result.as_deref() {
			None => vec![],
			Some(WIT_STRING) => vec![ValType::I32],
			Some(wit) => vec![core_scalar(wit).ok_or_else(|| format!("{name}: {wit} cannot cross the component boundary yet"))?.0],
		};
		let target_kinds: Vec<_> = user_fn.params.iter().map(param_kind).collect();
		let result_kind = user_fn.return_kind;
		let first_local = parameters.len() as u32;
		let (text, length, address) = (first_local, first_local + 1, first_local + 2);
		let locals = vec![ValType::Ref(self.node_ref(true)), ValType::I32, ValType::I32];
		let mut failure = None;
		self.component_function(&format!("{interface}#{name}"), parameters, result, locals, |s, f| {
			let mut core = 0;
			for (wit, kind) in function.parameters.iter().zip(&target_kinds) {
				match core_scalar(wit) {
					Some((from, signed)) => {
						f.instruction(&I::LocalGet(core));
						if let Err(problem) = s.emit_scalar_in(f, from, signed, *kind) {
							failure.get_or_insert(format!("{name}: {problem}"));
						}
						core += 1;
					}
					None => {
						Self::emit_list(f, &[I::LocalGet(core), I::LocalGet(core + 1)]);
						s.emit_call(f, "new_text");
						core += 2;
					}
				}
			}
			f.instruction(&I::Call(target));
			match (function.result.as_deref(), returns_text) {
				// a program's function always gives a value
				(None, _) => {
					f.instruction(&I::Drop);
				}
				(Some(_), true) => s.emit_text_out(f, text, length, address),
				(Some(wit), false) => {
					let (to, _) = core_scalar(wit).expect("checked above");
					if let Err(problem) = s.emit_scalar_out(f, result_kind, to) {
						failure.get_or_insert(format!("{name}: {problem}"));
					}
				}
			}
		});
		failure.map_or(Ok(()), Err)
	}

	/// A core scalar on the stack as the program's parameter of `kind`
	fn emit_scalar_in(&mut self, f: &mut Function, from: ValType, signed: bool, kind: crate::type_kinds::Kind) -> Result<(), String> {
		match (from, self.storage_type(kind)) {
			(ValType::I32, ValType::I64) => f.instruction(if signed { &I::I64ExtendI32S } else { &I::I64ExtendI32U }),
			(ValType::I64, ValType::I64) | (ValType::F64, ValType::F64) => f,
			(ValType::F32, ValType::F64) => f.instruction(&I::F64PromoteF32),
			(ValType::I32 | ValType::I64, ValType::Ref(_)) => {
				if from == ValType::I32 {
					f.instruction(if signed { &I::I64ExtendI32S } else { &I::I64ExtendI32U });
				}
				self.emit_call(f, "new_int");
				f
			}
			(ValType::F32 | ValType::F64, ValType::Ref(_)) => {
				if from == ValType::F32 {
					f.instruction(&I::F64PromoteF32);
				}
				self.emit_call(f, "new_float");
				f
			}
			(from, to) => return Err(format!("a {from:?} cannot be its {to:?} parameter")),
		};
		Ok(())
	}

	/// The program's result of `kind` on the stack as the core scalar `to`
	fn emit_scalar_out(&mut self, f: &mut Function, kind: crate::type_kinds::Kind, to: ValType) -> Result<(), String> {
		match (self.storage_type(kind), to) {
			(ValType::I64, ValType::I32) => f.instruction(&I::I32WrapI64),
			(ValType::I64, ValType::I64) | (ValType::F64, ValType::F64) => f,
			(ValType::F64, ValType::F32) => f.instruction(&I::F32DemoteF64),
			(ValType::Ref(_), ValType::I32 | ValType::I64) => {
				self.emit_call(f, "get_int_value");
				if to == ValType::I32 {
					f.instruction(&I::I32WrapI64);
				}
				f
			}
			(from, to) => return Err(format!("its {from:?} result cannot be a {to:?}")),
		};
		Ok(())
	}

	/// The text node on the stack written to a fresh return area as pointer and length: the area's address
	fn emit_text_out(&mut self, f: &mut Function, text: u32, length: u32, address: u32) {
		self.emit_call(f, super::text_builtins::TEXT_OF);
		f.instruction(&I::LocalSet(text));
		Self::emit_list(f, &[I::I32Const(RETURN_AREA_BYTES + RETURN_AREA_ALIGNMENT), I::LocalSet(length)]);
		self.emit_text_allocation(f, length, address);
		Self::emit_aligned_up(f, address, I::I32Const(RETURN_AREA_ALIGNMENT));
		f.instruction(&I::LocalGet(address));
		self.emit_text_field(f, text, 0);
		f.instruction(&I::I32Store(WORD));
		f.instruction(&I::LocalGet(address));
		self.emit_text_field(f, text, 1);
		f.instruction(&I::I32Store(MemArg { offset: LENGTH_OFFSET, ..WORD }));
		f.instruction(&I::LocalGet(address));
	}
}
