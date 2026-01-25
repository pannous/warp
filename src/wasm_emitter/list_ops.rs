//! List and string operation functions for WASM

use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction::I32Const;
use ValType::Ref;

impl WasmGcEmitter {
	/// Emit list and string operation helper functions
	pub(crate) fn emit_list_ops(&mut self) {
		let node_ref = self.node_ref(false);
		let node_ref_nullable = self.node_ref(true);

		// list_at(list: ref $Node, index: i64) -> i64
		// Get the numeric value of the element at index (1-based)
		// Traverses linked list: for index N, follow value pointer N-1 times, return data
		if self.should_emit_function("list_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			// current = list
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop while index > 1: current = current.value, index--
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if index <= 1, break
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64LeS);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// index = index - 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::LocalSet(1));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));
			// Get the inner node's data field (which holds the i64_box)
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field of the element node
			});
			// Cast to ref $i64box and get the i64 value
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.i64_box_type)));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.i64_box_type,
				field_index: 0,
			});

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_at");
			self.exports.export("list_at", ExportKind::Func, idx);
		}

		// list_node_at(list: ref $Node, index: i64) -> ref $Node
		// Get the element node at index (1-based), returns the node itself (for symbol/text lists)
		if self.should_emit_function("list_node_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			// current = list
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop while index > 1: current = current.value, index--
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if index <= 1, break
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64LeS);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// index = index - 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::LocalSet(1));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node and return it directly
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_node_at");
			self.exports.export("list_node_at", ExportKind::Func, idx);
		}

		// node_count(node: ref $Node) -> i64
		// Count the number of elements in a list/block by traversing the value chain
		if self.should_emit_function("node_count") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut().ty().function(vec![Ref(node_ref)], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=node, 1=count, 2=current
			let mut func = Function::new(vec![(1, ValType::I64), (1, Ref(node_ref_nullable))]);

			// count = 0
			func.instruction(&Instruction::I64Const(0));
			func.instruction(&Instruction::LocalSet(1));

			// current = node
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop: while current is not null, count++ and current = current.value
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if current is null, break
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::RefIsNull);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// count = count + 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Add);
			func.instruction(&Instruction::LocalSet(1));

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// return count
			func.instruction(&Instruction::LocalGet(1));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_count");
			self.exports.export("node_count", ExportKind::Func, idx);
		}

		// string_char_at(node: ref $Node, index: i64) -> ref $Node
		// Get the character at index (1-based) from a Text/Symbol node, returns Codepoint node
		if self.should_emit_function("string_char_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.data (which is a ref $String)
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field
			});
			// Cast anyref to ref $String
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.string_type)));

			// Get ptr from $String
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.string_type,
				field_index: 0, // ptr
			});

			// Add (index - 1) to ptr for 1-based indexing
			func.instruction(&Instruction::LocalGet(1)); // index
			func.instruction(&Instruction::I32WrapI64);
			func.instruction(&I32Const(1));
			func.instruction(&Instruction::I32Sub);
			func.instruction(&Instruction::I32Add); // ptr + (index - 1)

			// Load byte from memory at that address
			func.instruction(&Instruction::I32Load8U(MemArg {
				offset: 0,
				align: 0,
				memory_index: 0,
			}));

			// Create Codepoint node with this value
			self.emit_call(&mut func, "new_codepoint");

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("string_char_at");
			self.exports.export("string_char_at", ExportKind::Func, idx);
		}

		// node_index_at(node: ref $Node, index: i64) -> ref $Node
		// Runtime dispatch: for Text/Symbol call string_char_at, for List/Block call list_node_at
		if self.should_emit_function("node_index_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.kind
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0, // kind field
			});

			// Check if kind is Text (3) or Symbol (5)
			// kind == 3 || kind == 5 means it's a string
			func.instruction(&Instruction::I64Const(3)); // Kind::Text
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Text - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Check for Symbol
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0,
			});
			func.instruction(&Instruction::I64Const(5)); // Kind::Symbol
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Symbol - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Otherwise it's a list - call list_node_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "list_node_at");
			func.instruction(&Instruction::End); // end inner if
			func.instruction(&Instruction::End); // end outer if

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_index_at");
			self.exports.export("node_index_at", ExportKind::Func, idx);
		}

		// list_set_at(list: ref $Node, index: i64, value: i64) -> i64
		// Set the numeric value at index (1-based) and return the value
		if self.should_emit_function("list_set_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64, ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=value, 3=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			// current = list
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(3));

			// Loop while index > 1: current = current.value, index--
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if index <= 1, break
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64LeS);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(3));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(3));

			// index = index - 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Sub);
			func.instruction(&Instruction::LocalSet(1));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// Get current.data (which is the wrapper Node for the element)
			func.instruction(&Instruction::LocalGet(3));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field
			});
			// Cast anyref to ref $Node
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));

			// Create new i64box with the value
			func.instruction(&Instruction::LocalGet(2)); // value
			func.instruction(&Instruction::StructNew(self.type_manager.i64_box_type));

			// Set the inner node's data field to the new i64box
			func.instruction(&Instruction::StructSet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field
			});

			// Return the value
			func.instruction(&Instruction::LocalGet(2));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_set_at");
			self.exports.export("list_set_at", ExportKind::Func, idx);
		}

		// string_set_char_at(node: ref $Node, index: i64, value: i64) -> i64
		// Set the character at index (1-based) in a Text/Symbol node, returns the value
		if self.should_emit_function("string_set_char_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64, ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.data (which is a ref $String)
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field
			});
			// Cast anyref to ref $String
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.string_type)));

			// Get ptr from $String
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.string_type,
				field_index: 0, // ptr
			});

			// Calculate address: ptr + (index - 1) for 1-based indexing
			func.instruction(&Instruction::LocalGet(1)); // index
			func.instruction(&Instruction::I32WrapI64);
			func.instruction(&I32Const(1));
			func.instruction(&Instruction::I32Sub);
			func.instruction(&Instruction::I32Add); // address = ptr + (index - 1)

			// Store the value as a byte
			func.instruction(&Instruction::LocalGet(2)); // value
			func.instruction(&Instruction::I32WrapI64);
			func.instruction(&Instruction::I32Store8(MemArg {
				offset: 0,
				align: 0,
				memory_index: 0,
			}));

			// Return the value
			func.instruction(&Instruction::LocalGet(2));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("string_set_char_at");
			self.exports.export("string_set_char_at", ExportKind::Func, idx);
		}

		// node_set_at(node: ref $Node, index: i64, value: i64) -> i64
		// Runtime dispatch for index assignment: string_set_char_at or list_set_at
		if self.should_emit_function("node_set_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64, ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.kind
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0, // kind field
			});

			// Check if kind is Text (3) or Symbol (5)
			func.instruction(&Instruction::I64Const(3)); // Kind::Text
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
			// It's a Text - call string_set_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::LocalGet(2));
			self.emit_call(&mut func, "string_set_char_at");
			func.instruction(&Instruction::Else);
			// Check for Symbol
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0,
			});
			func.instruction(&Instruction::I64Const(5)); // Kind::Symbol
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
			// It's a Symbol - call string_set_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::LocalGet(2));
			self.emit_call(&mut func, "string_set_char_at");
			func.instruction(&Instruction::Else);
			// Otherwise it's a list - call list_set_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::LocalGet(2));
			self.emit_call(&mut func, "list_set_at");
			func.instruction(&Instruction::End); // end inner if
			func.instruction(&Instruction::End); // end outer if

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_set_at");
			self.exports.export("node_set_at", ExportKind::Func, idx);
		}
	}
}
