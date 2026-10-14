//! The `effects f` query: a function's effects as the program's answer
use super::*;

/// Rust API: resolved effects of `function` in `code`
pub fn effects_of(code: &str, function: &str) -> Option<EffectSet> {
	EffectReport::of(&crate::warp_parser::WarpParser::parse(code)).effects_of(function)
}

/// The effects and then the events: as a value texts like the other reflection words' answers (`f.params`), `"IO"`,
/// `["IO" "ask"]`, `"Pure"`; else symbols, `IO`, `(IO ask)`, `Pure`
/// `data (State IO)`: the symbols as a value, not as variables to read
pub(super) fn quoted(answer: Node) -> Node {
	match answer {
		Node::Error(_) => answer,
		_ => Node::List(
			vec![Node::Symbol(crate::lowering::blocks::DATA_WORD.into()), answer],
			Bracket::None,
			crate::node::Separator::Space,
		),
	}
}

/// P230: symbols, `(State IO)`, `Pure`
pub(super) fn effects_node(effects: EffectSet, events: &BTreeSet<String>) -> Node {
	let mut names: Vec<Node> =
		effects.iter().map(|effect| format!("{effect:?}")).chain(events.iter().cloned()).map(Node::Symbol).collect();
	match names.len() {
		0 => Node::Symbol(PURE.into()),
		1 => names.remove(0),
		_ => Node::List(names, Bracket::Round, crate::node::Separator::Space),
	}
}

/// `IO, ask`, `Pure`
pub(super) fn effect_names(effects: EffectSet, events: &BTreeSet<String>) -> String {
	let names: Vec<String> = effects.iter().map(|effect| format!("{effect:?}")).chain(events.iter().cloned()).collect();
	if names.is_empty() {
		PURE.to_string()
	} else {
		names.join(", ")
	}
}

pub(super) fn statements_before_last(program: &Node) -> Node {
	match program.drop_meta() {
		Node::List(items, bracket, separator) if !items.is_empty() => {
			Node::List(items[..items.len() - 1].to_vec(), bracket.clone(), separator.clone())
		}
		_ => Node::Empty,
	}
}

pub(super) fn last_statement(program: &Node) -> &Node {
	match program.drop_meta() {
		Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(bracket, separator) => {
			items.last().unwrap_or(program)
		}
		_ => program,
	}
}

/// `f.effects` (`effects of f`) → `f`
pub(super) fn effects_query(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::Key(function, Op::Dot, word) if word.is_symbol(EFFECTS_WORD) => {
			function.symbol_name().map(str::to_string)
		}
		_ => None,
	}
}
