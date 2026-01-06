// more specific than NodeKind! i32 ≠ int64 etc
enum Type {
	Longs,
	Reals,
	Bools,
}

/// Node variant tags (for runtime type checking)
/// Old combined Number type - kept for compatibility
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
	Empty = 0,
	Number = 1, // no bool in wasm ok
	Text = 2,
	Codepoint = 3, // seems a bit out of whack here <<
	Symbol = 4,
	Key = 5,
	// Pair = 6 - REMOVED, use List or Key instead
	// Tag = 7 - REMOVED, use Key instead
	Block = 8,
	List = 9,
	Data = 10,
	Meta = 11,
	Error = 12,
	Externref,
}

/// Compact 3-field Node tags - separate Int/Float for cleaner dispatch
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeTag {
	Empty = 0,
	Int = 1,       // i64 value (boxed in $i64box)
	Float = 2,     // f64 value (boxed in $f64box) - separate from Int!
	Text = 3,      // string (via $String struct)
	Codepoint = 4, // char as i31ref
	Symbol = 5,    // string (via $String struct)
	Key = 6,       // data=key node, value=value node
	Pair = 7,      // data=left, value=right
	Block = 8,     // curly braces {}
	List = 9,      // square brackets []
	Data = 10,     // arbitrary data container
	Meta = 11,     // metadata wrapper
	Error = 12,    // error node
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

