//! Guards: `try`/`catch`/`finally` and fallback words over a guarded phrase
use super::*;
use crate::warp_parser::context::Context;

/// The words that start the fallback of `try X else Y`: `else`, classical `catch`, Python's `except` (P60)
const FALLBACK_WORDS: [&str; 3] = [ELSE_KEYWORD, "catch", "except"];
/// `try X catch Y finally Z`: Z runs after either, the value stays X's or Y's
const FINALLY_KEYWORD: &str = "finally";
/// The hidden name `try X finally Z` binds X's Error to, to hand it on as the value
const UNCAUGHT_ERROR: &str = "uncaught·error";
/// The hidden name `except ParseError:` binds the Error to, to test its class
const CAUGHT_ERROR: &str = "caught·error";

/// What a fallback catches: the name the Error is bound to and the class it must be (decision error-classes)
#[derive(Default)]
pub(crate) struct Caught {
	name: Option<String>,
	class: Option<String>,
}
/// `check C` is `assert C` (user, card check-assert)
const GUARD_MARKERS: [(&str, &str); 3] = [("try", TRY_MARKER), ("assert", ASSERT_MARKER), ("check", ASSERT_MARKER)];

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
		let start = self.pos;
		let guard = Context { stops_at_else: true, equals_compares: marker == ASSERT_MARKER, ..self.context };
		let guarded = self.within(guard, Self::parse_guarded_phrase);
		let written: String = self.chars[start..self.pos].iter().collect();
		self.skip_spaces();
		if self.fallback_word_ahead().is_some() {
			let mut guard = guarded;
			// `except A: a except B: b`: an Error A does not catch passes on to the next one; an `else` after the
			// fallback is an enclosing `if`'s
			let mut first = true;
			while let Some((word, ahead)) = self.fallback_word_ahead().filter(|(word, _)| first || *word != ELSE_KEYWORD) {
				first = false;
				self.advance_by(ahead + word.len());
				self.skip_spaces();
				let caught = if word == ELSE_KEYWORD { Caught::default() } else { self.parse_caught(word) };
				self.skip_python_colon();
				// `try X else print "msg"`: a braceless call as on the guarded side (card try-print)
				let fallback = self.parse_guarded_phrase();
				guard = guarded_by(marker, guard, fallback, caught);
				self.skip_spaces();
			}
			return self.with_finally(guard);
		}
		let (fallback, caught) = if marker == TRY_MARKER {
			// `r = try X` (user, card failed-raised) and `try X finally Z` are `try X catch e { e }`: a failure of X,
			// trap included, stays the value, which `r failed` tests
			(Symbol(UNCAUGHT_ERROR.to_string()), Caught { name: Some(UNCAUGHT_ERROR.to_string()), class: None })
		} else if marker == ASSERT_MARKER {
			// the error names the condition as written, before lowering rewrote it
			(Node::Text(format!("{ASSERTION_FAILED}: {}", written.trim())), Caught::default())
		} else {
			(Empty, Caught::default())
		};
		self.with_finally(guarded_by(marker, guarded, fallback, caught))
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

	/// `catch e`, `except ZeroDivisionError`, `except ValueError as e`: the name the Error is bound to, the class it
	/// must be
	pub(crate) fn parse_caught(&mut self, word: &str) -> Caught {
		if !self.is_identifier_start(0) {
			return Caught::default();
		}
		let first = self.word_at(0);
		self.advance_by(first.chars().count());
		self.skip_spaces();
		if self.matches_keyword("as") {
			self.advance_by(2);
			self.skip_spaces();
			let name = self.word_at(0);
			self.advance_by(name.chars().count());
			self.skip_spaces();
			return Caught { name: Some(name), class: Some(first) };
		}
		match word == "catch" {
			true => Caught { name: Some(first), class: None },
			false => Caught { name: None, class: Some(first) },
		}
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

/// The marker call `marker(X, Y)`, with the name Y reads the caught Error by as a fourth item (`catch e { … }`, P67);
/// `except ParseError as e` binds `e: ParseError`, which lowering tests before Y runs
fn guarded_by(marker: &str, guarded: Node, fallback: Node, caught: Caught) -> Node {
	let binding = match caught {
		Caught { name, class: Some(class) } => Some(Node::Key(
			Box::new(Symbol(name.unwrap_or_else(|| CAUGHT_ERROR.to_string()))),
			Op::Colon,
			Box::new(Symbol(class)),
		)),
		Caught { name, class: None } => name.filter(|name| mentions(&fallback, name)).map(Symbol),
	};
	Node::List(
		[vec![Symbol(marker.to_string()), guarded, fallback], binding.into_iter().collect()].concat(),
		Bracket::Round,
		Separator::None,
	)
}
