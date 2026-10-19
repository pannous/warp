//! A function whose body is one wasm instruction (decision wasm-instruction-body): `floor(a: float): float := wasm
//! f64.floor` compiles to its parameters pushed in order and the instruction. The instructions on ints (i64) and floats
//! (f64), the storage kinds of warp's int and float parameters and results.

use crate::node::Node;
use crate::operators::Op;
use crate::words::WASM_INSTRUCTION_CALL;
use wasm_encoder::Instruction as I;

/// The instruction a definition's body names, through what lowering adds for its result type: `… as float`, or
/// `{the result = …; the result}` of an int
pub fn wasm_body_instruction(body: &Node) -> Option<&str> {
	match body.drop_meta() {
		Node::Key(value, Op::As, _) => wasm_body_instruction(value),
		Node::List(items, ..) if items.len() == 1 => wasm_body_instruction(&items[0]),
		Node::List(items, ..) if items.len() == 2 && items[0].drop_meta() == &Node::Symbol(WASM_INSTRUCTION_CALL.into()) => {
			match items[1].drop_meta() {
				Node::Text(name) => Some(name),
				_ => None,
			}
		}
		Node::List(items, ..) if items.len() == 2 => match items[0].drop_meta() {
			Node::Key(target, Op::Assign, value) if target.drop_meta() == items[1].drop_meta() => wasm_body_instruction(value),
			_ => None,
		},
		_ => None,
	}
}

/// The instruction of its text name, e.g. "f64.sqrt"
pub fn instruction(name: &str) -> Option<I<'static>> {
	Some(match name {
		"f64.abs" => I::F64Abs,
		"f64.neg" => I::F64Neg,
		"f64.ceil" => I::F64Ceil,
		"f64.floor" => I::F64Floor,
		"f64.trunc" => I::F64Trunc,
		"f64.nearest" => I::F64Nearest,
		"f64.sqrt" => I::F64Sqrt,
		"f64.add" => I::F64Add,
		"f64.sub" => I::F64Sub,
		"f64.mul" => I::F64Mul,
		"f64.div" => I::F64Div,
		"f64.min" => I::F64Min,
		"f64.max" => I::F64Max,
		"f64.copysign" => I::F64Copysign,
		"f64.convert_i64_s" => I::F64ConvertI64S,
		"f64.convert_i64_u" => I::F64ConvertI64U,
		"f64.reinterpret_i64" => I::F64ReinterpretI64,
		"i64.add" => I::I64Add,
		"i64.sub" => I::I64Sub,
		"i64.mul" => I::I64Mul,
		"i64.div_s" => I::I64DivS,
		"i64.div_u" => I::I64DivU,
		"i64.rem_s" => I::I64RemS,
		"i64.rem_u" => I::I64RemU,
		"i64.and" => I::I64And,
		"i64.or" => I::I64Or,
		"i64.xor" => I::I64Xor,
		"i64.shl" => I::I64Shl,
		"i64.shr_s" => I::I64ShrS,
		"i64.shr_u" => I::I64ShrU,
		"i64.rotl" => I::I64Rotl,
		"i64.rotr" => I::I64Rotr,
		"i64.clz" => I::I64Clz,
		"i64.ctz" => I::I64Ctz,
		"i64.popcnt" => I::I64Popcnt,
		"i64.trunc_f64_s" => I::I64TruncF64S,
		"i64.trunc_f64_u" => I::I64TruncF64U,
		"i64.reinterpret_f64" => I::I64ReinterpretF64,
		_ => return None,
	})
}
