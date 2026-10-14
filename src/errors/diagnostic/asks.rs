//! Ask: an ambiguity the compiler warns about (notes/welcoming.md). It never asks which reading was meant and never
//! remembers a choice: the same source always compiles the same way. A guessable ambiguity takes its default with a
//! "got it" warning, a dangerous one is an error naming the explicit forms.
use super::*;

/// What an ambiguity becomes: a warning (take the default reading, continue) or an error (too dangerous to guess)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
	Warning,
	Error,
}

/// One reading of an ambiguous construct and the explicit warp form that says it without ambiguity
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
		let fix = crate::errors::fixits::fix(&self.meaning, written, replacement);
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
		Ask {
			topic: topic.to_string(),
			written: topic.to_string(),
			question: question.into(),
			readings,
			default: 0,
			fallback,
			line: 0,
			column: 0,
		}
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
				let forms: Vec<String> = self
					.readings
					.iter()
					.map(|reading| format!("`{}` for {}", reading.explicit_form, reading.meaning))
					.collect();
				(format!("{} (too ambiguous to guess)", self.question), forms.join(" or "))
			}
		};
		// the expression "got it" for this one remembers: the question names it (`written` is only the replaced word, `upto`)
		let diagnostic =
			Diagnostic { message, line: self.line, column: self.column, fix: Some(fix), ..Default::default() }
				.about(&self.topic, &self.question);
		let fixes = self.readings.iter().map(|reading| {
			reading
				.fix
				.clone()
				.unwrap_or_else(|| crate::errors::fixits::fix(&reading.meaning, &self.written, &reading.explicit_form))
		});
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
pub(super) fn silenced_by_comment(line: usize) -> bool {
	line > 0
		&& compile_context::read(|context| {
			context.source.lines().nth(line - 1).is_some_and(|text| text.to_lowercase().contains(GOT_IT_COMMENT))
		})
}

thread_local! {
	static ACKNOWLEDGER: std::cell::RefCell<Option<std::rc::Rc<dyn Acknowledger>>> = const { std::cell::RefCell::new(None) };
	static ACKNOWLEDGED_TOPICS: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
	static ACKNOWLEDGEMENTS_FILE: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
	static NOTES_SHOWN: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
}

/// Who acknowledges the warnings and notes of later compilations on this thread; `None` (the default): nobody
pub fn set_acknowledger(acknowledger: Option<std::rc::Rc<dyn Acknowledger>>) {
	ACKNOWLEDGER.with(|current| *current.borrow_mut() = acknowledger);
}

/// Run `body` with another acknowledger and fresh acknowledgement and note memory, then restore all three
pub fn with_acknowledger<R>(acknowledger: impl Acknowledger + 'static, body: impl FnOnce() -> R) -> R {
	use crate::util::scoped;
	scoped(&ACKNOWLEDGER, Some(std::rc::Rc::new(acknowledger)), || {
		scoped(&ACKNOWLEDGED_TOPICS, Default::default(), || {
			scoped(&ACKNOWLEDGEMENTS_FILE, None, || scoped(&NOTES_SHOWN, Default::default(), body))
		})
	})
}

/// Remember "got it" across runs in `path` (one `ack:<topic> = acknowledged` per line), loading the ones already there
/// The `ack:` lines of the answers file earlier versions wrote, appended to the acknowledgements file when it lacks
/// them, so acknowledged notes stay quiet after the rename
pub fn adopt_acknowledgements(old_file: &str, file: impl AsRef<std::path::Path>) {
	let file = file.as_ref();
	let Ok(old) = std::fs::read_to_string(old_file) else {
		return;
	};
	let current = std::fs::read_to_string(file).unwrap_or_default();
	let is_acknowledgement = |line: &&str| {
		line.split_once(" = ")
			.is_some_and(|(key, value)| key.trim().starts_with(ACKNOWLEDGED_PREFIX) && value.trim() == ACKNOWLEDGED)
	};
	let adopted: Vec<&str> = old
		.lines()
		.filter(is_acknowledgement)
		.filter(|line| !current.lines().any(|known| known.trim() == line.trim()))
		.collect();
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
	let saved = ACKNOWLEDGEMENTS_FILE
		.with(|file| file.borrow().as_ref().and_then(|path| std::fs::read_to_string(path).ok()))
		.unwrap_or_default();
	let topics = saved
		.lines()
		.filter_map(|line| line.rsplit_once(" = "))
		.filter(|(_, value)| value.trim() == ACKNOWLEDGED)
		.filter_map(|(key, _)| key.trim().strip_prefix(ACKNOWLEDGED_PREFIX).map(str::to_string));
	ACKNOWLEDGED_TOPICS.with(|acknowledged| *acknowledged.borrow_mut() = topics.collect());
}

/// A new program (eval, compile) starts with no assumptions, no notes shown and only the acknowledgements of the
/// file: what one program assumed or showed never reaches the next one on the same thread
pub fn begin_program() {
	diagnostics(|diagnostics| diagnostics.assumptions.clear());
	NOTES_SHOWN.with(|notes| notes.borrow_mut().clear());
	load_acknowledgements();
}

/// Acknowledged for the whole topic, or for this written expression
pub(crate) fn is_acknowledged(topic: &str, written: &str) -> bool {
	let acknowledger = ACKNOWLEDGER.with(|current| current.borrow().clone());
	[topic.to_string(), expression_key(topic, written)].iter().any(|key| {
		ACKNOWLEDGED_TOPICS.with(|acknowledged| acknowledged.borrow().contains(key))
			|| acknowledger.as_ref().is_some_and(|acknowledger| acknowledger.has_acknowledged(key))
	})
}

fn remember_acknowledged(topic: &str) {
	ACKNOWLEDGED_TOPICS.with(|acknowledged| acknowledged.borrow_mut().insert(topic.to_string()));
	let Some(path) = ACKNOWLEDGEMENTS_FILE.with(|file| file.borrow().clone()) else {
		return;
	};
	let mut topics: Vec<String> =
		ACKNOWLEDGED_TOPICS.with(|acknowledged| acknowledged.borrow().iter().cloned().collect());
	topics.sort();
	let lines: String = topics.iter().map(|topic| format!("{ACKNOWLEDGED_PREFIX}{topic} = {ACKNOWLEDGED}\n")).collect();
	if let Err(problem) = std::fs::write(&path, lines) {
		eprintln!("warning: could not remember \"got it\" in {}: {problem}", path.display());
	}
}

/// After a warning or note of `topic` was shown, once per run: does the user say "got it", for this expression or all of
/// the kind? Then it is not shown again
pub(crate) fn offer_acknowledgement(topic: &str, written: &str, level: Level) {
	if !NOTES_SHOWN.with(|shown| shown.borrow_mut().insert(topic.to_string())) {
		return;
	}
	// "got it?" about something nobody saw (below the shown level) is not asked
	let acknowledger = ACKNOWLEDGER.with(|current| current.borrow().clone()).filter(|_| is_shown(level));
	match acknowledger.map_or(GotIt::No, |acknowledger| acknowledger.acknowledge(topic, written)) {
		GotIt::This => remember_acknowledged(&expression_key(topic, written)),
		GotIt::All => remember_acknowledged(topic),
		GotIt::No => {}
	}
}

/// Educate with "got it": the hint (`written` → `preferred`, and why) is shown once per run until the user
/// acknowledges it, then never again; it never blocks non-interactive runs, which just show the hint
pub fn educate_once(topic: &str, written: &str, preferred: &str, reason: &str) {
	let fix = crate::normalize::rewrite(written, preferred, reason);
	advise_once(topic, written, preferred, reason, Some(fix));
}

/// `educate_once` with advice (normalize::advise): `fix` is its "I meant" edit, none for `subscribe before the loop`
pub fn advise_once(topic: &str, written: &str, preferred: &str, reason: &str, fix: Option<crate::errors::fixits::Fix>) {
	let silenced =
		is_quiet() || is_acknowledged(topic, written) || NOTES_SHOWN.with(|shown| shown.borrow().contains(topic));
	if !silenced {
		crate::normalize::show_hint(written, preferred, reason, fix, topic);
	}
}

/// Another language's word for a warp word (`__add__` for `plus`, user 2026-10-06): it works, with a got-it note
/// naming the warp word and its "I meant: <warp word>" fix; a word warp needs not at all (`data class`, `val x`) is
/// written with the word it stands before
pub fn note_alias(written: &str, warp_word: &str) {
	let (foreign_word, reason) = match written.strip_suffix(warp_word).map(str::trim) {
		Some(dropped) if !dropped.is_empty() => (dropped, format!("{dropped} is superfluous: write {warp_word}")),
		_ => (written, format!("warp says {warp_word}")),
	};
	educate_once(&format!("alias-{foreign_word}"), written, warp_word, &reason);
}

/// The reading the program gets: an error-fallback ambiguity is an error naming every explicit form; otherwise the
/// default, with a warning shown until the user says "got it" (an error under `use strict`, acknowledged or not)
pub fn ask(question: &Ask) -> Result<usize, Node> {
	note_said();
	let diagnostic = question.diagnostic();
	if question.fallback == Fallback::Error {
		return Err(diagnostic.into_error());
	}
	// a pass that runs twice (the emitter) asks the same question at the same place once
	let asked_before = diagnostics(|diagnostics| {
		let asked_before = diagnostics.assumptions.contains(&diagnostic);
		diagnostics.assumptions.push(diagnostic.clone());
		asked_before
	});
	if warning_mode().fails(diagnostic.level) {
		return Err(diagnostic.into_error());
	}
	if asked_before {
		return Ok(question.default);
	}
	report(&[diagnostic])?;
	Ok(question.default)
}

/// The defaults unanswered Asks took on this thread since the last call: a later runtime error names them,
/// since a wrong guess there typically fails far away from the ambiguous source
pub fn take_assumptions() -> Vec<Diagnostic> {
	diagnostics(|diagnostics| std::mem::take(&mut diagnostics.assumptions))
}
