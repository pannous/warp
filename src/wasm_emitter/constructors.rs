//! Constructor generation using macros

/// Generates WASM constructor functions for Node types
///
/// This macro consolidates the repetitive pattern of creating constructor functions
/// Each constructor:
/// 1. Checks if it should be emitted (tree-shaking)
/// 2. Creates a function type
/// 3. Registers the function
/// 4. Emits the Kind tag
/// 5. Creates the Node struct with appropriate data/value fields
/// 6. Exports the function
#[macro_export]
macro_rules! emit_constructor {
	// Pattern 1: Empty (no params, null data/value)
	($self:expr, $name:literal, $kind:expr, empty) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut().ty().function(vec![], vec![Ref(node_ref)]);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::RefNull(any_heap_type()));
			func.instruction(&Instruction::RefNull(HeapType::Concrete($self.type_manager.node_type)));
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};

	// Pattern 2: Boxed i64 (box in i64box struct)
	($self:expr, $name:literal, $kind:expr, boxed_i64) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut().ty().function(vec![ValType::I64], vec![Ref(node_ref)]);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructNew($self.type_manager.i64_box_type));
			func.instruction(&Instruction::RefNull(HeapType::Concrete($self.type_manager.node_type)));
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};

	// Pattern 3: Boxed f64 (box in f64box struct)
	($self:expr, $name:literal, $kind:expr, boxed_f64) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut().ty().function(vec![ValType::F64], vec![Ref(node_ref)]);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructNew($self.type_manager.f64_box_type));
			func.instruction(&Instruction::RefNull(HeapType::Concrete($self.type_manager.node_type)));
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};

	// Pattern 4: i31ref codepoint
	($self:expr, $name:literal, $kind:expr, i31ref) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut().ty().function(vec![ValType::I32], vec![Ref(node_ref)]);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::RefI31);
			func.instruction(&Instruction::RefNull(HeapType::Concrete($self.type_manager.node_type)));
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};

	// Pattern 5: String struct (text, symbol) - takes ptr and len
	($self:expr, $name:literal, $kind:expr, string_struct) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut()
				.ty()
				.function(vec![ValType::I32, ValType::I32], vec![Ref(node_ref)]);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::LocalGet(0)); // ptr
			func.instruction(&Instruction::LocalGet(1)); // len
			func.instruction(&Instruction::StructNew($self.type_manager.string_type));
			func.instruction(&Instruction::RefNull(HeapType::Concrete($self.type_manager.node_type)));
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};

	// Pattern 6: Two Node refs (for type: name, body)
	($self:expr, $name:literal, $kind:expr, two_nodes) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), Ref(node_ref)], vec![Ref(node_ref)]);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::LocalGet(0)); // name as data
			func.instruction(&Instruction::LocalGet(1)); // body as value
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};

	// Pattern 7: Key (Node, Node, i64 op_info) - kind includes op encoding
	($self:expr, $name:literal, $kind:expr, key_with_op) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut().ty().function(
				vec![Ref(node_ref), Ref(node_ref), ValType::I64],
				vec![Ref(node_ref)],
			);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			// kind = (op_info << 8) | Kind::Key
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::I64Const(8));
			func.instruction(&Instruction::I64Shl);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::I64Or);
			func.instruction(&Instruction::LocalGet(0)); // key as data
			func.instruction(&Instruction::LocalGet(1)); // value as value
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};

	// Pattern 8: List (nullable Node, nullable Node, i64 bracket_info)
	($self:expr, $name:literal, $kind:expr, list_with_bracket) => {{
		if $self.should_emit_function($name) {
			let node_ref = $self.node_ref(false);
			let node_ref_nullable = $self.node_ref(true);
			let func_type = $self.type_manager.types().len();
			$self.type_manager.types_mut().ty().function(
				vec![Ref(node_ref_nullable), Ref(node_ref_nullable), ValType::I64],
				vec![Ref(node_ref)],
			);
			$self.functions.function(func_type);
			let mut func = Function::new(vec![]);
			// kind = (bracket_info << 8) | Kind::List
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::I64Const(8));
			func.instruction(&Instruction::I64Shl);
			$self.emit_kind(&mut func, $kind);
			func.instruction(&Instruction::I64Or);
			func.instruction(&Instruction::LocalGet(0)); // first as data
			func.instruction(&Instruction::LocalGet(1)); // rest as value
			func.instruction(&Instruction::StructNew($self.type_manager.node_type));
			func.instruction(&Instruction::End);
			$self.code.function(&func);
			let idx = $self.register_func($name);
			$self.exports.export($name, ExportKind::Func, idx);
		}
	}};
}

/// Emit all basic Node constructors using the macro
pub fn emit_all_constructors(emitter: &mut crate::wasm_emitter::WasmGcEmitter) {
	use crate::type_kinds::{any_heap_type, Kind};
	use wasm_encoder::*;
	use ValType::Ref;

	// Simple constructors
	emit_constructor!(emitter, "new_empty", Kind::Empty, empty);
	emit_constructor!(emitter, "new_int", Kind::Int, boxed_i64);
	emit_constructor!(emitter, "new_float", Kind::Float, boxed_f64);
	emit_constructor!(emitter, "new_codepoint", Kind::Codepoint, i31ref);
	emit_constructor!(emitter, "new_text", Kind::Text, string_struct);
	emit_constructor!(emitter, "new_symbol", Kind::Symbol, string_struct);
	emit_constructor!(emitter, "new_key", Kind::Key, key_with_op);
	emit_constructor!(emitter, "new_type", Kind::TypeDef, two_nodes);
	emit_constructor!(emitter, "new_list", Kind::List, list_with_bracket);
}
