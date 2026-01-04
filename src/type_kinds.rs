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
	// Tag = 7 - REMOVED, use Key instead
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

