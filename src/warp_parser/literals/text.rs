//! Texts: quoted strings, escapes, f-strings and their holes, dedented multiline texts
use super::*;
use crate::warp_parser::context::Context;

/// The most code points an emoji in code may have (a family ZWJ sequence has 7 to 11, a subdivision flag 7)
pub(crate) const LONGEST_EMOJI: usize = 32;

/// The length of `chars` before the `}` that closes an already open `{`, skipping `//` and `/* */` comments
pub(crate) fn closing_brace(chars: &[char]) -> Option<usize> {
	let (mut depth, mut at) = (1, 0);
	while at < chars.len() {
		match (chars[at], chars.get(at + 1)) {
			('/', Some('/')) => at += chars[at..].iter().position(|&ch| ch == '\n').unwrap_or(chars.len() - at),
			('/', Some('*')) => {
				at += chars[at..].windows(2).position(|pair| pair == ['*', '/']).map_or(chars.len() - at, |end| end + 1)
			}
			('{', _) => depth += 1,
			('}', _) if depth == 1 => return Some(at),
			('}', _) => depth -= 1,
			_ => {}
		}
		at += 1;
	}
	None
}

/// The lines without the indentation all non-blank ones share, without the blank lines around them
pub(crate) fn without_common_indentation(source: &str) -> String {
	dedented_lines(source).into_iter().map(|line| line.text).collect::<Vec<_>>().join("\n")
}

/// The lines of `source` without the indentation all non-blank ones share, without the blank lines around them, and
/// the first without any
pub(crate) fn dedented_lines(source: &str) -> Vec<DedentedLine> {
	let lines: Vec<&str> = source.lines().collect();
	let is_blank = |line: &&str| line.trim().is_empty();
	let common =
		lines.iter().filter(|line| !is_blank(line)).map(|line| line.len() - line.trim_start().len()).min().unwrap_or(0);
	let first = lines.iter().position(|line| !is_blank(line)).unwrap_or(lines.len());
	let end = lines.iter().rposition(|line| !is_blank(line)).map_or(first, |last| last + 1);
	let mut kept: Vec<DedentedLine> = (first..end)
		.map(|line| DedentedLine {
			line,
			dropped: common,
			text: lines[line].get(common..).unwrap_or("").trim_end().to_string(),
		})
		.collect();
	if let Some(opening) = kept.first_mut() {
		let text = opening.text.trim_start().to_string();
		opening.dropped += opening.text.chars().count() - text.chars().count();
		opening.text = text;
	}
	kept
}

impl WarpParser {
	pub(crate) fn parse_string(&mut self) -> Node {
		self.read_text()
	}

	fn read_text(&mut self) -> Node {
		let quote = self.current_char();
		// «text» closes with », every other quote with itself
		let closing = if quote == '«' { '»' } else { quote };
		let (quote_line, quote_column) = self.get_position();
		self.advance(); // skip opening quote

		// `Hello ${name}`: a JavaScript template literal is warp's interpolated text (card text-backtick)
		let interpolates = (quote == '"' || quote == BACKTICK)
			&& !self.options.data_mode
			&& !self.options.xml_mode
			&& !self.options.wit_mode;
		if quote == BACKTICK {
			set_hint_position(quote_line, quote_column);
			crate::errors::diagnostic::educate_once(
				BACKTICK_TOPIC,
				"`…${x}…`",
				"\"…\\(x)…\"",
				"warp writes texts in double quotes",
			);
		}
		let mut s = String::new();
		// the same literal in injection::parts syntax (holes `${expr}`, literal dollars `$$`), kept while it has a hole
		let mut template = String::new();
		let mut is_template = false;
		loop {
			let ch = self.current_char();
			if ch == '\0' {
				let message = format!(
					"Unterminated string: the text opened at {quote_line}:{quote_column} has no closing `{closing}`"
				);
				return missing_closer(message, closing, crate::errors::fixits::end_of_line(&self.input, quote_line));
			}
			if ch == closing {
				self.advance(); // skip closing quote
				if is_template {
					return crate::text::interpolation::template_text(&template);
				}
				return self.quoted_text(s.nfc().collect(), quote, (quote_line, quote_column), interpolates);
			}
			let escaped_brace = self.context.brace_holes && matches!((ch, self.peek_char(1)), ('{', '{') | ('}', '}'));
			if escaped_brace {
				self.advance(); // `{{` is the brace itself
			}
			let hole = match ch {
				'{' if self.context.brace_holes && !escaped_brace => {
					self.advance();
					self.text_until_closing('{', '}').map(Some)
				}
				'$' if interpolates => self.parse_dollar_hole(),
				'\\' if interpolates && matches!(self.peek_char(1), '(' | '{') => self.parse_swift_hole().map(Some),
				_ => Ok(None),
			};
			match hole {
				Err(message) => return error(&message),
				Ok(Some(expression)) => {
					template.push_str(&format!("${{{expression}}}"));
					is_template = true;
					continue;
				}
				Ok(None) => {}
			}
			// a bare `$name` is text here but stays a hole for sql/sh templates (injection::parts), so it is not escaped
			let bare_dollar_name = ch == '$' && (self.peek_char(1).is_alphabetic() || self.peek_char(1) == '_');
			let literal = if ch == '\\' {
				self.advance();
				let escaped = self.current_char();
				is_template |= interpolates && escaped == '$'; // an escaped dollar is never a hole, also in sql/sh
				match escaped {
					'\0' => continue, // `"\` ends the source: the loop reports the unterminated text
					':' if interpolates && self.peek_char(1).is_ascii_alphabetic() => {
						let characters = match self.entity_escape() {
							Ok(characters) => characters,
							Err(strict) => return strict,
						};
						s.push_str(characters);
						template.push_str(characters);
						continue;
					}
					'n' => '\n',
					't' => '\t',
					'r' => '\r',
					'e' => ESCAPE_CHARACTER,
					'x' if self.peek_char(1).is_ascii_hexdigit() && self.peek_char(2).is_ascii_hexdigit() => {
						self.hex_byte_escape()
					}
					'u' if self.peek_char(1) == '{' => or_return_error!(self.unicode_escape()),
					c => c,
				}
			} else {
				ch
			};
			self.advance();
			s.push(literal);
			match literal {
				'$' if !bare_dollar_name => template.push_str("$$"),
				c => template.push(c),
			}
		}
	}

	/// The text between quotes (normalized: an escape like `e\u{301}` may leave it unnormalized); one character is a
	/// Codepoint whichever quote is used, so only longer texts have a canonical quote
	fn quoted_text(&self, text: String, quote: char, (line, column): (usize, usize), interpolates: bool) -> Node {
		let mut chars = text.chars();
		let (Some(c), None) = (chars.next(), chars.next()) else {
			if !text.is_empty() {
				set_hint_position(line, column);
				norm::quotes(quote, &text);
			}
			return Node::text(&text);
		};
		match self.number_cast_follows().filter(|_| interpolates && !c.is_ascii_digit()) {
			Some(number_type) => {
				let message = format!(
					"\"{c}\" as {number_type}: a text is no number (user decision #35); codepoint('{c}') is the code point of the character"
				);
				let explicit = format!("{}('{c}') as {number_type}", crate::words::CODEPOINT);
				Diagnostic { message, line, column, ..Default::default() }
					.fix(&explicit)
					.offer("the code point of the character", format!("\"{c}\" as {number_type}"), explicit)
					.into_error()
			}
			None => Node::codepoint(c),
		}
	}

	/// At the `:` of `\:name` in a text: the entity's characters, read past the name; an unknown name gives the
	/// backslash alone, the name follows as written
	fn entity_escape(&mut self) -> Result<&'static str, Node> {
		let Some((name, length)) = crate::text::uniscript_entities::entity_name_at(&self.chars, self.pos - 1) else {
			unreachable!("a letter follows")
		};
		let Some(characters) = crate::text::uniscript_entities::entity(&name) else {
			self.warn_unknown_entity(&name, self.column - 1)?;
			return Ok("\\");
		};
		self.advance_by(length - 1);
		Ok(characters)
	}

	/// `\:xyzzy` names no entity: it stays as written and warns; in strict mode the warning is the error
	pub(crate) fn warn_unknown_entity(&self, name: &str, column: usize) -> Result<(), Node> {
		let message = crate::text::uniscript_entities::unknown_entity(name);
		crate::errors::diagnostic::report(&[Diagnostic { message, line: self.line_nr, column, ..Default::default() }])
	}

	/// Python's `f"hi {name}"` at the cursor: interpolated text whose holes are braces, `"hi \(name)"`
	pub(crate) fn parse_f_string(&mut self) -> Option<Node> {
		if self.current_char() != F_STRING_PREFIX || self.peek_char(1) != '"' || self.options.data_mode {
			return None;
		}
		set_hint_position(self.line_nr, self.column);
		crate::errors::diagnostic::educate_once(
			F_STRING_TOPIC,
			"f\"…{x}…\"",
			"\"…\\(x)…\"",
			"warp text interpolates without a prefix",
		);
		self.advance();
		Some(self.within(Context { brace_holes: true, ..self.context }, Self::parse_string))
	}

	/// `\x1b` after the backslash: the character of the two hex digits, left at the second
	fn hex_byte_escape(&mut self) -> char {
		let hex: String = [self.peek_char(1), self.peek_char(2)].iter().collect();
		self.advance();
		self.advance();
		char::from(u8::from_str_radix(&hex, 16).expect("two hex digits"))
	}

	/// `\u{e9}` after the backslash: the code point of the hex digits; the closing `}` is left for the caller to skip
	pub(crate) fn unicode_escape(&mut self) -> Result<char, String> {
		self.advance(); // skip `u`, now at `{`
		let mut hex = String::new();
		while self.peek_char(1).is_ascii_hexdigit() {
			self.advance();
			hex.push(self.current_char());
		}
		if self.peek_char(1) != '}' {
			return Err(format!("\\u{{{hex}…: a unicode escape is hex digits in braces, like \\u{{e9}}"));
		}
		self.advance(); // now at `}`
		u32::from_str_radix(&hex, 16)
			.ok()
			.and_then(char::from_u32)
			.ok_or_else(|| format!("\\u{{{hex}}} is no unicode code point"))
	}

	/// `${expr}` or `$(expr)` inside interpolated text: the hole's expression, `None` for a plain dollar (`$5`, `$x`,
	/// `$ `). User decision D1: "only the one with the curly braces must interpolate the other is text like dollar
	/// money"; card dollar-paren (user, 2026-10-07): `$()` behaves just like `${}`.
	pub(crate) fn parse_dollar_hole(&mut self) -> Result<Option<String>, String> {
		let open = self.peek_char(1);
		let close = match open {
			'{' => '}',
			'(' => ')',
			_ => return Ok(None),
		};
		self.advance_by(2);
		let expression = self.text_until_closing(open, close)?;
		Ok(Some(expression))
	}

	/// `\(expr)` inside interpolated text: its expression; `\{expr}` too. User, 2026-10-09: `${}` `$()` `\()` `\{}` are all fine, no hint
	pub(crate) fn parse_swift_hole(&mut self) -> Result<String, String> {
		let open = self.peek_char(1);
		self.advance_by(2);
		match open {
			'(' => self.text_until_closing('(', ')'),
			_ => self.text_until_closing('{', '}'),
		}
	}

	/// The source up to the bracket closing an already opened `open`, skipping quoted text; consumes the closing bracket
	pub(crate) fn text_until_closing(&mut self, open: char, close: char) -> Result<String, String> {
		let start = self.pos;
		let mut depth = 1;
		let mut quote: Option<char> = None;
		loop {
			let ch = self.current_char();
			match (ch, quote) {
				('\0', _) => return Err(format!("unterminated interpolation hole: missing `{close}`")),
				('\\', Some(_)) => self.advance(),
				(c, Some(q)) if c == q => quote = None,
				('"' | '\'', None) => quote = Some(ch),
				(c, None) if c == open => depth += 1,
				(c, None) if c == close => {
					depth -= 1;
					if depth == 0 {
						let expression: String = self.chars[start..self.pos].iter().collect();
						self.advance();
						return if expression.trim().is_empty() {
							Err("empty interpolation hole".into())
						} else {
							Ok(expression)
						};
					}
				}
				_ => {}
			}
			self.advance();
		}
	}

	/// `🌍`, `🇩🇪`, `👍🏽` in code, one user-perceived character: one code point is a codepoint, several a text
	pub(crate) fn parse_emoji(&mut self) -> Node {
		let ahead: String = self.chars[self.pos..].iter().take(LONGEST_EMOJI).collect();
		let emoji = crate::extensions::strings::grapheme_clusters(&ahead)[0].to_string();
		self.advance_by(emoji.chars().count());
		let mut code_points = emoji.chars();
		match (code_points.next(), code_points.next()) {
			(Some(single), None) => Node::Char(single),
			_ => Node::Text(emoji),
		}
	}
}
