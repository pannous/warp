//! if/then/else, else-if chains, the elvis operator

use super::*;
use crate::warp_parser::context::Context;

/// Python's `elif`, Perl's and Ruby's `elsif`, PHP's `elseif`: all `else if`
const ELSE_IF_WORDS: [&str; 3] = ["elif", "elsif", "elseif"];
const ELVIS_WORD: &str = "elvis";

/// The word between a condition and its branch: `if c then x`
const THEN_WORD: &str = "then";

impl WarpParser {
	/// A branch without braces takes the whole statement: `if c then s += 5 else s = 0` assigns in the branch, where
	/// assignment's weaker binding would end the branch at `s` and assign to the whole if. The value stops at `else`.
	pub(crate) fn branch_assignment(&mut self, target: Node, branch_bp: u8) -> Node {
		self.skip_spaces();
		let Some((assignment, chars)) =
			self.peek_operator().filter(|(op, _)| *op == Op::Assign || op.is_compound_assign())
		else {
			return target;
		};
		self.advance_by(chars);
		self.skip_whitespace();
		let value = self.parse_expr(branch_bp);
		Node::Key(Box::new(target), assignment, Box::new(value))
	}

	pub(crate) fn finish_if_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let then_block = self.parse_atom(); // parse { block }
			let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(rhs));
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
			return self.parse_optional_else(if_then, ElseParseMode::Atom);
		}

		if let Node::Key(cond, Op::Colon, then_expr) = &rhs {
			let if_cond = Node::Key(Box::new(Empty), Op::If, cond.clone());
			// the body runs to the end of the statement, `if c: x+=1`, but not into the `else`
			let then_expr = self.within(Context { stops_at_else: true, ..self.context }, |parser| {
				parser.continue_colon_body(then_expr)
			});
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_expr));
			return self.parse_optional_else(if_then, ElseParseMode::Expr);
		}

		// `if (x < 0) return -1`: C's parenthesized condition, then the statement on the same line
		let is_grouped = matches!(rhs.drop_meta(), Node::List(_, Bracket::Round, _));
		if is_grouped && self.at_body_start() && !self.matches_keyword("then") && !self.matches_keyword("else") {
			let then_expr = self.within(Context { stops_at_else: true, ..self.context }, |parser| parser.parse_expr(0));
			let if_then =
				Node::Key(Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(rhs))), Op::Then, Box::new(then_expr));
			return self.parse_optional_else(if_then, ElseParseMode::Expr);
		}

		Node::Key(Box::new(Empty), Op::If, Box::new(rhs))
	}

	/// A branch of an if, `print x+1` of `else print x+1` printing its one expression
	pub(crate) fn parse_branch(&mut self, min_bp: u8) -> Node {
		match self.take_braceless_print() {
			true => {
				let first = self.expression_binding(min_bp);
				self.print_call_from(first, |parser| parser.expression_binding(min_bp)) // `else print x, y`, `else print ord x`
			}
			false => self.parse_expr(min_bp),
		}
	}

	/// `else body`, and `else if` / `elif` / `elsif` / `elseif` chaining a whole new `if … {…} else …`
	pub(crate) fn parse_optional_else(&mut self, if_then: Node, mode: ElseParseMode) -> Node {
		self.skip_spaces();
		// `}` then `else …` on the next line belongs to this if
		if self.else_on_a_later_line() {
			while matches!(self.current_char(), ' ' | '\t' | '\n' | '\r') {
				self.advance();
			}
		}
		let else_expr = if let Some(word) = self.at_else_if_word() {
			self.advance_by(word.len());
			self.parse_else_if()
		} else if self.matches_keyword("else") {
			self.advance_by("else".len());
			self.skip_spaces();
			match mode {
				_ if self.matches_keyword("if") => {
					self.advance_by("if".len());
					self.parse_else_if()
				}
				_ if self.current_char() == ':' => {
					self.advance(); // Python's `else: body`
					self.parse_branch(0)
				}
				ElseParseMode::Atom if self.current_char() == '{' => self.parse_atom(),
				_ => self.parse_branch(0), // `if c {1} else 3+1`, `if c {…} else print x+1`: the rest of the expression
			}
		} else {
			return if_then;
		};
		Node::Key(Box::new(if_then), Op::Else, Box::new(else_expr))
	}

	/// Do line breaks and blanks, then the word `else`, follow the cursor
	pub(crate) fn else_on_a_later_line(&self) -> bool {
		let gap = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t' | '\n' | '\r')).count();
		let crosses_line = (0..gap).any(|at| self.peek_char(at) == '\n');
		let word = ELSE_KEYWORD.chars().enumerate().all(|(i, c)| self.peek_char(gap + i) == c)
			&& !is_identifier_char(self.peek_char(gap + ELSE_KEYWORD.len()));
		crosses_line && word
	}

	pub(crate) fn at_else_if_word(&self) -> Option<&'static str> {
		ELSE_IF_WORDS.into_iter().find(|word| self.matches_keyword(word))
	}

	pub(crate) fn parse_else_if(&mut self) -> Node {
		let condition = self.parse_prefix_operand(Op::If);
		self.finish_if_prefix(condition)
	}

	/// `if b {y}` or `if b then y` after a statement (`if a {x} if b {y}`) starts the next statement: a trailing `if`
	/// guards without a branch of its own. Scans the condition from `offset` to the end of the statement for a block
	/// (a spaced `{` outside brackets: `point{x:1}` is data) or `then`
	pub(crate) fn condition_has_branch(&self, offset: usize) -> bool {
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
				(None, 't')
					if depth == 0 && !is_identifier_char(self.peek_char(at - 1)) && self.word_at(at) == THEN_WORD =>
				{
					return true
				}
				_ => {}
			}
		}
		false
	}

	/// `a ?: b` is `a ? a : b`; a left side that is not a plain name is evaluated once, into a hidden variable
	pub(crate) fn try_parse_elvis(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let (question_bp, _) = Op::Question.binding_power();
		if self.current_char() != '?' || self.peek_char(1) != ':' || question_bp < min_bp {
			return None;
		}
		self.advance_by(2);
		self.skip_whitespace();
		let alternative = self.parse_expr(question_bp + 1); // left-assoc: `a ?: b ?: c` is `(a ?: b) ?: c`, the same value
		let elvis = |value: Node| {
			Node::Key(
				Box::new(value.clone()),
				Op::Question,
				Box::new(Node::Key(Box::new(value), Op::Colon, Box::new(alternative))),
			)
		};
		if matches!(lhs.drop_meta(), Node::Symbol(_)) {
			return Some(elvis(lhs.clone()));
		}
		self.elvis_operands += 1;
		let operand = Symbol(format!("{ELVIS_WORD}·{}", self.elvis_operands));
		let store = Node::Key(Box::new(operand.clone()), Op::Assign, Box::new(lhs.clone()));
		Some(Node::List(vec![store, elvis(operand)], Bracket::Round, Separator::Semicolon))
	}
}
