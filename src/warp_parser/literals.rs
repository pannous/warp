//! Literals: values, strings with escapes and holes, numbers with units, fractions and exponents

use super::*;

const BACKTICK: char = '`';
/// The "got it" topic of the hint on backtick texts
const BACKTICK_TOPIC: &str = "backtick text";
const F_STRING_TOPIC: &str = "python f-string";
const F_STRING_PREFIX: char = 'f';
const FLOAT_WORD: &str = "float";
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

	pub(super) fn parse_string(&mut self) -> Node {
		self.reading_atom(AtomKind::Text, Self::read_text)
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
			crate::diagnostic::educate_once(
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
				return missing_closer(message, closing, crate::fixits::end_of_line(&self.input, quote_line));
			}
			if ch == closing {
				self.advance(); // skip closing quote
				if is_template {
					return crate::interpolation::template_text(&template);
				}
				return self.quoted_text(s.nfc().collect(), quote, (quote_line, quote_column), interpolates);
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
						let Some((name, length)) = crate::uniscript_entities::entity_name_at(&self.chars, self.pos - 1)
						else {
							unreachable!("a letter follows")
						};
						let Some(characters) = crate::uniscript_entities::entity(&name) else {
							if let Err(strict) = self.warn_unknown_entity(&name, self.column - 1) {
								return strict;
							}
							s.push('\\');
							template.push('\\');
							continue; // the name follows as written
						};
						self.advance_by(length - 1);
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
		match self
			.number_cast_follows()
			.filter(|_| interpolates && !c.is_ascii_digit())
		{
			Some(number_type) => {
				let message = format!("\"{c}\" as {number_type}: a text is no number (user decision #35); codepoint('{c}') is the code point of the character");
				let explicit = format!("{}('{c}') as {number_type}", crate::words::CODEPOINT);
				Diagnostic {
					message,
					line,
					column,
					..Default::default()
				}
				.fix(&explicit)
				.offer(
					"the code point of the character",
					format!("\"{c}\" as {number_type}"),
					explicit,
				)
				.into_error()
			}
			None => Node::codepoint(c),
		}
	}

	/// `\:xyzzy` names no entity: it stays as written and warns; in strict mode the warning is the error
	pub(super) fn warn_unknown_entity(&self, name: &str, column: usize) -> Result<(), Node> {
		let message = crate::uniscript_entities::unknown_entity(name);
		crate::diagnostic::report(&[Diagnostic {
			message,
			line: self.line_nr,
			column,
			..Default::default()
		}])
	}

	/// Python's `f"hi {name}"` at the cursor: interpolated text whose holes are braces, `"hi \(name)"`
	pub(super) fn parse_f_string(&mut self) -> Option<Node> {
		if self.current_char() != F_STRING_PREFIX || self.peek_char(1) != '"' || self.options.data_mode {
			return None;
		}
		set_hint_position(self.line_nr, self.column);
		crate::diagnostic::educate_once(
			F_STRING_TOPIC,
			"f\"…{x}…\"",
			"\"…\\(x)…\"",
			"warp text interpolates without a prefix",
		);
		self.advance();
		let outer = std::mem::replace(&mut self.brace_holes, true);
		let text = self.parse_string();
		self.brace_holes = outer;
		Some(text)
	}

	/// `shader { … }` (or `wgsl { … }`) at the cursor: WGSL as a sublanguage, the braces' content verbatim as text
	/// without its common indentation (card g_oFJc); braces inside WGSL comments do not count
	pub(super) fn parse_shader_block(&mut self) -> Option<Node> {
		if self.options.data_mode {
			return None;
		}
		let word = SHADER_WORDS
			.iter()
			.find(|word| word.chars().enumerate().all(|(index, ch)| self.peek_char(index) == ch))?;
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
				crate::fixits::end_of_source(&self.input),
			));
		};
		let source: String = self.chars[start..end].iter().collect();
		self.advance_by(end + 1 - self.pos);
		Some(crate::graphics::shader_holes::shader_text(&without_common_indentation(
			&source,
		)))
	}

	/// `\x1b` after the backslash: the character of the two hex digits, left at the second
	fn hex_byte_escape(&mut self) -> char {
		let hex: String = [self.peek_char(1), self.peek_char(2)].iter().collect();
		self.advance();
		self.advance();
		char::from(u8::from_str_radix(&hex, 16).expect("two hex digits"))
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
			return Err(format!(
				"\\u{{{hex}…: a unicode escape is hex digits in braces, like \\u{{e9}}"
			));
		}
		self.advance(); // now at `}`
		u32::from_str_radix(&hex, 16)
			.ok()
			.and_then(char::from_u32)
			.ok_or_else(|| format!("\\u{{{hex}}} is no unicode code point"))
	}

	/// The number type of a following `as int` / `as float` …
	pub(super) fn number_cast_follows(&self) -> Option<String> {
		let rest: String = self.chars[self.pos..].iter().take(32).collect();
		let words: Vec<&str> = rest.split_whitespace().take(2).collect();
		let number_type = words.get(1)?.trim_end_matches(|c: char| !is_identifier_char(c));
		use crate::type_kinds::Kind::{Float, Int};
		let is_number = matches!(crate::analyzer::builtin_type_kind(number_type), Some(Int | Float))
			&& !number_type.starts_with("bool");
		(words.first() == Some(&"as") && is_number).then(|| number_type.to_string())
	}

	/// `${expr}` or `$(expr)` inside interpolated text: the hole's expression, `None` for a plain dollar (`$5`, `$x`,
	/// `$ `). User decision D1: "only the one with the curly braces must interpolate the other is text like dollar
	/// money"; card dollar-paren (user, 2026-10-07): `$()` behaves just like `${}`.
	pub(super) fn parse_dollar_hole(&mut self) -> Result<Option<String>, String> {
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
	pub(super) fn parse_swift_hole(&mut self) -> Result<String, String> {
		let open = self.peek_char(1);
		self.advance_by(2);
		match open {
			'(' => self.text_until_closing('(', ')'),
			_ => self.text_until_closing('{', '}'),
		}
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
	pub(super) fn parse_number(&mut self) -> Node {
		let start = self.pos;
		let number = self.parse_number_value();
		let literal: String = self.chars[start..self.pos].iter().collect();
		let number = self.keep_source_literal(number, &literal);
		match self
			.literal_type_suffix()
			.or_else(|| self.scientific_float(&number, &literal))
		{
			Some(number_type) => Node::Key(Box::new(number), Op::As, Box::new(Node::Symbol(number_type))),
			None => self.with_juxtaposed_factor(number),
		}
	}

	/// `1.5e-7` is a float, as written by measurements and other programs (decision exact-decimals); `1e3` stays the
	/// integer 1000 and `0.5` the exact decimal. Code only: data keeps the number it reads.
	fn scientific_float(&self, number: &Node, literal: &str) -> Option<String> {
		let is_scientific = matches!(number, Node::Number(Number::Float(_))) && literal.contains(['e', 'E']);
		(is_scientific && !self.options.data_mode).then(|| FLOAT_WORD.to_string())
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
		let tight = (next.is_alphabetic() || next == '_' || next == '(' || next == crate::numbers::units::DEGREE)
			&& !self.at_ordinal_suffix();
		// `1500 meters` stays the pair the unit lowering reads (`3010 meters`), joined so it binds tighter than `>`
		if !tight
			&& self
				.spaced_word()
				.is_some_and(|word| crate::numbers::units::is_long_unit_name(&word))
		{
			self.skip_spaces();
			let unit = self.parse_atom();
			return Node::List(vec![number, unit], Bracket::None, Separator::Space);
		}
		if !tight && !self.at_spaced_unit() {
			return number;
		}
		if !tight {
			self.hint_glued_unit(&number);
		}
		self.skip_spaces();
		let factor = self.parse_atom();
		let factor = self.try_parse_superscript_power(&factor, 0).unwrap_or(factor);
		Node::Key(Box::new(number), Op::Mul, Box::new(factor))
	}

	/// `2 km` multiplies only when `km` is a known unit; any other spaced word keeps the list `[2 foo]`, and so does a
	/// unit word that starts the next entry or assignment: `{w:2 h:3}`; a spaced colon ends a condition instead:
	/// `if x > 1500 m : "far"`. σ joins like a unit: `5 ± 1 σ` is the Gaussian `5 ± 1σ` (card trailing-symbol)
	pub(super) fn at_spaced_unit(&self) -> bool {
		// `2 m²`: the power is no part of the unit's name
		self.spaced_word().is_some_and(|word| {
			word == crate::numbers::uncertain::SIGMA
				|| crate::numbers::units::names_unit(word.trim_end_matches(|c: char| superscript_digit(c).is_some()))
		})
	}

	/// `3 km` reads as `3km`, the form units are written in (user 2026-10-09: a soft hint)
	fn hint_glued_unit(&self, number: &Node) {
		let Some(unit) = self.spaced_word().filter(|word| word != crate::numbers::uncertain::SIGMA) else {
			return;
		};
		let amount = number.serialize();
		let (line, column) = self.get_position();
		set_hint_position(line, column);
		crate::normalize::hint(
			&format!("{amount} {unit}"),
			&format!("{amount}{unit}"),
			"a unit is written on its number",
		);
	}

	/// The word after one or more spaces, unless it starts the next entry or assignment
	fn spaced_word(&self) -> Option<String> {
		let rest = &self.chars[self.pos..];
		let spaces = rest.iter().take_while(|c| **c == ' ').count();
		let word: String = rest[spaces..].iter().take_while(|c| is_identifier_char(**c)).collect();
		let after_word = &rest[spaces + word.chars().count()..];
		let next_two: String = after_word.iter().skip_while(|c| **c == ' ').take(2).collect();
		let starts_entry = after_word.first() == Some(&':') || (next_two.starts_with('=') && next_two != "==");
		(spaces > 0 && !word.is_empty() && !starts_entry).then_some(word)
	}

	pub(super) fn at_ordinal_suffix(&self) -> bool {
		let word: String = self.chars[self.pos..]
			.iter()
			.take_while(|c| is_identifier_char(**c))
			.collect();
		ORDINAL_SUFFIXES.contains(&word.as_str())
	}

	/// Tight conversion written on the literal: `0.1f` → float, `0.1l` → exact, `0.1:float` / `1.5:int` → that type.
	/// Code only: in data `version: 1.10` and `n:int` stay key/value.
	pub(super) fn literal_type_suffix(&mut self) -> Option<String> {
		if self.options.data_mode {
			return None;
		}
		let suffix = LITERAL_SUFFIXES
			.iter()
			.find(|(letter, _)| *letter == self.current_char());
		if let Some((_, number_type)) = suffix.filter(|_| !is_identifier_char(self.peek_char(1))) {
			self.advance();
			return Some(number_type.to_string());
		}
		if self.current_char() != ':' {
			return None;
		}
		let name: String = self.chars[self.pos + 1..]
			.iter()
			.take_while(|c| is_identifier_char(**c))
			.collect();
		// `0.1:float`, `1.5:int`, `x:exact`: a number type binds tightly, unlike the loose `as`
		if !crate::type_tests::is_number_type_word(&name) {
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
		self.reading_atom(AtomKind::Number, Self::read_number_value)
	}

	fn read_number_value(&mut self) -> Node {
		// RFC 3339 / RFC 9557 literal: 2024-01-31, 2024-01-31T10:00Z, 2024-01-31T10:00[Europe/Berlin]
		// Not in data mode yet: the data path round-trips source text and has no date serialization
		let literal_len = if self.options.data_mode {
			0
		} else {
			crate::time::literal_len(&self.chars[self.pos..])
		};
		if literal_len > 0 {
			let literal = self.take_chars(literal_len);
			return Node::data(crate::time::TimeLiteral(literal));
		}
		let version_len = crate::versions::literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal = self.take_chars(version_len);
			return Node::Symbol(literal);
		}
		let mut num_str = String::new();

		// a leading plus is the unary plus prefix (peek_prefix_operator)
		if self.current_char() == '-' {
			num_str.push('-');
			self.advance();
		}

		if self.current_char() == '0'
			&& matches!(self.peek_char(1), 'b' | 'B')
			&& matches!(self.peek_char(2), '0' | '1')
		{
			self.advance_by(2);
			let bits: String = self
				.take_while_char(|ch| matches!(ch, '0' | '1' | '_'))
				.replace('_', "");
			return i64::from_str_radix(&format!("{num_str}{bits}"), 2)
				.map(Node::int)
				.unwrap_or_else(|_| error(&format!("Invalid binary: 0b{bits}")));
		}
		if self.current_char() == '0' && matches!(self.peek_char(1), 'x' | 'X') {
			self.advance_by(2);
			let hex_str = self.take_while_char(|ch| ch.is_ascii_hexdigit());
			return i64::from_str_radix(&hex_str, 16)
				.map(Node::int)
				.unwrap_or_else(|_| error(&format!("Invalid hex: 0x{}", hex_str)));
		}

		if let Some(fraction) = self.vulgar_fraction_literal() {
			return fraction;
		}

		self.push_digits(&mut num_str);
		if let Some(repeating) = self.repeating_decimal_literal(&num_str) {
			return repeating;
		}
		let mut is_float = false;
		// Only consume . as decimal if NOT followed by another . (range operator) or a word (`4.square` calls square)
		let after_point = self.peek_char(1);
		if self.current_char() == '.' && after_point != '.' && !(after_point.is_alphabetic() || after_point == '_') {
			is_float = true;
			num_str.push('.');
			self.advance();
			self.push_digits(&mut num_str);
		}
		let exponent = or_return_error!(self.parse_exponent());

		match exponent {
			// 1e3 is the exact integer 1000, like the literal it abbreviates
			Some(exp) if !is_float && exp >= 0 => {
				if exp > MAX_INTEGER_EXPONENT {
					return error(&format!("Exponent too large: {}e{}", num_str, exp));
				}
				num_str.push_str(&"0".repeat(exp as usize));
				self.integer_node(&num_str)
			}
			Some(exp) => float_node(&format!("{}e{}", num_str, exp)),
			None if is_float => float_node(&num_str),
			None => {
				let number = self.integer_node(&num_str);
				self.with_time_unit(number)
			}
		}
	}

	/// `0.3̅`, a combining overline on each repeating digit, is the exact ratio it spells, 1/3 (card repeating-decimals)
	pub(super) fn repeating_decimal_literal(&mut self, integer: &str) -> Option<Node> {
		if self.current_char() != '.' {
			return None;
		}
		let fraction = &self.chars[self.pos + 1..];
		let (fixed, period, length) = overlined_fraction(fraction)?;
		self.advance_by(1 + length);
		let (negative, integer) = integer
			.strip_prefix('-')
			.map_or((false, integer), |digits| (true, digits));
		Some(Node::Number(Number::repeating_decimal(
			negative, integer, &fixed, &period,
		)))
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

	/// `1 month`, `24 hours`: an integer followed by a time unit word is a duration; `1 month 2 days` is one duration
	pub(super) fn with_time_unit(&mut self, number: Node) -> Node {
		let Node::Number(Number::Int(count)) = &number else {
			return number;
		};
		if self.options.data_mode {
			return number;
		}
		let Some((unit, length)) = self.time_unit_at(0) else {
			return number;
		};
		self.advance_by(length);
		let mut duration = unit.times(*count);
		while let Some((following, length)) = self.following_duration() {
			self.advance_by(length);
			duration = duration.plus(following);
		}
		Node::data(duration)
	}

	/// The unit word `offset` chars ahead and its length with leading spaces; `{year:1970 month:1}`: a unit word that
	/// starts the next entry or assignment is no unit
	fn time_unit_at(&self, offset: usize) -> Option<(crate::time::Duration, usize)> {
		let (unit, length) = crate::time::unit_after(&self.chars[self.pos + offset..])?;
		let after_word: String = self.chars[self.pos + offset + length..]
			.iter()
			.skip_while(|c| **c == ' ')
			.take(2)
			.collect();
		let starts_entry = after_word.starts_with(':') || (after_word.starts_with('=') && after_word != "==");
		(!starts_entry).then_some((unit, length))
	}

	/// ` 2 days` after a duration: the next amount of the same duration, and how many chars it takes
	fn following_duration(&self) -> Option<(crate::time::Duration, usize)> {
		let rest = &self.chars[self.pos..];
		let spaces = rest.iter().take_while(|c| **c == ' ').count();
		let digits = rest[spaces..].iter().take_while(|c| c.is_ascii_digit()).count();
		if spaces == 0 || digits == 0 {
			return None;
		}
		let count: i64 = rest[spaces..spaces + digits].iter().collect::<String>().parse().ok()?;
		let (unit, length) = self.time_unit_at(spaces + digits)?;
		Some((unit.times(count), spaces + digits + length))
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
		exponent
			.parse::<i64>()
			.map(Some)
			.map_err(|_| format!("Invalid exponent: e{}", exponent))
	}

	/// `?` directly after a type name, then `=` (not `==`) or a closing bracket or separator
	pub(super) fn ends_optional_type(&self, next: char, after: char) -> bool {
		matches!(next, ')' | ']' | '}' | ',' | ';' | '\n' | '\r' | '\0') || (next == '=' && after != '=')
	}

	/// In a type's fields, blanks and then the next field `right: …` after `width` characters: `left: Node? right: Node?` on
	/// one line is two optional fields, no ternary
	pub(super) fn next_field_follows(&self, width: usize) -> bool {
		let blanks = (width..)
			.take_while(|at| matches!(self.peek_char(*at), ' ' | '\t'))
			.count();
		let name = self.word_at(width + blanks).chars().count();
		let colon = width + blanks + name;
		self.type_fields.is_some()
			&& blanks > 0
			&& name > 0
			&& self.peek_char(colon) == ':'
			&& !matches!(self.peek_char(colon + 1), '=' | ':')
	}

	/// Do blanks and then `bracket` follow (`c {`, not the glued `c{`)
	pub(super) fn blanks_then(&self, bracket: char) -> bool {
		let blanks = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		blanks > 0 && self.peek_char(blanks) == bracket
	}

	/// Do blanks and then a block of fields follow the cursor: `{}` or `{ name: …`, never a statement block like `{ out += x }`
	/// `assignments`: C#'s object initializer `Point { X = 3 }` of a declared class sets its fields with `=` too
	pub(super) fn block_after_blanks(&self, assignments: bool) -> bool {
		let blanks_from = |start: usize| {
			(start..)
				.take_while(|at| matches!(self.peek_char(*at), ' ' | '\t'))
				.count()
		};
		let blanks = blanks_from(0);
		if blanks == 0 || self.peek_char(blanks) != '{' {
			return false;
		}
		// the block's first entry may start on the next line (`Person {⏎ name: "Alice" …}`)
		let first = blanks
			+ 1 + (blanks + 1..)
			.take_while(|at| self.peek_char(*at).is_whitespace())
			.count();
		let name = (first..)
			.take_while(|at| self.peek_char(*at).is_alphanumeric() || self.peek_char(*at) == '_')
			.count();
		let after_name = first + name + blanks_from(first + name);
		let assigns = assignments && self.peek_char(after_name) == '=' && self.peek_char(after_name + 1) != '=';
		self.peek_char(first) == '}'
			|| (name > 0 && ((self.peek_char(after_name) == ':' && self.peek_char(after_name + 1) != '=') || assigns))
	}

	/// Do blanks and then a closing bracket, `,`, `;`, an assignment `=`, a comment or the line end follow `offset`:
	/// nothing a ternary could be followed by
	pub(super) fn closes_after_blanks(&self, offset: usize) -> bool {
		let blanks = (offset..)
			.take_while(|at| matches!(self.peek_char(*at), ' ' | '\t'))
			.count();
		let at = offset + blanks;
		let comment_starts = self.peek_char(at) == '/' && matches!(self.peek_char(at + 1), '/' | '*');
		blanks > 0 && (comment_starts || self.ends_optional_type(self.peek_char(at), self.peek_char(at + 1)))
	}
}

/// The length of `chars` before the `}` that closes an already open `{`, skipping `//` and `/* */` comments
fn closing_brace(chars: &[char]) -> Option<usize> {
	let (mut depth, mut at) = (1, 0);
	while at < chars.len() {
		match (chars[at], chars.get(at + 1)) {
			('/', Some('/')) => {
				at += chars[at..]
					.iter()
					.position(|&ch| ch == '\n')
					.unwrap_or(chars.len() - at)
			}
			('/', Some('*')) => {
				at += chars[at..]
					.windows(2)
					.position(|pair| pair == ['*', '/'])
					.map_or(chars.len() - at, |end| end + 1)
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
fn without_common_indentation(source: &str) -> String {
	dedented_lines(source)
		.into_iter()
		.map(|line| line.text)
		.collect::<Vec<_>>()
		.join("\n")
}

/// A line of a shader block's text: which line of the block it is (0: the one of its `{`), how many characters before
/// it the text dropped, and the text
pub struct DedentedLine {
	pub line: usize,
	pub dropped: usize,
	pub text: String,
}

/// The lines of `source` without the indentation all non-blank ones share, without the blank lines around them, and
/// the first without any
fn dedented_lines(source: &str) -> Vec<DedentedLine> {
	let lines: Vec<&str> = source.lines().collect();
	let is_blank = |line: &&str| line.trim().is_empty();
	let common = lines
		.iter()
		.filter(|line| !is_blank(line))
		.map(|line| line.len() - line.trim_start().len())
		.min()
		.unwrap_or(0);
	let first = lines.iter().position(|line| !is_blank(line)).unwrap_or(lines.len());
	let end = lines
		.iter()
		.rposition(|line| !is_blank(line))
		.map_or(first, |last| last + 1);
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

/// Each `shader { … }` (or `wgsl { … }`) block of `code`: the line and column right after its `{`, and its text's lines
pub fn shader_blocks(code: &str) -> Vec<((usize, usize), Vec<DedentedLine>)> {
	let chars: Vec<char> = code.chars().collect();
	let is_name_part = |at: usize| chars.get(at).is_some_and(|ch| ch.is_alphanumeric() || *ch == '_');
	let mut blocks = vec![];
	let mut at = 0;
	while at < chars.len() {
		let word = SHADER_WORDS.iter().find(|word| {
			!is_name_part(at.wrapping_sub(1)) && chars[at..].starts_with(&word.chars().collect::<Vec<_>>())
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
		let Some(length) = (chars.get(open) == Some(&'{'))
			.then(|| closing_brace(&chars[start..]))
			.flatten()
		else {
			at = open;
			continue;
		};
		let before = &chars[..start];
		let line = 1 + before.iter().filter(|&&ch| ch == '\n').count();
		let column = 1 + before.iter().rev().take_while(|&&ch| ch != '\n').count();
		let source: String = chars[start..start + length].iter().collect();
		blocks.push(((line, column), dedented_lines(&source)));
		at = start + length;
	}
	blocks
}

/// The fixed and the overlined digits of a fraction (`16̅`: "1", "6") and its length in characters; none without an
/// overline, or with a plain digit after an overlined one
fn overlined_fraction(chars: &[char]) -> Option<(String, String, usize)> {
	let (mut fixed, mut period, mut at) = (String::new(), String::new(), 0);
	while let Some(&digit) = chars.get(at).filter(|ch| ch.is_ascii_digit()) {
		let overlined = chars.get(at + 1) == Some(&OVERLINE);
		if overlined {
			period.push(digit);
			at += 2;
		} else if period.is_empty() {
			fixed.push(digit);
			at += 1;
		} else {
			return None;
		}
	}
	(!period.is_empty()).then_some((fixed, period, at))
}

/// The float written `text`, or the error naming it; written with more digits than a float keeps, the exact ratio
fn float_node(text: &str) -> Node {
	if let Some(exact) = crate::extensions::numbers::long_decimal(text) {
		return Node::Number(exact);
	}
	text.parse::<f64>()
		.map(Node::float)
		.unwrap_or_else(|_| error(&format!("Invalid float: {text}")))
}
