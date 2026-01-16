use std::fmt;
use serde::{Deserialize, Serialize};

/// Keywords that introduce function definitions
pub const FUNCTION_KEYWORDS: [&str; 5] = ["fun", "fn", "def", "define", "function"];

pub fn is_function_keyword(s: &str) -> bool {
	FUNCTION_KEYWORDS.contains(&s)
}

// node[i]

// Wasp ABI GC Node representation design:
// This is a single struct that can represent any node type

// todo move node layout to wasp_abi.rs
// todo ... any change to node layout must be reflected in wasm_gc_reader.rs wasp_abi.md ...

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
	Mod,  // %
	Pow,  // ^  **

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

	// Logical operators
	And, // and  &&  ∧
	Or,  // or   ||  ⋁
	Xor, // xor  ⊻
	Not, // not  !  ¬

	// Prefix operators (unary)
	Neg,  // - (unary minus)
	Sqrt, // √
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

	// User(Node), allow user defined operators, name==symbol

	None, // implicit/unknown
}

impl Op {
	/// Binding power: (left_bp, right_bp)
	/// Higher = tighter binding. Right > left means right-associative.
	/// Suffix operators: (left_bp, 0) - only binds to left
	/// Prefix operators: (0, right_bp) - only binds to right
	pub fn binding_power(&self) -> (u8, u8) {
		match self {
			// Suffix operators (bind very tight to left, no right operand)
			Op::Square | Op::Cube => (200, 0),
			Op::Inc | Op::Dec => (195, 0),

			// Member access (tightest infix)
			Op::Dot => (180, 181),
			Op::Scope => (175, 176),
			Op::Hash => (170, 171), // index operator #

			// Power (right-assoc: 2^3^4 = 2^(3^4))
			Op::Pow => (160, 159),

			// Multiplicative (left-assoc)
			Op::Mul | Op::Div | Op::Mod => (150, 151),

			// Additive (left-assoc)
			Op::Add | Op::Sub => (140, 141),

			// Range
			Op::Range | Op::To => (130, 131),

			// Type cast (binds tighter than comparison: 1.5 as int == 1)
			Op::As => (125, 126),

			// Comparison (left-assoc, no chaining)
			Op::Lt | Op::Gt | Op::Le | Op::Ge => (120, 121),
			Op::Eq | Op::Ne => (115, 116),

			// Logical (left-assoc with and > or)
			Op::And => (100, 101),
			Op::Xor => (95, 96),
			Op::Or => (90, 91),

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
			Op::FatArrow => (70, 69), // right-assoc: a => b
			Op::Define => (60, 59),   // right-assoc: a:=b:=c → a:=(b:=c)
			Op::Assign => (60, 59),   // right-assoc: a=b=c → a=(b=c)

			// Compound assignment (same precedence as assignment)
			Op::AddAssign | Op::SubAssign | Op::MulAssign | Op::DivAssign |
			Op::ModAssign | Op::PowAssign | Op::AndAssign | Op::OrAssign |
			Op::XorAssign => (60, 59),

			// Prefix operators (no left operand, binds to right)
			Op::Neg | Op::Not | Op::Sqrt | Op::Abs => (0, 190),

			Op::None => (0, 0),
		}
	}

	/// The string representation of this operator
	pub fn as_str(&self) -> &'static str {
		match self {
			// Structural
			Op::Colon => ":",
			Op::Dot => ".",
			Op::Scope => "::",
			Op::Define => ":=",
			Op::Assign => "=",
			Op::Arrow => "->",
			Op::FatArrow => "=>",

			// Arithmetic
			Op::Add => "+",
			Op::Sub => "-",
			Op::Mul => "*",
			Op::Div => "/",
			Op::Mod => "%",
			Op::Pow => "^",

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

			// Logical
			Op::And => "and",
			Op::Or => "or",
			Op::Xor => "xor",
			Op::Not => "not",

			// Prefix
			Op::Neg => "-",
			Op::Sqrt => "√",
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
		matches!(self, Op::Neg | Op::Not | Op::Sqrt | Op::Abs)
	}

	/// Check if this is a suffix-only operator
	pub fn is_suffix(&self) -> bool {
		matches!(self, Op::Inc | Op::Dec | Op::Square | Op::Cube)
	}

	/// Check if this operator is right-associative
	pub fn is_right_assoc(&self) -> bool {
		let (l, r) = self.binding_power();
		l > 0 && r > 0 && r < l
	}

	/// Check if this is a binary arithmetic operator
	pub fn is_arithmetic(&self) -> bool {
		matches!(self, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Pow)
	}

	/// Check if this is a comparison operator
	pub fn is_comparison(&self) -> bool {
		matches!(self, Op::Eq | Op::Ne | Op::Lt | Op::Gt | Op::Le | Op::Ge)
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