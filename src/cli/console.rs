//! `warp console`: the interactive session, its state carried from line to line
use super::*;

#[derive(Default)]
struct ConsoleSession {
	state: Vec<String>,
}

impl ConsoleSession {
	fn source_with(&self, input: &str) -> String {
		self.state.iter().map(String::as_str).chain([input]).collect::<Vec<_>>().join("\n")
	}

	fn remember(&mut self, input: &str, value: &Node) {
		if !matches!(value.drop_meta(), Node::Error(_)) && console_state_line(&warp_parser::parse(input)) {
			self.state.push(input.to_string());
		}
	}
}

/// Tab after `receiver.` lists the receiver's members (card member-completion, lowering/data/members.rs), knowing the
/// session's variables: `xs = [1 2]` then `xs.` Tab lists count, sum, map …
impl rustyline::completion::Completer for ConsoleSession {
	type Candidate = String;

	fn complete(
		&self,
		line: &str,
		position: usize,
		_: &rustyline::Context<'_>,
	) -> rustyline::Result<(usize, Vec<String>)> {
		let typed = &line[..position];
		let word_start =
			typed.trim_end_matches(|character: char| character.is_alphanumeric() || character == '_').len();
		let Some(receiver) = typed[..word_start].strip_suffix('.') else {
			return Ok((position, vec![]));
		};
		let prefix = &typed[word_start..];
		let names = warp::members::member_names(&self.source_with(receiver));
		Ok((word_start, names.into_iter().filter(|name| name.starts_with(prefix)).collect()))
	}
}

impl rustyline::hint::Hinter for ConsoleSession {
	type Hint = String;
}
impl rustyline::highlight::Highlighter for ConsoleSession {}
impl rustyline::validate::Validator for ConsoleSession {}
impl rustyline::Helper for ConsoleSession {}

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
			matches!(items.first().and_then(Node::symbol_name), Some("let" | "var" | "const" | "global"))
				&& items.iter().skip(1).any(console_state_line)
		}
		_ => false,
	}
}

pub(crate) fn console() {
	use rustyline::error::ReadlineError;
	use rustyline::{CompletionType, Config, Editor};

	println!("Interactive console (Ctrl+D to exit)");

	// one Tab lists the members, as bash does with show-all-if-ambiguous
	let config = Config::builder().completion_type(CompletionType::List).completion_show_all_if_ambiguous(true).build();
	let mut rl = match Editor::<ConsoleSession, rustyline::history::DefaultHistory>::with_config(config) {
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
	rl.set_helper(Some(ConsoleSession::default()));
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
				let session = rl.helper_mut().expect("the console's helper is set above");
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
