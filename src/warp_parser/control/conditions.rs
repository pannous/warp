//! Conditions: if/while heads, statement modifiers `x++ while c`, condition words `x empty`

use super::*;
use crate::words::COUNT_WORD;
use crate::words::IS_ERROR_WORD;

/// A control word behind a statement: `x++ while c`, `a = 2 if c`, `i++ until c`, `a = 2 unless c`
pub(crate) fn guarded_statement(statement: Node, guard: Op, word: &str, condition: Node) -> Node {
	if guard == Op::While {
		// a trailing `while` tests before the first round like the leading one; it is no do-while (user decision #22)
		let (statement, test) =
			(crate::normalize::operand_text(&statement), crate::normalize::operand_text(&condition));
		crate::normalize::hint(
			&format!("{statement} {word} {test}"),
			&format!("{word} {test} {{ {statement} }}"),
			"a trailing loop word tests before the first round, the statement may never run",
		);
	}
	let condition = if NEGATED_CONTROL_WORDS.iter().any(|(negated, _)| *negated == word) {
		Node::Key(Box::new(Empty), Op::Not, Box::new(condition))
	} else {
		condition
	};
	match guard {
		Op::While => while_do(condition, statement),
		_ => {
			Node::Key(Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(condition))), Op::Then, Box::new(statement))
		}
	}
}

/// `where`, alone or the last word of `for i in xs where`: a word whose condition follows
pub(crate) fn ends_with_clause_word(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => CLAUSE_WORDS.contains(&word.as_str()),
		Node::List(items, Bracket::None, _) => items.last().is_some_and(ends_with_clause_word),
		_ => false,
	}
}

impl WarpParser {
	/// The condition of `if` and `while`: `=` compares, and a braceless call takes a variable argument as in a branch
	/// (`if area certainly > 10 then`, `while square i < 10`)
	pub(crate) fn parse_condition(&mut self) -> Node {
		let condition_bp = Op::If.binding_power().1;
		let outer = self.branch_bp.replace(condition_bp);
		let condition = self.with_equals_comparing(true, |parser| parser.parse_expr(condition_bp));
		self.branch_bp = outer;
		condition
	}

	pub(crate) fn with_equals_comparing<T>(&mut self, compares: bool, parse: impl FnOnce(&mut Self) -> T) -> T {
		let outer = std::mem::replace(&mut self.equals_compares, compares);
		let node = parse(self);
		self.equals_compares = outer;
		node
	}

	/// `x empty`, `x missing`, `x is absent` … at the end of a condition are `not x` (wiki/null.md); `x failed` is `is_error(x)`.
	/// The word must end the condition: a block, colon, `then`/`else`/`and`/`or` or the end of the statement follows.
	/// `x is empty` holds for ø, "", [] and {} alike (wiki/null.md: empty checks missing or empty): `count(x) == 0`, as a
	/// number or a bool counts one.
	pub(crate) fn try_parse_test_word(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if self.options.data_mode || min_bp > Op::Not.binding_power().1 || lhs.is_empty_node() {
			return None;
		}
		let is_length = "is".len();
		let after_is = self.matches_keyword("is");
		let word_start = if after_is {
			is_length + (is_length..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count()
		} else {
			0
		};
		let word = TEST_WORDS.iter().find(|word| {
			word.chars().enumerate().all(|(index, letter)| self.peek_char(word_start + index) == letter)
				&& !is_identifier_char(self.peek_char(word_start + word.len()))
		})?;
		let end = word_start + word.len();
		let blanks = (end..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let next = self.peek_char(end + blanks);
		let word_ends_condition = !self.operand_follows(end)
			|| matches!(next, '{' | ':')
			|| crate::operators::is_operator_word(&self.word_at(end + blanks));
		if !word_ends_condition {
			return None;
		}
		self.advance_by(end);
		if *word == FAILED_WORD {
			return Some(call(IS_ERROR_WORD, vec![lhs.clone()]));
		}
		if after_is && *word == EMPTY_WORD {
			let counted = call(COUNT_WORD, vec![lhs.clone()]);
			return Some(Node::Key(Box::new(counted), Op::Eq, Box::new(Node::int(0))));
		}
		Some(Node::Key(Box::new(Empty), Op::Not, Box::new(lhs.clone())))
	}

	/// `unless`/`until` in front of a statement: the `if`/`while` it reads as, with the condition negated afterwards
	pub(crate) fn peek_negated_control_word(&self) -> Option<(Op, usize)> {
		if self.options.data_mode {
			return None;
		}
		NEGATED_CONTROL_WORDS.iter().find(|(word, _)| self.matches_keyword(word)).map(|(word, op)| (*op, word.len()))
	}
}

/// `print it if it%2`, `do {i++} while c`: a trailing `if` guards and a trailing `while` loops the whole command, not
/// its last word, as in Ruby and Perl (card glued-loop)
pub(crate) fn guarded_command(items: &[Node]) -> Option<Node> {
	let (last, command) = items.split_last()?;
	if !matches!(command.first()?.drop_meta(), Symbol(_)) {
		return None;
	}
	let whole =
		|tail: &Node| Node::List([command, std::slice::from_ref(tail)].concat(), Bracket::None, Separator::Space);
	match last.drop_meta() {
		Node::Key(head, Op::Do, body) => match head.drop_meta() {
			Node::Key(nothing, Op::While, condition) if **nothing == Node::Empty => {
				Some(while_do((**condition).clone(), whole(body)))
			}
			_ => None,
		},
		Node::Key(condition, Op::Then, guarded) => match condition.drop_meta() {
			Node::Key(nothing, Op::If, _) if **nothing == Node::Empty => {
				Some(Node::Key(condition.clone(), Op::Then, Box::new(whole(guarded))))
			}
			_ => None,
		},
		_ => None,
	}
}

/// The condition of the `if`/`while` head inside a parsed conditional or loop, negated: `unless c {…}` is `if not c {…}`
pub(crate) fn negate_condition(conditional: Node) -> Node {
	match conditional {
		Node::Key(head, op @ (Op::Then | Op::Else | Op::Do), body) => {
			Node::Key(Box::new(negate_condition(*head)), op, body)
		}
		Node::Key(empty, op @ (Op::If | Op::While), condition) if matches!(empty.drop_meta(), Empty) => {
			Node::Key(empty, op, Box::new(Node::Key(Box::new(Empty), Op::Not, condition)))
		}
		other => other,
	}
}
