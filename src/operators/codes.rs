//! Operators as numbers: the codes Key nodes carry in their kind bits through the WASM GC Node, and back
use super::*;

/// Every operator by its code in the kind field of a Key node (`(code << 8) | Kind::Key`); the first five are the
/// codes earlier modules stored, the rest follow, so a quoted expression (`data 1+2`) reads back with its operator
pub const OP_CODES: [Op; 66] = [
	Op::None,
	Op::Colon,
	Op::Assign,
	Op::Define,
	Op::Dot,
	Op::SafeDot,
	Op::Scope,
	Op::Arrow,
	Op::Add,
	Op::Sub,
	Op::Mul,
	Op::Div,
	Op::Mod,
	Op::Rem,
	Op::Pow,
	Op::Shl,
	Op::Shr,
	Op::AddAssign,
	Op::SubAssign,
	Op::MulAssign,
	Op::DivAssign,
	Op::ModAssign,
	Op::PowAssign,
	Op::AndAssign,
	Op::OrAssign,
	Op::XorAssign,
	Op::Lt,
	Op::Gt,
	Op::Le,
	Op::Ge,
	Op::Eq,
	Op::Ne,
	Op::Similar,
	Op::And,
	Op::Or,
	Op::Xor,
	Op::Not,
	Op::Neg,
	Op::Sqrt,
	Op::Cbrt,
	Op::Abs,
	Op::Inc,
	Op::Dec,
	Op::Question,
	Op::If,
	Op::Then,
	Op::Else,
	Op::While,
	Op::Do,
	Op::Hash,
	Op::Range,
	Op::To,
	Op::As,
	Op::PlusMinus,
	Op::Coalesce,
	Op::Identical,
	Op::NotIdentical,
	Op::Rough,
	Op::LogBase,
	Op::LogOf,
	Op::BitAnd,
	Op::BitOr,
	Op::ArrowUp,
	Op::ArrowDown,
	Op::FloorDiv,
	Op::FloorDivAssign,
];

/// The bits of the operation codes (Op::is_operation), codes 0..64 then 64..128, for a run-time test of a Key's code
pub fn operation_code_masks() -> [i64; 2] {
	let mut masks = [0i64; 2];
	for (code, _) in OP_CODES.iter().enumerate().filter(|(_, op)| op.is_operation()) {
		masks[code / 64] |= 1 << (code % 64);
	}
	masks
}

/// Encode Op as i64 for storage in kind field
pub fn op_to_code(op: &Op) -> i64 {
	OP_CODES.iter().position(|known| known == op).unwrap_or(0) as i64
}

/// The operator written `text` (Op::as_str's inverse): `*` is Mul
pub fn op_named(text: &str) -> Option<Op> {
	OP_CODES.iter().find(|op| op.as_str() == text).copied()
}

/// Decode i64 back to Op
pub fn code_to_op(code: i64) -> Op {
	usize::try_from(code).ok().and_then(|index| OP_CODES.get(index)).copied().unwrap_or(Op::None)
}

/// What Node::serialize writes between an entry's key and value, by operator code: `x+1`, `0.1 as float`, nothing for
/// an instance `p{x:1}` (Op::None); a code no operator has reads as `:`
pub fn written_operator(code: usize) -> String {
	match OP_CODES.get(code).map(Op::as_str) {
		None => Op::Colon.as_str().to_string(),
		Some(word) if word.starts_with(char::is_alphabetic) => format!(" {word} "),
		Some(symbol) => symbol.to_string(),
	}
}
