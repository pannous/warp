//! Compiler diagnostics: what went wrong, where, and how to fix it.
//! Errors become `Node::Error` values (never panics); warnings (lints) are reported and compilation continues.

use crate::fixits::Fix;
use crate::node::Node;
use std::fmt;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diagnostic {
	pub message: String,
	pub line: usize,
	pub column: usize,
	/// How to fix it, in words (shown after `; fix: `)
	pub fix: Option<String>,
	/// The same as edits a host applies: one per reading the user might have meant (src/fixits.rs)
	pub fixes: Vec<Fix>,
	/// The kind of warning, which "got it" silences (an Ask's topic)
	pub topic: Option<String>,
	/// What "got it" for this expression only remembers: `topic@expression` (expression_key)
	pub expression_key: Option<String>,
}

impl Diagnostic {
	/// A diagnostic at the source position of `node`, else of its first part that has one (0:0 if the parser recorded none)
	pub fn at(node: &Node, message: impl Into<String>) -> Self {
		let (line, column) = position(node).unwrap_or_default();
		Diagnostic { message: message.into(), line, column, ..Default::default() }
	}

	pub fn message(self, message: impl Into<String>) -> Self {
		Diagnostic { message: message.into(), ..self }
	}

	pub fn fix(mut self, replacement: impl Into<String>) -> Self {
		self.fix = Some(replacement.into());
		self
	}

	/// Offer a reading as an applicable fix: `written` (as in the source, spacing aside) becomes `replacement`
	pub fn offer(self, meaning: impl Into<String>, written: impl Into<String>, replacement: impl Into<String>) -> Self {
		self.offering(crate::fixits::fix(meaning, written, replacement))
	}

	pub fn offering(mut self, offered: Fix) -> Self {
		if offered.changes_something() {
			self.fixes.push(offered);
		}
		self
	}

	/// The error, remembered with its fixes for a host to offer (take_error_diagnostics)
	pub fn into_error(self) -> Node {
		crate::node::error(&self.remembered())
	}

	/// The error's text, the error remembered with its fixes for a host to offer (an emitter error is a text)
	pub fn remembered(self) -> String {
		let text = self.to_string();
		if !self.fixes.is_empty() {
			note_said();
			ERROR_DIAGNOSTICS.with(|errors| errors.borrow_mut().push(self));
		}
		text
	}
}

/// The colors of terminal output (ANSI codes)
#[derive(Clone, Copy)]
pub enum Color {
	Green = 32,
	Yellow = 33,
	Cyan = 36,
	Gray = 90,
}

/// `text` in `color` when stderr is a terminal and NO_COLOR is unset; plain text in a file, a pipe or an editor's
/// output panel, which would show the raw escape codes. All colored output goes through here.
pub fn paint(color: Color, text: &str) -> String {
	static COLORED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
	let colored = *COLORED.get_or_init(|| std::io::IsTerminal::is_terminal(&std::io::stderr()) && std::env::var_os("NO_COLOR").is_none());
	if colored { format!("\x1b[{}m{text}\x1b[0m", color as u8) } else { text.to_string() }
}

/// Does a message end its description with a position ` at line:column`
pub fn names_position(message: &str) -> bool {
	message_position(message).is_some()
}

/// The position ` at line:column` that ends a message's description
pub fn message_position(message: &str) -> Option<(usize, usize)> {
	let description = message.split("; fix: ").next().unwrap_or(message);
	let (_, place) = description.rsplit_once(" at ")?;
	let (line, column) = place.split_once(':')?;
	let number = |digits: &str| (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| digits.parse().ok()).flatten();
	Some((number(line)?, number(column)?))
}

/// Where a failed program's error is: the position its message names (also for an error no diagnostic made)
pub fn error_position(result: &Node) -> Option<(usize, usize)> {
	let Node::Error(message) = result.drop_meta() else { return None };
	let Node::Text(message) = message.drop_meta() else { return None };
	message_position(message)
}

/// The source line at a position with `^^` under the word there (one `^` under any other character); tabs before it
/// stay tabs so the marks line up
pub fn excerpt(source: &str, line: usize, column: usize) -> Option<String> {
	let text = source.lines().nth(line.checked_sub(1)?)?;
	let before: Vec<char> = text.chars().take(column.checked_sub(1)?).collect();
	if before.len() + 1 != column {
		return None;
	}
	let is_word = |c: &char| c.is_alphanumeric() || *c == '_';
	let word = text.chars().skip(before.len()).take_while(is_word).count().max(1);
	let indent: String = before.iter().map(|&c| if c == '\t' { '\t' } else { ' ' }).collect();
	let gutter = " ".repeat(line.to_string().len());
	Some(format!("  {line} | {text}\n  {gutter} | {indent}{}", "^".repeat(word)))
}

/// Print the line a later warning or error names, from this program (the CLI's programs; a host shows them itself)
pub fn show_lines_of(source: &str) {
	*SHOWN_SOURCE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(source.to_string());
}

/// The line of the shown program at a position, with its marks
pub fn shown_excerpt(line: usize, column: usize) -> Option<String> {
	SHOWN_SOURCE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_deref().and_then(|source| excerpt(source, line, column))
}

/// The text of a node for a message: its serialization, or the source from its position to the end of the statement
/// when a lowering pass rebuilt it with compiler temporaries (`range_value·item`), which the program never wrote
pub fn written_text(node: &Node) -> String {
	let serialized = node.serialize().trim().to_string();
	if !serialized.contains(crate::analyzer::TEMPORARY_SEPARATOR) {
		return serialized;
	}
	written_statement(node).unwrap_or(serialized)
}

/// The source of the program from `node`'s position to the end of its statement, as the program wrote it
pub fn written_statement(node: &Node) -> Option<String> {
	let source_line = |(line, column): (usize, usize)| SOURCE.with(|source| {
		let rest: String = source.borrow().lines().nth(line.checked_sub(1)?)?.chars().skip(column.saturating_sub(1)).collect();
		Some(statement_prefix(&rest).trim().to_string())
	});
	position(node).and_then(source_line).filter(|text| !text.is_empty())
}

/// The start of `text` up to the end of its statement: a `;` or a closing bracket it did not open (`[cube 1..n]`)
fn statement_prefix(text: &str) -> &str {
	let mut depth = 0usize;
	for (offset, character) in text.char_indices() {
		match character {
			'(' | '[' | '{' => depth += 1,
			')' | ']' | '}' if depth == 0 => return &text[..offset],
			')' | ']' | '}' => depth -= 1,
			';' if depth == 0 => return &text[..offset],
			_ => {}
		}
	}
	text
}

/// The line and column the parser recorded for `node`, or for the first of its parts that has them: a node a lowering
/// pass rebuilt keeps the positions of the parts it was built from
pub fn position(node: &Node) -> Option<(usize, usize)> {
	if let Some(info) = node.get_lineinfo() {
		return Some((info.line_nr, info.column));
	}
	match node {
		Node::Meta { node, .. } => position(node),
		Node::Key(left, _, right) => position(left).or_else(|| position(right)),
		Node::List(items, _, _) => items.iter().find_map(position),
		_ => None,
	}
}

impl fmt::Display for Diagnostic {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.message)?;
		// 0:0 is no position: a diagnostic of something written nowhere in the source
		if self.line > 0 {
			write!(f, " at {}:{}", self.line, self.column)?;
		}
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
	/// Neither shown nor errors: a compile repeating one whose warnings were shown already
	Quiet,
}

/// `use strict` in wasp source makes warnings errors for that program
const STRICT_PRAGMA: [&str; 2] = [PRAGMA_WORD, "strict"];
const PRAGMA_WORD: &str = "use";
/// `use comments`: a comment before a binding becomes its meta information (P114, lowering/meta_entries.rs)
const COMMENTS_PRAGMA: &str = "comments";
/// A "got it" is remembered as the line `ack:<topic> = acknowledged` in the acknowledgements file
const ACKNOWLEDGED: &str = "acknowledged";
const ACKNOWLEDGED_PREFIX: &str = "ack:";

thread_local! {
	static WARNING_MODE: std::cell::Cell<WarningMode> = const { std::cell::Cell::new(WarningMode::Warn) };
	static RUNTIME_WARNINGS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
	static COMPILE_WARNINGS: std::cell::RefCell<Vec<Diagnostic>> = const { std::cell::RefCell::new(Vec::new()) };
	static ERROR_DIAGNOSTICS: std::cell::RefCell<Vec<Diagnostic>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The program the CLI runs: its warnings and errors print the line they name (show_lines_of); process-wide, as a
/// program may compile on another thread
static SHOWN_SOURCE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

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

/// Run `body` saying nothing (no warnings, hints or "got it?"): the compile of the executable a run leaves repeats the
/// run's compile, which said it all (card cli-running)
pub fn quietly<R>(body: impl FnOnce() -> R) -> R {
	with_warning_mode(WarningMode::Quiet, || crate::normalize::without_hints(body))
}

fn is_quiet() -> bool {
	warning_mode() == WarningMode::Quiet
}

thread_local! {
	/// How many diagnostics this thread has said (warnings, errors with fixes, assumptions, hints): a step that says
	/// something must run again to say it again (analysis_memo.rs remembers only silent analyses)
	static SAID: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn said() -> u64 {
	SAID.with(|said| said.get())
}

pub(crate) fn note_said() {
	SAID.with(|said| said.set(said.get() + 1));
}

/// Compile-time warnings: printed in Warn mode, the first one is the error in Error mode
pub fn report(warnings: &[Diagnostic]) -> Result<(), Node> {
	if !warnings.is_empty() {
		note_said();
	}
	match (warning_mode(), warnings.first()) {
		(WarningMode::Error, Some(first)) => Err(first.clone().into_error()),
		(WarningMode::Quiet, _) => Ok(()),
		_ => {
			for warning in warnings {
				eprintln!("warning: {warning}");
				if let Some(excerpt) = shown_excerpt(warning.line, warning.column) {
					eprintln!("{excerpt}");
				}
			}
			COMPILE_WARNINGS.with(|reported| reported.borrow_mut().extend_from_slice(warnings));
			Ok(())
		}
	}
}

/// The compile-time warnings reported on this thread since the last call (a host without a terminal shows them)
pub fn take_warnings() -> Vec<Diagnostic> {
	COMPILE_WARNINGS.with(|reported| std::mem::take(&mut *reported.borrow_mut()))
}

/// The errors with fixes made on this thread since the last call (the page offers their fixes when the program fails)
pub fn take_error_diagnostics() -> Vec<Diagnostic> {
	ERROR_DIAGNOSTICS.with(|errors| std::mem::take(&mut *errors.borrow_mut()))
}

/// A warning the running program reports with `warning(message)`
pub fn report_runtime_warning(message: &str) {
	note_said();
	eprintln!("warning: {message}");
	RUNTIME_WARNINGS.with(|warnings| warnings.borrow_mut().push(message.to_string()));
}

/// The runtime warnings reported on this thread since the last call
pub fn take_runtime_warnings() -> Vec<String> {
	RUNTIME_WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

/// `use <word>` as a statement: a pragma of the program
fn is_pragma(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(),
		[using, pragma] if matches!(using.drop_meta(), Node::Symbol(s) if s == PRAGMA_WORD) && matches!(pragma.drop_meta(), Node::Symbol(s) if s == word)))
}

/// The program without its top-level `use <word>` statements, and whether it had one
pub fn without_pragma(program: Node, word: &str) -> (Node, bool) {
	if is_pragma(&program, word) {
		return (Node::Empty, true);
	}
	match program {
		Node::List(items, bracket, separator) if items.iter().any(|item| is_pragma(item, word)) => {
			let rest = items.into_iter().filter(|item| !is_pragma(item, word)).collect();
			(Node::List(rest, bracket, separator), true)
		}
		Node::Meta { node, data } => {
			let (inner, found) = without_pragma(*node, word);
			(Node::Meta { node: Box::new(inner), data }, found)
		}
		other => (other, false),
	}
}

/// The program without its top-level `use strict` statements, and whether it had one
pub fn without_strict_pragma(program: Node) -> (Node, bool) {
	without_pragma(program, STRICT_PRAGMA[1])
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
	/// The edit that says this reading, when it is not the Ask's `written` text replaced by `explicit_form`
	/// (`let n =` for `n =`, where the explicit form shows `let n = …`)
	pub fix: Option<Fix>,
}

pub fn reading(meaning: &str, explicit_form: &str) -> Reading {
	Reading { meaning: meaning.to_string(), explicit_form: explicit_form.to_string(), fix: None }
}

impl Reading {
	pub fn replacing(self, written: impl Into<String>, replacement: impl Into<String>) -> Self {
		let fix = crate::fixits::fix(&self.meaning, written, replacement);
		self.fixed_by(fix)
	}

	pub fn fixed_by(self, fix: Fix) -> Self {
		Reading { fix: Some(fix), ..self }
	}
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
		let (line, column) = position(node).unwrap_or_default();
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
		let expression_key = Some(expression_key(&self.topic, &self.question));
		let diagnostic = Diagnostic { message, line: self.line, column: self.column, fix: Some(fix), topic: Some(self.topic.clone()), fixes: vec![], expression_key };
		let fixes = self.readings.iter().map(|reading| reading.fix.clone().unwrap_or_else(|| crate::fixits::fix(&reading.meaning, &self.written, &reading.explicit_form)));
		fixes.fold(diagnostic, Diagnostic::offering)
	}
}

/// The answer to "got it?" (user, 2026-10-05): this expression only (remembered by its written text), all of this
/// kind (the topic), or no (keep reminding)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GotIt {
	This,
	All,
	No,
}

/// Who says "got it" to a warning or note so it is never shown again: the terminal in the CLI, the playground's
/// buttons, a list in tests. Nobody (the default) means the warning simply shows
pub trait Acknowledger {
	/// Whether the user said "got it" to `key` before this run (a topic, or `topic@written` for one expression; a host
	/// keeping its own list, like the playground)
	fn has_acknowledged(&self, _key: &str) -> bool {
		false
	}

	/// Asked once per run after a warning or note of `topic` was shown for the `written` expression
	fn acknowledge(&self, topic: &str, written: &str) -> GotIt;
}

/// Acknowledges the listed topics for all expressions, for tests
pub struct Acknowledging(pub Vec<String>);

impl Acknowledger for Acknowledging {
	fn acknowledge(&self, topic: &str, _written: &str) -> GotIt {
		match self.0.iter().any(|acknowledged| acknowledged == topic) {
			true => GotIt::All,
			false => GotIt::No,
		}
	}
}

/// Asks "got it?" on the terminal after the warning or note
pub struct TerminalAcknowledger;

impl Acknowledger for TerminalAcknowledger {
	fn acknowledge(&self, _topic: &str, _written: &str) -> GotIt {
		eprint!("      got it? [y = this expression, a = all of this kind, n/Enter = keep reminding] ");
		let mut line = String::new();
		match std::io::stdin().read_line(&mut line).map(|_| line.trim().to_lowercase()).as_deref() {
			Ok("y" | "yes") => GotIt::This,
			Ok("a" | "all") => GotIt::All,
			_ => GotIt::No,
		}
	}
}

/// The key "got it" for one expression is remembered by: `topic@written`, on one line
pub fn expression_key(topic: &str, written: &str) -> String {
	format!("{topic}@{}", written.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// A line of the program that says `// got it`: its warnings and notes stay quiet (user, 2026-10-05)
const GOT_IT_COMMENT: &str = "// got it";

/// Whether the source line `line` (1-based) of the program being compiled carries a `// got it` comment
fn silenced_by_comment(line: usize) -> bool {
	line > 0 && SOURCE.with(|source| source.borrow().lines().nth(line - 1).is_some_and(|text| text.to_lowercase().contains(GOT_IT_COMMENT)))
}

thread_local! {
	static ACKNOWLEDGER: std::cell::RefCell<Option<std::rc::Rc<dyn Acknowledger>>> = const { std::cell::RefCell::new(None) };
	static ACKNOWLEDGED_TOPICS: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
	static ACKNOWLEDGEMENTS_FILE: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
	static NOTES_SHOWN: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
	static ASSUMPTIONS: std::cell::RefCell<Vec<Diagnostic>> = const { std::cell::RefCell::new(Vec::new()) };
	/// The source of the program being compiled, for its `// got it` comments
	static SOURCE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
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
/// The `ack:` lines of the answers file earlier versions wrote, appended to the acknowledgements file when it lacks
/// them, so acknowledged notes stay quiet after the rename
pub fn adopt_acknowledgements(old_file: &str, file: &str) {
	let Ok(old) = std::fs::read_to_string(old_file) else { return };
	let current = std::fs::read_to_string(file).unwrap_or_default();
	let is_acknowledgement = |line: &&str| line.split_once(" = ").is_some_and(|(key, value)| key.trim().starts_with(ACKNOWLEDGED_PREFIX) && value.trim() == ACKNOWLEDGED);
	let adopted: Vec<&str> = old.lines().filter(is_acknowledgement).filter(|line| !current.lines().any(|known| known.trim() == line.trim())).collect();
	if adopted.is_empty() {
		return;
	}
	let separator = if current.is_empty() || current.ends_with('\n') { "" } else { "\n" };
	if let Err(failure) = std::fs::write(file, format!("{current}{separator}{}\n", adopted.join("\n"))) {
		eprintln!("could not adopt the acknowledgements of {old_file}: {failure}");
	}
}

pub fn use_acknowledgements_file(path: impl Into<std::path::PathBuf>) {
	ACKNOWLEDGEMENTS_FILE.with(|file| *file.borrow_mut() = Some(path.into()));
	load_acknowledgements();
}

/// The topics acknowledged in the acknowledgements file, none without one
fn load_acknowledgements() {
	let saved = ACKNOWLEDGEMENTS_FILE.with(|file| file.borrow().as_ref().and_then(|path| std::fs::read_to_string(path).ok())).unwrap_or_default();
	let topics = saved.lines().filter_map(|line| line.rsplit_once(" = "))
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

/// Acknowledged for the whole topic, or for this written expression
fn is_acknowledged(topic: &str, written: &str) -> bool {
	let acknowledger = ACKNOWLEDGER.with(|current| current.borrow().clone());
	[topic.to_string(), expression_key(topic, written)].iter().any(|key| {
		ACKNOWLEDGED_TOPICS.with(|acknowledged| acknowledged.borrow().contains(key))
			|| acknowledger.as_ref().is_some_and(|acknowledger| acknowledger.has_acknowledged(key))
	})
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

/// After a warning or note of `topic` was shown, once per run: does the user say "got it", for this expression or all of
/// the kind? Then it is not shown again
fn offer_acknowledgement(topic: &str, written: &str) {
	if !NOTES_SHOWN.with(|shown| shown.borrow_mut().insert(topic.to_string())) {
		return;
	}
	let acknowledger = ACKNOWLEDGER.with(|current| current.borrow().clone());
	match acknowledger.map_or(GotIt::No, |acknowledger| acknowledger.acknowledge(topic, written)) {
		GotIt::This => remember_acknowledged(&expression_key(topic, written)),
		GotIt::All => remember_acknowledged(topic),
		GotIt::No => {}
	}
}

/// Educate with "got it": the hint (`written` → `preferred`, and why) is shown once per run until the user
/// acknowledges it, then never again; it never blocks non-interactive runs, which just show the hint
pub fn educate_once(topic: &str, written: &str, preferred: &str, reason: &str) {
	noted_once(topic, written, || crate::normalize::hint(written, preferred, reason));
}

/// `educate_once` with advice that does not replace `written` (`subscribe before the loop`)
pub fn advise_once(topic: &str, written: &str, preferred: &str, reason: &str) {
	noted_once(topic, written, || crate::normalize::advise(written, preferred, reason));
}

fn noted_once(topic: &str, written: &str, show: impl FnOnce()) {
	let hints_off = crate::normalize::hint_mode() == crate::normalize::HintMode::Off || is_quiet();
	if hints_off || is_acknowledged(topic, written) || silenced_by_comment(crate::normalize::hint_line()) || NOTES_SHOWN.with(|shown| shown.borrow().contains(topic)) {
		return;
	}
	show();
	offer_acknowledgement(topic, written);
}

/// Another language's word for a wasp word (`__add__` for `plus`, user 2026-10-06): it works, with a got-it note
/// naming the wasp word and its "I meant: <wasp word>" fix; a word wasp needs not at all (`data class`, `val x`) is
/// written with the word it stands before
pub fn note_alias(written: &str, wasp_word: &str) {
	let (foreign_word, reason) = match written.strip_suffix(wasp_word).map(str::trim) {
		Some(dropped) if !dropped.is_empty() => (dropped, format!("{dropped} is superfluous: write {wasp_word}")),
		_ => (written, format!("wasp says {wasp_word}")),
	};
	educate_once(&format!("alias-{foreign_word}"), written, wasp_word, &reason);
}

/// The reading the program gets: an error-fallback ambiguity is an error naming every explicit form; otherwise the
/// default, with a warning shown until the user says "got it" (an error under `use strict`, acknowledged or not)
pub fn ask(question: &Ask) -> Result<usize, Node> {
	note_said();
	let diagnostic = question.diagnostic();
	if question.fallback == Fallback::Error {
		return Err(diagnostic.into_error());
	}
	ASSUMPTIONS.with(|assumptions| assumptions.borrow_mut().push(diagnostic.clone()));
	if warning_mode() == WarningMode::Error {
		return Err(diagnostic.into_error());
	}
	// the expression "got it" for this one remembers: the question names it (`written` is only the replaced word, `upto`)
	let expression = &question.question;
	if !is_quiet() && !is_acknowledged(&question.topic, expression) && !silenced_by_comment(question.line) {
		report(&[diagnostic])?;
		offer_acknowledgement(&question.topic, expression);
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
	SOURCE.with(|source| *source.borrow_mut() = code.to_string());
	match code.lines().any(|line| line.split_whitespace().eq(STRICT_PRAGMA)) {
		true => with_warning_mode(WarningMode::Error, body),
		false => body(),
	}
}

/// Run `body` with warnings as errors when the program says `use strict`, and comments as meta information when it
/// says `use comments`
pub fn in_program_mode<R>(program: Node, body: impl FnOnce(Node) -> R) -> R {
	let (program, comments) = without_pragma(program, COMMENTS_PRAGMA);
	let previous = COMMENTS_AS_META.with(|flag| flag.replace(comments));
	let result = match without_strict_pragma(program) {
		(program, true) => with_warning_mode(WarningMode::Error, || body(program)),
		(program, false) => body(program),
	};
	COMMENTS_AS_META.with(|flag| flag.set(previous));
	result
}

thread_local! {
	static COMMENTS_AS_META: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Did the program being compiled say `use comments` (P114: off by default, the compiler pays nothing for comments)
pub fn comments_as_meta() -> bool {
	COMMENTS_AS_META.with(std::cell::Cell::get)
}
