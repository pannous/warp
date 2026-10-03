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
/// A "got it" is remembered as the line `ack:<topic> = acknowledged` in the acknowledgements file
const ACKNOWLEDGED: &str = "acknowledged";
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
// Ask: an ambiguity the compiler warns about (notes/welcoming.md). It never asks which reading was meant and never
// remembers a choice: the same source always compiles the same way. A guessable ambiguity takes its default with a
// "got it" warning, a dangerous one is an error naming the explicit forms.
// ============================================================================

/// What an ambiguity becomes: a warning (take the default reading, continue) or an error (too dangerous to guess)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
	Warning,
	Error,
}

/// One reading of an ambiguous construct and the explicit wasp form that says it without ambiguity
/// (the seed of a later "change the code" action that rewrites `written` to it)
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
	pub meaning: String,
	pub explicit_form: String,
}

pub fn reading(meaning: &str, explicit_form: &str) -> Reading {
	Reading { meaning: meaning.to_string(), explicit_form: explicit_form.to_string() }
}

/// An ambiguity: its readings, the default reading and whether guessing it is allowed (`fallback`)
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
	/// Stable name of this kind of ambiguity (`upto`, `kotlin-range`): "got it" is remembered per topic
	pub topic: String,
	/// The ambiguous source text, which an explicit form replaces
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

	/// The diagnostic: the question, the default taken (or none), and how to say it explicitly
	fn diagnostic(&self) -> Diagnostic {
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

/// Who says "got it" to a warning or note so it is never shown again: the terminal in the CLI, the playground's
/// buttons, a list in tests. Nobody (the default) means the warning simply shows
pub trait Acknowledger {
	/// Whether the user said "got it" to `topic` before this run (a host keeping its own list, like the playground)
	fn has_acknowledged(&self, _topic: &str) -> bool {
		false
	}

	/// Asked once per run after a warning or note of `topic` was shown: does the user say "got it" now?
	fn acknowledge(&self, topic: &str) -> bool;
}

/// Acknowledges the listed topics, for tests
pub struct Acknowledging(pub Vec<String>);

impl Acknowledger for Acknowledging {
	fn acknowledge(&self, topic: &str) -> bool {
		self.0.iter().any(|acknowledged| acknowledged == topic)
	}
}

/// Asks "got it?" on the terminal after the warning or note
pub struct TerminalAcknowledger;

impl Acknowledger for TerminalAcknowledger {
	fn acknowledge(&self, _topic: &str) -> bool {
		eprint!("      got it? [y = don't tell me again, Enter = keep reminding] ");
		let mut line = String::new();
		std::io::stdin().read_line(&mut line).is_ok_and(|_| matches!(line.trim(), "y" | "Y" | "yes"))
	}
}

thread_local! {
	static ACKNOWLEDGER: std::cell::RefCell<Option<std::rc::Rc<dyn Acknowledger>>> = const { std::cell::RefCell::new(None) };
	static ACKNOWLEDGED_TOPICS: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
	static ACKNOWLEDGEMENTS_FILE: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
	static NOTES_SHOWN: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
	static ASSUMPTIONS: std::cell::RefCell<Vec<Diagnostic>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Who acknowledges the warnings and notes of later compilations on this thread; `None` (the default): nobody
pub fn set_acknowledger(acknowledger: Option<std::rc::Rc<dyn Acknowledger>>) {
	ACKNOWLEDGER.with(|current| *current.borrow_mut() = acknowledger);
}

/// Run `body` with another acknowledger and fresh acknowledgement and note memory, then restore all three
pub fn with_acknowledger<R>(acknowledger: impl Acknowledger + 'static, body: impl FnOnce() -> R) -> R {
	let previous = ACKNOWLEDGER.with(|current| current.replace(Some(std::rc::Rc::new(acknowledger))));
	let acknowledged = ACKNOWLEDGED_TOPICS.with(|topics| std::mem::take(&mut *topics.borrow_mut()));
	let file = ACKNOWLEDGEMENTS_FILE.with(|file| file.borrow_mut().take());
	let shown = NOTES_SHOWN.with(|notes| std::mem::take(&mut *notes.borrow_mut()));
	let result = body();
	ACKNOWLEDGER.with(|current| *current.borrow_mut() = previous);
	ACKNOWLEDGED_TOPICS.with(|topics| *topics.borrow_mut() = acknowledged);
	ACKNOWLEDGEMENTS_FILE.with(|current| *current.borrow_mut() = file);
	NOTES_SHOWN.with(|notes| *notes.borrow_mut() = shown);
	result
}

/// Remember "got it" across runs in `path` (one `ack:<topic> = acknowledged` per line), loading the ones already there
pub fn use_acknowledgements_file(path: impl Into<std::path::PathBuf>) {
	ACKNOWLEDGEMENTS_FILE.with(|file| *file.borrow_mut() = Some(path.into()));
	load_acknowledgements();
}

/// The topics acknowledged in the acknowledgements file, none without one
fn load_acknowledgements() {
	let saved = ACKNOWLEDGEMENTS_FILE.with(|file| file.borrow().as_ref().and_then(|path| std::fs::read_to_string(path).ok())).unwrap_or_default();
	let topics = saved.lines().filter_map(|line| line.split_once(" = "))
		.filter(|(_, value)| value.trim() == ACKNOWLEDGED)
		.filter_map(|(key, _)| key.trim().strip_prefix(ACKNOWLEDGED_PREFIX).map(str::to_string));
	ACKNOWLEDGED_TOPICS.with(|acknowledged| *acknowledged.borrow_mut() = topics.collect());
}

/// A new program (eval, compile) starts with no assumptions, no notes shown and only the acknowledgements of the
/// file: what one program assumed or showed never reaches the next one on the same thread
pub fn begin_program() {
	ASSUMPTIONS.with(|assumptions| assumptions.borrow_mut().clear());
	NOTES_SHOWN.with(|notes| notes.borrow_mut().clear());
	load_acknowledgements();
}

fn is_acknowledged(topic: &str) -> bool {
	let acknowledger = ACKNOWLEDGER.with(|current| current.borrow().clone());
	ACKNOWLEDGED_TOPICS.with(|acknowledged| acknowledged.borrow().contains(topic))
		|| acknowledger.is_some_and(|acknowledger| acknowledger.has_acknowledged(topic))
}

fn remember_acknowledged(topic: &str) {
	ACKNOWLEDGED_TOPICS.with(|acknowledged| acknowledged.borrow_mut().insert(topic.to_string()));
	let Some(path) = ACKNOWLEDGEMENTS_FILE.with(|file| file.borrow().clone()) else { return };
	let mut topics: Vec<String> = ACKNOWLEDGED_TOPICS.with(|acknowledged| acknowledged.borrow().iter().cloned().collect());
	topics.sort();
	let lines: String = topics.iter().map(|topic| format!("{ACKNOWLEDGED_PREFIX}{topic} = {ACKNOWLEDGED}\n")).collect();
	if let Err(problem) = std::fs::write(&path, lines) {
		eprintln!("warning: could not remember \"got it\" in {}: {problem}", path.display());
	}
}

/// After a warning or note of `topic` was shown, once per run: does the user say "got it"? Then it is never shown again
fn offer_acknowledgement(topic: &str) {
	if !NOTES_SHOWN.with(|shown| shown.borrow_mut().insert(topic.to_string())) {
		return;
	}
	let acknowledger = ACKNOWLEDGER.with(|current| current.borrow().clone());
	if acknowledger.is_some_and(|acknowledger| acknowledger.acknowledge(topic)) {
		remember_acknowledged(topic);
	}
}

/// Educate with "got it": the hint (`written` → `preferred`, and why) is shown once per run until the user
/// acknowledges it, then never again; it never blocks non-interactive runs, which just show the hint
pub fn educate_once(topic: &str, written: &str, preferred: &str, reason: &str) {
	let hints_off = crate::normalize::hint_mode() == crate::normalize::HintMode::Off;
	if hints_off || is_acknowledged(topic) || NOTES_SHOWN.with(|shown| shown.borrow().contains(topic)) {
		return;
	}
	crate::normalize::hint(written, preferred, reason);
	offer_acknowledgement(topic);
}

/// The reading the program gets: an error-fallback ambiguity is an error naming every explicit form; otherwise the
/// default, with a warning shown until the user says "got it" (an error under `use strict`, acknowledged or not)
pub fn ask(question: &Ask) -> Result<usize, Node> {
	let diagnostic = question.diagnostic();
	if question.fallback == Fallback::Error {
		return Err(diagnostic.into_error());
	}
	ASSUMPTIONS.with(|assumptions| assumptions.borrow_mut().push(diagnostic.clone()));
	if warning_mode() == WarningMode::Error {
		return Err(diagnostic.into_error());
	}
	if !is_acknowledged(&question.topic) {
		report(&[diagnostic])?;
		offer_acknowledgement(&question.topic);
	}
	Ok(question.default)
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
