//! `warp console`: the interactive session, its state carried from line to line
use super::*;

#[derive(Default)]
struct ConsoleSession {
	state: Vec<String>,
}

impl ConsoleSession {
	fn source_with(&self, input: &str) -> String {
		self.state
			.iter()
			.map(String::as_str)
			.chain([input])
			.collect::<Vec<_>>()
			.join("\n")
	}

	fn remember(&mut self, input: &str, value: &Node) {
		if !matches!(value.drop_meta(), Node::Error(_)) && console_state_line(&warp_parser::parse(input)) {
			self.state.push(input.to_string());
		}
	}
}

pub(crate) fn console_state_line(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, op, _) => {
			matches!(
				op,
				warp::operators::Op::Assign
					| warp::operators::Op::Define
					| warp::operators::Op::Inc
					| warp::operators::Op::Dec
			) || op.is_compound_assign()
		}
		Node::List(items, node::Bracket::None, separator) if separator.separates_statements() => {
			items.iter().all(console_state_line)
		}
		Node::List(items, node::Bracket::None, node::Separator::Space) => {
			matches!(
				items.first().and_then(Node::symbol_name),
				Some("let" | "var" | "const" | "global")
			) && items.iter().skip(1).any(console_state_line)
		}
		_ => false,
	}
}

pub(crate) fn console() {
	use rustyline::error::ReadlineError;
	use rustyline::DefaultEditor;

	println!("Interactive console (Ctrl+D to exit)");

	let mut rl = match DefaultEditor::new() {
		Ok(editor) => editor,
		Err(e) => {
			eprintln!("Failed to initialize readline: {}", e);
			return;
		}
	};

	// Load history if exists
	let history_path = dirs_home().join(".warp_history");
	let _ = rl.load_history(&history_path);

	let mut interrupted = false; // a ctrl-c on an empty line twice in a row ends the console
	let mut session = ConsoleSession::default();
	loop {
		let read = rl.readline("🌀"); // warp language symbol for prompt
		let interrupted_before = interrupted;
		interrupted = matches!(read, Err(ReadlineError::Interrupted));
		match read {
			Err(ReadlineError::Interrupted) if interrupted_before => break,
			Err(ReadlineError::Interrupted) => {
				println!("^C (again to exit)");
				continue;
			}
			Ok(line) => {
				let input = line.trim();
				if input.is_empty() {
					continue;
				}
				if input == "exit" || input == "quit" {
					break;
				}
				let _ = rl.add_history_entry(input);
				let value = eval(&session.source_with(input));
				session.remember(input, &value);
				show(&value, RESULT_MARK);
			}
			Err(ReadlineError::Eof) => break,
			Err(e) => {
				eprintln!("Error: {}", e);
				break;
			}
		}
	}

	// Save history
	let _ = rl.save_history(&history_path);
}
