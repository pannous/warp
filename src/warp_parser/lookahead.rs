//! Looking ahead: what can start an atom, operators and control words ahead, guards, attributes

use super::*;

/// `∜x`, read as `√√x`
pub(super) const FOURTH_ROOT: char = '∜';
/// The prefix operators written as words: `sqrt x`, `cbrt x`, `abs x`, `norm x` (card g-1pvQ: norm is a synonym of abs),
/// `not x`
pub const PREFIX_OPERATOR_WORDS: [(&str, Op); 5] = [
	("sqrt", Op::Sqrt),
	("cbrt", Op::Cbrt),
	("abs", Op::Abs),
	("norm", Op::Abs),
	("not", Op::Not),
];
/// operators::OPERATOR_SPELLINGS, the longest first: `<=` before `<`, `is less than` before `is`
static LONGEST_SPELLINGS_FIRST: std::sync::LazyLock<Vec<(&str, Op)>> = std::sync::LazyLock::new(|| {
	let mut spellings: Vec<_> = crate::operators::all_spellings().collect();
	spellings.sort_by_key(|(spelling, _)| std::cmp::Reverse(spelling.chars().count()));
	spellings
});
/// Infix operators that may follow a suffix: `10% + 1`, `x abs * 2`
const SUFFIX_FOLLOWERS: [char; 6] = ['+', '-', '*', '/', '<', '>'];
/// Operator words that stay operators before a colon: `if c then: a else: b`, `defp f(x), do: x`
const BLOCK_COLON_WORDS: [&str; 3] = ["then", "else", "do"];
/// The digits of a CSS hex color: `#fff`, `#8884`, `#1e1e1e`, `#00ff0080`
const CSS_COLOR_DIGITS: [usize; 4] = [3, 4, 6, 8];
/// What follows an id selector of hex letters (`#fed { … }`, `#add, #b`, `#bad:hover`), never a color
const SELECTOR_FOLLOWERS: [char; 8] = ['{', ',', '.', ':', '>', '+', '~', '['];
/// What a style sheet's selector starts after: the sheet's or a rule's brace, a declaration's end, a selector list's comma
const SELECTOR_STARTS_AFTER: [char; 5] = ['{', '}', ';', ',', '\0'];

/// What ends a path literal: whitespace, a closing bracket or a separator
pub(super) fn ends_path(c: char) -> bool {
	c.is_whitespace() || matches!(c, '\0' | ']' | ')' | '}' | ',' | ';')
}

impl WarpParser {
	/// Check if current character can start an atom (for implicit application)
	pub(super) fn can_start_atom(&self) -> bool {
		let ch = self.current_char();
		ch.is_alphanumeric()
			|| ch == '_'
			|| ch == '"'
			|| ch == '\''
			|| ch == '('
			|| ch == '['
			|| ch == '{'
			|| self.number_starts_at(0)
			|| self.starts_function_reference()
			|| self.starts_path_literal()
			|| self.url_follows()
	}

	/// `function add` (P82): the function itself where a name and then the end of the expression follow the keyword;
	/// `function add(x) {…}` and `function add x := …` define it
	pub(super) fn function_reference_after(&mut self, keyword: &str) -> Option<Node> {
		if !FUNCTION_REFERENCE_WORDS.contains(&keyword) || self.options.data_mode {
			return None;
		}
		let blanks = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		if blanks == 0 || !self.peek_char(blanks).is_alphabetic() {
			return None;
		}
		let name: String = (blanks..)
			.map(|at| self.peek_char(at))
			.take_while(|ch| is_identifier_char(*ch))
			.collect();
		let after = blanks + name.chars().count();
		let rest = (after..)
			.find(|at| !matches!(self.peek_char(*at), ' ' | '\t'))
			.unwrap_or(after);
		// `map function square on [1 2 3]`: the iteration words' `on` ends it too
		let on_follows = (rest..)
			.map(|at| self.peek_char(at))
			.take_while(|ch| is_identifier_char(*ch))
			.collect::<String>()
			== crate::words::ON_WORD;
		if !matches!(self.peek_char(rest), '\0' | ';' | ',' | ')' | ']' | '}' | '\n') && !on_follows {
			return None;
		}
		self.advance_by(after);
		self.after_function_keyword = false;
		Some(crate::closures::function_reference(name))
	}

	/// `&name`: a reference to the function `name`, an `&` glued to the word after it and not to a word before it (`a &b`, `f(&g)`)
	/// `./mozart.mp3`, `../songs/a.wav`: `./` or `../` glued to what follows and not to an operand before it starts a
	/// file path without quotes; `xs ./ 2` and `xs./2` divide each element
	pub(super) fn starts_path_literal(&self) -> bool {
		let dots = if self.peek_char(1) == '.' { 2 } else { 1 };
		let glued_to_operand =
			is_identifier_char(self.prev_char()) || matches!(self.prev_char(), ')' | ']' | '}' | '"' | '\'');
		self.current_char() == '.'
			&& self.peek_char(dots) == '/'
			&& !ends_path(self.peek_char(dots + 1))
			&& !glued_to_operand
	}

	pub(super) fn starts_function_reference(&self) -> bool {
		self.current_char() == '&' && self.peek_char(1).is_alphabetic() && !is_identifier_char(self.prev_char())
	}

	/// Elixir's capture `&(&1 * 2)`: an `&(` not glued to a word before it, whose group reads an `&1`
	pub(super) fn starts_capture(&self) -> bool {
		if self.current_char() != '&' || self.peek_char(1) != '(' || is_identifier_char(self.prev_char()) {
			return false;
		}
		let mut depth = 0;
		for at in 1.. {
			match self.peek_char(at) {
				'(' => depth += 1,
				')' if depth == 1 => return false,
				')' => depth -= 1,
				'&' if self.peek_char(at + 1).is_ascii_digit() => return true,
				'\0' => return false,
				_ => {}
			}
		}
		false
	}

	/// A digit, or a leading-dot decimal like `.5`
	pub(super) fn number_starts_at(&self, offset: usize) -> bool {
		let ch = self.peek_char(offset);
		ch.is_ascii_digit() || (ch == '.' && self.peek_char(offset + 1).is_ascii_digit())
	}

	/// An operand may start after `offset` characters and following blanks: not a closing bracket, separator or the end
	pub(super) fn operand_follows(&self, offset: usize) -> bool {
		let mut position = offset;
		while matches!(self.peek_char(position), ' ' | '\t') {
			position += 1;
		}
		!matches!(
			self.peek_char(position),
			'\0' | '\n' | '\r' | '>' | ')' | ']' | '}' | ',' | ';' | '='
		)
	}

	/// Check if character terminates a URL
	pub(super) fn is_url_terminator(&self, ch: char) -> bool {
		ch == '\0'
			|| ch == ' '
			|| ch == '\t'
			|| ch == '\n'
			|| ch == '\r'
			|| ch == ';'
			|| ch == ')'
			|| ch == ']'
			|| ch == '}'
			|| ch == '>'
			|| ch == '"'
			|| ch == '\''
			|| ch == ','
			|| ch == '«'
	}

	/// `{from:1 to:2}`: an operator word directly before a key colon names the key; `else:` and `then:` open blocks
	fn word_names_key(&self) -> bool {
		let word: String = (0..)
			.map(|offset| self.peek_char(offset))
			.take_while(|&c| is_identifier_char(c))
			.collect();
		let after = word.chars().count();
		!word.is_empty()
			&& !BLOCK_COLON_WORDS.contains(&word.as_str())
			&& self.peek_char(after) == ':'
			&& !matches!(self.peek_char(after + 1), ':' | '=')
	}

	/// Peek ahead for an infix operator, returns (Op, chars_to_consume) if found: the longest spelling of
	/// operators::OPERATOR_SPELLINGS that fits here and its context allows
	pub(super) fn peek_operator(&self) -> Option<(Op, usize)> {
		if (self.current_char(), self.peek_char(1)) == ('?', ':') {
			return None; // the elvis `?:` is no ternary, `try_parse_elvis` takes it
		}
		// `fetch ://host/path`: the colon starts a URL (card pannous-com), no key
		if self.word_names_key() || self.starts_path_literal() || self.url_follows() {
			return None;
		}
		if let Some(length) = self.down_to_length() {
			return Some((Op::To, length));
		}
		LONGEST_SPELLINGS_FIRST
			.iter()
			.filter(|(spelling, _)| self.spelled_here(spelling))
			.find_map(|(spelling, op)| self.operator_in_context(spelling, *op))
			.map(|(op, spelling)| (op, spelling.chars().count()))
	}

	/// The spelling starts here; a word ends here too: `and` but not `android`, `plus a` but not the call `plus(a, b)`
	fn spelled_here(&self, spelling: &str) -> bool {
		let length = spelling.chars().count();
		let after = self.peek_char(length);
		spelling.chars().enumerate().all(|(i, c)| self.peek_char(i) == c)
			&& !(spelling.ends_with(char::is_alphabetic) && (is_identifier_char(after) || after == '('))
	}

	/// The operator a fitting spelling is here, None where the context reads it otherwise
	fn operator_in_context<'a>(&self, spelling: &'a str, op: Op) -> Option<(Op, &'a str)> {
		let (after, prev) = (self.peek_char(spelling.chars().count()), self.prev_char());
		let allowed = match spelling {
			"else" => !self.else_arm_key(),
			"if" => !self.condition_has_branch(2),
			"until" => self.in_for_header,
			"?." => after.is_alphabetic() || after == '_', // `x?.name`; `x ?.5 : 1` is a ternary
			// R's `x <- 3` assigns with a note, the cramped `x<-3` is an error (P145, left_arrow_assignment)
			"<-" => after.is_whitespace() || !prev.is_whitespace(),
			"<<" | "<" | ">" => !self.options.wit_mode, // angle brackets only delimit type arguments
			// `list<list<int>>` closes two generics, a shift has an operand behind it
			">>" => !self.options.wit_mode && self.operand_follows(2),
			"+-" => after.is_whitespace() && prev.is_whitespace(),
			"/" => after != '/', // Don't treat // as division - it's a comment
			// `a .5` is a list of two values, `a.5` a member access
			"." => !(prev.is_whitespace() && self.number_starts_at(0)),
			"&" => !(self.starts_function_reference() && prev.is_whitespace()), // `map &square xs`
			_ => true,
		};
		// `solid #8884`: the color is the next word of the value; `#main .x`: the descendant selector's next part
		let selector_part =
			self.in_style_sheet && matches!(spelling, "." | "#") && prev.is_whitespace() && self.is_identifier_start(1);
		let style_word =
			spelling.chars().count() == 1 && (self.css_color_length().is_some() || self.at_pseudo_class_selector());
		let op = if spelling == "=" && self.equals_compares {
			Op::Eq
		} else {
			op
		};
		(allowed && !selector_part && !style_word).then_some((op, spelling))
	}

	/// `unless`/`until` in front of a statement: the `if`/`while` it reads as, with the condition negated afterwards
	pub(super) fn peek_negated_control_word(&self) -> Option<(Op, usize)> {
		if self.options.data_mode {
			return None;
		}
		STATEMENT_MODIFIERS
			.iter()
			.find(|(word, _, negated)| *negated && self.matches_keyword(word))
			.map(|(word, op, _)| (*op, word.len()))
	}

	/// `10 down to 1`: the length of the words `down to` (Kotlin's `downTo`), the range counting down
	fn down_to_length(&self) -> Option<usize> {
		if !self.matches_keyword(DOWN_WORD) {
			return None;
		}
		let blanks = (DOWN_WORD.len()..)
			.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
			.count();
		(blanks > 0 && self.word_at(DOWN_WORD.len() + blanks) == TO_WORD)
			.then(|| DOWN_WORD.len() + blanks + TO_WORD.len())
	}

	/// `try` or `assert` followed by an operand: the words that guard a statement
	pub(super) fn peek_guard_word(&self) -> Option<&'static str> {
		self.guard_word_ahead().map(|(_, marker)| marker)
	}

	/// The guard word here and its marker
	fn guard_word_ahead(&self) -> Option<(&'static str, &'static str)> {
		if self.options.data_mode {
			return None;
		}
		GUARD_MARKERS.iter().copied().find(|(word, _)| {
			self.matches_keyword(word) && matches!(self.peek_char(word.len()), ' ' | '\t' | '{' | ':')
		})
	}

	/// `try X else Y` and `assert C else X`, as the marker call `marker(X, Y)` that `library_words` lowers.
	/// X runs to the `else` (it may be an assignment); `assert C` alone has ø for the message.
	pub(super) fn parse_guard(&mut self, marker: &'static str) -> Node {
		let word_length = self.guard_word_ahead().map_or(0, |(word, _)| word.len());
		self.advance_by(word_length);
		self.skip_spaces();
		self.skip_python_colon();
		let outer = std::mem::replace(&mut self.stops_at_else, true);
		let start = self.pos;
		let guarded = self.with_equals_comparing(marker == ASSERT_MARKER, |parser| parser.parse_guarded_phrase());
		let written: String = self.chars[start..self.pos].iter().collect();
		self.stops_at_else = outer;
		self.skip_spaces();
		let mut caught = None;
		let fallback = if let Some((word, ahead)) = self.fallback_word_ahead() {
			self.advance_by(ahead + word.len());
			self.skip_spaces();
			caught = if word == ELSE_KEYWORD {
				None
			} else {
				self.parse_caught_name(word)
			};
			self.skip_python_colon();
			// `try X else print "msg"`: a braceless call as on the guarded side (card try-print)
			self.parse_guarded_phrase()
		} else if marker == TRY_MARKER {
			// `r = try X` (user, card failed-raised) and `try X finally Z` are `try X catch e { e }`: a failure of X,
			// trap included, stays the value, which `r failed` tests
			caught = Some(UNCAUGHT_ERROR.to_string());
			Symbol(UNCAUGHT_ERROR.to_string())
		} else if marker == ASSERT_MARKER {
			// the error names the condition as written, before lowering rewrote it
			Node::Text(format!("{ASSERTION_FAILED}: {}", written.trim()))
		} else {
			Empty
		};
		// `catch e { … }` (P67): the name the fallback reads the caught Error by, a fourth item
		let binding = caught.filter(|name| mentions(&fallback, name)).map(Symbol);
		let guard = Node::List(
			[
				vec![Symbol(marker.to_string()), guarded, fallback],
				binding.into_iter().collect(),
			]
			.concat(),
			Bracket::Round,
			Separator::None,
		);
		self.with_finally(guard)
	}

	/// `… finally {Z}` after a guard: `(finally·N = guard; Z; finally·N)`, Z runs and the guard's value stays
	fn with_finally(&mut self, guard: Node) -> Node {
		let Some(ahead) = self.finally_ahead() else {
			return guard;
		};
		self.advance_by(ahead + FINALLY_KEYWORD.len());
		self.skip_spaces();
		let cleanup = if self.current_char() == ':' {
			self.colon_body(":")
		} else {
			self.rest_of_statement()
		};
		self.finally_blocks += 1;
		let value = Symbol(format!("{FINALLY_KEYWORD}·{}", self.finally_blocks));
		let held = Node::Key(Box::new(value.clone()), Op::Assign, Box::new(guard));
		Node::List(vec![held, cleanup, value], Bracket::Round, Separator::Semicolon)
	}

	/// The blanks before a following `finally`, if one follows
	fn finally_ahead(&self) -> Option<usize> {
		let ahead = (0..)
			.take_while(|&offset| self.peek_char(offset).is_whitespace())
			.count();
		(self.word_at(ahead) == FINALLY_KEYWORD).then_some(ahead)
	}

	/// `after C return V`: the marker call `after·return(C, V)`, when a `return` follows on the statement (outside brackets);
	/// `after tested: body` without one stays a call listener (variable_signals)
	pub(super) fn try_parse_after_return(&mut self) -> Option<Node> {
		if self.options.data_mode || !self.matches_keyword(AFTER_KEYWORD) || !self.return_ahead() {
			return None;
		}
		self.advance_by(AFTER_KEYWORD.len());
		let condition = self.parse_expr(0);
		self.skip_spaces();
		if !self.matches_keyword(RETURN_KEYWORD) {
			return Some(error("`after` needs a `return`: `after C return V`"));
		}
		self.advance_by(RETURN_KEYWORD.len());
		let value = self.parse_expr(0);
		Some(call(AFTER_MARKER, vec![condition, value]))
	}

	/// At an opening bracket: a comma separates the group's own items (`[a -1, 2]`), not one of a nested group or text
	pub(super) fn group_holds_comma(&self) -> bool {
		let (mut depth, mut quote) = (0i32, None);
		for offset in 1.. {
			let ch = self.peek_char(offset);
			match (quote, ch) {
				(_, '\0') => return false,
				(Some(open), _) if ch == open => quote = None,
				(Some(_), _) => {}
				(None, '"' | '\'') => quote = Some(ch),
				(None, '(' | '[' | '{') => depth += 1,
				(None, ')' | ']' | '}') if depth == 0 => return false,
				(None, ')' | ']' | '}') => depth -= 1,
				(None, ',') if depth == 0 => return true,
				_ => {}
			}
		}
		false
	}

	/// Is there a `return` word later on this statement, outside brackets
	fn return_ahead(&self) -> bool {
		let mut depth = 0i32;
		for offset in AFTER_KEYWORD.len().. {
			match self.peek_char(offset) {
				'\0' | '\n' | ';' => return false,
				'(' | '[' | '{' => depth += 1,
				')' | ']' | '}' if depth == 0 => return false,
				')' | ']' | '}' => depth -= 1,
				c if depth == 0 && c.is_whitespace() && self.word_at(offset + 1) == RETURN_KEYWORD => return true,
				_ => {}
			}
		}
		false
	}

	/// `and print "x"`: an `and` followed by a statement, a word with an argument after it
	pub(super) fn and_starts_statement(&self) -> bool {
		if self.options.data_mode || !self.matches_keyword(AND_KEYWORD) {
			return false;
		}
		let blanks = |from: usize| {
			(from..)
				.take_while(|&at| matches!(self.peek_char(at), ' ' | '\t'))
				.count()
		};
		let word_start = AND_KEYWORD.len() + blanks(AND_KEYWORD.len());
		let word = self.word_at(word_start);
		if word.is_empty() || joins_operands(&word) || !self.peek_char(word_start).is_alphabetic() {
			return false;
		}
		let word_end = word_start + word.chars().count();
		let argument = word_end + blanks(word_end);
		if argument == word_end {
			return false; // `and f(3)`, `and x`: an operand
		}
		let next = self.peek_char(argument);
		let argument_word = self.word_at(argument);
		(next.is_alphanumeric() || matches!(next, '"' | '\'' | '[' | '{')) && !joins_operands(&argument_word)
	}

	/// The word that starts the fallback of `try`: `else`, or its classical synonyms `catch` and Python's `except` (P60),
	/// with how many blanks (line breaks included) come before it
	pub(super) fn fallback_word_ahead(&self) -> Option<(&'static str, usize)> {
		let ahead = (0..)
			.take_while(|&offset| self.peek_char(offset).is_whitespace())
			.count();
		FALLBACK_WORDS
			.iter()
			.find(|word| self.word_at(ahead) == **word)
			.map(|word| (*word, ahead))
	}

	/// `catch e`, `except ZeroDivisionError`, `except ValueError as e`: the name the error would be bound to
	pub(super) fn parse_caught_name(&mut self, word: &str) -> Option<String> {
		if !self.is_identifier_start(0) {
			return None;
		}
		let mut name = self.word_at(0);
		self.advance_by(name.chars().count());
		self.skip_spaces();
		if self.matches_keyword("as") {
			self.advance_by(2);
			self.skip_spaces();
			name = self.word_at(0);
			self.advance_by(name.chars().count());
			self.skip_spaces();
			return Some(name);
		}
		(word == "catch").then_some(name) // `except ZeroDivisionError` names a type, `catch e` a binding
	}

	/// Python's `try:` and `except:` end with a colon; the block may start on the next line
	pub(super) fn skip_python_colon(&mut self) {
		if self.current_char() == ':' {
			self.advance();
			self.skip_whitespace();
		}
	}

	/// Either part of `try X else Y`: one expression, or a braceless call of several (`try raise "boom" else print 3`)
	pub(super) fn parse_guarded_phrase(&mut self) -> Node {
		let mut items = vec![self.parse_expr(0)];
		loop {
			self.skip_spaces();
			if FALLBACK_WORDS
				.iter()
				.chain([&FINALLY_KEYWORD])
				.any(|word| self.matches_keyword(word))
				|| matches!(self.current_char(), '\0' | '\n' | '\r' | ';' | ')' | ']' | '}' | ',')
				// `try X else 7 // note`: the comment ends the phrase, the next line is not part of it
				|| self.at_line_comment()
			{
				break;
			}
			let position = self.pos;
			let item = self.parse_expr(0);
			if self.pos == position {
				break;
			}
			items.push(item);
		}
		Node::single_or_list(items, Bracket::None, Separator::Space)
	}

	/// `norm() := …`, `def abs(): …`, `fun sqrt() { … }`: the word `length` characters ahead names what is defined, not
	/// the operator. A function of that name is refused (analyzer check_operator_word_functions, P141); a method is
	/// called as `p.norm()` (card class-method-named)
	fn defines_named(&self, length: usize) -> bool {
		if self.peek_char(length) != '(' {
			return false;
		}
		let mut depth = 0;
		let mut offset = length;
		loop {
			match self.peek_char(offset) {
				'(' => depth += 1,
				')' if depth == 1 => break,
				')' => depth -= 1,
				'\0' | '\n' => return false,
				_ => {}
			}
			offset += 1;
		}
		let mut next = offset + 1;
		while matches!(self.peek_char(next), ' ' | '\t') {
			next += 1;
		}
		match (self.peek_char(next), self.peek_char(next + 1)) {
			(':', '=') => true,
			(':' | '{', _) => self.after_function_keyword_word(),
			_ => false,
		}
	}

	/// `def ` right before here: what follows heads a definition
	pub(super) fn after_function_keyword_word(&self) -> bool {
		let before: String = self.chars[..self.pos]
			.iter()
			.rev()
			.skip_while(|c| c.is_whitespace())
			.take_while(|c| c.is_alphanumeric())
			.collect();
		crate::operators::is_function_keyword(&before.chars().rev().collect::<String>())
	}

	/// `Int` or `float` after the colon of a definition's head: its result type, not its body
	pub(super) fn result_type_follows(&self) -> bool {
		let word: String = self.chars[self.pos..]
			.iter()
			.skip_while(|c| **c == ' ' || **c == '\t')
			.take_while(|c| c.is_alphanumeric() || **c == '_')
			.collect();
		word.starts_with(char::is_uppercase) || crate::analyzer::type_word_kind(&word).is_some()
	}

	/// In a style sheet `#8884` or `#fff` in a value is a CSS hex color, not the count or index operator (card
	/// css-block): its length with the `#`. Ids starting with a letter stay selectors where one is followed by a block
	/// or another selector part (`#fed { … }`); an id never starts with a digit
	pub(super) fn css_color_length(&self) -> Option<usize> {
		if !self.in_style_sheet || self.current_char() != '#' {
			return None;
		}
		let digits = (1..)
			.take_while(|&offset| self.peek_char(offset).is_ascii_hexdigit())
			.count();
		let after = self.peek_char(digits + 1);
		if !CSS_COLOR_DIGITS.contains(&digits) || after.is_alphanumeric() || matches!(after, '_' | '-') {
			return None;
		}
		let next = (digits + 1..)
			.map(|offset| self.peek_char(offset))
			.find(|ch| !ch.is_whitespace())
			.unwrap_or('\0');
		(self.peek_char(1).is_ascii_digit() || !SELECTOR_FOLLOWERS.contains(&next)).then_some(digits + 1)
	}

	/// `:root { … }`, `:hover { … }` starting a rule of a style sheet: a selector, not the colon of a declaration (card
	/// markup-style-block)
	pub(super) fn at_pseudo_class_selector(&self) -> bool {
		let previous = self.chars[..self.pos]
			.iter()
			.rev()
			.find(|ch| !ch.is_whitespace())
			.copied()
			.unwrap_or('\0');
		self.in_style_sheet
			&& self.current_char() == ':'
			&& self.is_identifier_start(1)
			&& SELECTOR_STARTS_AFTER.contains(&previous)
	}

	/// Peek for prefix operators (unary operators that bind to right operand)
	pub(super) fn peek_prefix_operator(&self) -> Option<(Op, usize)> {
		if self.matches_keyword("while") {
			return Some((Op::While, 5));
		}
		if let Some((word, op)) = PREFIX_OPERATOR_WORDS
			.into_iter()
			.find(|(word, _)| self.matches_keyword(word))
		{
			return (!self.defines_named(word.len())).then_some((op, word.len()));
		}
		if self.matches_keyword("if") {
			return Some((Op::If, 2));
		}

		let (c1, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		let variable_follows = c3.is_alphabetic() || c3 == '_';
		match c1 {
			'-' if c2 == '>' => None, // Ruby's stabby lambda `->(x) { … }` is an atom
			'+' if c2 == '+' && variable_follows => Some((Op::Inc, 2)),
			'-' if c2 == '-' && variable_follows => Some((Op::Dec, 2)),
			// unary plus glued to its operand, `+5`, `+x`, `+(a)`: the operand itself; a spaced `+` stays the operator
			// word (`fold + xs`)
			'+' if c2.is_alphanumeric() || matches!(c2, '_' | '(' | '.') => Some((Op::Add, 1)),
			'-' => Some((Op::Neg, 1)),
			dash if glyph_operator(dash) == Some(Op::Sub) => Some((Op::Neg, 1)),
			'!' | '¬' => Some((Op::Not, 1)),
			'√' | FOURTH_ROOT => Some((Op::Sqrt, 1)), // ∜x is √√x (expressions.rs)
			'∛' => Some((Op::Cbrt, 1)),
			'‖' => Some((Op::Abs, 1)),
			'#' => self.css_color_length().is_none().then_some((Op::Hash, 1)), // prefix # means count/length
			// `> 100 => "big"` (C#'s relational pattern): a comparison without its left side, a match arm compares the
			// subject; a glued `<tag` stays a bracket
			'>' | '<' if !self.options.xml_mode && !self.options.data_mode => {
				let (op, length) = match (c1, c2) {
					('>', '=') => (Op::Ge, 2),
					('<', '=') => (Op::Le, 2),
					('>', _) => (Op::Gt, 1),
					_ => (Op::Lt, 1),
				};
				matches!(self.peek_char(length), ' ' | '\t').then_some((op, length))
			}
			_ => None,
		}
	}

	/// Peek for suffix operators (unary operators that bind to left operand)
	pub(super) fn peek_suffix_operator(&self) -> Option<(Op, usize)> {
		match (self.current_char(), self.peek_char(1)) {
			('+', '+') => Some((Op::Inc, 2)),
			('-', '-') => Some((Op::Dec, 2)),
			// `width: 50% height: 10px`: in a style sheet a % glued to a number is a percentage (card web-styles-limits)
			('%', _) if self.in_style_sheet && self.prev_char().is_ascii_digit() => Some((Op::Mod, 1)),
			// `10%`, `10% + x`: a percent, no remainder, when no operand follows
			('%', next) if next != '=' && self.expression_ends_after(1) => Some((Op::Mod, 1)),
			// `ℯ⌟`: the natural log when no base follows
			('⌟', _) if self.expression_ends_after(1) => Some((Op::LogOf, 1)),
			_ => None,
		}
	}

	/// After the suffix at offset 0 (`%`, `abs`) of `offset` characters: the end of the expression, or an infix operator
	/// standing apart (`10% + 1`, never the `-3` of `7 % -3`)
	pub(super) fn expression_ends_after(&self, offset: usize) -> bool {
		let mut position = offset;
		while matches!(self.peek_char(position), ' ' | '\t') {
			position += 1;
		}
		let infix_apart = position > offset
			&& SUFFIX_FOLLOWERS.contains(&self.peek_char(position))
			&& matches!(self.peek_char(position + 1), ' ' | '\t');
		!self.operand_follows(offset) || infix_apart
	}

	/// `$main`, `$ii_i`: names keep their sigil (WAT identifiers, DOM selectors)
	pub(super) fn parse_dollar_name(&mut self) -> Node {
		self.symbol_after(1, |name| Symbol(format!("${name}")))
	}

	pub(super) fn unwrap_single(group: Node) -> Node {
		match group {
			Node::List(mut items, _, _) if items.len() == 1 => items.remove(0),
			other => other,
		}
	}

	/// `@name` or `@name(value)` annotates the atom that follows: `@version(2) @draft tee{a:1}`.
	/// `@name:value` (inside a literal: `point{x:1 @source:"gps"}`) is the meta entry `@name`, never a field;
	/// after a dot, `p.@name` names the meta key itself.
	pub(super) fn parse_attribute(&mut self) -> Node {
		let after_dot = self.pos > 0 && self.chars[self.pos - 1] == '.';
		self.advance(); // skip '@'
		let name = or_return_error!(self.parse_symbol());
		if after_dot {
			return Symbol(format!("{ATTRIBUTE_MARK}{name}"));
		}
		if self.type_fields.is_some() && self.reads_instance_variable() {
			return Node::Key(
				Box::new(Symbol(RECEIVER_WORD.to_string())),
				Op::Dot,
				Box::new(Symbol(name)),
			);
		}
		if self.current_char() == ':' && self.peek_char(1) != '=' {
			self.advance();
			self.skip_spaces();
			return Node::Key(
				Box::new(Symbol(format!("{ATTRIBUTE_MARK}{name}"))),
				Op::Colon,
				Box::new(self.parse_atom()),
			);
		}
		let value = self.attribute_value();
		self.parse_atom().with_attribute(&name, value)
	}

	/// Ruby's instance variable `@x` in a class body, after its name: an operator, the end of the statement, or the line
	/// end before an `end` follows (an annotation `@deprecated fun f()` stands before a word or its own value)
	fn reads_instance_variable(&self) -> bool {
		let blanks = (0..)
			.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
			.count();
		match self.peek_char(blanks) {
			'\n' => {
				let next_line = blanks
					+ 1 + (0..)
					.take_while(|&offset| matches!(self.peek_char(blanks + 1 + offset), ' ' | '\t'))
					.count();
				let word: String = (next_line..)
					.map(|offset| self.peek_char(offset))
					.take_while(|ch| is_identifier_char(*ch))
					.collect();
				word == END_KEYWORD
			}
			'(' => false,
			next => !is_identifier_char(next) && next != '@',
		}
	}

	/// The value of `@name(value)`, true for a bare `@name`
	fn attribute_value(&mut self) -> Node {
		if self.current_char() == '(' {
			Self::unwrap_single(self.parse_bracketed('('))
		} else {
			Node::True
		}
	}

	/// `ys = xs.map(f) @parallel`: an `@name` or `@name(value)` with nothing after it on the statement annotates the
	/// expression before it (wiki/Purpose.md), where a leading one would annotate an empty atom
	pub(super) fn try_parse_trailing_attribute(&mut self, annotated: &Node) -> Option<Node> {
		if self.current_char() != '@' || !self.peek_char(1).is_alphabetic() {
			return None;
		}
		let name = self.word_at(1);
		let mut end = 1 + name.chars().count();
		if self.peek_char(end) == '(' {
			let mut depth = 0;
			loop {
				match self.peek_char(end) {
					'(' => depth += 1,
					')' if depth == 1 => break,
					')' => depth -= 1,
					'\0' | '\n' => return None,
					_ => {}
				}
				end += 1;
			}
			end += 1;
		}
		let after = end
			+ (end..)
				.take_while(|&at| matches!(self.peek_char(at), ' ' | '\t'))
				.count();
		if !matches!(self.peek_char(after), '\0' | '\n' | '\r' | ';' | ',' | ')' | ']' | '}') {
			return None;
		}
		self.advance_by(1 + name.chars().count());
		let value = self.attribute_value();
		Some(annotated.clone().with_attribute(&name, value))
	}
}
