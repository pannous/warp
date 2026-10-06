//! Timers (notes/system_signals.md): `on every 5 seconds {body}` at the main level is the handler function
//! `on·every·0() := {global …; body}` and the call `signal_every(0, 5000)` where it is written, which starts the timer;
//! the runtime runs the handler at the program's check points (crates/warp-runtime/src/system_signals.rs) and, in
//! `warp run`, after main while the timer lives. The duration is constant (`50 ms`, `5 seconds`, `1 min`).
//! `on file "notes.txt" change {body}` likewise is `on·file·0() := {…}` and `signal_watch(0, "notes.txt")`.
//! `exit` and `exit()` as statements are `exit(0)` (P121: the host word ends the run with that code).

use crate::declarations::word;
use crate::diagnostic::Diagnostic;
use crate::event_signals::{function_with_globals, main_level_variables};
use crate::node::{Bracket, Node, Separator};

const ON_WORD: &str = "on";
const EVERY_WORD: &str = "every";
const FILE_WORD: &str = "file";
const CHANGE_WORDS: [&str; 2] = ["change", "changes"];

pub fn lower(program: Node) -> Node {
	let program = if defines(&program, crate::host::EXIT) { program } else { bare_exits(program) };
	lower_timers(program)
}

fn lower_timers(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	if !statements.iter().any(|statement| timer(statement).is_some() || file_watch(statement).is_some()) {
		return program;
	}
	let main_variables = main_level_variables(&statements);
	let mut lowered = vec![];
	let (mut count, mut watches) = (0, 0);
	for statement in statements {
		if let Some((path, body)) = file_watch(&statement) {
			let handler = format!("{}{watches}", crate::host::FILE_HANDLER_PREFIX);
			lowered.push(function_with_globals(&handler, false, &[body], &main_variables));
			let start = [Node::Symbol(crate::host::SIGNAL_WATCH.to_string()), Node::int(watches as i64), path];
			lowered.push(Node::List(start.to_vec(), Bracket::Round, Separator::None));
			watches += 1;
			continue;
		}
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

/// `on file "notes.txt" change {body}`: the path (a text) and the body
fn file_watch(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [on, file, path, change, body] = items.as_slice() else { return None };
	let watched = word(on) == ON_WORD && word(file) == FILE_WORD && CHANGE_WORDS.contains(&word(change).as_str());
	let path_is_text = matches!(path.drop_meta(), Node::Text(_));
	(watched && path_is_text && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))).then(|| (path.clone(), body.clone()))
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

/// `exit` or `exit()` as a statement: `exit(0)`
fn bare_exits(node: Node) -> Node {
	let is_exit = |item: &Node| match item.drop_meta() {
		Node::Symbol(name) => name == crate::host::EXIT,
		Node::List(items, Bracket::Round, _) => items.len() == 1 && word(&items[0]) == crate::host::EXIT,
		_ => false,
	};
	let exit_zero = || Node::List(vec![Node::Symbol(crate::host::EXIT.to_string()), Node::int(0)], Bracket::Round, Separator::None);
	match node {
		Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(&bracket, &separator) || bracket == Bracket::Curly => {
			Node::List(items.into_iter().map(|item| if is_exit(&item) { exit_zero() } else { bare_exits(item) }).collect(), bracket, separator)
		}
		other => other.map_children(bare_exits),
	}
}

/// Does the program define or assign the name itself (`exit := …`, `def exit(…)`, `exit = …`)
fn defines(program: &Node, name: &str) -> bool {
	let mut defined = false;
	program.visit(&mut |part| if let Node::Key(target, op, _) = part {
		let head = match target.drop_meta() {
			Node::List(items, _, _) => items.first().map(word).unwrap_or_default(),
			other => word(other),
		};
		defined |= head == name && matches!(op, crate::operators::Op::Assign | crate::operators::Op::Define);
	});
	defined
}
