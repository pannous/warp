//! Suffixes: superscripts, suffix operators, bang and test words, subscripts, slices, implicit application

use super::*;

/// The word between a condition and its branch: `if c then x`
const THEN_WORD: &str = "then";

impl WarpParser {
	/// The exponent written in superscript digits and signs at the cursor, its length in characters and whether it has a sign:
	/// ⁴ → (4, 1), ¹² → (12, 2), ⁻¹ → (-1, 2), ²⁺³ → (5, 3)
	pub(super) fn superscript_exponent(&self) -> Option<(i64, usize, bool)> {
		let (mut exponent, mut length, mut signed) = (0i64, 0usize, false);
		loop {
			let sign = match self.chars.get(self.pos + length).copied().and_then(superscript_sign) {
				Some(sign) => sign,
				None if length == 0 => 1,
				None => break,
			};
			let sign_length = self.chars.get(self.pos + length).copied().and_then(superscript_sign).map_or(0, |_| 1);
			let digits: Vec<i64> = self.chars.iter().skip(self.pos + length + sign_length).map_while(|ch| superscript_digit(*ch)).collect();
			if digits.is_empty() {
				break;
			}
			let run = digits.iter().fold(0i64, |run, digit| run.saturating_mul(10).saturating_add(*digit));
			exponent = exponent.saturating_add(sign * run);
			signed |= sign_length > 0;
			length += sign_length + digits.len();
		}
		(length > 0).then_some((exponent, length, signed))
	}

	/// The exponent of superscript terms with a letter among them: `ⁿ` → n, `ⁿ⁺ᵐ` → n+m, `ⁿ⁺¹` → n+1; its length in
	/// characters and whether it starts with ⁻
	pub(super) fn superscript_variable_exponent(&self) -> Option<(Node, usize, bool)> {
		let at = |offset: usize| self.chars.get(self.pos + offset).copied();
		let (mut terms, mut length, mut has_letter) = (vec![], 0usize, false);
		loop {
			let sign = at(length).and_then(superscript_sign);
			let start = length + usize::from(sign.is_some());
			let letters: String = (start..).map_while(|offset| at(offset).and_then(superscript_letter)).collect();
			let digits: Vec<i64> = (start..).map_while(|offset| at(offset).and_then(superscript_digit)).collect();
			let term = if !letters.is_empty() {
				has_letter = true;
				length = start + letters.chars().count();
				Node::Symbol(letters)
			} else if !digits.is_empty() {
				length = start + digits.len();
				Node::int(digits.iter().fold(0i64, |run, digit| run.saturating_mul(10).saturating_add(*digit)))
			} else {
				break;
			};
			terms.push((sign.unwrap_or(1), term));
		}
		if !has_letter {
			return None;
		}
		let negative = terms.first().is_some_and(|(sign, _)| *sign < 0);
		let mut terms = terms.into_iter();
		let (_, first) = terms.next()?;
		let exponent = terms.fold(first, |sum, (sign, term)| Node::Key(Box::new(sum), if sign < 0 { Op::Sub } else { Op::Add }, Box::new(term)));
		Some((exponent, length, negative))
	}

	/// `x⁴` is x^4, `x⁻¹` is 1/x, `2ⁿ` is 2^n; the single digits ² and ³ keep their dedicated square and cube operators
	pub(super) fn try_parse_superscript_power(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if let Some((exponent, length, negative)) = self.superscript_variable_exponent() {
			if Op::Pow.binding_power().0 < min_bp {
				return None;
			}
			self.advance_by(length);
			let power = Node::Key(Box::new(lhs.clone()), Op::Pow, Box::new(exponent));
			return Some(if negative { Node::Key(Box::new(Node::int(1)), Op::Div, Box::new(power)) } else { power });
		}
		let (exponent, length, signed) = self.superscript_exponent()?;
		let (op, right) = match (exponent, length, signed) {
			(2, 1, false) => (Op::Square, Empty),
			(3, 1, false) => (Op::Cube, Empty),
			_ => (Op::Pow, Node::int(exponent.abs())),
		};
		if op.binding_power().0 < min_bp {
			return None;
		}
		self.advance_by(length);
		let power = Node::Key(Box::new(lhs.clone()), op, Box::new(right));
		Some(if exponent < 0 { Node::Key(Box::new(Node::int(1)), Op::Div, Box::new(power)) } else { power })
	}

	pub(super) fn try_parse_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if let Some(power) = self.try_parse_superscript_power(lhs, min_bp) {
			return Some(power);
		}
		let (op, chars) = self.peek_suffix_operator()?;
		let (l_bp, _) = op.binding_power();
		if l_bp < min_bp {
			return None;
		}
		self.advance_by(chars);
		Some(Node::Key(Box::new(lhs.clone()), op, Box::new(Empty)))
	}

	/// `{a*a}!` and `f!` at the end of a statement evaluate the block or name, which a block does on the spot anyway.
	/// After a function or method the `!` mutates in place (user decision D2, by position): `x.upper!` and `upper(x)!`
	/// assign the result back to x; in `upper x!` the name is marked for crate::mutation to do the same.
	pub(super) fn try_parse_evaluate_bang(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		// `xs#2!` runs xs#2, not the index 2: the suffix binds looser than `#`, tighter than arithmetic (`x!+1`)
		if min_bp > BANG_BP {
			return None;
		}
		let mutated = crate::mutation::mutated_variable(lhs).cloned();
		// `o.s1!`: a field (no method) may hold a block too (wiki/charged.md section 4)
		// any expression may be run (wiki/charged.md section 5): `xs#2!`, `f(x)!`; Empty too (`ø!` P73 force → unwrap)
		// `name! email?`: glued to its name and followed by a space, the `!` is a suffix even when an operand follows
		// `x!+1`: glued to its name and followed by an infix operator, the `!` is a suffix too
		// `x!!`: run fully, a suffix as well
		let glued_suffix = !self.prev_char().is_whitespace() && (matches!(self.peek_char(1), ' ' | '\t' | '!') || INFIX_AFTER_BANG.contains(&self.peek_char(1)));
		if self.current_char() != '!' || self.peek_char(1) == '=' || (self.operand_follows(1) && !glued_suffix) {
			return None;
		}
		self.advance();
		// `x!!` runs a block fully, nested blocks too (wiki/charged.md section 5)
		let fully = self.current_char() == '!' && self.peek_char(1) != '=';
		if fully {
			self.advance();
		}
		Some(match (mutated, lhs.drop_meta()) {
			// `o.s1!` runs a block field, `x.upper!` mutates (D2): blocks.rs knows which, mutation.rs mutates the rest
			(Some(_), Node::Key(_, Op::Dot, _)) if fully => crate::mutation::marked_fully(lhs.clone()),
			(Some(_), Node::Key(_, Op::Dot, _)) => crate::mutation::marked(lhs.clone()),
			(Some(variable), _) => Node::Key(Box::new(variable), Op::Assign, Box::new(lhs.clone())),
			(None, Node::Symbol(_)) if fully => crate::mutation::marked_fully(lhs.clone()),
			(None, Node::Symbol(_)) => crate::mutation::marked(lhs.clone()),
			// `{a*a}!` evaluates the block on the spot
			(None, Node::List(_, Bracket::Curly, _)) => lhs.clone(),
			// `ø!`: P73 force → unwrap (direct call; marking Empty is stripped by is_nothing / run_time_blocks)
			(None, Node::Empty) => Node::List(vec![Node::Symbol(crate::mutation::UNWRAP.to_string()), lhs.clone()], Bracket::Round, Separator::None),
			(None, _) if fully => crate::mutation::marked_fully(lhs.clone()),
			(None, _) => crate::mutation::marked(lhs.clone()),
		})
	}

	/// `x empty`, `x missing`, `x is absent` … at the end of a condition are `not x` (wiki/null.md); `x failed` is `is_error(x)`.
	/// The word must end the condition: a block, colon, `then`/`else`/`and`/`or` or the end of the statement follows.
	/// `x is empty` stays the comparison with ø.
	pub(super) fn try_parse_test_word(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if self.options.data_mode || min_bp > Op::Not.binding_power().1 || matches!(lhs.drop_meta(), Empty) {
			return None;
		}
		let is_length = "is".len();
		let after_is = self.matches_keyword("is");
		let word_start = if after_is { is_length + (is_length..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count() } else { 0 };
		let word = TEST_WORDS.iter().find(|word| word.chars().enumerate().all(|(index, letter)| self.peek_char(word_start + index) == letter) && !is_identifier_char(self.peek_char(word_start + word.len())))?;
		if after_is && *word == EMPTY_WORD {
			return None;
		}
		let end = word_start + word.len();
		let blanks = (end..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let next = self.peek_char(end + blanks);
		let word_ends_condition = !self.operand_follows(end) || matches!(next, '{' | ':') || CONDITION_FOLLOWERS.iter().any(|follower| {
			follower.chars().enumerate().all(|(index, letter)| self.peek_char(end + blanks + index) == letter) && !is_identifier_char(self.peek_char(end + blanks + follower.len()))
		});
		if !word_ends_condition {
			return None;
		}
		self.advance_by(end);
		if *word == FAILED_WORD {
			return Some(Node::List(vec![Symbol(IS_ERROR_CALL.to_string()), lhs.clone()], Bracket::Round, Separator::None));
		}
		Some(Node::Key(Box::new(Empty), Op::Not, Box::new(lhs.clone())))
	}

	/// A control word behind a statement: `x++ while c`, `a = 2 if c`, `i++ until c`, `a = 2 unless c`, `3 times {body}`
	pub(super) fn try_parse_control_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if self.options.data_mode || matches!(lhs.drop_meta(), Empty) {
			return None;
		}
		// `x = it times it`: with a value after it (no block) `times` binds like `*` anywhere
		let value_follows = self.value_after_times();
		if (min_bp <= TIMES_BP || self.times_fills_list() || (value_follows && min_bp <= Op::Mul.binding_power().0)) && self.matches_keyword(TIMES_WORD) {
			self.advance_by(TIMES_WORD.len());
			return Some(self.parse_times_loop(lhs.clone()));
		}
		let (word, guard, negated) = STATEMENT_MODIFIERS.iter().copied().find(|(word, _, _)| self.matches_keyword(word))?;
		if min_bp > 0 || (guard == Op::If && self.condition_has_branch(word.len())) {
			return None;
		}
		self.advance_by(word.len());
		self.skip_spaces();
		let condition = self.with_equals_comparing(true, |parser| parser.parse_expr(Op::If.binding_power().1));
		if guard == Op::While {
			// a trailing `while` tests before the first round like the leading one; it is no do-while (user decision #22)
			let (statement, test) = (crate::normalize::operand_text(lhs), crate::normalize::operand_text(&condition));
			crate::normalize::hint(&format!("{statement} {word} {test}"), &format!("{word} {test} {{ {statement} }}"),
				"a trailing loop word tests before the first round, the statement may never run");
		}
		let condition = if negated { Node::Key(Box::new(Empty), Op::Not, Box::new(condition)) } else { condition };
		Some(match guard {
			Op::While => while_do(condition, lhs.clone()),
			_ => Node::Key(Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(condition))), Op::Then, Box::new(lhs.clone())),
		})
	}

	/// `if b {y}` or `if b then y` after a statement (`if a {x} if b {y}`) starts the next statement: a trailing `if`
	/// guards without a branch of its own. Scans the condition from `offset` to the end of the statement for a block
	/// (a spaced `{` outside brackets: `point{x:1}` is data) or `then`
	pub(super) fn condition_has_branch(&self, offset: usize) -> bool {
		let mut depth = 0usize;
		let mut quote = None;
		for at in offset.. {
			let character = self.peek_char(at);
			match (quote, character) {
				(_, '\0') => return false,
				(Some(open), _) => quote = (character != open).then_some(open),
				(None, '"' | '\'') => quote = Some(character),
				(None, '(' | '[') => depth += 1,
				(None, ')' | ']') if depth == 0 => return false,
				(None, ')' | ']') => depth -= 1,
				(None, '{') if depth == 0 && matches!(self.peek_char(at - 1), ' ' | '\t') => return true,
				(None, '\n' | ';' | '}') if depth == 0 => return false,
				(None, 't') if depth == 0 && !is_identifier_char(self.peek_char(at - 1)) && self.word_at(at) == THEN_WORD => return true,
				_ => {}
			}
		}
		false
	}

	/// `a nand b` and `a ¬& b` are `not (a and b)`
	pub(super) fn try_parse_nand(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let spelling = NAND_SPELLINGS.iter().find(|spelling| self.matches_operator_word(spelling))?;
		let (and_bp, right_bp) = Op::And.binding_power();
		if and_bp < min_bp || matches!(lhs.drop_meta(), Empty) {
			return None;
		}
		self.set_hint_pos();
		norm::operator(spelling, false);
		self.advance_by(spelling.chars().count());
		self.skip_spaces();
		let rhs = self.parse_expr(right_bp);
		Some(Node::Key(Box::new(Empty), Op::Not, Box::new(Node::Key(Box::new(lhs.clone()), Op::And, Box::new(rhs)))))
	}

	/// The word or glyph pair `spelling` at the cursor; a word must not run into an identifier
	pub(super) fn matches_operator_word(&self, spelling: &str) -> bool {
		let matches = spelling.chars().enumerate().all(|(index, letter)| self.peek_char(index) == letter);
		matches && (!spelling.chars().all(char::is_alphabetic) || !is_identifier_char(self.peek_char(spelling.chars().count())))
	}

	/// `a ?: b` is `a ? a : b`; a left side that is not a plain name is evaluated once, into a hidden variable
	pub(super) fn try_parse_elvis(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let (question_bp, _) = Op::Question.binding_power();
		if self.current_char() != '?' || self.peek_char(1) != ':' || question_bp < min_bp {
			return None;
		}
		self.advance_by(2);
		self.skip_whitespace();
		let alternative = self.parse_expr(question_bp + 1); // left-assoc: `a ?: b ?: c` is `(a ?: b) ?: c`, the same value
		let elvis = |value: Node| Node::Key(Box::new(value.clone()), Op::Question, Box::new(Node::Key(Box::new(value), Op::Colon, Box::new(alternative))));
		if matches!(lhs.drop_meta(), Node::Symbol(_)) {
			return Some(elvis(lhs.clone()));
		}
		self.elvis_operands += 1;
		let operand = Symbol(format!("{ELVIS_WORD}·{}", self.elvis_operands));
		let store = Node::Key(Box::new(operand.clone()), Op::Assign, Box::new(lhs.clone()));
		Some(Node::List(vec![store, elvis(operand)], Bracket::Round, Separator::Semicolon))
	}

	/// `hex 1010` is `0x1010`; the digits start with a number, `hex ff` stays a call of `hex` on `ff`
	pub(super) fn try_parse_hex_word(&mut self) -> Option<Node> {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		if blanks == 0 || !self.peek_char(blanks).is_ascii_digit() {
			return None;
		}
		let end = (blanks..).find(|&offset| !self.peek_char(offset).is_ascii_hexdigit())?;
		if is_identifier_char(self.peek_char(end)) {
			return None;
		}
		let digits: String = (blanks..end).map(|offset| self.peek_char(offset)).collect();
		self.advance_by(end);
		Some(i64::from_str_radix(&digits, 16).map(Node::int).unwrap_or_else(|_| error(&format!("Invalid hex: hex {digits}"))))
	}

	/// `times [x]` fills a list, binding to the count right before it: `xs = 100 times [0]`
	pub(super) fn times_fills_list(&self) -> bool {
		let after_word = TIMES_WORD.len();
		let blanks = (after_word..).take_while(|&offset| self.peek_char(offset) == ' ').count();
		self.peek_char(after_word + blanks) == '['
	}

	/// `N times {body}` and `N times: body` count with a hidden variable of its own, so nested loops do not meet
	/// `times` at the cursor followed by a value (a name, a number or a text), no block
	fn value_after_times(&self) -> bool {
		if !self.matches_keyword(TIMES_WORD) {
			return false;
		}
		let after = (TIMES_WORD.len()..).find(|at| !matches!(self.peek_char(*at), ' ' | '\t')).unwrap_or(TIMES_WORD.len());
		let next = self.peek_char(after);
		next.is_ascii_digit() || matches!(next, '"' | '\'') || self.is_identifier_start(after)
	}

	pub(super) fn parse_times_loop(&mut self, count: Node) -> Node {
		self.skip_spaces();
		let body = match self.current_char() {
			'{' => self.parse_atom(),
			':' => {
				self.advance();
				self.with_equals_comparing(false, |parser| parser.parse_expr(0))
			}
			'[' => {
				let list = self.parse_atom();
				return crate::analyzer::filled_list(count, &list).unwrap_or_else(|| error("`n times [x]` repeats one element: `3 times [0]`"));
			}
			// `3 times "ab"`, `3 times greeting`: the text repeated (a non-text is an error where its kind is known)
			// `3 times 4`: numbers multiply (list_emitter.rs emit_text_times)
			quote_or_letter if matches!(quote_or_letter, '"' | '\'') || self.is_identifier_start(0) || quote_or_letter.is_ascii_digit() => return Node::List(vec![Symbol(TEXT_TIMES.to_string()), count, self.parse_atom()], Bracket::Round, Separator::None),
			_ => return error("`times` needs a body: `3 times {…}`"),
		};
		self.times_loops += 1;
		let counter = Symbol(format!("{TIMES_WORD}·{}", self.times_loops));
		// the count is evaluated once, before the first round (user decision #22): the body may change what it reads
		let (count_binding, count) = match count.drop_meta() {
			Node::Number(_) => (None, count),
			_ => {
				let held = Symbol(format!("{TIMES_WORD}·count·{}", self.times_loops));
				(Some(Node::Key(Box::new(held.clone()), Op::Assign, Box::new(count))), held)
			}
		};
		let zero_to_count = Node::Key(Box::new(Node::Number(Number::Int(0))), Op::Range, Box::new(count));
		let rounds = Node::List(vec![Symbol("for".to_string()), counter, Symbol("in".to_string()), zero_to_count, body], Bracket::None, Separator::Space);
		match count_binding {
			Some(binding) => Node::List(vec![binding, rounds], Bracket::Round, Separator::Semicolon),
			None => rounds,
		}
	}

	pub(super) fn try_parse_subscript(&mut self, lhs: &Node, min_bp: u8, subscript_bp: u8) -> Option<Node> {
		// a space before the bracket makes it a list, `f [1, 2]`; only `xs[1]` indexes
		if self.current_char() != '[' || min_bp > subscript_bp || self.prev_char().is_whitespace() || is_unindexable_keyword(lhs) {
			return None;
		}
		if let Some(list_type) = self.try_parse_array_type_suffix(lhs) {
			return Some(list_type);
		}
		let before_bracket = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.advance(); // skip '['
		self.skip_whitespace();

		let mut indices = vec![self.parse_slice_from_start().unwrap_or_else(|| self.parse_expr(0))];
		self.skip_whitespace();

		while self.current_char() == ',' {
			self.advance(); // skip ','
			self.skip_whitespace();
			indices.push(self.parse_expr(0));
			self.skip_whitespace();
		}

		if self.current_char() != ']' {
			(self.pos, self.line_nr, self.column, self.current_line) = before_bracket; // `foo [1 2 3]`: a list argument, not an index
			return None;
		}
		self.advance(); // skip ']'
		if let Some(node) = self.try_attributed_body(lhs, &indices) {
			return Some(node);
		}
		// `int[n]` is n zeros of the type unless int is a variable (analyzer lower_declarations): no indexing hint
		let names_a_type = matches!(lhs.drop_meta(), Node::Symbol(word) if crate::analyzer::zero_list(Empty, word).is_some());
		if slice_bounds(&indices[0]).is_none() && !names_a_type && !self.names_a_key(&indices[0]) {
			crate::normalize::set_position_of(lhs);
			norm::index_operator(&crate::normalize::operand_text(lhs), &crate::normalize::operand_text(&indices[0]), true);
		}

		Some(indices.into_iter().fold(lhs.clone(), subscript))
	}

	/// `a[id=1]{…}` (wiki/reference.md): attributes in brackets before a body are its meta entries, `a{@id:1 …}`
	fn try_attributed_body(&mut self, lhs: &Node, indices: &[Node]) -> Option<Node> {
		let attributes: Vec<Node> = indices.iter().flat_map(attribute_items).map(|item| match item.drop_meta() {
			Node::Key(key, Op::Assign | Op::Colon, value) if matches!(key.drop_meta(), Symbol(_)) => {
				Some(Node::Key(Box::new(Symbol(format!("{ATTRIBUTE_MARK}{}", key.name()))), Op::Colon, value.clone()))
			}
			_ => None,
		}).collect::<Option<_>>()?;
		let mut offset = 0;
		while self.peek_char(offset) == ' ' {
			offset += 1;
		}
		if !matches!(lhs.drop_meta(), Symbol(_)) || self.peek_char(offset) != '{' {
			return None;
		}
		self.skip_spaces();
		let Node::List(items, bracket, separator) = self.parse_bracketed('{') else { return None };
		let body = Node::List([attributes, items].concat(), bracket, separator);
		Some(Node::Key(Box::new(lhs.clone()), Op::Colon, Box::new(body)))
	}

	/// `[:end]` and `[:]`, a slice from the start: `ø:end`
	pub(super) fn parse_slice_from_start(&mut self) -> Option<Node> {
		if self.current_char() != ':' {
			return None;
		}
		self.advance(); // skip ':'
		self.skip_whitespace();
		let end = if self.current_char() == ']' { Empty } else { self.parse_expr(0) };
		Some(Node::Key(Box::new(Empty), Op::Colon, Box::new(end)))
	}

	/// The Java/C array type `int[]` is the list type `[int]`
	pub(super) fn try_parse_array_type_suffix(&mut self, lhs: &Node) -> Option<Node> {
		let Node::Symbol(word) = lhs.drop_meta() else { return None };
		if !is_identifier_char(self.prev_char()) || self.peek_char(1) != ']' || !names_a_type(word) {
			return None;
		}
		self.advance_by(2);
		Some(Node::List(vec![lhs.clone()], Bracket::Square, Separator::Space))
	}

	pub(super) fn try_parse_implicit_application(
		&mut self,
		lhs: &Node,
		min_bp: u8,
		argument_bp: u8,
		max_bp_for_application: u8,
	) -> Option<Node> {
		let lhs_is_callable = match lhs.drop_meta() {
			Node::Symbol(_) => true,
			Node::List(_, Bracket::None, _) => true, // Function call from implicit application
			_ => false,
		};
		let lhs_is_defined_function = matches!(lhs.drop_meta(), Node::Symbol(name) if self.functions.contains(name));
		let lhs_is_operand_word = matches!(lhs.drop_meta(), Node::Symbol(name) if crate::library_words::is_operand_word(name));
		let ch = self.current_char();
		let in_assignment_context = min_bp <= 60 || self.branch_bp == Some(min_bp); // Assignment r_bp is 59
		let arg_is_non_identifier = ch.is_numeric()
			|| self.number_starts_at(0)
			|| ch == '"'
			|| ch == '\''
			|| ch == '('
			|| ch == '['
			|| ch == '{'
			|| ch == '-'
			|| self.starts_function_reference();
		let should_apply = in_assignment_context || arg_is_non_identifier || lhs_is_defined_function || lhs_is_operand_word;

		// At statement level a list `f a b` is a call with all its items; a function of the implicit `it` takes one argument,
		// so `f 3-1 > 15` compares `f(3-1)` just like the operand `1 + f 3-1 > 15` does
		let takes_one_argument = lhs_is_defined_function && !self.functions_with_parameters.contains(&lhs.name());
		if (min_bp == 0 && !takes_one_argument) || min_bp > max_bp_for_application || self.glued_pair_bp == Some(min_bp) {
			return None;
		}
		if !lhs_is_callable || !self.can_start_atom() || !should_apply || (ch == '{' && self.in_for_header) {
			return None;
		}

		let arg = self.parse_expr(argument_bp);
		if arg == Empty {
			return None;
		}

		Some(Node::List(
			vec![lhs.clone(), arg],
			Bracket::None,
			Separator::Space,
		))
	}
}

/// `id=1 kind="x"` in brackets: each attribute
fn attribute_items(index: &Node) -> Vec<Node> {
	match index.drop_meta() {
		Node::List(items, Bracket::None, _) => items.clone(),
		single => vec![single.clone()],
	}
}
