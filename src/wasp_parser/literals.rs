//! Literals: values, strings with escapes and holes, numbers with units, fractions and exponents

use super::*;

const BACKTICK: char = '`';
/// The "got it" topic of the hint on backtick texts
const BACKTICK_TOPIC: &str = "backtick text";
const F_STRING_TOPIC: &str = "python f-string";
const F_STRING_PREFIX: char = 'f';

impl WaspParser {
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
			'>' => match self.try_parse_operator_value() {
				Some(operator) => operator,
				None => return Empty,
			},
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			// Everything else goes through parse_expr for operator chaining
			_ => self.parse_expr(0),
		};

		if node == Empty {
			return node;
		}
		let node = node.with_meta_data(LineInfo {
			line_nr,
			column,
			#[cfg(debug_assertions)]
			line: self.current_line.s(),
		});
		// Attach preceding comment as metadata
		if let Some(comment_text) = comment {
			node.with_comment(comment_text)
		} else {
			node
		}
	}

	pub(super) fn parse_string(&mut self) -> Node {
		let quote = self.current_char();
		// «text» closes with », every other quote with itself
		let closing = if quote == '«' { '»' } else { quote };
		let (quote_line, quote_column) = self.get_position();
		self.advance(); // skip opening quote

		// `Hello ${name}`: a JavaScript template literal is wasp's interpolated text (card text-backtick)
		let interpolates = (quote == '"' || quote == BACKTICK) && !self.options.data_mode && !self.options.xml_mode && !self.options.wit_mode;
		if quote == BACKTICK {
			set_hint_position(quote_line, quote_column);
			crate::diagnostic::educate_once(BACKTICK_TOPIC, "`…${x}…`", "\"…\\(x)…\"", "wasp writes texts in double quotes");
		}
		let mut s = String::new();
		// the same literal in injection::parts syntax (holes `${expr}`, literal dollars `$$`), kept while it has a hole
		let mut template = String::new();
		let mut is_template = false;
		loop {
			let ch = self.current_char();
			if ch == '\0' {
				return error(&format!("Unterminated string: the text opened at {quote_line}:{quote_column} has no closing `{closing}`"));
			}
			if ch == closing {
				self.advance(); // skip closing quote
				let s: String = s.nfc().collect(); // an escape (`e\u{301}`) may leave it unnormalized
				if is_template {
					return crate::interpolation::template_text(&template);
				}
				// a one-character string is a Codepoint whichever quote is used, so only longer strings have a canonical quote
				if s.chars().count() > 1 {
					set_hint_position(quote_line, quote_column);
					norm::quotes(quote, &s);
				}
				// quotes with exactly one character become Codepoint
				let mut chars = s.chars();
				if let Some(c) = chars.next() {
					if chars.next().is_none() {
						if let Some(number_type) = self.number_cast_follows().filter(|_| interpolates && !c.is_ascii_digit()) {
							let message = format!("\"{c}\" as {number_type}: a text is no number (user decision #35); codepoint('{c}') is the code point of the character");
							let explicit = format!("{}('{c}') as {number_type}", crate::library_words::CODEPOINT);
							return Diagnostic { message, line: quote_line, column: quote_column, ..Default::default() }.fix(&explicit)
								.offer("the code point of the character", format!("\"{c}\" as {number_type}"), explicit).into_error();
						}
						return Node::codepoint(c);
					}
				}
				return Node::text(&s);
			}
			let escaped_brace = self.brace_holes && matches!((ch, self.peek_char(1)), ('{', '{') | ('}', '}'));
			if escaped_brace {
				self.advance(); // `{{` is the brace itself
			}
			let hole = match ch {
				'{' if self.brace_holes && !escaped_brace => {
					self.advance();
					self.text_until_closing('{', '}').map(Some)
				}
				'$' if interpolates => self.parse_dollar_hole(),
				'\\' if interpolates && self.peek_char(1) == '(' => self.parse_swift_hole().map(Some),
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
					':' if interpolates && self.peek_char(1).is_ascii_alphabetic() => {
						let Some((name, length)) = crate::uniscript_entities::entity_name_at(&self.chars, self.pos - 1) else { unreachable!("a letter follows") };
						let Some(character) = crate::uniscript_entities::entity(&name) else { return error(&crate::uniscript_entities::unknown_entity(&name)) };
						(0..length - 2).for_each(|_| self.advance());
						character
					}
					'n' => '\n',
					't' => '\t',
					'r' => '\r',
					'u' if self.peek_char(1) == '{' => match self.unicode_escape() {
						Ok(c) => c,
						Err(message) => return error(&message),
					},
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

	/// Python's `f"hi {name}"` at the cursor: interpolated text whose holes are braces, `"hi \(name)"`
	pub(super) fn parse_f_string(&mut self) -> Option<Node> {
		if self.current_char() != F_STRING_PREFIX || self.peek_char(1) != '"' || self.options.data_mode {
			return None;
		}
		set_hint_position(self.line_nr, self.column);
		crate::diagnostic::educate_once(F_STRING_TOPIC, "f\"…{x}…\"", "\"…\\(x)…\"", "wasp text interpolates without a prefix");
		self.advance();
		let outer = std::mem::replace(&mut self.brace_holes, true);
		let text = self.parse_string();
		self.brace_holes = outer;
		Some(text)
	}

	/// `\u{e9}` after the backslash: the code point of the hex digits; the closing `}` is left for the caller to skip
	pub(super) fn unicode_escape(&mut self) -> Result<char, String> {
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
		u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).ok_or_else(|| format!("\\u{{{hex}}} is no unicode code point"))
	}

	/// The number type of a following `as int` / `as float` …
	pub(super) fn number_cast_follows(&self) -> Option<String> {
		let rest: String = self.chars[self.pos..].iter().take(32).collect();
		let words: Vec<&str> = rest.split_whitespace().take(2).collect();
		let number_type = words.get(1)?.trim_end_matches(|c: char| !is_identifier_char(c));
		use crate::type_kinds::Kind::{Float, Int};
		let is_number = matches!(crate::analyzer::builtin_type_kind(number_type), Some(Int | Float)) && !number_type.starts_with("bool");
		(words.first() == Some(&"as") && is_number).then(|| number_type.to_string())
	}

	/// `${expr}` inside interpolated text: the hole's expression, `None` for a plain dollar (`$5`, `$x`, `$ `).
	/// User decision D1: "only the one with the curly braces must interpolate the other is text like dollar money".
	pub(super) fn parse_dollar_hole(&mut self) -> Result<Option<String>, String> {
		let (line, column) = self.get_position();
		let next = self.peek_char(1);
		let expression = if next == '{' {
			self.advance_by(2);
			let expression = self.text_until_closing('{', '}')?;
			set_hint_position(line, column);
			norm::interpolation(&format!("${{{expression}}}"), &expression);
			expression
		} else {
			return Ok(None);
		};
		Ok(Some(expression))
	}

	/// `\(expr)` inside interpolated text, the canonical hole: its expression
	pub(super) fn parse_swift_hole(&mut self) -> Result<String, String> {
		self.advance_by(2);
		self.text_until_closing('(', ')')
	}

	/// The source up to the bracket closing an already opened `open`, skipping quoted text; consumes the closing bracket
	pub(super) fn text_until_closing(&mut self, open: char, close: char) -> Result<String, String> {
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
						return if expression.trim().is_empty() { Err("empty interpolation hole".into()) } else { Ok(expression) };
					}
				}
				_ => {}
			}
			self.advance();
		}
	}
	pub(super) fn parse_number(&mut self) -> Node {
		let start = self.pos;
		let number = self.parse_number_value();
		let literal: String = self.chars[start..self.pos].iter().collect();
		let number = self.keep_source_literal(number, &literal);
		match self.literal_type_suffix() {
			Some(number_type) => Node::Key(Box::new(number), Op::As, Box::new(Node::Symbol(number_type))),
			None => self.with_juxtaposed_factor(number),
		}
	}

	/// `2x` is 2*x, `3(4)` is 3*(4), `3x²` is 3*(x²): a number directly followed by a symbol or `(` multiplies.
	/// Code only: data keeps `size: 3px`. Ordinals `1st` `2nd` `3rd` `4th` are no products.
	pub(super) fn with_juxtaposed_factor(&mut self, number: Node) -> Node {
		if self.options.data_mode || matches!(number, Node::Error(_)) {
			return number;
		}
		let next = self.current_char();
		// `2ⁿ` raises, it multiplies nothing (try_parse_superscript_power)
		if superscript_letter(next).is_some() {
			return number;
		}
		let tight = (next.is_alphabetic() || next == '_' || next == '(') && !self.at_ordinal_suffix();
		if !tight && !self.at_spaced_unit() {
			return number;
		}
		self.skip_spaces();
		let factor = self.parse_atom();
		let factor = self.try_parse_superscript_power(&factor, 0).unwrap_or(factor);
		Node::Key(Box::new(number), Op::Mul, Box::new(factor))
	}

	/// `2 km` multiplies only when `km` is a known unit; any other spaced word keeps the list `[2 foo]`, and so does a
	/// unit word that starts the next entry or assignment: `{w:2 h:3}`
	pub(super) fn at_spaced_unit(&self) -> bool {
		let rest = &self.chars[self.pos..];
		let spaces = rest.iter().take_while(|c| **c == ' ').count();
		let word: String = rest[spaces..].iter().take_while(|c| is_identifier_char(**c)).collect();
		let after_word: String = rest[spaces + word.chars().count()..].iter().skip_while(|c| **c == ' ').take(2).collect();
		let starts_entry = after_word.starts_with(':') || (after_word.starts_with('=') && after_word != "==");
		// `2 m²`: the power is no part of the unit's name
		let unit = word.trim_end_matches(|c: char| superscript_digit(c).is_some());
		spaces > 0 && crate::units::is_unit(unit) && !starts_entry
	}

	pub(super) fn at_ordinal_suffix(&self) -> bool {
		let word: String = self.chars[self.pos..].iter().take_while(|c| is_identifier_char(**c)).collect();
		ORDINAL_SUFFIXES.contains(&word.as_str())
	}

	/// Tight conversion written on the literal: `0.1f` → float, `0.1l` → exact, `0.1:float` / `1.5:int` → that type.
	/// Code only: in data `version: 1.10` and `n:int` stay key/value.
	pub(super) fn literal_type_suffix(&mut self) -> Option<String> {
		if self.options.data_mode {
			return None;
		}
		let suffix = LITERAL_SUFFIXES.iter().find(|(letter, _)| *letter == self.current_char());
		if let Some((_, number_type)) = suffix.filter(|_| !is_identifier_char(self.peek_char(1))) {
			self.advance();
			return Some(number_type.to_string());
		}
		if self.current_char() != ':' {
			return None;
		}
		let name: String = self.chars[self.pos + 1..].iter().take_while(|c| is_identifier_char(**c)).collect();
		if !LITERAL_NUMBER_TYPES.contains(&name.as_str()) {
			return None;
		}
		self.advance_by(1 + name.chars().count());
		Some(name)
	}

	/// Data round-trips its literals: `zip: 01234` stays `01234`, `version: 1.10` stays `1.10`
	pub(super) fn keep_source_literal(&self, number: Node, literal: &str) -> Node {
		let is_lossy = matches!(number, Node::Number(_)) && number.serialize() != literal;
		if self.options.data_mode && is_lossy {
			number.with_source_literal(literal)
		} else {
			number
		}
	}

	pub(super) fn parse_number_value(&mut self) -> Node {
		// RFC 3339 / RFC 9557 literal: 2024-01-31, 2024-01-31T10:00Z, 2024-01-31T10:00[Europe/Berlin]
		// Not in data mode yet: the data path round-trips source text and has no date serialization
		let literal_len = if self.options.data_mode { 0 } else { crate::time::literal_len(&self.chars[self.pos..]) };
		if literal_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + literal_len].iter().collect();
			self.advance_by(literal_len);
			return Node::data(crate::time::TimeLiteral(literal));
		}
		let version_len = crate::versions::literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + version_len].iter().collect();
			self.advance_by(version_len);
			return Node::Symbol(literal);
		}
		let mut num_str = String::new();

		// a leading plus is the unary plus prefix (peek_prefix_operator)
		if self.current_char() == '-' {
			num_str.push('-');
			self.advance();
		}

		// Check for hexadecimal: 0x or 0X
		if self.current_char() == '0' {
			let next_ch = self.peek_char(1);
			if next_ch == 'x' || next_ch == 'X' {
				// Parse hexadecimal
				self.advance(); // skip '0'
				self.advance(); // skip 'x'
				let mut hex_str = String::new();
				loop {
					let ch = self.current_char();
					if !ch.is_ascii_hexdigit() {
						break;
					}
					hex_str.push(ch);
					self.advance();
				}
				return i64::from_str_radix(&hex_str, 16)
					.map(Node::int)
					.unwrap_or_else(|_| error(&format!("Invalid hex: 0x{}", hex_str)));
			}
		}

		if let Some(fraction) = self.vulgar_fraction_literal() {
			return fraction;
		}

		self.push_digits(&mut num_str);
		let mut is_float = false;
		// Only consume . as decimal if NOT followed by another . (range operator) or a word (`4.square` calls square)
		let after_point = self.peek_char(1);
		if self.current_char() == '.' && after_point != '.' && !(after_point.is_alphabetic() || after_point == '_') {
			is_float = true;
			num_str.push('.');
			self.advance();
			self.push_digits(&mut num_str);
		}
		let exponent = match self.parse_exponent() {
			Ok(exponent) => exponent,
			Err(e) => return error(&e),
		};

		match exponent {
			// 1e3 is the exact integer 1000, like the literal it abbreviates
			Some(exp) if !is_float && exp >= 0 => {
				if exp > MAX_INTEGER_EXPONENT {
					return error(&format!("Exponent too large: {}e{}", num_str, exp));
				}
				num_str.push_str(&"0".repeat(exp as usize));
				self.integer_node(&num_str)
			}
			Some(exp) => format!("{}e{}", num_str, exp)
				.parse::<f64>()
				.map(Node::float)
				.unwrap_or_else(|_| error(&format!("Invalid float: {}e{}", num_str, exp))),
			None if is_float => num_str
				.parse::<f64>()
				.map(Node::float)
				.unwrap_or_else(|_| error(&format!("Invalid float: {}", num_str))),
			None => {
				let number = self.integer_node(&num_str);
				self.with_time_unit(number)
			}
		}
	}

	/// `⅓` is the exact ratio 1/3; a number written directly after it multiplies: `⅓9` is 3
	pub(super) fn vulgar_fraction_literal(&mut self) -> Option<Node> {
		let (numerator, denominator) = vulgar_fraction(self.current_char())?;
		self.advance();
		let ratio = Number::ratio(Number::Int(numerator), Number::Int(denominator));
		let fraction = Node::Number(ratio);
		if !self.current_char().is_ascii_digit() {
			return Some(fraction);
		}
		let factor = self.parse_number_value();
		Some(Node::Key(Box::new(fraction), Op::Mul, Box::new(factor)))
	}

	/// `1 month`, `24 hours`: an integer followed by a time unit word is a duration
	pub(super) fn with_time_unit(&mut self, number: Node) -> Node {
		let Node::Number(Number::Int(count)) = &number else {
			return number;
		};
		if self.options.data_mode {
			return number;
		}
		let Some((unit, length)) = crate::time::unit_after(&self.chars[self.pos..]) else {
			return number;
		};
		self.advance_by(length);
		Node::data(unit.times(*count))
	}

	pub(super) fn integer_node(&self, digits: &str) -> Node {
		crate::extensions::numbers::Number::parse_integer(digits)
			.map(Node::Number)
			.unwrap_or_else(|| error(&format!("Invalid int: {}", digits)))
	}

	/// Digits with `_` separators between them: 1_000_000
	pub(super) fn push_digits(&mut self, num_str: &mut String) {
		loop {
			let ch = self.current_char();
			// '三'.is_numeric() is true but not ASCII
			// Exclude superscripts and vulgar fractions: suffix operators and literals of their own
			if ch.is_numeric() && !is_number_glyph(ch) {
				num_str.push(ch);
				self.advance();
			} else if ch == '_' && self.prev_char().is_ascii_digit() && self.peek_char(1).is_ascii_digit() {
				self.advance();
			} else {
				break;
			}
		}
	}

	/// Scientific notation suffix: e3, E-3, e+03 (only when digits follow, so `2em` stays a unit)
	pub(super) fn parse_exponent(&mut self) -> Result<Option<i64>, String> {
		let (e, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		let signed = (c2 == '+' || c2 == '-') && c3.is_ascii_digit();
		if !(e == 'e' || e == 'E') || !(c2.is_ascii_digit() || signed) {
			return Ok(None);
		}
		self.advance();
		let mut exponent = String::new();
		if signed {
			exponent.push(self.current_char());
			self.advance();
		}
		self.push_digits(&mut exponent);
		exponent.parse::<i64>().map(Some).map_err(|_| format!("Invalid exponent: e{}", exponent))
	}

	/// `?` directly after a type name, then `=` (not `==`) or a closing bracket or separator
	pub(super) fn ends_optional_type(&self, next: char, after: char) -> bool {
		matches!(next, ')' | ']' | '}' | ',' | ';' | '\n' | '\r' | '\0') || (next == '=' && after != '=')
	}

	/// Do blanks and then a block of fields follow the cursor: `{}` or `{ name: …`, never a statement block like `{ out += x }`
	pub(super) fn block_after_blanks(&self) -> bool {
		let blanks_from = |start: usize| (start..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let blanks = blanks_from(0);
		if blanks == 0 || self.peek_char(blanks) != '{' {
			return false;
		}
		// the block's first entry may start on the next line (`Person {⏎ name: "Alice" …}`)
		let first = blanks + 1 + (blanks + 1..).take_while(|at| self.peek_char(*at).is_whitespace()).count();
		let name = (first..).take_while(|at| self.peek_char(*at).is_alphanumeric() || self.peek_char(*at) == '_').count();
		let after_name = first + name + blanks_from(first + name);
		self.peek_char(first) == '}' || (name > 0 && self.peek_char(after_name) == ':' && self.peek_char(after_name + 1) != '=')
	}

	/// Do blanks and then a closing bracket, `,`, `;` or the line end follow `offset`: nothing a ternary could be followed by
	pub(super) fn closes_after_blanks(&self, offset: usize) -> bool {
		let blanks = (offset..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		blanks > 0 && matches!(self.peek_char(offset + blanks), ')' | ']' | '}' | ',' | ';' | '\n' | '\r' | '\0')
	}
}
