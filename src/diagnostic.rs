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
