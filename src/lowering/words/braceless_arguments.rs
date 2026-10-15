//! P68 (user): a braceless call whose argument holds arithmetic, `square 3 + square(4)`, reads as square(3 + square(4));
//! the reading stays, with a got-it warning naming both readings. Every function alike (card p68-warning): the
//! program's, a library word (`floor 2.5 + 1`, `sin 0 + 1`), an operator word (`sqrt 4 + 5`). Asked in negated_calls,
//! which knows which words are functions there, before the reals and constant passes fold such calls.

use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
use crate::lowering::negated_calls::PRINT_WORDS;
use crate::node::{Bracket, Node, Separator};

/// The got-it topic of `square 3 + square(4)`
const BRACELESS_ARGUMENT_TOPIC: &str = "braceless-call-operator";

/// `f 3 + 4` with a function `f`, or `sqrt 4 + 5`: the warning, or the error the got-it settings make of it
pub(crate) fn ask_braceless_argument(node: &Node, is_function: impl Fn(&Node) -> bool) -> Result<(), Node> {
	match node {
		Node::List(items, Bracket::None, Separator::Space) => match items.as_slice() {
			// `print 1 + 2` has one reading: nothing goes on with print's value
			[function, argument] if is_function(function) && !PRINT_WORDS.contains(&function.name().as_str()) => {
				ask_whole_argument(&function.name(), argument)
			}
			_ => Ok(()),
		},
		// the innermost Meta holds the operator word's mark: asked once, not again under each position around it
		Node::Meta { node: inner, .. } if !matches!(inner.as_ref(), Node::Meta { .. }) => {
			match crate::lowering::definitions::closures::unfolded_operator_word(node) {
				Some([word, argument]) => ask_whole_argument(&word.name(), &argument),
				None => Ok(()),
			}
		}
		_ => Ok(()),
	}
}

fn ask_whole_argument(name: &str, argument: &Node) -> Result<(), Node> {
	let Node::Key(left, op, right) = argument.drop_meta() else {
		return Ok(());
	};
	if !op.is_arithmetic() || matches!(left.drop_meta(), Node::Empty) {
		return Ok(());
	}
	let (left, right) = (call_text(left), call_text(right));
	let whole = format!("{name}({left} {op} {right})");
	let first = format!("{name}({left}) {op} {right}");
	let question = Ask::new(
		BRACELESS_ARGUMENT_TOPIC,
		format!("`{name} {left} {op} {right}` is {whole}, not {first}"),
		vec![reading("the whole expression", &whole), reading("only the first operand", &first)],
		Fallback::Warning,
	)
	.written(&format!("{name} {left} {op} {right}"))
	.at_node(argument);
	ask(&question).map(|_| ())
}

/// An operand as written in a warning: a call `f 4` / `f(4)` as f(4), anything else as normalize writes it
fn call_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::List(items, Bracket::None | Bracket::Round, _)
			if items.len() > 1 && matches!(items[0].drop_meta(), Node::Symbol(_)) =>
		{
			format!(
				"{}({})",
				items[0].drop_meta().name(),
				items[1..].iter().map(call_text).collect::<Vec<_>>().join(", ")
			)
		}
		_ => crate::normalize::operand_text(node),
	}
}
