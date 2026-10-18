//! The parser's context: what the enclosing constructs say about the text being parsed (inside a condition `=`
//! compares, inside a `for` header a block is the body, …), one value set for a stretch of parsing by within()
use super::WarpParser;

#[derive(Clone, Copy, Default)]
pub(crate) struct Context {
	pub(crate) base_indent: usize,
	/// Inside a block indented with spaces below a trailing `:` spaces count as indent too (elsewhere only tabs do)
	pub(crate) indent_counts_spaces: bool,
	/// Inside an `if`/`while` condition `=` compares instead of assigning (wiki/Bad.md)
	pub(crate) equals_compares: bool,
	/// Inside the iterable of `for x in …` a block is the loop body, never an argument: `for i in 0..n {…}`
	pub(crate) in_for_header: bool,
	/// The binding power of a glued pair's value (`for:email`): that value is one atom, no call of what follows
	pub(crate) glued_pair_bp: Option<u8>,
	/// Where the innermost bracketed group opened (line, column): an unclosed one names it
	pub(crate) group_start: (usize, usize),
	/// While the `then` body of `if c: body else …` is parsed, `else` ends it instead of joining it
	pub(crate) stops_at_else: bool,
	/// The binding power of the `then` or `else` branch or the condition being parsed: there a braceless call takes a
	/// variable argument (`then count xs`, `if square x > 5`), as at assignment level
	pub(crate) branch_bp: Option<u8>,
	/// Directly inside a list literal `[…]` without commas: a space and a glued sign start the next item (`[a -1]`,
	/// decision list-sign)
	pub(crate) in_spaced_list: bool,
	/// Inside `do … end`: the `end` keyword closes the statement list
	pub(crate) stops_at_end: bool,
	/// Inside Python's `f"…{x}…"`: braces are holes, `{{` and `}}` the braces themselves
	pub(crate) brace_holes: bool,
	/// Parsing an argument of a braceless call at statement level (`sleep 1s …`): an `and` followed by a statement ends it
	pub(crate) in_command: bool,
	/// Parsing the block of a data literal (`a{ … }`, not a declared type's constructor): a spaced child `c { d:3 }` there
	/// is the child node of the glued `c{ d:3 }`, as no call with a block can be meant (card spaced-child)
	pub(crate) in_data_literal: bool,
	/// Parsing the rules of a style sheet `style{ … }`: a blank before `.x` or `#x` is CSS's descendant combinator, which
	/// starts the next part of a selector instead of a member access (`#main .x`, card web-styles-parser)
	pub(crate) in_style_sheet: bool,
	/// Parsing the one argument of a braceless call (`square xs |> sum`): the pipeline after it takes the whole call
	pub(crate) pipe_takes_call: bool,
	/// inside Elixir's capture `&(…)`, where `&1` is its first argument
	pub(crate) in_capture: bool,
}

impl Context {
	/// The head of `for`, `if` or `while`: a block after it is the body, never an argument
	pub(crate) fn in_header(&self) -> bool {
		self.in_for_header || self.equals_compares
	}
}

impl WarpParser {
	/// `parse` run in `context`, the enclosing context back afterwards
	pub(crate) fn within<T>(&mut self, context: Context, parse: impl FnOnce(&mut Self) -> T) -> T {
		let outer = std::mem::replace(&mut self.context, context);
		let parsed = parse(self);
		self.context = outer;
		parsed
	}
}
