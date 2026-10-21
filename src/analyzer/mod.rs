use crate::compiler::context::{Context, Param, UserFunctionDef};
use crate::compiler::local::Local;
use crate::errors::diagnostic::Diagnostic;
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::type_kinds::Kind;
use crate::words::{CONSTANT_KEYWORDS, PRINT_WORD};
use std::collections::{HashMap, HashSet};

const RETURNING_KEYWORDS: [&str; 2] = ["return", "yield"];

/// Check if a node is pure data (not a statement/function call)
fn is_data_node(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty => true,
		Node::Symbol(s) => !is_function_keyword(s),
		Node::List(items, bracket, separator) => {
			!is_unbracketed_block(items, bracket, separator) && items.iter().all(is_data_node)
		}
		Node::Key(_, Op::Colon, _) => true, // Key-value pairs are data
		Node::Key(left, op, right) if op.is_arithmetic() => is_data_node(left) && is_data_node(right), // `[1+2, 3]` keeps both items
		Node::Key(left, Op::Hash, right) => is_data_node(left) && is_data_node(right), // a count or element: `[#a, a#1]`
		Node::Key(value, Op::As, _) => is_data_node(value),                            // `[1.5f 2.5f]`
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
		Node::List(list_items, Bracket::None, separator)
			if is_unbracketed_block(list_items, &Bracket::None, separator) =>
		{
			*bracket != Bracket::Square || list_items.last().is_some_and(|last| is_statement(last, &Bracket::None))
		}
		// a group that runs statements, `(y=1; y)`, or prints: in a block it runs, it is not an item
		Node::List(list_items, Bracket::Round, _)
			if *bracket != Bracket::Square && list_items.iter().any(|inner| is_statement(inner, &Bracket::Round)) =>
		{
			true
		}
		// as `sleep(10)` and `sleep_ms(10)`, which give nothing
		Node::List(list_items, _, _)
			if *bracket != Bracket::Square
				&& matches!(list_items.as_slice(), [word, _] if word.is_symbol(PRINT_WORD) || word.is_symbol(crate::host::SLEEP) || word.is_symbol(crate::host::SLEEP_MS)) =>
		{
			true
		}
		Node::List(list_items, _, _) if list_items.len() >= 2 => {
			list_items[0].symbol_name().is_some_and(heads_statement)
		}
		_ => false,
	}
}

/// Words other than the function and import keywords that head a statement: `return x`, `$destructure (h, o) value`
/// (`h, o = value` lowered, tuples.rs), `cell_set(c, v)` (the assignment of a nonlocal variable,
/// lowering/definitions/nonlocal_cells.rs), the task check and the signal listeners' update
const STATEMENT_HEADS: [&str; 5] = [
	"return",
	crate::tuples::DESTRUCTURE,
	crate::host::TASK_CHECK,
	crate::wasm_emitter::functions::cells::CELL_SET,
	crate::wasm_emitter::functions::cells::SIGNAL_LISTENERS_SET,
];

fn heads_statement(word: &str) -> bool {
	is_function_keyword(word) || crate::modules::is_import_keyword(word) || STATEMENT_HEADS.contains(&word)
}

/// `{a;b;c}` as a definition body: a sequence of statements to run, unlike the value list `{1 2 3}`
pub(crate) fn is_statement_block(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(_, Bracket::Curly, Separator::Semicolon | Separator::Newline) => true,
		Node::List(items, Bracket::Curly, _) => matches!(items.as_slice(), [single] if is_update(single.drop_meta())), // `{x=x+1}`, `{n++}`
		_ => false,
	}
}

mod annotations;
mod booleans;
mod checks;
mod counting;
mod imports;
mod inference;
mod inferred_kinds;
mod main_globals;
mod upcast_fields;
mod user_functions;
mod variables;
pub use upcast_fields::check_upcast_fields;
pub mod effects;
mod list_views;
pub use annotations::*;
pub use booleans::*;
pub use checks::*;
pub use counting::*;
// the declaration and list forms the checks and the inference know (lowering/definitions/declaration_forms.rs,
// lowering/collections/list_mutations.rs and filled_lists.rs)
use crate::lowering::declaration_forms::*;
use crate::lowering::filled_lists::*;
use crate::lowering::list_mutations::*;
pub(crate) use imports::signature_kind;
pub use imports::*;
pub use inference::*;
pub use inferred_kinds::{covers, inferred_kinds, kind_slots, KindSlots};
pub use main_globals::*;
pub use user_functions::*;
pub use variables::*;
