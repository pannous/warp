//! Kinds of nodes: `Kind` and how a node's kind field packs it with the bracket, operator, marks and separator
use crate::type_kinds;
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::ValType::Ref;
use wasm_encoder::{AbstractHeapType, HeapType, RefType, ValType};
mod field_storage;
pub use field_storage::*;
mod user_types;
pub use user_types::*;

/// A node's kind field holds the `Kind` in its low KIND_BITS bits and extra info above them:
/// the bracket of a list (Curly=0, Square=1, Round=2 …) or the operator of a key
pub const KIND_BITS: i64 = 8;
pub const KIND_MASK: i64 = 0xFF;
pub const CURLY_BRACKET_INFO: i64 = 0;
pub const SQUARE_BRACKET_INFO: i64 = 1;
pub const CURLY_LIST_KIND: i64 = (CURLY_BRACKET_INFO << KIND_BITS) | Kind::List as i64;
pub const SQUARE_LIST_KIND: i64 = (SQUARE_BRACKET_INFO << KIND_BITS) | Kind::List as i64;
/// true/false: a shallow type of its own (user, card bool-type), an Int 1/0 marked above the kind bits, so everything
/// that reads the kind masked takes it as the Int it is, and printing, `type` and reading back see a bool
pub const BOOL_INFO: i64 = 1;
pub const BOOL_KIND: i64 = (BOOL_INFO << KIND_BITS) | Kind::Int as i64;
/// An Int shown in hex, 0xFF (card hex-printing, wasm_emitter/hex.rs): marked only where it becomes a Node
pub const HEX_INFO: i64 = 2;
/// An Int shown in binary, 0b101 (card binary-int, wasm_emitter/radix.rs)
pub const BINARY_INFO: i64 = 3;
/// The bit of a bool in the kind masks of run-time type tests (node_kind_in): no Kind is that high
pub const BOOL_MASK_BIT: i64 = 62;
/// P215: the head cell of a declared list carries the run-time kinds its element type admits, its element mark, in the
/// kind field above the bracket byte; everything that reads the bracket masks it with KIND_MASK
pub const ELEMENT_MARK_SHIFT: i64 = 16;
/// A mark holds the kind mask's bits of every Kind, room for new kinds included, then the bool bit
const MARK_KIND_BITS: i64 = 24;
const _: () = assert!(
	(Kind::Function as i64) < MARK_KIND_BITS,
	"every Kind has its bit in the element mark"
);
pub const MARK_KINDS_MASK: i64 = (1 << MARK_KIND_BITS) - 1;
pub const MARK_BOOL_BIT: i64 = MARK_KIND_BITS;
/// The separator of a list (card keep-list: `,` `;` newline, space; 0 for none), in the kind field above the element
/// mark; the bracket byte and the kind stay where they were, so whatever masks them reads them as before
pub const SEPARATOR_SHIFT: i64 = 48;
pub const SEPARATOR_MASK: i64 = 0x7 << SEPARATOR_SHIFT;
const _: () = assert!(
	MARK_BOOL_BIT + ELEMENT_MARK_SHIFT < SEPARATOR_SHIFT,
	"the separator lies above the element mark"
);
/// A kind field without its element mark, for the comparisons of whole kinds (bracket and kind)
pub const UNMARKED_KIND: i64 = (1 << ELEMENT_MARK_SHIFT) - 1;
const _: () = assert!(
	MARK_BOOL_BIT + ELEMENT_MARK_SHIFT < 63,
	"the element mark fits in the kind field"
);

/// The element mark of a node_kind_in mask, shifted to its place in the kind field
pub fn element_mark(mask: i64) -> i64 {
	let compact = (mask & MARK_KINDS_MASK) | ((mask >> BOOL_MASK_BIT) & 1) << MARK_BOOL_BIT;
	compact << ELEMENT_MARK_SHIFT
}

/// Node type tags for runtime type checking and WASM encoding
/// Compact repr(u8) for efficient storage in WASM GC structs
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Kind {
	#[default]
	Empty = 0, // void / no value
	Int = 1,        // i64 value (boxed in $i64box)
	Float = 2,      // f64 value (boxed in $f64box)
	Text = 3,       // string (via $String struct)
	Codepoint = 4,  // char/i32 as i31ref
	Symbol = 5,     // string (via $String struct)
	Key = 6,        // data=key node, value=value node (also used for pairs)
	Block = 7,      // curly braces {}
	List = 8,       // square brackets []
	Data = 9,       // arbitrary data container
	Meta = 10,      // metadata wrapper
	Error = 11,     // error node
	TypeDef = 12,   // type definition: name + body (fields)
	Pointer = 13,   // FFI pointer (i64 handle)
	Int32 = 14,     // explicit i32 (for FFI)
	Float32 = 15,   // explicit f32 (for FFI)
	Function = 16,  // closure: data = $Closure struct (typed function reference + captured values), value = name symbol
	Uncertain = 17, // x ± σ: data = $f64 array [value, source id, contribution, …] (wasm_emitter/uncertain.rs)
	Time = 18,      // an instant: data = $i64box(nanoseconds since 1970 UTC) (wasm_emitter/times.rs)
}

impl Kind {
	/// Check if this is an integer type (WASM i64 local)
	pub fn is_int(&self) -> bool {
		matches!(self, Kind::Int | Kind::Int32)
	}

	/// Check if this is a float type (WASM f64 local)
	pub fn is_float(&self) -> bool {
		matches!(self, Kind::Float | Kind::Float32)
	}

	/// Check if this is a primitive numeric type (stored as WASM primitive)
	pub fn is_primitive(&self) -> bool {
		matches!(
			self,
			Kind::Int | Kind::Int32 | Kind::Float | Kind::Float32 | Kind::Codepoint | Kind::Pointer
		)
	}

	/// Check if this is a reference type (stored as WASM ref $Node)
	pub fn is_ref(&self) -> bool {
		!self.is_primitive()
	}
}

impl std::fmt::Display for Kind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Kind::Empty => write!(f, "empty"),
			Kind::Int => write!(f, "int"), // warp uses "int" for i64
			Kind::Int32 => write!(f, "i32"),
			Kind::Float => write!(f, "float"), // warp uses "float" for f64
			Kind::Float32 => write!(f, "f32"),
			Kind::Text => write!(f, "text"), // warp uses "text" for strings
			Kind::Codepoint => write!(f, "codepoint"),
			Kind::Symbol => write!(f, "symbol"),
			Kind::Key => write!(f, "key"),
			Kind::Block => write!(f, "block"),
			Kind::List => write!(f, "list"),
			Kind::Data => write!(f, "data"),
			Kind::Meta => write!(f, "meta"),
			Kind::Error => write!(f, "error"),
			Kind::TypeDef => write!(f, "typedef"),
			Kind::Pointer => write!(f, "pointer"),
			Kind::Function => write!(f, "function"),
			Kind::Uncertain => write!(f, "uncertain"),
			Kind::Time => write!(f, "instant"),
		}
	}
}

/// Alias for backward compatibility
pub type NodeKind = Kind;

pub enum AstKind {
	Declaration,
	Expression,
	Statement,
	While,
	For,
	If,
	Function,
	Return,
	Call,
	Parameter,
	Body,
	Assignment,
	Literal,
	Identifier,
}
