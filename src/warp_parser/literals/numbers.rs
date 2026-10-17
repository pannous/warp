//! Numbers: integers, floats, exponents, repeating decimals, vulgar fractions, ordinals, literal type suffixes
use super::*;

/// The litre's symbols, glued to a number only with `use units` (refused_glued_litre)
const LITRE_SYMBOLS: [char; 2] = ['L', 'l'];

/// The fixed and the overlined digits of a fraction (`16̅`: "1", "6") and its length in characters; none without an
/// overline, or with a plain digit after an overlined one
pub(crate) fn overlined_fraction(chars: &[char]) -> Option<(String, String, usize)> {
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
pub(crate) fn float_node(text: &str) -> Node {
	if let Some(exact) = crate::extensions::numbers::long_decimal(text) {
		return Node::Number(exact);
	}
	text.parse::<f64>().map(Node::float).unwrap_or_else(|_| error(&format!("Invalid float: {text}")))
}

/// Numerator and denominator of a Unicode vulgar fraction character: ½ → (1, 2), ⅚ → (5, 6), ↉ → (0, 3)
pub(crate) fn vulgar_fraction(ch: char) -> Option<(i64, i64)> {
	let decomposed: String = std::iter::once(ch).nfkd().collect();
	let (numerator, denominator) = decomposed.split_once(FRACTION_SLASH)?;
	Some((numerator.parse().ok()?, denominator.parse().ok()?))
}

/// Characters that are numeric for Unicode but written next to a number as operators or literals of their own
pub(crate) fn is_number_glyph(ch: char) -> bool {
	// every such glyph is a numeric character outside ASCII: asked of every identifier character, this skips the
	// decomposition of vulgar_fraction for all others
	!ch.is_ascii() && ch.is_numeric() && (superscript_digit(ch).is_some() || vulgar_fraction(ch).is_some())
}

/// The number a text spells if the whole text is one number literal, so `int "12a"` never guesses; "1/3" spells the
/// exact ratio of two Ints, as the code 1/3 does (card fraction-number)
pub fn number_in_text(text: &str) -> Option<Number> {
	let text = text.trim();
	if let Some((numerator, denominator)) = text.split_once('/') {
		return match (number_in_text(numerator)?, number_in_text(denominator)?) {
			(numerator @ Number::Int(_), denominator @ Number::Int(_)) => Some(Number::ratio(numerator, denominator)),
			_ => None,
		};
	}
	let spells_number = !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || "+-._eE".contains(c));
	if !spells_number {
		return None;
	}
	// read as data: code marks `2.5e-2` `as float` (scientific_float), a text spells just the number
	match parse_data(text).drop_meta() {
		Node::Number(number) => Some(*number),
		_ => None,
	}
}

pub(crate) fn real(exact: Exact) -> Node {
	Node::Number(Number::real(Real::Exact(exact)))
}

impl WarpParser {
	/// The number type of a following `as int` / `as float` …
	pub(crate) fn number_cast_follows(&self) -> Option<String> {
		let rest: String = self.chars[self.pos..].iter().take(32).collect();
		let words: Vec<&str> = rest.split_whitespace().take(2).collect();
		let number_type = words.get(1)?.trim_end_matches(|c: char| !is_identifier_char(c));
		use crate::type_kinds::Kind::{Float, Int};
		let is_number = matches!(crate::analyzer::builtin_type_kind(number_type), Some(Int | Float))
			&& !number_type.starts_with("bool");
		(words.first() == Some(&"as") && is_number).then(|| number_type.to_string())
	}

	pub(crate) fn parse_number(&mut self) -> Node {
		let start = self.pos;
		let number = self.parse_number_value();
		let literal: String = self.chars[start..self.pos].iter().collect();
		let number = self.keep_source_literal(number, &literal);
		if let Some(refusal) = self.refused_glued_litre(&literal) {
			return refusal;
		}
		match self.literal_type_suffix().or_else(|| self.scientific_float(&number, &literal)) {
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

	pub(crate) fn at_ordinal_suffix(&self) -> bool {
		let word: String = self.chars[self.pos..].iter().take_while(|c| is_identifier_char(**c)).collect();
		ORDINAL_SUFFIXES.contains(&word.as_str())
	}

	/// `2L`, `2l` glued: the litre, which C's long-double suffix spells too, so a program says `use units` to mean it
	/// (decision litre-suffix); `2 L` with a space is the litre anyway
	fn refused_glued_litre(&self, literal: &str) -> Option<Node> {
		let unit = self.current_char();
		if is_identifier_char(self.peek_char(1)) || !self.glued_litre_refused(&unit.to_string()) {
			return None;
		}
		Some(crate::node::error(&format!(
			"{literal}{unit}: {unit} is the litre unit here; add `use units` (for an exact number write {literal} as exact)"
		)))
	}

	/// Is `unit` the litre, which this program may not glue to a number (no `use units`)
	pub(crate) fn glued_litre_refused(&self, unit: &str) -> bool {
		let is_litre = LITRE_SYMBOLS.iter().any(|symbol| unit == symbol.to_string());
		is_litre && !self.options.data_mode && !self.source_uses(crate::modules::UNITS_MODULE)
	}

	/// Does a line of the source say `use units` (or `import`, `require`)
	fn source_uses(&self, module: &str) -> bool {
		let source: String = self.chars.iter().collect();
		source.lines().any(|line| {
			let words: Vec<&str> = line.split_whitespace().take(2).collect();
			matches!(words.as_slice(), [keyword, name] if crate::modules::USE_KEYWORDS.contains(keyword)
				&& name.trim_end_matches(';') == module)
		})
	}

	/// Tight conversion written on the literal: `0.1f` → float, `0.1:float` / `1.5:int` → that type.
	/// Code only: in data `version: 1.10` and `n:int` stay key/value.
	pub(crate) fn literal_type_suffix(&mut self) -> Option<String> {
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
		// `0.1:float`, `1.5:int`, `x:exact`: a number type binds tightly, unlike the loose `as`
		if !crate::type_tests::is_number_type_word(&name) {
			return None;
		}
		self.advance_by(1 + name.chars().count());
		Some(name)
	}

	/// Data round-trips its literals: `zip: 01234` stays `01234`, `version: 1.10` stays `1.10`
	pub(crate) fn keep_source_literal(&self, number: Node, literal: &str) -> Node {
		let is_lossy = matches!(number, Node::Number(_)) && number.serialize() != literal;
		if self.options.data_mode && is_lossy {
			number.with_source_literal(literal)
		} else {
			number
		}
	}

	pub(crate) fn parse_number_value(&mut self) -> Node {
		self.reading_atom(AtomKind::Number, Self::read_number_value)
	}

	fn read_number_value(&mut self) -> Node {
		// RFC 3339 / RFC 9557 literal: 2024-01-31, 2024-01-31T10:00Z, 2024-01-31T10:00[Europe/Berlin]
		// Not in data mode yet: the data path round-trips source text and has no date serialization
		let literal_len = if self.options.data_mode { 0 } else { crate::time::literal_len(&self.chars[self.pos..]) };
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
			let bits: String = self.take_while_char(|ch| matches!(ch, '0' | '1' | '_')).replace('_', "");
			return i64::from_str_radix(&format!("{num_str}{bits}"), 2)
				.map(Node::int)
				.unwrap_or_else(|_| error(&format!("Invalid binary: 0b{bits}")));
		}
		if self.current_char() == '0' && matches!(self.peek_char(1), 'x' | 'X') {
			self.advance_by(2);
			let mut hex_str = String::new();
			// `_` only between hex digits: `0xFF_FF`
			while self.current_char().is_ascii_hexdigit()
				|| (self.current_char() == '_' && !hex_str.is_empty() && self.peek_char(1).is_ascii_hexdigit())
			{
				if self.current_char() != '_' {
					hex_str.push(self.current_char());
				}
				self.advance();
			}
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
	pub(crate) fn repeating_decimal_literal(&mut self, integer: &str) -> Option<Node> {
		if self.current_char() != '.' {
			return None;
		}
		let fraction = &self.chars[self.pos + 1..];
		let (fixed, period, length) = overlined_fraction(fraction)?;
		self.advance_by(1 + length);
		let (negative, integer) = integer.strip_prefix('-').map_or((false, integer), |digits| (true, digits));
		Some(Node::Number(Number::repeating_decimal(negative, integer, &fixed, &period)))
	}

	/// `⅓` is the exact ratio 1/3; a number written directly after it multiplies: `⅓9` is 3
	pub(crate) fn vulgar_fraction_literal(&mut self) -> Option<Node> {
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

	pub(crate) fn integer_node(&self, digits: &str) -> Node {
		crate::extensions::numbers::Number::parse_integer(digits)
			.map(Node::Number)
			.unwrap_or_else(|| error(&format!("Invalid int: {}", digits)))
	}

	/// Digits with `_` separators between them: 1_000_000
	pub(crate) fn push_digits(&mut self, num_str: &mut String) {
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
	pub(crate) fn parse_exponent(&mut self) -> Result<Option<i64>, String> {
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
}
