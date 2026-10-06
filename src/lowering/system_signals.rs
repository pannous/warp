//! Timers (notes/system_signals.md): `on every 5 seconds {body}` at the main level is the handler function
//! `on·every·0() := {global …; body}` and the call `signal_every(0, 5000)` where it is written, which starts the timer;
//! the runtime runs the handler at the program's check points (crates/warp-runtime/src/system_signals.rs) and, in
//! `warp run`, after main while the timer lives. The duration is constant (`50 ms`, `5 seconds`, `1 min`).

use crate::declarations::word;
use crate::diagnostic::Diagnostic;
use crate::event_signals::{function_with_globals, main_level_variables};
use crate::node::{Bracket, Node, Separator};

const ON_WORD: &str = "on";
const EVERY_WORD: &str = "every";

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	if !statements.iter().any(|statement| timer(statement).is_some()) {
		return program;
	}
	let main_variables = main_level_variables(&statements);
	let mut lowered = vec![];
	let mut count = 0;
	for statement in statements {
		let Some((duration, body)) = timer(&statement) else {
			lowered.push(statement);
			continue;
		};
		let Some(milliseconds) = crate::units::milliseconds(&duration) else {
			return Diagnostic::at(&statement, format!("on every needs a constant duration like `on every 5 seconds {{…}}`, got {}", duration.serialize().trim())).into_error();
		};
		let handler = format!("{}{count}", crate::host::TIMER_HANDLER_PREFIX);
		lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
		let start = [Node::Symbol(crate::host::SIGNAL_EVERY.to_string()), Node::int(count as i64), Node::int(milliseconds)];
		lowered.push(Node::List(start.to_vec(), Bracket::Round, Separator::None));
		count += 1;
	}
	Node::List(lowered, bracket, separator)
}

/// `on every 50 ms {body}`, `on every 5 seconds: body`: the duration and the body
fn timer(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [on, every, rest @ ..] = items.as_slice() else { return None };
	if word(on) != ON_WORD || word(every) != EVERY_WORD {
		return None;
	}
	let (last, duration_words) = rest.split_last()?;
	let (duration_end, body) = match last.drop_meta() {
		Node::List(_, Bracket::Curly, _) => (None, last.clone()),
		_ => {
			let (end, body) = crate::declarations::handler_parts(last)?;
			(Some(end), body)
		}
	};
	let words: Vec<Node> = duration_words.iter().cloned().chain(duration_end).collect();
	let duration = match words.as_slice() {
		[] => return None,
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	Some((duration, body))
}
