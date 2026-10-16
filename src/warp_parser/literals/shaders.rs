//! Shaders: embedded shader blocks
use super::*;

/// Each `shader { … }` (or `wgsl { … }`) block of `code`: the line and column right after its `{`, and its text's lines
pub fn shader_blocks(code: &str) -> Vec<((usize, usize), Vec<DedentedLine>)> {
	let chars: Vec<char> = code.chars().collect();
	shader_texts(&chars, |_| true)
		.into_iter()
		.map(|text| {
			let before = &chars[..text.start];
			let line = 1 + before.iter().filter(|&&ch| ch == '\n').count();
			let column = 1 + before.iter().rev().take_while(|&&ch| ch != '\n').count();
			let source: String = chars[text].iter().collect();
			((line, column), dedented_lines(&source))
		})
		.collect()
}

/// `code` with the text of each shader block blanked to spaces (line breaks kept): only its warp is left to read.
/// `is_code(at)`: whether the character at `at` is warp code, so a comment or a text naming `wgsl {` starts no block
pub fn without_shader_texts(code: &str, is_code: impl Fn(usize) -> bool) -> String {
	let mut chars: Vec<char> = code.chars().collect();
	for text in shader_texts(&chars, is_code) {
		chars[text].iter_mut().filter(|ch| **ch != '\n').for_each(|ch| *ch = ' ');
	}
	chars.into_iter().collect()
}

/// Where the text between the braces of each shader block is, in characters
fn shader_texts(chars: &[char], is_code: impl Fn(usize) -> bool) -> Vec<std::ops::Range<usize>> {
	let is_name_part = |at: usize| chars.get(at).is_some_and(|ch| ch.is_alphanumeric() || *ch == '_');
	let mut texts = vec![];
	let mut at = 0;
	while at < chars.len() {
		let word = SHADER_WORDS.iter().find(|word| {
			is_code(at)
				&& !is_name_part(at.wrapping_sub(1))
				&& chars[at..].starts_with(&word.chars().collect::<Vec<_>>())
		});
		let Some(word) = word.filter(|word| !is_name_part(at + word.len())) else {
			at += 1;
			continue;
		};
		let mut open = at + word.len();
		while matches!(chars.get(open), Some(' ' | '\t')) {
			open += 1;
		}
		let start = open + 1;
		let Some(length) = (chars.get(open) == Some(&'{')).then(|| closing_brace(&chars[start..])).flatten() else {
			at = open;
			continue;
		};
		texts.push(start..start + length);
		at = start + length;
	}
	texts
}

impl WarpParser {
	/// `shader { … }` (or `wgsl { … }`) at the cursor: WGSL as a sublanguage, the braces' content verbatim as text
	/// without its common indentation (card g_oFJc); braces inside WGSL comments do not count
	pub(crate) fn parse_shader_block(&mut self) -> Option<Node> {
		if self.options.data_mode {
			return None;
		}
		let word =
			SHADER_WORDS.iter().find(|word| word.chars().enumerate().all(|(index, ch)| self.peek_char(index) == ch))?;
		let mut open = word.len();
		if self.is_identifier_start(open) || self.peek_char(open).is_ascii_digit() {
			return None;
		}
		while matches!(self.peek_char(open), ' ' | '\t') {
			open += 1;
		}
		if self.peek_char(open) != '{' {
			return None;
		}
		let start = self.pos + open + 1;
		let Some(end) = closing_brace(&self.chars[start..]).map(|length| start + length) else {
			self.advance_by(self.chars.len() - self.pos);
			return Some(missing_closer(
				format!("the {{ of `{word} {{ … }}` is never closed"),
				"}",
				crate::errors::fixits::end_of_source(&self.input),
			));
		};
		let source: String = self.chars[start..end].iter().collect();
		self.advance_by(end + 1 - self.pos);
		Some(crate::graphics::shader_holes::shader_text(&without_common_indentation(&source)))
	}
}
