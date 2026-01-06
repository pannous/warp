/// Node type tags for runtime type checking and WASM encoding
/// Compact repr(u8) for efficient storage in WASM GC structs
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeTag {
	Empty = 0,
	Int = 1,       // i64 value (boxed in $i64box)
	Float = 2,     // f64 value (boxed in $f64box)
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
	Type = 13,     // type definition: name + body (fields)
}

/// Alias for backward compatibility
pub type NodeKind = NodeTag;

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

