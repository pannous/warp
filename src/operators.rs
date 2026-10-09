use std::fmt;
use serde::{Deserialize, Serialize};

/// Keywords that introduce function definitions
pub const FUNCTION_KEYWORDS: [&str; 6] = ["fun", "fn", "def", "define", "function", "func"];
/// Right binding power of `as`: higher than every infix operator, so the target type is a single atom
pub const TYPE_OPERAND_BP: u8 = 250;
/// Left binding power of the suffix operators ² ³ ++ --: below `.` (180) and `#` (170), above `^` (160)
const SUFFIX_BP: u8 = 165;
/// Right binding power of the prefix operators √ ∛ abs: between ^ (160) and the suffix operators (SUFFIX_BP)
const PREFIX_BP: u8 = 162;

/// Unicode spellings of operators (wiki/alias.md): glyph, operator and the canonical spelling the style hints suggest.
/// One table for the lexer and the hints; the dashes 0x2010..0x2015 and the minus sign 0x2212 all mean `-`.
pub const GLYPH_OPERATORS: [(char, Op, &str); 17] = [
	('∧', Op::And, "and"), ('⋀', Op::And, "and"),
	('∨', Op::Or, "or"), ('⋁', Op::Or, "or"),
	('⊻', Op::Xor, "xor"),
	// the unit product of a printed quantity (`6 m·kg`); generated names use `·` only after parsing
	('·', Op::Mul, "*"), ('⋅', Op::Mul, "*"),
	('≟', Op::Eq, "=="), ('≡', Op::Eq, "=="), ('﹦', Op::Eq, "=="),
	('‐', Op::Sub, "-"), ('‑', Op::Sub, "-"), ('‒', Op::Sub, "-"), ('–', Op::Sub, "-"), ('—', Op::Sub, "-"), ('―', Op::Sub, "-"), ('−', Op::Sub, "-"),
];

pub fn glyph_operator(glyph: char) -> Option<(Op, &'static str)> {
	GLYPH_OPERATORS.iter().find(|(known, _, _)| *known == glyph).map(|(_, op, canonical)| (*op, *canonical))
}

pub fn is_function_keyword(s: &str) -> bool {
	FUNCTION_KEYWORDS.contains(&s)
}

// node[i]

// Warp ABI GC Node representation design:
// This is a single struct that can represent any node type

// todo move node layout to warp_abi.rs
// todo ... any change to node layout must be reflected in wasm_gc_reader.rs warp_abi.md ...

/* restructure the whole emitter emit_node_instructions serialization to use
(type $Node (struct
	(field $kind i64)
	(field $data anyref)
	(field $value (ref null $$Node))
))
**/

/// Operator for Key nodes - distinguishes different binding operations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Op {
	// Structural operators (existing)
	Colon,    // :   type annotation and object construction person:{name:"Joe" age:42}
	Dot,      // .   member access
	SafeDot,  // ?.  member access that is ø when the receiver is ø
	Coalesce, // ??  the left value unless it is ø, then the right (Swift, C#, JS)
	Scope,    // ::  scope resolution
	Define,   // :=  definition
	Assign,   // =   assignment
	Arrow,    // ->  arrow/return type
	FatArrow, // =>  fat arrow/lambda

	// Arithmetic operators
	Add,  // +
	Sub,  // -
	Mul,  // *  ×  ⋅
	Div,  // /  ÷
	Mod,  // %  mod  Euclidean: 0 ≤ a % b < |b|
	Rem,  // rem  truncated remainder, sign of the dividend (C, Java, JS, Rust %)
	Pow,  // ^  **
	LogBase, // ⌞  b⌞x: log base b of x (P226)
	LogOf,   // ⌟  x⌟: the natural log of x, x⌟b: log base b of x (P226)
	Shl,  // <<  shift left: a * 2^n
	Shr,  // >>  shift right: floor(a / 2^n)

	// Compound assignment operators (x op= y → x = x op y)
	AddAssign, // +=
	SubAssign, // -=
	MulAssign, // *=
	DivAssign, // /=
	ModAssign, // %=
	PowAssign, // ^=  **=
	AndAssign, // &&=  and=
	OrAssign,  // ||=  or=
	XorAssign, // ^=   xor=

	// Comparison operators
	Lt,  // <
	Gt,  // >
	Le,  // <=  ≤
	Ge,  // >=  ≥
	Eq,  // ==
	Ne,  // !=  ≠
	Identical,    // ===  equal and of the same type: 0 === false is false (card zero-false)
	NotIdentical, // !==
	Similar, // ≈  ⋍  circa  approximately: equal within the relative `tolerance` (default 1e-9)
	Rough,   // ~  ~~: looser than ≈, within `rough_tolerance` (default 1%), texts without surrounding punctuation (P211)

	// Logical operators
	And, // and  &&  ∧
	Or,  // or   ||  ⋁
	Xor, // xor  ⊻
	Not, // not  !  ¬

	// Prefix operators (unary)
	Neg,  // - (unary minus)
	Sqrt, // √
	Cbrt, // ∛
	Abs,  // ‖...‖

	// Suffix operators
	Inc,    // ++
	Dec,    // --
	Square, // ²
	Cube,   // ³

	// Ternary
	Question, // ? (ternary condition)

	// Conditional
	If,   // if
	Then, // then
	Else, // else

	// Loop
	While, // while
	Do,    // do (used with while)

	// Index/Range
	Hash,  // #  (1-based index)
	Range, // ..
	To,    // to  ...  …

	// Type conversion
	As, // as  (type cast)

	PlusMinus, // ±  +-  a value with tolerance

	// User(Node), allow user defined operators, name==symbol

	None, // implicit/unknown
}

impl Op {
	/// Binding power: (left_bp, right_bp)
	/// Higher = tighter binding. Right > left means right-associative.
	/// Suffix operators: (left_bp, 0) - only binds to left
	/// Prefix operators: (0, right_bp) - only binds to right
	pub const fn binding_power(&self) -> (u8, u8) {
		match self {
			// Suffix operators (no right operand), below member access and indexing: `xs#1++`, `bags#1.n++` change the
			// whole place and `p.x²` squares it, not its index or field name
			Op::Square | Op::Cube | Op::Inc | Op::Dec => (SUFFIX_BP, 0),

			// Member access (tightest infix)
			Op::Dot | Op::SafeDot => (180, 181),
			Op::Scope => (175, 176),
			// index operator #: its index is an atom, so `w#1.upper()` is `(w#1).upper()`
			Op::Hash => (170, 182),

			// Power (right-assoc: 2^3^4 = 2^(3^4))
			Op::Pow => (160, 159),
			Op::LogBase | Op::LogOf => (160, 161),


			// Multiplicative (left-assoc)
			Op::Mul | Op::Div | Op::Mod | Op::Rem => (150, 151),

			// Shifts sit between additive and range: 1 << 2 + 3 → 1 << (2+3)
			Op::Shl | Op::Shr => (135, 136),

			// Additive (left-assoc)
			Op::Add | Op::Sub | Op::PlusMinus => (140, 141),

			// Range
			Op::Range | Op::To => (130, 131),

			// Type conversion converts the whole arithmetic expression to its left (C#, TypeScript), not the nearest
			// operand (Rust, Kotlin): 2 * 1.5 as int → (2*1.5) as int → 3, 1.5 as int == 1; ungrouped mixes are linted
			// the target type is one atom: 0.1 as float + 0.2 → (0.1 as float) + 0.2, never 0.1 as (float + 0.2)
			Op::As => (125, TYPE_OPERAND_BP),

			// Ordering: one level, the parser chains them: a<b<c → a<b and b<c
			Op::Lt | Op::Gt | Op::Le | Op::Ge => (120, 121),

			// Equality binds weaker and never chains: a<b == c<d compares the two results, a==b==c is ambiguous
			Op::Eq | Op::Ne | Op::Identical | Op::NotIdentical | Op::Similar | Op::Rough => (115, 116),

			// Logical not binds weaker than comparison: not a==b → not (a==b)
			Op::Not => (0, 105),

			// Logical (left-assoc with and > or)
			Op::And => (100, 101),
			Op::Xor => (95, 96),
			Op::Or => (90, 91),
			Op::Coalesce => (93, 92), // right-assoc: a ?? b ?? c → a ?? (b ?? c)

			// Ternary: ? needs lower right bp so : can bind within
			Op::Question => (85, 79), // right-assoc, lower than Colon's left bp (80)

			// Conditional: if-then-else has similar precedence to ternary
			Op::If => (0, 78),    // prefix: if binds condition until then
			Op::Then => (77, 76), // then binds until else
			Op::Else => (75, 74), // else binds the rest

			// Loop: while condition do body
			Op::While => (0, 78), // prefix: while binds condition until do/block
			Op::Do => (77, 10),   // do binds very loosely to capture whole body including assignments

			// Structural/Key operators (existing, adjusted for consistency)
			Op::Colon => (80, 81),    // type annotation: a:b:c → a:(b:c)
			Op::Arrow => (70, 69),    // right-assoc: a->b->c → a->(b->c)
			Op::FatArrow => (70, 58), // right-assoc: a => b; the body takes an assignment as in JS: x => total += x
			Op::Define => (60, 59),   // right-assoc: a:=b:=c → a:=(b:=c)
			Op::Assign => (60, 59),   // right-assoc: a=b=c → a=(b=c)

			// Compound assignment (same precedence as assignment)
			Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign |
			Op::ModAssign | Op::PowAssign | Op::AndAssign | Op::OrAssign |
			Op::XorAssign => (60, 59),

			// Prefix operators (no left operand, binds to right): looser than a suffix power (√x² is √(x²)) and member
			// access (√p.x is √(p.x)), tighter than ^ (√x^2 is (√x)^2)
			Op::Sqrt | Op::Cbrt | Op::Abs => (0, PREFIX_BP),
			// Unary minus binds weaker than power: -2^2 → -(2^2)
			Op::Neg => (0, 155),

			Op::None => (0, 0),
		}
	}

	/// The string representation of this operator
	pub fn as_str(&self) -> &'static str {
		match self {
			// Structural
			Op::Colon => ":",
			Op::Dot => ".",
			Op::SafeDot => "?.",
			Op::Coalesce => "??",
			Op::Scope => "::",
			Op::Define => ":=",
			Op::Assign => "=",
			Op::Arrow => "->",
			Op::FatArrow => "=>",

			// Arithmetic
			Op::Add => "+",
			Op::Sub => "-",
			Op::PlusMinus => "±",
			Op::Mul => "*",
			Op::Div => "/",
			Op::Mod => "%",
			Op::Rem => "rem",
			Op::Pow => "^",
			Op::LogBase => "⌞",
			Op::LogOf => "⌟",
			Op::Shl => "<<",
			Op::Shr => ">>",

			// Compound assignment
			Op::AddAssign => "+=",
			Op::SubAssign => "-=",
			Op::MulAssign => "*=",
			Op::DivAssign => "/=",
			Op::ModAssign => "%=",
			Op::PowAssign => "^=",
			Op::AndAssign => "&&=",
			Op::OrAssign => "||=",
			Op::XorAssign => "^^=",

			// Comparison
			Op::Lt => "<",
			Op::Gt => ">",
			Op::Le => "<=",
			Op::Ge => ">=",
			Op::Eq => "==",
			Op::Ne => "!=",
			Op::Identical => "===",
			Op::NotIdentical => "!==",
			Op::Similar => "≈",
			Op::Rough => "~",

			// Logical
			Op::And => "and",
			Op::Or => "or",
			Op::Xor => "xor",
			Op::Not => "not",

			// Prefix
			Op::Neg => "-",
			Op::Sqrt => "√",
			Op::Cbrt => "∛",
			Op::Abs => "‖",

			// Suffix
			Op::Inc => "++",
			Op::Dec => "--",
			Op::Square => "²",
			Op::Cube => "³",

			// Ternary/Index/Range
			Op::Question => "?",
			Op::Hash => "#",
			Op::Range => "..",
			Op::To => "to",

			// Type conversion
			Op::As => "as",

			// Conditional
			Op::If => "if",
			Op::Then => "then",
			Op::Else => "else",

			// Loop
			Op::While => "while",
			Op::Do => "do",

			Op::None => "",
		}
	}

	/// Check if this is a prefix-only operator
	pub fn is_prefix(&self) -> bool {
		matches!(self, Op::Neg | Op::Not | Op::Sqrt | Op::Cbrt | Op::Abs)
	}

	/// Check if this is a suffix-only operator
	pub fn is_suffix(&self) -> bool {
		matches!(self, Op::Inc | Op::Dec | Op::Square | Op::Cube)
	}

	/// The exponent a suffix power operator stands for: x² is x^2, x³ is x^3
	pub fn suffix_exponent(&self) -> Option<i64> {
		match self {
			Op::Square => Some(2),
			Op::Cube => Some(3),
			_ => None,
		}
	}

	/// Check if this operator is right-associative
	pub fn is_right_assoc(&self) -> bool {
		let (l, r) = self.binding_power();
		l > 0 && r > 0 && r < l
	}

	/// Check if this is a binary arithmetic operator
	pub fn is_arithmetic(&self) -> bool {
		matches!(self, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Rem | Op::Pow)
	}

	/// Check if this is a shift operator, defined on exact Ints only
	pub fn is_shift(&self) -> bool {
		matches!(self, Op::Shl | Op::Shr)
	}

	/// Check if this is a comparison operator
	pub fn is_comparison(&self) -> bool {
		matches!(self, Op::Eq | Op::Ne | Op::Identical | Op::NotIdentical | Op::Lt | Op::Gt | Op::Le | Op::Ge)
	}

	/// Ordering comparisons chain (a<b<c), equality does not
	pub fn is_ordering(&self) -> bool {
		matches!(self, Op::Lt | Op::Gt | Op::Le | Op::Ge)
	}

	pub fn is_equality(&self) -> bool {
		matches!(self, Op::Eq | Op::Ne)
	}

	/// Check if this is a logical operator (and, or, xor)
	pub fn is_logical(&self) -> bool {
		matches!(self, Op::And | Op::Or | Op::Xor)
	}

	/// Check if this is a compound assignment operator (+=, -=, etc.)
	pub fn is_compound_assign(&self) -> bool {
		matches!(
			self,
			Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign |
			Op::ModAssign | Op::PowAssign | Op::AndAssign | Op::OrAssign | Op::XorAssign
		)
	}

	/// Get the base operator for a compound assignment (AddAssign -> Add, etc.)
	pub fn base_op(&self) -> Op {
		match self {
			Op::AddAssign => Op::Add,
			Op::SubAssign => Op::Sub,
			Op::MulAssign => Op::Mul,
			Op::DivAssign => Op::Div,
			Op::ModAssign => Op::Mod,
			Op::PowAssign => Op::Pow,
			Op::AndAssign => Op::And,
			Op::OrAssign => Op::Or,
			Op::XorAssign => Op::Xor,
			_ => *self,
		}
	}
}

impl fmt::Display for Op {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

/// Every operator by its code in the kind field of a Key node (`(code << 8) | Kind::Key`); the first five are the
/// codes earlier modules stored, the rest follow, so a quoted expression (`data 1+2`) reads back with its operator
const OP_CODES: [Op; 63] = [
	Op::None, Op::Colon, Op::Assign, Op::Define, Op::Dot,
	Op::SafeDot, Op::Scope, Op::Arrow, Op::FatArrow, Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Rem, Op::Pow,
	Op::Shl, Op::Shr, Op::AddAssign, Op::SubAssign, Op::MulAssign, Op::DivAssign, Op::ModAssign, Op::PowAssign,
	Op::AndAssign, Op::OrAssign, Op::XorAssign, Op::Lt, Op::Gt, Op::Le, Op::Ge, Op::Eq, Op::Ne, Op::Similar, Op::And,
	Op::Or, Op::Xor, Op::Not, Op::Neg, Op::Sqrt, Op::Cbrt, Op::Abs, Op::Inc, Op::Dec, Op::Square, Op::Cube,
	Op::Question, Op::If, Op::Then, Op::Else, Op::While, Op::Do, Op::Hash, Op::Range, Op::To, Op::As, Op::PlusMinus,
	Op::Coalesce, Op::Identical, Op::NotIdentical, Op::Rough, Op::LogBase, Op::LogOf,
];

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
