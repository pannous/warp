// more specific than NodeKind! i32 ≠ int64 etc
enum Type {
	Longs,
	Reals,
	Bools,
}

/// Node variant tags (for runtime type checking)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
	Empty = 0,
	Number = 1,
	Text = 2,
	Codepoint = 3, // seems a bit out of whack here <<
	Symbol = 4,
	Key = 5,
	Pair = 6,
	Tag = 7,
	Block = 8,
	List = 9,
	Data = 10,
	Meta = 11,
	Error = 12,
	Externref,
}

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

// Wasp ABI GC Node representation design:
// This is a single struct that can represent any node type
/*
(type $node (struct
  (field $name_ptr i32)
  (field $name_len i32)
  (field $tag i32)
  (field $int_value i64)
  (field $float_value f64)
  (field $text_ptr i32)
  (field $text_len i32)
  (field $left (ref null $node))  // recursive reference
  (field $right (ref null $node))
  (field $meta (ref null $node))
 ))
 */

// todo move node layout to wasp_abi.rs
// any change to node layout must be reflected in wasm_gc_reader.rs wasp_abi.md ... todo ...
