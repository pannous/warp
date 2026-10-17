//! Operators: `Op`, the operator of a Key node, and what kind of operation each one is
use serde::{Deserialize, Serialize};
use std::fmt;

mod binding_power;
pub use binding_power::*;
mod codes;
pub use codes::*;
mod spellings;
pub use spellings::*;
mod aliases;
pub use aliases::*;

/// Keywords that introduce function definitions
pub const FUNCTION_KEYWORDS: [&str; 6] = ["fun", "fn", "def", "define", "function", "func"];
/// `infix operator ⊕ := …` defines the function `operator_⊕`, which each use of ⊕ calls
pub const OPERATOR_FUNCTION_PREFIX: &str = "operator_";

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
	Colon,     // :   type annotation and object construction person:{name:"Joe" age:42}
	Dot,       // .   member access
	SafeDot,   // ?.  member access that is ø when the receiver is ø
	Coalesce,  // ??  the left value unless it is ø, then the right (Swift, C#, JS)
	Scope,     // ::  scope resolution
	Define,    // :=  definition
	Assign,    // =   assignment
	Arrow,     // ->  arrow/lambda/return type
	FatArrow,  // =>  only while parsing: the spelling binds and reads as in JS; parse() makes it Arrow, one arrow for warp
	ArrowUp,   // ↑  e.g. exponential x^n
	ArrowDown, // ↓ e.g. logarithmic   log(x)/log(base)

	// Arithmetic operators
	Add,      // +
	Sub,      // -
	Mul,      // *  ×  ⋅
	Div,      // /  ÷
	FloorDiv, // /!  div  the Euclidean quotient that goes with %: floor_quotient (warp_parser FLOOR_QUOTIENT)
	Mod,      // %  mod  Euclidean: 0 ≤ a % b < |b|
	Rem,      // rem  truncated remainder, sign of the dividend (C, Java, JS, Rust %)
	Pow,      // ^  **
	LogBase,  // ⌞  b⌞x: log base b of x (P226)
	LogOf,    // ⌟  x⌟: the natural log of x, x⌟b: log base b of x (P226)
	Shl,      // <<  shift left: a * 2^n
	Shr,      // >>  shift right: floor(a / 2^n)

	// Compound assignment operators (x op= y → x = x op y)
	AddAssign,      // +=
	SubAssign,      // -=
	MulAssign,      // *=
	DivAssign,      // /=
	FloorDivAssign, // //=
	ModAssign,      // %=
	PowAssign,      // ^=  **=
	AndAssign,      // &&=  and=
	OrAssign,       // ||=  or=
	XorAssign,      // ^=   xor=

	// Comparison operators
	Lt,           // <
	Gt,           // >
	Le,           // <=  ≤
	Ge,           // >=  ≥
	Eq,           // ==
	Ne,           // !=  ≠
	Identical,    // ===  equal and of the same type: 0 === false is false (card zero-false)
	NotIdentical, // !==
	Similar,      // ≈  ⋍  circa  approximately: equal within the relative `tolerance` (default 1e-9)
	Rough, // ~  ~~: looser than ≈, within `rough_tolerance` (default 1%), texts without surrounding punctuation (P211)

	// Logical operators
	And, // and  &&  ∧
	Or,  // or   ||  ⋁
	Xor, // xor  ⊻
	Not, // not  !  ¬
	// `&` and `|`: the bits of two whole numbers, else the logical and/or (card operator-shall)
	BitAnd, // &
	BitOr,  // |

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
	Hash,      // #  (1-based index)
	Subscript, // xs[i]  only while parsing: parse() makes it xs#(i+1) or a slice (warp_parser sugar.rs)
	Range,     // ..
	To,        // to  ...  …

	// Type conversion
	As, // as  (type cast)

	PlusMinus, // ±  +-  a value with tolerance

	// Parsed only, the parser builds their nodes: the pipeline `xs |> f` is the call `f(xs)`, the membership `x in xs`
	// and `xs contains x` the list [x in xs]
	Pipe, // |>
	In,   // in

	// User(Node), allow user defined operators, name==symbol
	None, // implicit/unknown
}

impl Op {
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

	/// Whether this comparison holds for two values ordered so (`<=` holds for Less and Equal)
	pub fn holds(&self, ordering: std::cmp::Ordering) -> bool {
		use std::cmp::Ordering::*;
		match self {
			Op::Eq | Op::Identical => ordering == Equal,
			Op::Ne | Op::NotIdentical => ordering != Equal,
			Op::Lt => ordering == Less,
			Op::Le => ordering != Greater,
			Op::Gt => ordering == Greater,
			_ => ordering != Less, // Ge
		}
	}

	/// Check if this is a binary arithmetic operator
	pub fn is_arithmetic(&self) -> bool {
		matches!(self, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::FloorDiv | Op::Mod | Op::Rem | Op::Pow)
	}

	/// Check if this is a shift operator, defined on exact Ints only
	pub fn is_shift(&self) -> bool {
		matches!(self, Op::Shl | Op::Shr)
	}

	/// Check if this is a comparison operator
	pub fn is_comparison(&self) -> bool {
		matches!(self, Op::Eq | Op::Ne | Op::Identical | Op::NotIdentical | Op::Lt | Op::Gt | Op::Le | Op::Ge)
	}

	/// A comparison or a similarity (≈, ~): `combine 1.1 with 2.2 ≈ 3.3` compares the phrase's value
	pub fn is_relation(&self) -> bool {
		self.is_comparison() || matches!(self, Op::Similar | Op::Rough)
	}

	/// Ordering comparisons chain (a<b<c), equality does not
	pub fn is_ordering(&self) -> bool {
		matches!(self, Op::Lt | Op::Gt | Op::Le | Op::Ge)
	}

	pub fn is_equality(&self) -> bool {
		matches!(self, Op::Eq | Op::Ne)
	}

	/// An operator the parser knows but the language gives no meaning of its own: a declaration gives it one,
	/// `infix operator ↑(x: number, n: number) := …` (lib/prelude.warp), and its use is the call of that declaration
	pub fn is_declared_only(&self) -> bool {
		matches!(self, Op::ArrowUp | Op::ArrowDown)
	}

	/// Check if this is a logical operator (and, or, xor)
	pub fn is_logical(&self) -> bool {
		matches!(self, Op::And | Op::Or | Op::Xor)
	}

	/// `&` and `|` as the logical and/or they are on anything but two whole numbers; every other operator itself
	pub fn logical_twin(&self) -> Op {
		match self {
			Op::BitAnd => Op::And,
			Op::BitOr => Op::Or,
			_ => *self,
		}
	}

	pub fn is_bitwise(&self) -> bool {
		matches!(self, Op::BitAnd | Op::BitOr)
	}

	/// An operator computing a value from its operands, `1+1`, `a<b`, `-x`: a Key of it is an operation, while a pair
	/// `a:1`, a binding `a=1`, a lookup `a.b` stay keys (decision parse-type)
	pub fn is_operation(&self) -> bool {
		self.is_arithmetic()
			|| self.is_shift()
			|| self.is_relation()
			|| self.is_logical()
			|| self.is_bitwise()
			|| (self.is_prefix() || self.is_suffix()) && !self.is_write()
			|| matches!(self, Op::LogBase | Op::LogOf | Op::PlusMinus | Op::Coalesce | Op::ArrowUp | Op::ArrowDown)
	}

	/// Check if this is a compound assignment operator (+=, -=, etc.)
	pub fn is_compound_assign(&self) -> bool {
		matches!(
			self,
			Op::AddAssign
				| Op::SubAssign
				| Op::MulAssign
				| Op::DivAssign
				| Op::FloorDivAssign
				| Op::ModAssign
				| Op::PowAssign
				| Op::AndAssign
				| Op::OrAssign
				| Op::XorAssign
		)
	}

	/// `=`, `+=` and the like, `++`, `--`: an operator that writes its variable
	pub fn is_write(&self) -> bool {
		*self == Op::Assign || self.is_compound_assign() || matches!(self, Op::Inc | Op::Dec)
	}

	/// Get the base operator for a compound assignment (AddAssign -> Add, etc.)
	pub fn base_op(&self) -> Op {
		match self {
			Op::AddAssign => Op::Add,
			Op::SubAssign => Op::Sub,
			Op::MulAssign => Op::Mul,
			Op::DivAssign => Op::Div,
			Op::FloorDivAssign => Op::FloorDiv,
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
