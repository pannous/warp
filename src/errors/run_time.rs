//! Errors a program raised while running: the messages for traps and `raise`, and the hints added to them.

use crate::node::{Bracket, Node};
use crate::operators::Op;

pub(crate) const INDEX_OUT_OF_RANGE: &str = "index out of range";
pub(crate) const UNWRAPPED_EMPTY: &str = "unwrapped ø";
pub(crate) const RANGE_END_HINT: &str = "hint: `..` excludes the end; `...` or `to` include it";
/// The word that raises an error, as hints write it
pub(crate) const FAIL_WORD: &str = "fail";
/// The words of an exception (library_words lowers them to the call raise(X)), unless the program defines one
pub(crate) const RAISE_WORDS: [&str; 4] = ["raise", "throw", FAIL_WORD, "panic"];

pub(crate) fn has_exclusive_range(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Range, _) => true,
		Node::Key(left, _, right) => has_exclusive_range(left) || has_exclusive_range(right),
		Node::List(items, _, _) => items.iter().any(has_exclusive_range),
		_ => false,
	}
}

/// A runtime error fails far from its cause: it names the range hint and the defaults unanswered Asks took
pub(crate) fn explain_runtime_error(result: Node, exclusive_range: bool) -> Node {
	let assumptions = crate::errors::diagnostic::take_assumptions();
	let Node::Error(reason) = &result else { return result };
	let Node::Text(message) = reason.drop_meta() else {
		return result;
	};
	let mut lines = vec![message.clone()];
	if exclusive_range && message.starts_with(INDEX_OUT_OF_RANGE) {
		lines.push(RANGE_END_HINT.to_string());
	}
	let mut assumed: Vec<(String, Vec<String>)> = Vec::new();
	for assumption in assumptions {
		let guess = format!("{}; fix: {}", assumption.message, assumption.fix.unwrap_or_default());
		let position = format!("{}:{}", assumption.line, assumption.column);
		match assumed.iter_mut().find(|(known, _)| *known == guess) {
			Some((_, positions)) => positions.push(position),
			None => assumed.push((guess, vec![position])),
		}
	}
	lines.extend(assumed.into_iter().map(|(guess, positions)| format!("assumed at {}: {guess}", positions.join(", "))));
	match lines.len() {
		1 => result,
		_ => crate::node::error(&lines.join("\n  ")),
	}
}

/// The message of `error "…"`, or of `raise …`, which always fails the run
pub(crate) fn returned_error_message(value: &Node) -> Option<&Node> {
	match value.drop_meta() {
		Node::List(items, Bracket::Round, _)
			if items.len() == 2
				&& matches!(items[0].drop_meta(), Node::Symbol(word) if word == "error" || word == crate::words::RAISE_WORD) =>
		{
			Some(&items[1])
		}
		// `raise X` / `throw X` before library_words makes them the call raise(X)
		Node::List(items, Bracket::None, _)
			if items.len() == 2
				&& matches!(items[0].drop_meta(), Node::Symbol(word) if RAISE_WORDS.contains(&word.as_str())) =>
		{
			Some(&items[1])
		}
		_ => None,
	}
}

/// The run computed past its time limit: it probably does not terminate, or needs more time
pub fn out_of_time(limit: std::time::Duration) -> Node {
	crate::node::error(&format!(
		"out of time after {} s of computing in one turn: the program may not terminate (give it more with {}=<seconds>, or count steps with warp fuel <steps>)",
		limit.as_secs_f64(),
		crate::util::TIME_LIMIT_VARIABLE))
}

/// The run used up its fuel: it probably does not terminate, or needs a larger budget
pub fn out_of_fuel(steps: u64) -> Node {
	crate::node::error(&format!(
		"out of fuel after {steps} steps: the program may not terminate (raise the budget with {}=<steps> or warp fuel <steps>)",
		crate::util::FUEL_VARIABLE))
}
