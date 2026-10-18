//! Jumps: return, await, emit

use super::*;
use crate::words::RETURN_WORD;
use crate::words::{EMIT_ALIASES, EMIT_WORDS};

/// `await job` waits for a task; its operand binds like the operand of a unary minus (Op::Neg)
const AWAIT_KEYWORD: &str = "await";
/// `await all jobs`: every task of a list (P47)
const AWAIT_ALL_WORD: &str = "all";
/// `await x` binds its operand like unary minus
const AWAIT_OPERAND_BP: u8 = Op::Neg.binding_power().1;
const AFTER_KEYWORD: &str = "after";

impl WarpParser {
	/// `await a + await b`: `await` takes its operand like a unary minus, so each task is awaited before the sum.
	/// `await(x)` (a call or a definition of a function `await`) and `await = …` stay as they are
	pub(crate) fn try_parse_await(&mut self) -> Option<Node> {
		if !self.matches_keyword(AWAIT_KEYWORD) {
			return None;
		}
		let before_keyword = self.mark();
		self.advance_by(AWAIT_KEYWORD.len());
		let called = self.current_char() == '(';
		self.skip_spaces();
		let names_a_variable = self
			.peek_operator()
			.is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		if called || names_a_variable || matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0' | ')') {
			self.rewind(before_keyword);
			return None;
		}
		// `await all jobs` (P47): every task of a list
		if self.matches_keyword(AWAIT_ALL_WORD)
			&& self.peek_char(AWAIT_ALL_WORD.len()) == ' '
			&& (self.is_identifier_start(AWAIT_ALL_WORD.len() + 1)
				|| matches!(self.peek_char(AWAIT_ALL_WORD.len() + 1), '[' | '('))
		{
			self.advance_by(AWAIT_ALL_WORD.len());
			self.skip_spaces();
			let list = self.parse_expr(AWAIT_OPERAND_BP);
			let all = Node::List(vec![Symbol(AWAIT_ALL_WORD.to_string()), list], Bracket::None, Separator::Space);
			return Some(Node::List(vec![Symbol(AWAIT_KEYWORD.to_string()), all], Bracket::None, Separator::Space));
		}
		let operand = self.parse_expr(AWAIT_OPERAND_BP);
		Some(Node::List(vec![Symbol(AWAIT_KEYWORD.to_string()), operand], Bracket::None, Separator::Space))
	}

	/// `1 + emit ask`, `net * (1 + ask tax_rate)` (card price-net), `2 * emit stop the machine{reason: "x"} + 1`: the phrase `emit ask` is one operand, the words
	/// of the event and its data bind like the operand of a unary minus; the same Space list a statement `emit ask` is
	pub(crate) fn try_parse_emit(&mut self) -> Option<Node> {
		let keyword = EMIT_WORDS.into_iter().chain(EMIT_ALIASES).find(|keyword| self.matches_keyword(keyword))?;
		let before_keyword = self.mark();
		self.advance_by(keyword.len());
		self.skip_spaces();
		if !self.is_identifier_start(0) || self.peek_operator().is_some() {
			self.rewind(before_keyword);
			return None;
		}
		let mut phrase = vec![Symbol(keyword.to_string())];
		while self.is_identifier_start(0) || (phrase.len() > 1 && matches!(self.current_char(), '0'..='9' | '"' | '\''))
		{
			// `send alarm{} to "x"`: a word operator takes the whole phrase, as without this
			if self.peek_operator().is_some() {
				self.rewind(before_keyword);
				return None;
			}
			phrase.push(self.parse_expr(AWAIT_OPERAND_BP));
			self.skip_spaces();
		}
		Some(Node::List(phrase, Bracket::None, Separator::Space))
	}

	/// `return` takes the whole expression after it: `return -1` is no subtraction from `return`
	pub(crate) fn try_parse_return(&mut self) -> Option<Node> {
		if !self.matches_keyword(RETURN_WORD) {
			return None;
		}
		let before_keyword = self.mark();
		self.advance_by(RETURN_WORD.len());
		self.skip_spaces();
		let names_a_variable = self
			.peek_operator()
			.is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		let ends_statement = matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0');
		if !ends_statement && !self.at_body_start() || names_a_variable {
			self.rewind(before_keyword); // `return := …` as a name
			return None;
		}
		// the binding of an assigned value: `return square n` returns the braceless call, like `x = square n`; then what
		// binds looser, Python's `return a if c else b`
		let value = if ends_statement {
			Node::Empty
		} else {
			let value = self.parse_expr(Op::Assign.binding_power().1);
			self.continue_expr(value, 0)
		};
		// returning ø, bare (`if t == ø { return }`) or written (`return ø`), is the word alone (lower_bare_returns)
		Some(match value {
			returned if matches!(returned.drop_meta(), Node::Empty) => Symbol(RETURN_WORD.to_string()),
			value => Node::List(vec![Symbol(RETURN_WORD.to_string()), value], Bracket::None, Separator::Space),
		})
	}

	/// `after C return V`: the marker call `after·return(C, V)`, when a `return` follows on the statement (outside brackets);
	/// `after tested: body` without one stays a call listener (variable_signals)
	pub(crate) fn try_parse_after_return(&mut self) -> Option<Node> {
		if self.options.data_mode || !self.matches_keyword(AFTER_KEYWORD) || !self.return_ahead() {
			return None;
		}
		self.advance_by(AFTER_KEYWORD.len());
		let condition = self.parse_expr(0);
		self.skip_spaces();
		if !self.matches_keyword(RETURN_WORD) {
			return Some(error("`after` needs a `return`: `after C return V`"));
		}
		self.advance_by(RETURN_WORD.len());
		let value = self.parse_expr(0);
		Some(call(AFTER_MARKER, vec![condition, value]))
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
				c if depth == 0 && c.is_whitespace() && self.word_at(offset + 1) == RETURN_WORD => return true,
				_ => {}
			}
		}
		false
	}
}
