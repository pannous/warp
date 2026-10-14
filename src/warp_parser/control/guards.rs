//! Guards: `try`/`catch`/`finally` and fallback words over a guarded phrase
use super::*;

impl WarpParser {
	/// `try` or `assert` followed by an operand: the words that guard a statement
	pub(crate) fn peek_guard_word(&self) -> Option<&'static str> {
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
	pub(crate) fn parse_guard(&mut self, marker: &'static str) -> Node {
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
			caught = if word == ELSE_KEYWORD { None } else { self.parse_caught_name(word) };
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
			[vec![Symbol(marker.to_string()), guarded, fallback], binding.into_iter().collect()].concat(),
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
		let cleanup = if self.current_char() == ':' { self.colon_body(":") } else { self.rest_of_statement() };
		self.finally_blocks += 1;
		let value = Symbol(format!("{FINALLY_KEYWORD}·{}", self.finally_blocks));
		let held = Node::Key(Box::new(value.clone()), Op::Assign, Box::new(guard));
		Node::List(vec![held, cleanup, value], Bracket::Round, Separator::Semicolon)
	}

	/// The blanks before a following `finally`, if one follows
	fn finally_ahead(&self) -> Option<usize> {
		let ahead = (0..).take_while(|&offset| self.peek_char(offset).is_whitespace()).count();
		(self.word_at(ahead) == FINALLY_KEYWORD).then_some(ahead)
	}

	/// The word that starts the fallback of `try`: `else`, or its classical synonyms `catch` and Python's `except` (P60),
	/// with how many blanks (line breaks included) come before it
	pub(crate) fn fallback_word_ahead(&self) -> Option<(&'static str, usize)> {
		let ahead = (0..).take_while(|&offset| self.peek_char(offset).is_whitespace()).count();
		FALLBACK_WORDS.iter().find(|word| self.word_at(ahead) == **word).map(|word| (*word, ahead))
	}

	/// `catch e`, `except ZeroDivisionError`, `except ValueError as e`: the name the error would be bound to
	pub(crate) fn parse_caught_name(&mut self, word: &str) -> Option<String> {
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

	/// Either part of `try X else Y`: one expression, or a braceless call of several (`try raise "boom" else print 3`)
	pub(crate) fn parse_guarded_phrase(&mut self) -> Node {
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
}
