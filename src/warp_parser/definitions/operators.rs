//! User operators: `infix operator ⊕`, their glyphs, precedence declarations, operator methods and calls
use super::*;

/// The got-it note on `infix divides(d, n) := …`, the alias of `infix operator divides(d, n) := …` (card g_mnvA)
const INFIX_WORD_TOPIC: &str = "infix-operator-word";

/// A glyph a program may declare as an operator: symbols with a non-ASCII character (`‼`, `⊕`), never a letter or digit.
/// Superscript digits and signs too (P48, user-decided) (`suffix operator ³`, `prefix operator ⁻`, wiki/operator.md)
pub(crate) fn is_operator_glyph(glyph: &str) -> bool {
	let is_superscript = |c: char| superscript_digit(c).is_some() || superscript_sign(c).is_some();
	!glyph.is_empty()
		&& glyph.chars().all(|c| is_superscript(c) || (!c.is_alphanumeric() && !c.is_whitespace()))
		&& !glyph.is_ascii()
}

/// `suffix operator ⁰ := …` and the short `suffix ⁰ := …` (P48): the glyph and the words after `:=`
pub(crate) fn declared_glyph<'a>(words: &'a [&'a str]) -> Option<&'a str> {
	match words {
		["operator", glyph, ":=", ..] | [glyph, ":=", ..] => Some(glyph),
		_ => None,
	}
}

/// The operator a `prefix|suffix|infix` declaration declares: a glyph (`infix operator ⊕ := …`), or for infix a word with
/// optional parameters, `infix operator divides(d:int, n:int) := …` (card g_mnvA); `infix divides(d, n) := …` is the
/// alias without `operator`
pub(crate) fn declared_operator(kind: UserOperatorKind, words: &[&str]) -> Option<String> {
	if let Some(glyph) = declared_glyph(words).filter(|glyph| is_operator_glyph(glyph)) {
		return Some(glyph.to_string());
	}
	let head = match words {
		[OPERATOR_WORD, head, ..] | [head, ..] if kind == UserOperatorKind::Infix && words.contains(&":=") => head,
		_ => return None,
	};
	// a word, or a glyph with its parameters: `infix operator ↑(x: number, n: number) := …` (card easy-add)
	let word: String = match head.starts_with(char::is_alphabetic) {
		true => head.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect(),
		false => head.chars().take_while(|c| *c != '(').collect(),
	};
	let parameters = &head[word.len()..];
	let is_operator = (word.starts_with(char::is_alphabetic) && word != OPERATOR_WORD) || is_operator_glyph(&word);
	(is_operator && (parameters.is_empty() || parameters.starts_with('('))).then_some(word)
}

/// `divides`: a declared operator that is a word, ending where the word ends (not `dividesx`)
pub(crate) fn is_operator_word(glyph: &[char]) -> bool {
	glyph.first().is_some_and(|c| c.is_alphabetic())
}

/// The operators a source declares, and those of the standard modules it uses: `use regex` makes `matches` infix
/// (card matches-infix)
pub(crate) fn scan_user_operators(source: &str) -> Vec<UserOperator> {
	let mut found = declared_operators(source);
	for module in used_std_modules(source) {
		found.extend(declared_operators(module));
	}
	found
}

/// The operators a source itself declares: `prefix|suffix|infix operator ⊕ := body` and the pattern `a ⊕ b := body` (infix)
pub(crate) fn declared_operators(source: &str) -> Vec<UserOperator> {
	let mut found: Vec<UserOperator> = Vec::new();
	for statement in source.lines().flat_map(|line| line.split(';')) {
		let words: Vec<&str> = statement.split_whitespace().collect();
		let by_kind = words.split_first().and_then(|(word, rest)| {
			let (_, kind) = OPERATOR_KINDS.iter().find(|(keyword, _)| keyword == word)?;
			Some((declared_operator(*kind, rest)?, *kind))
		});
		let declared = by_kind.or_else(|| match words.as_slice() {
			[left, glyph, right, ":=", ..]
				if is_plain_name(left) && is_plain_name(right) && is_operator_glyph(glyph) =>
			{
				Some((glyph.to_string(), UserOperatorKind::Infix))
			}
			_ => None,
		});
		if let Some((glyph, kind)) = declared {
			found.push(UserOperator { glyph: glyph.chars().collect(), kind, level: kind.default_level() });
		}
	}
	// `operator ⊕ has precedence above *`, wherever it stands, once every operator is known
	for statement in source.lines().flat_map(|line| line.split(';')) {
		let words: Vec<&str> = statement.split_whitespace().collect();
		if let Ok(Some((glyph, level))) = precedence_declaration(&words, &found) {
			found
				.iter_mut()
				.filter(|operator| operator.glyph == glyph.chars().collect::<Vec<char>>())
				.for_each(|operator| operator.level = level);
		}
	}
	found.sort_by_key(|operator| std::cmp::Reverse(operator.glyph.len()));
	found
}

/// `operator ⊕ has precedence above|below [operator] Y`: the glyph ⊕ and its new level; Ok(None) for any other statement,
/// Err for a precedence declaration that cannot be followed (a built-in ⊕, an unknown Y)
pub(crate) fn precedence_declaration<'a>(
	words: &[&'a str],
	declared: &[UserOperator],
) -> Result<Option<(&'a str, u8)>, String> {
	let ["operator", glyph, "has", "precedence", direction, rest @ ..] = words else {
		return Ok(None);
	};
	let Some((_, above)) = PRECEDENCE_DIRECTIONS.iter().find(|(word, _)| word == direction) else {
		return Ok(None);
	};
	let other = match rest {
		["operator", other] | [other] => *other,
		_ => return Ok(None),
	};
	if !declared.iter().any(|operator| operator.glyph.iter().collect::<String>() == *glyph) {
		return Err(format!("operator {glyph} is built in: only a declared operator gets a precedence (`infix operator {glyph} := …` first)"));
	}
	let reference = match declared.iter().find(|operator| operator.glyph.iter().collect::<String>() == other) {
		Some(operator) => operator.level,
		None => built_in_level(other).ok_or_else(|| {
			format!("unknown operator {other} in `operator {glyph} has precedence {direction} {other}`")
		})?,
	};
	let level =
		if *above { reference.saturating_add(PRECEDENCE_STEP) } else { reference.saturating_sub(PRECEDENCE_STEP) };
	Ok(Some((glyph, level)))
}

/// The left binding power of a built-in infix operator written as `text` (`*`, `+`, `and`)
pub(crate) fn built_in_level(text: &str) -> Option<u8> {
	let parser =
		WarpParser::new_with_options(text.to_string(), ParserOptions { data_mode: true, ..ParserOptions::default() });
	match parser.peek_operator() {
		Some((op, length)) if length == text.chars().count() => Some(op.binding_power().0),
		_ => None,
	}
}

pub(crate) fn operator_call(glyph: &[char], arguments: Vec<Node>) -> Node {
	let name = format!("{OPERATOR_FUNCTION_PREFIX}{}", glyph.iter().collect::<String>());
	Node::List([vec![Symbol(name)], arguments].concat(), Bracket::Round, Separator::None)
}

impl WarpParser {
	/// `+(o) := …` defines the method of an operator by its glyph (a class's `plus`): the head is the call `+(o)`, no
	/// unary plus
	pub(crate) fn try_parse_operator_method_head(&mut self) -> Option<Node> {
		let (op, chars) = self.peek_operator()?;
		crate::operators::operator_method(op)?;
		let mut offset = chars + self.parenthesis_length(chars)?;
		while matches!(self.peek_char(offset), ' ' | '\t') {
			offset += 1;
		}
		if (self.peek_char(offset), self.peek_char(offset + 1)) != (':', '=') {
			return None;
		}
		self.advance_by(chars);
		let parameters = match self.parse_atom().drop_meta().clone() {
			Node::List(parameters, Bracket::Round, _) => parameters,
			Node::Empty => vec![],
			parameter => vec![parameter],
		};
		Some(Node::List([vec![Symbol(op.to_string())], parameters].concat(), Bracket::Round, Separator::None))
	}

	/// The length of the balanced `(…)` at `offset`, None when none starts there or it never closes
	pub(crate) fn parenthesis_length(&self, offset: usize) -> Option<usize> {
		if self.peek_char(offset) != '(' {
			return None;
		}
		let mut depth = 0;
		for length in 0.. {
			match self.peek_char(offset + length) {
				'(' => depth += 1,
				')' if depth == 1 => return Some(length + 1),
				')' => depth -= 1,
				'\0' => return None,
				_ => {}
			}
		}
		None
	}

	/// An infix operator alone as an argument or item, `sorted(xs, >)`, `[+, *]`: the operator as a value `(> ø ø)`, as
	/// `by: >` gives
	pub(crate) fn try_parse_operator_value(&mut self) -> Option<Node> {
		let (op, chars) = self.peek_operator()?;
		if self.current_char().is_alphabetic() {
			return None; // a word operator alone is a name: the parameter `to` of `hanoi(n, from, to, via)`
		}
		let blanks = (chars..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		// a pattern's rest `Point{x, ..}` (lowering/switch.rs rest_pattern) also ends a block
		let ends_rest = matches!(op, Op::Range | Op::To) && self.peek_char(chars + blanks) == '}';
		if !matches!(self.peek_char(chars + blanks), ')' | ',' | ']') && !ends_rest {
			return None;
		}
		self.advance_by(chars);
		Some(Node::Key(Box::new(Node::Empty), op, Box::new(Node::Empty)))
	}

	/// The declared operator written at the cursor, of one of the kinds
	pub(crate) fn user_operator_at(&self, kinds: &[UserOperatorKind]) -> Option<UserOperator> {
		let is_name_char = |c: char| c.is_alphanumeric() || c == '_';
		let whole_word = |operator: &UserOperator| {
			!is_operator_word(&operator.glyph) || !is_name_char(self.peek_char(operator.glyph.len()))
		};
		self.user_operators
			.iter()
			.find(|operator| {
				kinds.contains(&operator.kind)
					&& self.chars[self.pos..].starts_with(&operator.glyph)
					&& whole_word(operator)
			})
			.cloned()
	}

	/// `3‼`: the call of the suffix operator on the left operand
	pub(crate) fn try_parse_user_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Suffix])?;
		if min_bp > operator.level {
			return None;
		}
		self.advance_by(operator.glyph.len());
		Some(operator_call(&operator.glyph, vec![lhs.clone()]))
	}

	/// `2 ⊕ 3`: the call of the infix operator on both operands
	pub(crate) fn try_parse_user_infix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Infix])?;
		let (left_bp, right_bp) = (operator.level, operator.level + 1);
		if left_bp < min_bp {
			return None;
		}
		self.advance_by(operator.glyph.len());
		self.skip_whitespace();
		let rhs = self.parse_expr(right_bp);
		Some(operator_call(&operator.glyph, vec![lhs.clone(), rhs]))
	}

	/// `∆3`: the call of the prefix operator on the operand after it
	pub(crate) fn try_parse_user_prefix(&mut self) -> Option<Node> {
		let operator = self.user_operator_at(&[UserOperatorKind::Prefix])?;
		self.advance_by(operator.glyph.len());
		self.skip_spaces();
		let operand = self.parse_expr(operator.level);
		Some(operator_call(&operator.glyph, vec![operand]))
	}

	/// `prefix|suffix|infix operator ⊕ := body` defines the function `operator_⊕`: its parameter is `it` (prefix, suffix) or `a b` (infix).
	/// `operator ⊕ has precedence above +` is not supported.
	pub(crate) fn try_parse_operator_declaration(&mut self, word: &str) -> Option<Node> {
		// the keyword first: reading the rest of the line for every word was quadratic on a long line (card big-literals)
		let is_keyword = word == OPERATOR_WORD || OPERATOR_KINDS.iter().any(|(keyword, _)| *keyword == word);
		if !is_keyword || self.options != ParserOptions::default() {
			return None;
		}
		let rest: String = self.chars[self.pos..].iter().take_while(|c| **c != '\n' && **c != ';').collect();
		let words: Vec<&str> = rest.split_whitespace().collect();
		if word == "operator" && words.get(1) == Some(&"has") && words.get(2) == Some(&"precedence") {
			while !matches!(self.current_char(), '\n' | ';' | '\0') {
				self.advance();
			}
			// the pre-scan applied it (scan_user_operators); here only its error is reported
			let statement: Vec<&str> = std::iter::once("operator").chain(words.iter().copied()).collect();
			return Some(match precedence_declaration(&statement, &self.user_operators) {
				Ok(Some(_)) => Empty,
				Ok(None) => error(&format!(
					"an operator precedence is declared `operator ⊕ has precedence above|below Y`, not `operator {}`",
					words.join(" ")
				)),
				Err(message) => error(&message),
			});
		}
		let (_, kind) = OPERATOR_KINDS.iter().find(|(keyword, _)| *keyword == word)?;
		let glyph: Vec<char> = declared_operator(*kind, &words)?.chars().collect();
		let has_operator_word = words.first() == Some(&OPERATOR_WORD);
		self.skip_spaces();
		if has_operator_word {
			self.advance_by(OPERATOR_WORD.chars().count());
			self.skip_spaces();
		} else if is_operator_word(&glyph) {
			self.set_hint_pos();
			let name: String = glyph.iter().collect();
			crate::errors::diagnostic::educate_once(
				INFIX_WORD_TOPIC,
				&format!("{word} {name}"),
				&format!("{word} {OPERATOR_WORD} {name}"),
				"a declared operator says operator",
			);
		}
		self.advance_by(glyph.len());
		// `divides(d:int, n:int)`: the operands named and typed as a function's parameters
		let declared_parameters =
			(self.current_char() == '(').then(|| self.parse_expr(Op::Define.binding_power().0 + 1));
		self.skip_spaces();
		self.advance_by(":=".len());
		let body = self.parse_expr(0);
		let parameters = match kind {
			UserOperatorKind::Infix if let Some(declared) = declared_parameters => match declared.drop_meta() {
				Node::List(items, Bracket::Round, _) => items.clone(),
				one => vec![one.clone()],
			},
			// P48 names the operands `left` and `right`; `a` and `b` stay accepted
			UserOperatorKind::Infix => {
				let names =
					if mentions(&body, "left") || mentions(&body, "right") { ["left", "right"] } else { ["a", "b"] };
				names.map(|name| Symbol(name.to_string())).to_vec()
			}
			_ => vec![Symbol(crate::words::IMPLICIT_PARAMETER.to_string())],
		};
		let head = operator_call(&glyph, parameters);
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// Hint when the operator written at the cursor is not the canonical one; call once when the operator is consumed
	/// The operator as written, after hinting at its preferred spelling
	pub(crate) fn hint_operator(&self, chars: usize, is_prefix: bool) -> String {
		let written: String = (0..chars).map(|offset| self.peek_char(offset)).collect();
		self.set_hint_pos();
		norm::operator(&written, is_prefix);
		written
	}
}
