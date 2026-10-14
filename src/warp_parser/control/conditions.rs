//! Conditions: if/while heads, statement modifiers `x++ while c`, condition words `x empty`

use super::*;

/// A control word behind a statement: `x++ while c`, `a = 2 if c`, `i++ until c`, `a = 2 unless c`
pub(crate) fn guarded_statement(statement: Node, guard: Op, word: &str, condition: Node) -> Node {
	if guard == Op::While {
		// a trailing `while` tests before the first round like the leading one; it is no do-while (user decision #22)
		let (statement, test) = (
			crate::normalize::operand_text(&statement),
			crate::normalize::operand_text(&condition),
		);
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
		_ => Node::Key(
			Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(condition))),
			Op::Then,
			Box::new(statement),
		),
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
		if self.options.data_mode || min_bp > Op::Not.binding_power().1 || matches!(lhs.drop_meta(), Empty) {
			return None;
		}
		let is_length = "is".len();
		let after_is = self.matches_keyword("is");
		let word_start = if after_is {
			is_length
				+ (is_length..)
					.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
					.count()
		} else {
			0
		};
		let word = TEST_WORDS.iter().find(|word| {
			word.chars()
				.enumerate()
				.all(|(index, letter)| self.peek_char(word_start + index) == letter)
				&& !is_identifier_char(self.peek_char(word_start + word.len()))
		})?;
		let end = word_start + word.len();
		let blanks = (end..)
			.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
			.count();
		let next = self.peek_char(end + blanks);
		let word_ends_condition = !self.operand_follows(end)
			|| matches!(next, '{' | ':')
			|| crate::operators::is_operator_word(&self.word_at(end + blanks));
		if !word_ends_condition {
			return None;
		}
		self.advance_by(end);
		if *word == FAILED_WORD {
			return Some(call(IS_ERROR_CALL, vec![lhs.clone()]));
		}
		if after_is && *word == EMPTY_WORD {
			let counted = call(COUNT_CALL, vec![lhs.clone()]);
			return Some(Node::Key(Box::new(counted), Op::Eq, Box::new(Node::int(0))));
		}
		Some(Node::Key(Box::new(Empty), Op::Not, Box::new(lhs.clone())))
	}
}
