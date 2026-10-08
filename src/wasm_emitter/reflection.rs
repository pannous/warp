//! Reflection exports: a host without GC field access (JavaScript in the browser, web/playground) reads a result
//! node through these, the way wasm_reader.rs reads it with wasmtime. A payload is the `data` field of a node:
//! `reflect_tag` names its type, the other functions read one field of it.

use crate::type_kinds::any_heap_type;
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

/// `reflect_tag(payload)`: which struct a payload is; the JavaScript reader (web/playground/reader.js) uses the same numbers
pub const PAYLOAD_TAGS: [&str; 10] = ["null", "node", "int", "float", "string", "i31", "big_int", "ratio", "floats", "other"];

impl WasmGcEmitter {
	pub(super) fn emit_reflection(&mut self) {
		let any = Ref(RefType { nullable: true, heap_type: any_heap_type() });
		let node = Ref(self.node_ref(true));
		let tm = &self.type_manager;
		let (node_type, string, int, float, big, ratio, limbs, floats) =
			(tm.node_type, tm.string_type, tm.i64_box_type, tm.f64_box_type, tm.big_int_type, tm.ratio_type, tm.limbs_type, tm.float_array_type);
		let field = |struct_type_index: u32, field_index: u32| I::StructGet { struct_type_index, field_index };
		let cast = |type_index: u32| I::RefCastNonNull(HeapType::Concrete(type_index));

		self.reflect("reflect_data", vec![node], vec![any], &[I::LocalGet(0), field(node_type, 1)]);
		self.reflect("reflect_value", vec![node], vec![node], &[I::LocalGet(0), field(node_type, 2)]);
		self.reflect("reflect_i64", vec![any], vec![ValType::I64], &[I::LocalGet(0), cast(int), field(int, 0)]);
		self.reflect("reflect_f64", vec![any], vec![ValType::F64], &[I::LocalGet(0), cast(float), field(float, 0)]);
		self.reflect("reflect_i31", vec![any], vec![ValType::I32], &[I::LocalGet(0), I::RefCastNonNull(HeapType::I31), I::I31GetU]);
		self.reflect("reflect_text_ptr", vec![any], vec![ValType::I32], &[I::LocalGet(0), cast(string), field(string, 0)]);
		self.reflect("reflect_text_len", vec![any], vec![ValType::I32], &[I::LocalGet(0), cast(string), field(string, 1)]);
		self.reflect("reflect_negative", vec![any], vec![ValType::I32], &[I::LocalGet(0), cast(big), field(big, 0)]);
		self.reflect("reflect_limb_count", vec![any], vec![ValType::I32], &[I::LocalGet(0), cast(big), field(big, 1), I::ArrayLen]);
		self.reflect("reflect_limb", vec![any, ValType::I32], vec![ValType::I32], &[I::LocalGet(0), cast(big), field(big, 1), I::LocalGet(1), I::ArrayGet(limbs)]);
		self.reflect("reflect_numerator", vec![any], vec![any], &[I::LocalGet(0), cast(ratio), field(ratio, 0)]);
		self.reflect("reflect_denominator", vec![any], vec![any], &[I::LocalGet(0), cast(ratio), field(ratio, 1)]);

		// the parts of a ± value (Kind::Uncertain), only in programs that make one
		let reads_floats = self.should_emit_function(super::uncertain::UNCERTAIN_NEW);
		if reads_floats {
			self.reflect("reflect_float_count", vec![any], vec![ValType::I32], &[I::LocalGet(0), cast(floats), I::ArrayLen]);
			self.reflect("reflect_float", vec![any, ValType::I32], vec![ValType::F64], &[I::LocalGet(0), cast(floats), I::LocalGet(1), I::ArrayGet(floats)]);
		}

		// reflect_tag: the index in PAYLOAD_TAGS of the first type the payload is
		let mut tag = vec![I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty), I::I32Const(0), I::Return, I::End];
		let mut tested = vec![HeapType::Concrete(node_type), HeapType::Concrete(int), HeapType::Concrete(float), HeapType::Concrete(string),
			HeapType::I31, HeapType::Concrete(big), HeapType::Concrete(ratio)];
		if reads_floats {
			tested.push(HeapType::Concrete(floats));
		}
		for (index, heap_type) in tested.into_iter().enumerate() {
			tag.extend([I::LocalGet(0), I::RefTestNonNull(heap_type), I::If(BlockType::Empty), I::I32Const(index as i32 + 1), I::Return, I::End]);
		}
		tag.push(I::I32Const(PAYLOAD_TAGS.len() as i32 - 1));
		self.reflect("reflect_tag", vec![any], vec![ValType::I32], &tag);
	}

	fn reflect(&mut self, name: &'static str, params: Vec<ValType>, results: Vec<ValType>, body: &[Instruction]) {
		self.runtime_function(name, params, results, vec![], |_, f| Self::emit_list(f, body));
		self.exports.export(name, ExportKind::Func, self.func_index(name));
	}
}
