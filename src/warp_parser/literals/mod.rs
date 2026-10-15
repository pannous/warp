//! Literals: the value at the cursor: numbers, texts, units, paths, constants, shaders, data

use super::*;
use crate::words::FLOAT_WORD;
mod constants;
mod data;
mod numbers;
mod paths;
mod shaders;
mod text;
mod units;
pub(crate) use constants::*;
pub(crate) use numbers::*;
pub use paths::{is_path_literal, is_written_path, path_variables};
pub use shaders::*;
pub(crate) use text::*;

const BACKTICK: char = '`';
/// The "got it" topic of the hint on backtick texts
const BACKTICK_TOPIC: &str = "backtick text";
const F_STRING_TOPIC: &str = "python f-string";
const F_STRING_PREFIX: char = 'f';
/// U+0305 COMBINING OVERLINE, written on each repeating digit of a decimal: 0.3̅ (card repeating-decimals)
pub const OVERLINE: char = '\u{305}';
/// The words of a WGSL block, `shader { … }` (card g_oFJc)
const SHADER_WORDS: [&str; 2] = ["shader", "wgsl"];
/// `\e` in a text: the start of terminal codes like `\e[H`
const ESCAPE_CHARACTER: char = '\u{1b}';

impl WarpParser {
	/// Parse a complete value/expression - calls parse_expr(0) for operator chaining
	pub(super) fn parse_value(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();
		let comment = match (self.pending_comment.take(), comment) {
			(Some(before), Some(after)) => Some(format!("{before}\n{after}")),
			(before, after) => before.or(after),
		};

		// Capture position before parsing
		let (line_nr, column) = self.get_position();
		if self.is_at_line_end() {
			return Empty;
		};

		let ch = self.current_char();

		// Handle special non-expression cases first
		let node = match ch {
			';' => return Empty, // Semicolons handled by main parse loop
			// a `<…>` group's closer is handled by parse_bracketed; any other lone `>` is the operator value (`sorted(xs, >)`)
			// or a comparison without its left side (`> 100 => "big"`)
			'>' => match self.try_parse_operator_value() {
				Some(operator) => operator,
				None => self.parse_expr(0),
			},
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			// Everything else goes through parse_expr for operator chaining
			_ => self.parse_expr(0),
		};

		if node == Empty {
			return node;
		}
		let node = node.with_meta_data(LineInfo::at(line_nr, column, &self.current_line));
		// Attach preceding comment as metadata
		if let Some(comment_text) = comment {
			node.with_comment(comment_text)
		} else {
			node
		}
	}
}

/// A line of a shader block's text: which line of the block it is (0: the one of its `{`), how many characters before
/// it the text dropped, and the text
pub struct DedentedLine {
	pub line: usize,
	pub dropped: usize,
	pub text: String,
}
