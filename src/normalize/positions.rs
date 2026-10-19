//! Where a hint points (the position set while lowering) and hints captured instead of shown
use super::*;

/// Current source position for hint messages (thread-local)
#[derive(Debug, Clone, Default)]
pub struct HintPosition {
	pub file: Option<String>,
	pub line: usize,
	pub column: usize,
}

/// Run `action` without hints on this thread: a stage that repeats work already hinted (the emitter's rerun) says nothing twice.
/// Its positions stay its own: a side parse in the middle of the program's (the prelude read when a word is first
/// looked up) leaves the hint position where the program's hint is about to point
pub fn without_hints<T>(action: impl FnOnce() -> T) -> T {
	let position = compile_context::read(|context| context.hint_position.clone());
	compile_context::scoped(|context| &mut context.hint_position, position, || {
		compile_context::scoped(|context| &mut context.hints_muted, true, action)
	})
}

/// A hint as tests and hosts read it: a view of a Diagnostic of level Hint (see `capture_hints`)
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedHint {
	pub original: String,
	pub canonical: String,
	/// `line:column`, empty when the emitting stage knows no position
	pub position: String,
	pub reason: String,
	/// The "I meant" edit: `canonical` in place of `original` for a rewrite; for advice the edit its caller names
	/// (`global x=1` at main's assignment), none when the advice replaces no text (`on … before the loop`)
	pub fix: Option<crate::errors::fixits::Fix>,
}

impl CapturedHint {
	/// The preferred form as an applicable fix
	pub fn fix(&self) -> Option<crate::errors::fixits::Fix> {
		self.fix.clone()
	}

	/// The line and column of `position` (`file:line:column` or `line:column`), 0:0 for none
	pub fn line_and_column(&self) -> (usize, usize) {
		let mut parts = self.position.rsplit(':').map(|part| part.parse().unwrap_or(0));
		let column = parts.next().unwrap_or(0);
		(parts.next().unwrap_or(0), column)
	}
}

impl From<crate::errors::diagnostic::Diagnostic> for CapturedHint {
	fn from(hint: crate::errors::diagnostic::Diagnostic) -> Self {
		let position = match hint.line {
			0 => String::new(),
			line => crate::compiler::meta::place_text(hint.file.as_deref(), line, hint.column),
		};
		CapturedHint {
			original: hint.written.unwrap_or_default(),
			canonical: hint.preferred.unwrap_or_default(),
			position,
			reason: hint.message,
			fix: hint.fixes.into_iter().next(),
		}
	}
}

/// Run `action` and return the hints it emitted on this thread (hints are still printed)
pub fn capture_hints<T>(action: impl FnOnce() -> T) -> (T, Vec<CapturedHint>) {
	let (result, hints) = crate::errors::diagnostic::capture_hints(action);
	(result, hints.into_iter().map(CapturedHint::from).collect())
}

/// Set the current position for hint messages
pub fn set_hint_position(line: usize, column: usize) {
	let file =
		compile_context::read(|context| context.parsed_file.clone().or_else(|| context.hint_position.file.clone()));
	set_hint_position_in(file, line, column);
}

pub(crate) fn set_hint_position_in(file: Option<String>, line: usize, column: usize) {
	compile_context::change(|context| {
		context.hint_position.file = file;
		context.hint_position.line = line;
		context.hint_position.column = column;
	});
}

/// Set the current file for hint messages
pub fn set_hint_file(file: &str) {
	compile_context::change(|context| context.hint_position.file = Some(file.to_string()));
}

/// Clear the hint position
pub fn clear_hint_position() {
	compile_context::change(|context| context.hint_position = HintPosition::default());
}

/// Get position string in clickable format (file:line:col or line:col)
#[cfg(test)]
pub(super) fn position_string() -> String {
	compile_context::read(|context| {
		let p = &context.hint_position;
		if p.line == 0 {
			return String::new();
		}
		match &p.file {
			Some(f) => format!("{}:{}:{}", f, p.line, p.column),
			None => format!("{}:{}", p.line, p.column),
		}
	})
}
