//! Compiler diagnostics: what went wrong, where, and how to fix it.
//! Errors become `Node::Error` values (never panics); warnings (lints) are reported and compilation continues.

use crate::node::Node;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
	pub message: String,
	pub line: usize,
	pub column: usize,
	pub fix: Option<String>,
}

impl Diagnostic {
	/// A diagnostic at the source position of `node` (0:0 if the parser recorded none)
	pub fn at(node: &Node, message: impl Into<String>) -> Self {
		let (line, column) = node.get_lineinfo().map(|info| (info.line_nr, info.column)).unwrap_or_default();
		Diagnostic { message: message.into(), line, column, fix: None }
	}

	pub fn fix(mut self, replacement: impl Into<String>) -> Self {
		self.fix = Some(replacement.into());
		self
	}

	pub fn into_error(self) -> Node {
		crate::node::error(&self.to_string())
	}
}

impl fmt::Display for Diagnostic {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} at {}:{}", self.message, self.line, self.column)?;
		if let Some(fix) = &self.fix {
			write!(f, "; fix: {}", fix)?;
		}
		Ok(())
	}
}

/// Whether warnings (lints, `warning(message)`) are reported and compilation continues, or are errors
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WarningMode {
	#[default]
	Warn,
	Error,
}

/// `use strict` in wasp source makes warnings errors for that program
const STRICT_PRAGMA: [&str; 2] = ["use", "strict"];
/// The remembered answer of an acknowledged `educate_once` note, stored under `ack:<topic>`
pub const ACKNOWLEDGED: &str = "acknowledged";
const ACKNOWLEDGED_PREFIX: &str = "ack:";

thread_local! {
	static WARNING_MODE: std::cell::Cell<WarningMode> = const { std::cell::Cell::new(WarningMode::Warn) };
	static RUNTIME_WARNINGS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
	static COMPILE_WARNINGS: std::cell::RefCell<Vec<Diagnostic>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The warning mode of every later compilation on this thread (the CLI's `--strict`)
pub fn set_warning_mode(mode: WarningMode) {
	WARNING_MODE.with(|current| current.set(mode));
}

pub fn warning_mode() -> WarningMode {
	WARNING_MODE.with(|current| current.get())
}

/// Run `body` in another warning mode, then restore the previous one
pub fn with_warning_mode<R>(mode: WarningMode, body: impl FnOnce() -> R) -> R {
	let previous = WARNING_MODE.with(|current| current.replace(mode));
	let result = body();
	set_warning_mode(previous);
	result
}

/// Compile-time warnings: printed in Warn mode, the first one is the error in Error mode
pub fn report(warnings: &[Diagnostic]) -> Result<(), Node> {
	match (warning_mode(), warnings.first()) {
		(WarningMode::Error, Some(first)) => Err(first.clone().into_error()),
		_ => {
			warnings.iter().for_each(|warning| eprintln!("warning: {warning}"));
			COMPILE_WARNINGS.with(|reported| reported.borrow_mut().extend_from_slice(warnings));
			Ok(())
		}
	}
}

/// The compile-time warnings reported on this thread since the last call (a host without a terminal shows them)
pub fn take_warnings() -> Vec<Diagnostic> {
	COMPILE_WARNINGS.with(|reported| std::mem::take(&mut *reported.borrow_mut()))
}

/// A warning the running program reports with `warning(message)`
pub fn report_runtime_warning(message: &str) {
	eprintln!("warning: {message}");
	RUNTIME_WARNINGS.with(|warnings| warnings.borrow_mut().push(message.to_string()));
}

/// The runtime warnings reported on this thread since the last call
pub fn take_runtime_warnings() -> Vec<String> {
	RUNTIME_WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

fn is_strict_pragma(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.len() == 2
		&& items.iter().zip(STRICT_PRAGMA).all(|(item, word)| matches!(item.drop_meta(), Node::Symbol(s) if s == word)))
}

/// The program without its top-level `use strict` statements, and whether it had one
pub fn without_strict_pragma(program: Node) -> (Node, bool) {
	if is_strict_pragma(&program) {
		return (Node::Empty, true);
	}
	match program {
		Node::List(items, bracket, separator) if items.iter().any(is_strict_pragma) => {
			let rest = items.into_iter().filter(|item| !is_strict_pragma(item)).collect();
			(Node::List(rest, bracket, separator), true)
		}
		Node::Meta { node, data } => {
			let (inner, strict) = without_strict_pragma(*node);
			(Node::Meta { node: Box::new(inner), data }, strict)
		}
		other => (other, false),
	}
}

// ============================================================================
// Ask: an ambiguity the compiler asks the user about (notes/welcoming.md)
// ============================================================================

/// What an unanswerable Ask becomes: a warning (take the default reading, continue) or an error (too dangerous to guess)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
	Warning,
	Error,
}

/// One reading of an ambiguous construct and the explicit wasp form that says it without ambiguity
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
	pub meaning: String,
	pub explicit_form: String,
}

pub fn reading(meaning: &str, explicit_form: &str) -> Reading {
	Reading { meaning: meaning.to_string(), explicit_form: explicit_form.to_string() }
}

/// An ambiguity: asked interactively when a user can be reached, otherwise its `fallback` applies
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
	/// Stable name of this kind of ambiguity (`upto`, `kotlin-range`): answers are remembered per topic
	pub topic: String,
	/// The ambiguous source text, which the answer's explicit form replaces
	pub written: String,
	pub question: String,
	pub readings: Vec<Reading>,
	pub default: usize,
	pub fallback: Fallback,
	pub line: usize,
	pub column: usize,
}

impl Ask {
	pub fn new(topic: &str, question: impl Into<String>, readings: Vec<Reading>, fallback: Fallback) -> Self {
		Ask { topic: topic.to_string(), written: topic.to_string(), question: question.into(), readings, default: 0, fallback, line: 0, column: 0 }
	}

	pub fn written(self, source_text: &str) -> Self {
		Ask { written: source_text.to_string(), ..self }
	}

	pub fn at(self, line: usize, column: usize) -> Self {
		Ask { line, column, ..self }
	}

	pub fn at_node(self, node: &Node) -> Self {
		let (line, column) = node.get_lineinfo().map(|info| (info.line_nr, info.column)).unwrap_or_default();
		self.at(line, column)
	}

	/// The fallback diagnostic: the question, the default taken (or none), and how to say it explicitly
	fn unanswered(&self) -> Diagnostic {
		let (message, fix) = match self.fallback {
			Fallback::Warning => {
				let default = &self.readings[self.default];
				(format!("{} (taking {})", self.question, default.meaning), default.explicit_form.clone())
			}
			Fallback::Error => {
				let forms: Vec<String> = self.readings.iter().map(|reading| format!("`{}` for {}", reading.explicit_form, reading.meaning)).collect();
				(format!("{} (too ambiguous to guess)", self.question), forms.join(" or "))
			}
		};
		Diagnostic { message, line: self.line, column: self.column, fix: Some(fix) }
	}
}

/// Who answers an Ask: a terminal prompt in the CLI, scripted answers in tests, later a web page or an IDE.
/// `None` means no user can be reached and the Ask falls back.
pub trait Asker {
	fn answer(&self, ask: &Ask) -> Option<usize>;

	/// Whether the user acknowledges a note shown by `educate_once`, so it is never shown again
	fn acknowledge(&self, _topic: &str) -> bool {
		false
	}
}

/// Answers by topic, for tests: the reading whose meaning or explicit form is the scripted text;
/// the answer `ACKNOWLEDGED` acknowledges the note of that topic
pub struct ScriptedAnswers(pub Vec<(String, String)>);

impl ScriptedAnswers {
	fn scripted(&self, topic: &str) -> Option<&String> {
		self.0.iter().find(|(scripted_topic, _)| scripted_topic == topic).map(|(_, answer)| answer)
	}
}

impl Asker for ScriptedAnswers {
	fn answer(&self, ask: &Ask) -> Option<usize> {
		let wanted = self.scripted(&ask.topic)?;
		ask.readings.iter().position(|reading| reading.meaning == *wanted || reading.explicit_form == *wanted)
	}

	fn acknowledge(&self, topic: &str) -> bool {
		self.scripted(topic).is_some_and(|answer| answer == ACKNOWLEDGED)
	}
}

/// Asks on the terminal: the readings numbered, the default marked, Enter takes the default
pub struct TerminalAsker;

impl Asker for TerminalAsker {
	fn answer(&self, ask: &Ask) -> Option<usize> {
		eprintln!("\x1b[35mask\x1b[0m {}:{}: {}", ask.line, ask.column, ask.question);
		for (number, reading) in ask.readings.iter().enumerate() {
			let marker = if number == ask.default { " (default)" } else { "" };
			eprintln!("  [{}] {}: `{}`{}", number + 1, reading.meaning, reading.explicit_form, marker);
		}
		eprint!("which did you mean? [1-{}, Enter = {}] ", ask.readings.len(), ask.default + 1);
		let mut line = String::new();
		std::io::stdin().read_line(&mut line).ok().filter(|&bytes| bytes > 0)?; // end of input: nobody answers
		match line.trim() {
			"" => Some(ask.default),
			choice => choice.parse::<usize>().ok().filter(|n| (1..=ask.readings.len()).contains(n)).map(|n| n - 1),
		}
	}

	fn acknowledge(&self, _topic: &str) -> bool {
		eprint!("      got it? [y = don't tell me again, Enter = keep reminding] ");
		let mut line = String::new();
		std::io::stdin().read_line(&mut line).is_ok_and(|_| matches!(line.trim(), "y" | "Y" | "yes"))
	}
}

thread_local! {
	static ASKER: std::cell::RefCell<Option<std::rc::Rc<dyn Asker>>> = const { std::cell::RefCell::new(None) };
	static ANSWERS: std::cell::RefCell<std::collections::HashMap<String, String>> = std::cell::RefCell::new(Default::default());
	static ANSWERS_FILE: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
	static NOTES_SHOWN: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
	static ASSUMPTIONS: std::cell::RefCell<Vec<Diagnostic>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Who answers the Asks of later compilations on this thread; `None` (the default) never asks, every Ask falls back
pub fn set_asker(asker: Option<std::rc::Rc<dyn Asker>>) {
	ASKER.with(|current| *current.borrow_mut() = asker);
}

/// Run `body` with another asker and fresh answer and note memory, then restore all three
pub fn with_asker<R>(asker: impl Asker + 'static, body: impl FnOnce() -> R) -> R {
	let previous = ASKER.with(|current| current.replace(Some(std::rc::Rc::new(asker))));
	let remembered = ANSWERS.with(|answers| std::mem::take(&mut *answers.borrow_mut()));
	let shown = NOTES_SHOWN.with(|notes| std::mem::take(&mut *notes.borrow_mut()));
	let result = body();
	ASKER.with(|current| *current.borrow_mut() = previous);
	ANSWERS.with(|answers| *answers.borrow_mut() = remembered);
	NOTES_SHOWN.with(|notes| *notes.borrow_mut() = shown);
	result
}

/// Remember answers across runs in `path` (one `topic = explicit form` per line), loading the ones already there
pub fn use_answers_file(path: impl Into<std::path::PathBuf>) {
	let path = path.into();
	let saved = std::fs::read_to_string(&path).unwrap_or_default();
	ANSWERS.with(|answers| {
		let mut answers = answers.borrow_mut();
		for (topic, form) in saved.lines().filter_map(|line| line.split_once(" = ")) {
			answers.insert(topic.trim().to_string(), form.trim().to_string());
		}
	});
	ANSWERS_FILE.with(|file| *file.borrow_mut() = Some(path));
}

fn remember(topic: &str, explicit_form: &str) {
	ANSWERS.with(|answers| answers.borrow_mut().insert(topic.to_string(), explicit_form.to_string()));
	let Some(path) = ANSWERS_FILE.with(|file| file.borrow().clone()) else { return };
	let lines: String = ANSWERS.with(|answers| {
		let answers = answers.borrow();
		let mut topics: Vec<_> = answers.keys().collect();
		topics.sort();
		topics.iter().map(|topic| format!("{} = {}\n", topic, answers[*topic])).collect()
	});
	if let Err(problem) = std::fs::write(&path, lines) {
		eprintln!("warning: could not remember the answer in {}: {problem}", path.display());
	}
}

fn remembered_answer(topic: &str) -> Option<String> {
	ANSWERS.with(|answers| answers.borrow().get(topic).cloned())
}

fn remembered(ask: &Ask) -> Option<usize> {
	let form = remembered_answer(&ask.topic)?;
	ask.readings.iter().position(|reading| reading.explicit_form == form)
}

/// Educate with acknowledge-once: the hint (`written` → `preferred`, and why) is shown once per run until the user
/// acknowledges it; the acknowledgement is remembered like an answer (`ack:<topic> = acknowledged`), never blocks
/// non-interactive runs, which just show the hint
pub fn educate_once(topic: &str, written: &str, preferred: &str, reason: &str) {
	let key = format!("{ACKNOWLEDGED_PREFIX}{topic}");
	let first_this_run = NOTES_SHOWN.with(|shown| shown.borrow_mut().insert(key.clone()));
	let hints_off = crate::normalize::hint_mode() == crate::normalize::HintMode::Off;
	if !first_this_run || hints_off || remembered_answer(&key).is_some() {
		return;
	}
	crate::normalize::hint(written, preferred, reason);
	let asker = ASKER.with(|current| current.borrow().clone());
	if asker.is_some_and(|asker| asker.acknowledge(topic)) {
		remember(&key, ACKNOWLEDGED);
	}
}

/// The reading the user means: remembered, asked, or — when nobody can be asked — the fallback:
/// a warning taking the default (an error under `use strict`), or an error
pub fn ask(question: &Ask) -> Result<usize, Node> {
	let answered = remembered(question).or_else(|| {
		let asker = ASKER.with(|current| current.borrow().clone())?;
		let chosen = asker.answer(question)?;
		remember(&question.topic, &question.readings[chosen].explicit_form);
		Some(chosen)
	});
	if let Some(chosen) = answered {
		let explicit_form = &question.readings[chosen].explicit_form;
		crate::normalize::hint(&question.written, explicit_form, "as you answered; the explicit form needs no question");
		return Ok(chosen);
	}
	match question.fallback {
		Fallback::Warning => {
			let assumption = question.unanswered();
			ASSUMPTIONS.with(|assumptions| assumptions.borrow_mut().push(assumption.clone()));
			report(&[assumption]).map(|_| question.default)
		}
		Fallback::Error => Err(question.unanswered().into_error()),
	}
}

/// The defaults unanswered Asks took on this thread since the last call: a later runtime error names them,
/// since a wrong guess there typically fails far away from the ambiguous source
pub fn take_assumptions() -> Vec<Diagnostic> {
	ASSUMPTIONS.with(|assumptions| std::mem::take(&mut *assumptions.borrow_mut()))
}

/// Run `body` (the parser) with warnings as errors when a line of the source says `use strict`,
/// so the warnings of parse-time Asks obey it too
pub fn in_source_mode<R>(code: &str, body: impl FnOnce() -> R) -> R {
	match code.lines().any(|line| line.split_whitespace().eq(STRICT_PRAGMA)) {
		true => with_warning_mode(WarningMode::Error, body),
		false => body(),
	}
}

/// Run `body` with warnings as errors when the program says `use strict`
pub fn in_program_mode<R>(program: Node, body: impl FnOnce(Node) -> R) -> R {
	match without_strict_pragma(program) {
		(program, true) => with_warning_mode(WarningMode::Error, || body(program)),
		(program, false) => body(program),
	}
}
