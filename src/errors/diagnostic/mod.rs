//! Compiler diagnostics: what went wrong, where, and how to fix it.
//! Errors become `Node::Error` values (never panics); warnings (lints) are reported and compilation continues.

use crate::compiler::compile_context;
use crate::errors::fixits::Fix;
use crate::node::Node;
use std::fmt;

mod asks;
pub use asks::*;
mod positions;
pub use positions::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
	pub message: String,
	pub line: usize,
	pub column: usize,
	/// The module file of the position, None in the program itself (card module-error-file)
	pub file: Option<String>,
	/// How to fix it, in words (shown after `; fix: `)
	pub fix: Option<String>,
	/// The same as edits a host applies: one per reading the user might have meant (src/diagnostics/fixits.rs)
	pub fixes: Vec<Fix>,
	/// The kind of warning, which "got it" silences (an Ask's topic)
	pub topic: Option<String>,
	/// What "got it" for this expression only remembers: `topic@expression` (expression_key)
	pub expression_key: Option<String>,
	/// How much it matters: what is shown (show_from) and what fails under `use strict` (WarningMode::Strict)
	pub level: Level,
	/// A hint's text as written, and the form it prefers (the same for a note about the text itself)
	pub written: Option<String>,
	pub preferred: Option<String>,
}

/// How much a diagnostic matters (user, 2026-10-13: hint < weak < normal < severe; errors are no level, they stop the
/// program). A hint prefers another form, a weak warning notices how the program runs (`@gpu` maps on the CPU), a
/// normal warning a likely mistake, a severe one code that can never do what it says (a handler nothing emits)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Level {
	Hint,
	Weak,
	#[default]
	Normal,
	Severe,
}

impl Level {
	pub const ALL: [Level; 4] = [Level::Hint, Level::Weak, Level::Normal, Level::Severe];

	pub fn name(self) -> &'static str {
		match self {
			Level::Hint => "hint",
			Level::Weak => "weak",
			Level::Normal => "normal",
			Level::Severe => "severe",
		}
	}

	/// The level of its name (`--warnings=weak`, `--strict=severe`)
	pub fn named(name: &str) -> Option<Level> {
		Level::ALL.into_iter().find(|level| level.name() == name)
	}
}

impl Diagnostic {
	pub fn level(self, level: Level) -> Self {
		Diagnostic { level, ..self }
	}

	pub fn weak(self) -> Self {
		self.level(Level::Weak)
	}

	pub fn severe(self) -> Self {
		self.level(Level::Severe)
	}

	/// A diagnostic at the source position of `node`, else of its first part that has one (0:0 if the parser recorded none)
	pub fn at(node: &Node, message: impl Into<String>) -> Self {
		let position = line_info(node).unwrap_or_default();
		Diagnostic {
			message: message.into(),
			line: position.line_nr,
			column: position.column,
			file: position.file,
			..Default::default()
		}
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
		self.offering(crate::errors::fixits::fix(meaning, written, replacement))
	}

	pub fn offering(mut self, offered: Fix) -> Self {
		if offered.changes_something() {
			self.fixes.push(offered);
		}
		self
	}

	/// A warning of the kind `topic` about the expression `written`: shown until the user says "got it" to it or to all
	/// of the kind
	pub fn about(self, topic: &str, written: &str) -> Self {
		Diagnostic { topic: Some(topic.to_string()), expression_key: Some(expression_key(topic, written)), ..self }
	}

	/// The topic "got it" silences and the expression it names, for a warning that has them
	fn acknowledgeable(&self) -> Option<(&str, &str)> {
		let topic = self.topic.as_deref()?;
		let written = self.expression_key.as_deref()?.strip_prefix(topic)?.strip_prefix('@')?;
		Some((topic, written))
	}

	/// Said "got it" to before, or on a line that says `// got it`
	fn is_silenced(&self) -> bool {
		silenced_by_comment(self.line)
			|| self.acknowledgeable().is_some_and(|(topic, written)| is_acknowledged(topic, written))
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
			diagnostics(|diagnostics| diagnostics.errors.push(self));
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
	let colored = *COLORED
		.get_or_init(|| std::io::IsTerminal::is_terminal(&std::io::stderr()) && std::env::var_os("NO_COLOR").is_none());
	if colored {
		format!("\x1b[{}m{text}\x1b[0m", color as u8)
	} else {
		text.to_string()
	}
}

impl fmt::Display for Diagnostic {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.message)?;
		// 0:0 is no position: a diagnostic of something written nowhere in the source
		if self.line > 0 {
			// a position outside the program names its file: one recorded while a used file is parsed is in that file
			let file = self.file.clone().or_else(|| compile_context::read(|context| context.parsed_file.clone()));
			write!(f, " at {}", crate::compiler::meta::place_text(file.as_deref(), self.line, self.column))?;
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
	/// Every warning is an error, weak ones too
	Error,
	/// The warnings of this level and above are errors: `use strict` and `--strict` (STRICT_LEVEL), `--strict=<level>`
	Strict(Level),
	/// Neither shown nor errors: a compile repeating one whose warnings were shown already
	Quiet,
}

impl WarningMode {
	/// The lowest level that is an error; hints never are
	fn failing_from(self) -> Option<Level> {
		match self {
			WarningMode::Error => Some(Level::Weak),
			WarningMode::Strict(level) => Some(level.max(Level::Weak)),
			WarningMode::Warn | WarningMode::Quiet => None,
		}
	}

	pub fn fails(self, level: Level) -> bool {
		self.failing_from().is_some_and(|from| level >= from)
	}

	/// `use strict` inside a mode: the stricter of the two
	fn strictened(self) -> WarningMode {
		match self.failing_from() {
			Some(from) if from <= STRICT_LEVEL => self,
			_ => WarningMode::Strict(STRICT_LEVEL),
		}
	}
}

/// What `use strict` and `--strict` make errors: normal and severe warnings (user default, 2026-10-13)
pub const STRICT_LEVEL: Level = Level::Normal;

/// The lowest level printed, process-wide (Hint: everything); lower ones are still recorded for hosts and tests
static SHOWN_FROM: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(Level::Hint as u8);
/// Whether this process printed a hint: the CLI then ends with how to hide them
static ANY_HINT_PRINTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// WARP_HINTS=0 hides hints and notes as `--no-hints` does (card hints-toggle)
pub const HINTS_VARIABLE: &str = "WARP_HINTS";

/// Print the diagnostics of `level` and above (`--warnings=<level>`; `--no-hints` is Weak)
pub fn show_from(level: Level) {
	SHOWN_FROM.store(level as u8, std::sync::atomic::Ordering::Relaxed);
}

thread_local! {
	/// every_hint_of silences printing on its own thread only: other threads' compiles keep showing and asking
	static PRINTING_SILENCED: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
}

/// Whether diagnostics of `level` are printed
pub fn is_shown(level: Level) -> bool {
	!PRINTING_SILENCED.with(|silenced| *silenced.borrow())
		&& level as u8 >= SHOWN_FROM.load(std::sync::atomic::Ordering::Relaxed)
}

/// Whether this process printed a hint or note
pub fn any_hint_printed() -> bool {
	ANY_HINT_PRINTED.load(std::sync::atomic::Ordering::Relaxed)
}

/// `use strict` in warp source makes warnings errors for that program
const STRICT_PRAGMA: [&str; 2] = [PRAGMA_WORD, "strict"];
const PRAGMA_WORD: &str = "use";
/// Every pragma may end in `mode`: `use script mode` is `use script`, `use strict mode` is `use strict`
const PRAGMA_SUFFIX: &str = "mode";
/// `use relaxed`, the mirror of `use strict`: the program shows no hints, as `--no-hints` does on the command line
const RELAXED_PRAGMAS: [&str; 3] = ["relaxed", "lenient", "loose"];
/// `use script` (decision use-script): `use relaxed` with the scripting modules' words, as `use os file net shell` gives them
const SCRIPT_PRAGMA: &str = "script";
const SCRIPT_MODULES: [&str; 4] = ["os", "file", "net", "shell"];
/// `use comments`: a comment before a binding becomes its meta information (P114, lowering/data/meta_entries.rs)
const COMMENTS_PRAGMA: &str = "comments";
/// `use reflection` (or `use reflection mode`): function values answer where and how they were made
const REFLECTION_PRAGMA: &str = "reflection";
/// `use fuel`: the program's runs count fuel steps instead of stopping at the time limit (decision fuel-metering)
const FUEL_PRAGMA: &str = "fuel";

/// The pragmas that turn on a switch of the compile context while the program compiles
const FLAG_PRAGMAS: [(&str, compile_context::Flag); 3] = [
	(COMMENTS_PRAGMA, |context| &mut context.comments_as_meta),
	(REFLECTION_PRAGMA, |context| &mut context.reflection_mode),
	(FUEL_PRAGMA, |context| &mut context.counts_fuel),
];
/// A "got it" is remembered as the line `ack:<topic> = acknowledged` in the acknowledgements file
const ACKNOWLEDGED: &str = "acknowledged";
const ACKNOWLEDGED_PREFIX: &str = "ack:";

thread_local! {
	static RUNTIME_WARNINGS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
	static DIAGNOSTICS: std::cell::RefCell<Diagnostics> = std::cell::RefCell::new(Diagnostics::default());
}

/// What compiles on this thread said, kept for the hosts that take it (take_warnings, take_error_diagnostics,
/// take_assumptions, capture_hints)
#[derive(Default)]
struct Diagnostics {
	/// How many diagnostics were said (warnings, errors with fixes, assumptions, hints): a step that says something must
	/// run again to say it again (analysis_memo.rs remembers only silent analyses)
	said: u64,
	/// The compile-time warnings, every level but hints
	warnings: Vec<Diagnostic>,
	/// The errors with fixes
	errors: Vec<Diagnostic>,
	/// The defaults unanswered Asks took
	assumptions: Vec<Diagnostic>,
	/// How many capture_hints scopes are open: hints are recorded only for them (nobody else reads them)
	hint_captures: usize,
	captured_hints: Vec<Diagnostic>,
	/// How many recording_said scopes are open, and what say() said in them, in order: a kept program says it again
	/// when it runs from its kept module (program_cache.rs)
	spoken_recordings: usize,
	spoken: Vec<Diagnostic>,
	/// Under HintMode::Once, the hints shown already
	shown_hints: std::collections::HashSet<String>,
}

fn diagnostics<R>(use_them: impl FnOnce(&mut Diagnostics) -> R) -> R {
	DIAGNOSTICS.with(|diagnostics| use_them(&mut diagnostics.borrow_mut()))
}

/// The program the CLI runs: its warnings and errors print the line they name (show_lines_of); process-wide, as a
/// program may compile on another thread
static SHOWN_SOURCE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// The warning mode of every later compilation on this thread (the CLI's `--strict`)
pub fn set_warning_mode(mode: WarningMode) {
	compile_context::change(|context| context.warning_mode = mode);
}

pub fn warning_mode() -> WarningMode {
	compile_context::read(|context| context.warning_mode)
}

/// Run `body` in another warning mode, then restore the previous one
pub fn with_warning_mode<R>(mode: WarningMode, body: impl FnOnce() -> R) -> R {
	compile_context::scoped(|context| &mut context.warning_mode, mode, body)
}

/// Run `body` saying nothing (no warnings, hints or "got it?"): the compile of the executable a run leaves repeats the
/// run's compile, which said it all (card cli-running)
pub fn quietly<R>(body: impl FnOnce() -> R) -> R {
	with_warning_mode(WarningMode::Quiet, || crate::normalize::without_hints(body))
}

fn is_quiet() -> bool {
	warning_mode() == WarningMode::Quiet
}

pub fn said() -> u64 {
	diagnostics(|diagnostics| diagnostics.said)
}

pub(crate) fn note_said() {
	diagnostics(|diagnostics| diagnostics.said += 1);
}

/// Run `body`, also giving what say() said meanwhile and whether that was all it said (no assumption or error with fixes
/// counted beside it)
pub fn recording_said<R>(body: impl FnOnce() -> R) -> (R, Vec<Diagnostic>, bool) {
	let (start, said_before) = diagnostics(|diagnostics| {
		diagnostics.spoken_recordings += 1;
		(diagnostics.spoken.len(), diagnostics.said)
	});
	let result = body();
	diagnostics(|diagnostics| {
		diagnostics.spoken_recordings -= 1;
		let spoken = diagnostics.spoken.split_off(start);
		if diagnostics.spoken_recordings > 0 {
			diagnostics.spoken.extend(spoken.iter().cloned());
		}
		let only_spoken = diagnostics.said - said_before == spoken.len() as u64;
		(result, spoken, only_spoken)
	})
}

/// Compile-time diagnostics: the first one the warning mode fails at is the error; otherwise each is said (say), but
/// those said "got it" to
pub fn report(diagnostics: &[Diagnostic]) -> Result<(), Node> {
	if !diagnostics.is_empty() {
		note_said();
	}
	let mode = warning_mode();
	if let Some(failing) = diagnostics.iter().find(|diagnostic| mode.fails(diagnostic.level)) {
		return Err(failing.clone().into_error());
	}
	if mode != WarningMode::Quiet {
		for diagnostic in diagnostics.iter().filter(|diagnostic| !diagnostic.is_silenced()) {
			say(diagnostic.clone());
		}
	}
	Ok(())
}

/// The one way a diagnostic of any level reaches the user: recorded for hosts (take_warnings, capture_hints), printed
/// to stderr from the shown level up (show_from), and followed by its "got it?" when printed. Under HintMode::Once a
/// diagnostic said before is not said again, nor one acknowledged or on a `// got it` line. Whether it was printed
pub fn say(diagnostic: Diagnostic) -> bool {
	if diagnostic.is_silenced() || said_before(&diagnostic) {
		return false;
	}
	note_said();
	diagnostics(|diagnostics| {
		if diagnostics.spoken_recordings > 0 {
			diagnostics.spoken.push(diagnostic.clone());
		}
	});
	let printed = is_shown(diagnostic.level);
	if printed {
		print(&diagnostic);
	}
	if let Some((topic, written)) = diagnostic.acknowledgeable() {
		offer_acknowledgement(topic, written, diagnostic.level);
	}
	diagnostics(|diagnostics| match diagnostic.level {
		Level::Hint if diagnostics.hint_captures > 0 => diagnostics.captured_hints.push(diagnostic),
		Level::Hint => {}
		_ => diagnostics.warnings.push(diagnostic),
	});
	printed
}

/// Under HintMode::Once: whether this diagnostic was said before on this thread (remembers it otherwise)
fn said_before(diagnostic: &Diagnostic) -> bool {
	let key = match (&diagnostic.written, &diagnostic.preferred) {
		(Some(written), Some(preferred)) => format!("{written}|{preferred}"),
		_ => format!("{}@{}:{}", diagnostic.message, diagnostic.line, diagnostic.column),
	};
	crate::normalize::hint_mode() == crate::normalize::HintMode::Once
		&& !diagnostics(|diagnostics| diagnostics.shown_hints.insert(key))
}

/// Clear shown hints (useful for testing)
pub fn clear_shown_hints() {
	diagnostics(|diagnostics| diagnostics.shown_hints.clear());
}

/// A hint `prefer <preferred> over <written>` (a note when they are the same) with its reason below; a warning with the
/// source line it names
fn print(diagnostic: &Diagnostic) {
	match (&diagnostic.written, &diagnostic.preferred) {
		(Some(written), Some(preferred)) => {
			ANY_HINT_PRINTED.store(true, std::sync::atomic::Ordering::Relaxed);
			let position = match diagnostic.line {
				0 => String::new(),
				line => format!(
					" {}",
					paint(
						Color::Gray,
						&crate::compiler::meta::place_text(diagnostic.file.as_deref(), line, diagnostic.column)
					)
				),
			};
			// a note about the text as written (`await job within 100 ms`) prefers nothing else
			match preferred == written {
				true => eprintln!("{}{position}: {}", paint(Color::Cyan, "note"), paint(Color::Green, written)),
				false => eprintln!(
					"{}{position}: prefer {} over {}",
					paint(Color::Cyan, "hint"),
					paint(Color::Green, preferred),
					paint(Color::Yellow, written)
				),
			}
			eprintln!("      {}", diagnostic.message);
		}
		_ => {
			let label = match diagnostic.level {
				Level::Normal => "warning".to_string(),
				level => format!("warning ({})", level.name()),
			};
			eprintln!("{label}: {diagnostic}");
			if let Some(excerpt) = shown_excerpt(diagnostic.line, diagnostic.column) {
				eprintln!("{excerpt}");
			}
		}
	}
}

/// Whether this warning was said on this thread since the last take_warnings: a step emitted twice (an if's
/// condition) says its warning once
pub fn warned_before(warning: &Diagnostic) -> bool {
	diagnostics(|diagnostics| diagnostics.warnings.contains(warning))
}

/// The compile-time warnings (every level but hints) said on this thread since the last call (a host without a
/// terminal shows them)
pub fn take_warnings() -> Vec<Diagnostic> {
	diagnostics(|diagnostics| std::mem::take(&mut diagnostics.warnings))
}

/// Run `body` and return it with the hints said meanwhile on this thread (still printed as always)
pub fn capture_hints<T>(body: impl FnOnce() -> T) -> (T, Vec<Diagnostic>) {
	let start = diagnostics(|diagnostics| {
		diagnostics.hint_captures += 1;
		diagnostics.captured_hints.len()
	});
	let result = body();
	let hints = diagnostics(|diagnostics| {
		diagnostics.hint_captures -= 1;
		let start = start.min(diagnostics.captured_hints.len());
		diagnostics.captured_hints.split_off(start)
	});
	(result, hints)
}

/// The errors with fixes made on this thread since the last call (the page offers their fixes when the program fails)
pub fn take_error_diagnostics() -> Vec<Diagnostic> {
	diagnostics(|diagnostics| std::mem::take(&mut diagnostics.errors))
}

/// A warning the running program reports with `warning(message)`
pub fn report_runtime_warning(message: &str) {
	note_said();
	if is_shown(Level::Normal) {
		eprintln!("warning: {message}");
	}
	RUNTIME_WARNINGS.with(|warnings| warnings.borrow_mut().push(message.to_string()));
}

/// A runtime warning `notice` (its detail after a colon) only once per run, however often it happens
pub fn report_runtime_warning_once(notice: &str, detail: &str) {
	if !RUNTIME_WARNINGS.with(|warnings| warnings.borrow().iter().any(|warning| warning.starts_with(notice))) {
		report_runtime_warning(&format!("{notice}: {detail}"));
	}
}

/// The runtime warnings reported on this thread since the last call
pub fn take_runtime_warnings() -> Vec<String> {
	RUNTIME_WARNINGS.with(|warnings| std::mem::take(&mut *warnings.borrow_mut()))
}

/// `use <word>` (or `use <word> mode`) as a statement: a pragma of the program
fn is_pragma(node: &Node, word: &str) -> bool {
	let symbol = |node: &Node, name: &str| matches!(node.drop_meta(), Node::Symbol(s) if s == name);
	let Node::List(items, _, _) = node.drop_meta() else {
		return false;
	};
	match items.as_slice() {
		[using, pragma] => symbol(using, PRAGMA_WORD) && symbol(pragma, word),
		[using, pragma, mode] => symbol(using, PRAGMA_WORD) && symbol(pragma, word) && symbol(mode, PRAGMA_SUFFIX),
		_ => false,
	}
}

/// `use strict` in the program or `--strict`, not any warnings-as-errors mode: what also makes an i64 overflow trap
/// (decision i64-wraps)
pub fn is_strict() -> bool {
	matches!(warning_mode(), WarningMode::Strict(_)) || compile_context::read(|context| context.says_strict)
}

/// A source line saying `use <word>` (or `use <word> mode`)
fn says_pragma(line: &str, word: &str) -> bool {
	let words: Vec<&str> = line.split_whitespace().collect();
	words == [PRAGMA_WORD, word] || words == [PRAGMA_WORD, word, PRAGMA_SUFFIX]
}

/// The program without its top-level `use <word>` statements, and whether it had one
pub fn without_pragma(program: Node, word: &str) -> (Node, bool) {
	replaced_pragma(program, word, None)
}

/// The program with its top-level `use <word>` statements replaced by `replacement` (dropped when None), and whether it
/// had one
fn replaced_pragma(program: Node, word: &str, replacement: Option<&Node>) -> (Node, bool) {
	if is_pragma(&program, word) {
		return (replacement.cloned().unwrap_or(Node::Empty), true);
	}
	match program {
		Node::List(items, bracket, separator) if items.iter().any(|item| is_pragma(item, word)) => {
			let rest = items
				.into_iter()
				.filter_map(|item| match is_pragma(&item, word) {
					true => replacement.cloned(),
					false => Some(item),
				})
				.collect();
			(Node::List(rest, bracket, separator), true)
		}
		Node::Meta { node, data } => {
			let (inner, found) = replaced_pragma(*node, word, replacement);
			(Node::Meta { node: Box::new(inner), data }, found)
		}
		other => (other, false),
	}
}

/// The program with its `use script` as `use os file net shell`, and whether it had one
fn with_script_modules(program: Node) -> (Node, bool) {
	let modules = [PRAGMA_WORD].into_iter().chain(SCRIPT_MODULES).map(|word| Node::Symbol(word.into()));
	let uses = Node::List(modules.collect(), crate::node::Bracket::None, crate::node::Separator::Space);
	replaced_pragma(program, SCRIPT_PRAGMA, Some(&uses))
}

/// The program without its top-level `use strict` statements, and whether it had one
pub fn without_strict_pragma(program: Node) -> (Node, bool) {
	without_pragma(program, STRICT_PRAGMA[1])
}

/// Run `body` (the parser) with warnings as errors when a line of the source says `use strict`,
/// so the warnings of parse-time Asks obey it too
pub fn in_source_mode<R>(code: &str, body: impl FnOnce() -> R) -> R {
	let says = |word: &str| code.lines().any(|line| says_pragma(line, word));
	let strict = says(STRICT_PRAGMA[1]);
	compile_context::change(|context| {
		context.source = code.to_string();
		context.says_strict = strict;
	});
	let relaxed = RELAXED_PRAGMAS.into_iter().chain([SCRIPT_PRAGMA]).any(says);
	match strict {
		true => with_warning_mode(warning_mode().strictened(), || relaxed_if(relaxed, body)),
		false => relaxed_if(relaxed, body),
	}
}

/// Run `body` without hints when the program says `use relaxed`
fn relaxed_if<R>(relaxed: bool, body: impl FnOnce() -> R) -> R {
	match relaxed {
		true => crate::normalize::without_hints(body),
		false => body(),
	}
}

/// The program without its top-level `use relaxed` statements (or an alias), and whether it had one
fn without_relaxed_pragma(program: Node) -> (Node, bool) {
	RELAXED_PRAGMAS.into_iter().fold((program, false), |(program, found), word| {
		let (program, here) = without_pragma(program, word);
		(program, found || here)
	})
}

/// Run `body` with warnings as errors when the program says `use strict`, and comments as meta information when it
/// says `use comments`, and without hints when it says `use relaxed` or `use script`; `use reflection` turns on
/// reflection mode, `use fuel` fuel counting
pub fn in_program_mode<R>(program: Node, body: impl FnOnce(Node) -> R) -> R {
	let (program, flags) = FLAG_PRAGMAS.into_iter().fold((program, vec![]), |(program, mut flags), (word, flag)| {
		let (program, said) = without_pragma(program, word);
		flags.push((flag, said));
		(program, flags)
	});
	compile_context::scoped_flags(&flags, || in_written_mode(program, body))
}

fn in_written_mode<R>(program: Node, body: impl FnOnce(Node) -> R) -> R {
	let (program, script) = with_script_modules(program);
	let (program, relaxed) = without_relaxed_pragma(program);
	relaxed_if(relaxed || script, || match without_strict_pragma(program) {
		(program, true) => with_warning_mode(warning_mode().strictened(), || body(program)),
		(program, false) => body(program),
	})
}
