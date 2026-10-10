use crate::context::{Context, Param, UserFunctionDef};
use crate::diagnostic::Diagnostic;
use crate::extensions::numbers::Number;
use crate::function::{Function, FunctionRegistry};
use crate::local::Local;
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::type_kinds::Kind;
use std::collections::{HashMap, HashSet};

/// Property words that count the elements of a value: `size of x`, `x size`, `x.size`; `size` is a synonym of `count`
/// Words that declare a name assignable once: `const x=5`, `final x=5`
pub const CONSTANT_KEYWORDS: [&str; 4] = ["const", "constant", "final", "val"];
/// Statement words whose argument is never their property: `return count` is no `return.count`
const PRINT_CALL: &str = "print";
/// The error for a user function named like a type word; `{name}` is the word
const TYPE_WORD_FUNCTION_CLASH: &str = "{name} is a type; rename your function";
const RETURNING_KEYWORDS: [&str; 2] = ["return", "yield"];
const PROPERTYLESS_KEYWORDS: [&str; 6] = ["return", "yield", PRINT_CALL, "println", "puts", "not"];

fn is_constant_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if CONSTANT_KEYWORDS.contains(&word.as_str()))
}

/// `var x = 1` announces a reassignable variable, plain `x = 1`
const VAR_KEYWORD: &str = "var";
/// `let x = 1` may change, with a note teaching `var` (P159; check_constants)
const IMMUTABLE_LET: &str = "let";

pub(crate) fn is_declaration_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if is_declaration_word(word))
}

/// `var`, `let`, `const`, `val` …: a word declaring the name after it
pub(crate) fn is_declaration_word(word: &str) -> bool {
	CONSTANT_KEYWORDS.contains(&word) || word == VAR_KEYWORD || word == IMMUTABLE_LET
}

/// The words counting a value's elements, as `count x`, `x.size`, `len(x)`: the one table every pass asks, through
/// is_counting_word (a class's own size method is looked up in this order)
pub(crate) const COUNTING_WORDS: [&str; 4] = ["size", "count", "len", "length"];
/// `byte_size(x)`, `x.byte_size`: the bytes of x, as `x.bytes` (size is the element count, user decision P40)
const BYTE_SIZE: &str = "byte_size";
/// `x.number` counts too, but `number x` is the type conversion, not a count
const NUMBER_PROPERTY: &str = "number";

pub(crate) fn is_counting_property(word: &str) -> bool {
	word == NUMBER_PROPERTY || is_counting_word(word)
}

/// Check if a node is pure data (not a statement/function call)
fn is_data_node(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty => true,
		Node::Symbol(s) => !is_function_keyword(s),
		Node::List(items, bracket, separator) => !is_unbracketed_block(items, bracket, separator) && items.iter().all(is_data_node),
		Node::Key(_, Op::Colon, _) => true,  // Key-value pairs are data
		Node::Key(left, op, right) if op.is_arithmetic() => is_data_node(left) && is_data_node(right), // `[1+2, 3]` keeps both items
		Node::Key(left, Op::Hash, right) => is_data_node(left) && is_data_node(right), // a count or element: `[#a, a#1]`
		Node::Key(value, Op::As, _) => is_data_node(value), // `[1.5f 2.5f]`
		_ => false,
	}
}

/// An unbracketed `;`/newline list is a block: its items run in order and the last one is the value
/// (`1;2;3` → 3, `'hello';(1 2 3 4);10` → 10). A program is one, and `{…}` is a block literal that keeps its items as a
/// value until it is run as a function body or branch. `(…)` and `[…]` are lists and keep all items.
pub fn is_unbracketed_block(items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
	matches!(separator, Separator::Semicolon | Separator::Newline) && *bracket == Bracket::None && items.len() > 1
}

/// Items that run rather than compute a value, making their list a block: assignments, definitions, `i++`, imports.
/// Control flow counts too, except inside `[…]`: there `if c then a else b` is just a computed element, as `p#2` is.
pub fn is_statement(item: &Node, bracket: &Bracket) -> bool {
	match item.drop_meta() {
		Node::Key(_, Op::Assign | Op::Define | Op::Inc | Op::Dec, _) => true,
		Node::Key(_, op, _) if op.is_compound_assign() => true,
		Node::Key(_, Op::Then | Op::Else | Op::Do, _) => *bracket != Bracket::Square,
		Node::Key(left, Op::Colon, _) => left.is_symbol("global"),
		// a lowered `for` loop: `i=a; while …`; inside `[…]` a sequence ending in a value is a computed element (an awaited
		// task, `[a, b]` of task variables)
		Node::List(list_items, Bracket::None, separator) if is_unbracketed_block(list_items, &Bracket::None, separator) => {
			*bracket != Bracket::Square || list_items.last().is_some_and(|last| is_statement(last, &Bracket::None))
		}
		// a group that runs statements, `(y=1; y)`, or prints: in a block it runs, it is not an item
		Node::List(list_items, Bracket::Round, _) if *bracket != Bracket::Square && list_items.iter().any(|inner| is_statement(inner, &Bracket::Round)) => true,
		// as `sleep(10)`, which gives nothing
		Node::List(list_items, _, _) if *bracket != Bracket::Square && matches!(list_items.as_slice(), [word, _] if word.is_symbol(PRINT_CALL) || word.is_symbol(crate::host::SLEEP)) => true,
		Node::List(list_items, _, _) if list_items.len() >= 2 => {
			// `cell_set(c, v)` is the assignment of a nonlocal variable (lowering/nonlocal_cells.rs)
			// `$destructure (h, o) value` is `h, o = value` lowered (tuples.rs)
			matches!(list_items[0].drop_meta(), Node::Symbol(s) if is_function_keyword(s) || crate::modules::is_import_keyword(s) || ["return", crate::tuples::DESTRUCTURE, crate::host::TASK_CHECK, crate::wasm_emitter::cells::CELL_SET, crate::wasm_emitter::cells::SIGNAL_LISTENERS_SET].contains(&s.as_str()))
		}
		_ => false,
	}
}

/// `{a;b;c}` as a definition body: a sequence of statements to run, unlike the value list `{1 2 3}`
pub(crate) fn is_statement_block(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(_, Bracket::Curly, Separator::Semicolon | Separator::Newline) => true,
		Node::List(items, Bracket::Curly, _) => matches!(items.as_slice(), [single] if is_update(single.drop_meta())), // `{x=x+1}`, `{n++}`
		_ => false,
	}
}

mod inference;
mod variables;
mod checks;
mod declaration_lowering;
mod user_functions;
mod counting;
mod imports;
mod booleans;
mod upcast_fields;
pub use upcast_fields::check_upcast_fields;
mod list_views;
pub(crate) use imports::signature_kind;
pub use inference::*;
pub use variables::*;
pub use checks::*;
pub use declaration_lowering::*;
pub use user_functions::*;
pub use counting::*;
pub use imports::*;
pub use booleans::*;
