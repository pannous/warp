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

thread_local! {
	static WARNING_MODE: std::cell::Cell<WarningMode> = const { std::cell::Cell::new(WarningMode::Warn) };
	static RUNTIME_WARNINGS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
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
			Ok(())
		}
	}
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

/// Run `body` with warnings as errors when the program says `use strict`
pub fn in_program_mode<R>(program: Node, body: impl FnOnce(Node) -> R) -> R {
	match without_strict_pragma(program) {
		(program, true) => with_warning_mode(WarningMode::Error, || body(program)),
		(program, false) => body(program),
	}
}
