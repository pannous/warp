//! The small node forms many lowering passes build and test: a call of a word, a word

use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{FOR_WORD, IN_WORD, MATCH_WORD};
use std::cell::Cell;

pub(crate) use crate::node::{call, children_rewritten, int, symbol};

pub(crate) fn key(left: Node, op: Op, right: Node) -> Node {
	Node::Key(Box::new(left), op, Box::new(right))
}

/// `match subject { arms }`
pub(crate) fn matched(subject: Node, arms: Vec<Node>) -> Node {
	Node::List(
		vec![
			Node::Symbol(MATCH_WORD.into()),
			subject,
			Node::List(arms, Bracket::Curly, Separator::Semicolon),
		],
		Bracket::None,
		Separator::Space,
	)
}

/// The operands of `a or b or c` (`|` parses as or, between whole numbers as the bit or), else the node itself
pub(crate) fn or_operands(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::Key(left, Op::Or | Op::BitOr, right) => [or_operands(left), or_operands(right)].concat(),
		_ => vec![node.clone()],
	}
}

pub(crate) fn assign(target: Node, value: Node) -> Node {
	key(target, Op::Assign, value)
}

/// Where an entry is and its key: `x[k]` (parsed as `x#(k+1)`) and `x.k` (the text "k") as `x#k` was written, which
/// counts positions from 1 (`true`)
pub(crate) fn written_entry(target: &Node) -> Option<(&Node, Node, bool)> {
	let Node::Key(place, op, index) = target.drop_meta() else {
		return None;
	};
	match (op, index.drop_meta()) {
		(Op::Dot, Node::Symbol(field)) => Some((place, Node::Text(field.clone()), false)),
		(Op::Hash, Node::Key(key, Op::Add, one)) if *one.drop_meta() == int(1) => {
			Some((place, key.as_ref().clone(), false))
		}
		(Op::Hash, key) => Some((place, key.clone(), true)),
		_ => None,
	}
}

/// `play 440` alone or as the one statement of a block `{play 440}`: the words of a call (card statement-block)
pub(crate) fn is_spaced_call(bracket: &Bracket, separator: &Separator) -> bool {
	matches!(bracket, Bracket::None | Bracket::Curly) && *separator == Separator::Space
}

/// `f x`, `(f x)`, `{f x}`: words that call their head
pub(crate) fn is_words_call(bracket: &Bracket, separator: &Separator) -> bool {
	is_spaced_call(bracket, separator) || *bracket == Bracket::Round && *separator == Separator::Space
}

/// The call resolved from the words of `{play 440}` stays the block of its one statement, `{play(440)}`
pub(crate) fn in_block_as_written(bracket: &Bracket, call: Node) -> Node {
	match bracket {
		Bracket::Curly => Node::List(vec![call], Bracket::Curly, Separator::None),
		_ => call,
	}
}

/// `x = {a b}` assigns the data `{a b}`, no block of the statement `a b`: `x = {abs -3}` is that list
pub(crate) fn is_assigned_data(target: &Node, value: &Node) -> bool {
	matches!(target.drop_meta(), Node::Symbol(_)) && matches!(value.drop_meta(), Node::List(_, Bracket::Curly, _))
}

/// The parts of a node rewritten, the node itself kept as written
pub(crate) fn with_parts_rewritten(node: Node, rewrite: &mut dyn FnMut(Node) -> Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta {
			node: Box::new(with_parts_rewritten(*node, rewrite)),
			data,
		},
		other => children_rewritten(other, rewrite),
	}
}

/// Statements run in order: `a; b`, `{a; b}`
pub(crate) fn statement_list(statements: Vec<Node>, bracket: Bracket) -> Node {
	Node::List(statements, bracket, Separator::Semicolon)
}

/// `{a; b}`
pub(crate) fn block(statements: Vec<Node>) -> Node {
	statement_list(statements, Bracket::Curly)
}

pub(crate) fn is_block(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _))
}

/// The words `for i in …` of a loop: i
pub(crate) fn for_in_variable(items: &[Node]) -> Option<String> {
	match items {
		[keyword, variable, within, ..] if keyword.is_symbol(FOR_WORD) && within.is_symbol(IN_WORD) => {
			variable.symbol_name().map(String::from)
		}
		_ => None,
	}
}

/// `if condition then body`
pub(crate) fn if_then(condition: Node, body: Node) -> Node {
	key(key(Node::Empty, Op::If, condition), Op::Then, body)
}

pub(crate) fn if_then_else(condition: Node, body: Node, otherwise: Node) -> Node {
	key(if_then(condition, body), Op::Else, otherwise)
}

/// `a: 1`
pub(crate) fn is_colon_pair(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, _))
}

/// `=`, `:=` and the compound assignments `+=` …: the operators binding a name
pub(crate) fn is_binding(op: &Op) -> bool {
	matches!(op, Op::Assign | Op::Define) || op.is_compound_assign()
}

/// `def`, `fun`, `fn` …
pub(crate) fn is_function_keyword(node: &Node) -> bool {
	node.symbol_name().is_some_and(crate::operators::is_function_keyword)
}

/// `int`, `float`, `text`, …: a word naming a type
pub(crate) fn is_type_word(node: &Node) -> bool {
	node.symbol_name()
		.is_some_and(|word| crate::analyzer::type_word_kind(word).is_some())
}

/// An operand `&` and `|` may combine bit by bit (card operator-shall): a whole number, a name, a call or an
/// arithmetic; a truth value, comparison, text, list, float or type is no whole number, where they are and/or
pub(crate) fn may_be_whole_number(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(number) => matches!(number, Number::Int(_) | Number::BigInt(_)),
		Node::Symbol(name) => !is_type_word(node) && !name.starts_with(char::is_uppercase),
		Node::Key(_, op, _) => !(op.is_comparison() || op.is_logical() || *op == Op::Not),
		// a call `f(4)`, never `[1 2]` or the type-headed `text { … }`
		Node::List(items, bracket, _) => *bracket != Bracket::Square && items.first().is_none_or(may_be_whole_number),
		_ => false,
	}
}

/// The name a parameter declares: `x`, `x:int`, `x=1`
pub(crate) fn parameter_name(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, _, _) => parameter_name(name),
		_ => None,
	}
}

/// `f(…)`: a definition's head naming a function
pub(crate) fn is_call_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().and_then(Node::symbol_name).is_some())
}

/// `name words… last = body`, parsed as the items `name`, `words…`, `last = body`: the name, the words through `last`
/// and the body
pub(crate) fn spaced_statement(items: &[Node]) -> Option<(&Node, Vec<&str>, &Node)> {
	let (name, rest) = items.split_first()?;
	let (last, middle) = rest.split_last()?;
	let Node::Key(last_word, Op::Assign | Op::Define, body) = last.drop_meta() else {
		return None;
	};
	let words = middle
		.iter()
		.chain(std::iter::once(last_word.as_ref()))
		.map(Node::symbol_name)
		.collect::<Option<_>>()?;
	Some((name, words, body))
}

/// A parameter as the parameters it stands for: a group `(a, b)` its items, ø none
pub(crate) fn grouped_parameters(parameter: &Node) -> Vec<Node> {
	match parameter.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![parameter.clone()],
	}
}

/// Numbers a pass's temporaries apart: each `next_number` the next one, from 0 or from where it starts
#[derive(Default)]
pub(crate) struct Counter(Cell<usize>);

impl Counter {
	pub(crate) const fn starting_at(first: usize) -> Self {
		Counter(Cell::new(first))
	}

	pub(crate) fn next_number(&self) -> usize {
		self.0.replace(self.0.get() + 1)
	}
}

/// `x = value`: the name and the value
pub(crate) fn named_assignment(statement: &Node) -> Option<(String, Node)> {
	match statement.drop_meta() {
		Node::Key(name, Op::Assign, value) => Some((name.symbol_name()?.to_string(), value.as_ref().clone())),
		_ => None,
	}
}

/// How a function definition is written
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitionForm {
	/// `f(x) := body`, `f(x) = body`, `f(x): int := body`: Define or Assign
	Operator(Op),
	/// `def f(x): body`, `def f(x) := body`, `def f(x) -> int {…}`, `def f(x): int = body`: the operator after the head
	Keyword(Op),
	/// `def f(x) { body }`: head and block side by side, the parameters grouped `(f (x))`
	KeywordBlock,
}

/// One view of a function definition in any of its forms (DefinitionForm): what each pass takes apart
#[derive(Clone, Debug)]
pub struct FunctionHead<'a> {
	/// `(f x y)`
	pub head: &'a Node,
	pub name: &'a str,
	/// What follows the name in the head, as written: `x y` of `(f x y)`, `(x, y)` of `def f(x, y) {…}`
	pub written_parameters: &'a [Node],
	pub form: DefinitionForm,
	/// `int` of `f(x): int := …`, `def f(x) -> int {…}`, `def f(x): int: …`
	pub result: Option<&'a Node>,
	pub body: &'a Node,
	/// Everything after the operator: the body with a result type written before it (`int {…}` of `def f(x): int {…}`)
	pub written_body: &'a Node,
}

impl<'a> FunctionHead<'a> {
	/// The parameters: none for `f()`, the grouped ones of `def f(x, y) {…}`
	pub fn parameters(&self) -> Vec<&'a Node> {
		match self.written_parameters {
			[only] if only.is_nothing() => vec![],
			[Node::List(grouped, Bracket::Round, _)] => grouped.iter().collect(),
			parameters => parameters.iter().collect(),
		}
	}

	pub fn is_keyword_form(&self) -> bool {
		!matches!(self.form, DefinitionForm::Operator(_))
	}
}

/// `f(x) := body` and the other forms of a function definition (DefinitionForm), whose head is a word and what follows it
pub fn function_head(statement: &Node) -> Option<FunctionHead<'_>> {
	match statement.drop_meta() {
		Node::Key(target, op @ (Op::Define | Op::Assign), body) => {
			let (head, result) = typed_head(target);
			viewed(head, DefinitionForm::Operator(*op), result, body, body)
		}
		Node::List(items, _, _) => match items.as_slice() {
			[keyword, definition] if crate::operators::is_function_keyword(&keyword.drop_meta().name()) => {
				keyword_function_head(definition)
			}
			_ => None,
		},
		_ => None,
	}
}

fn keyword_function_head(definition: &Node) -> Option<FunctionHead<'_>> {
	match definition.drop_meta() {
		Node::List(parts, _, _) if parts.len() == 2 => {
			viewed(&parts[0], DefinitionForm::KeywordBlock, None, &parts[1], &parts[1])
		}
		Node::Key(target, op @ (Op::Define | Op::Assign), body) => {
			let (head, result) = typed_head(target);
			viewed(head, DefinitionForm::Keyword(*op), result, body, body)
		}
		Node::Key(head, op @ (Op::Colon | Op::Arrow), written_body) => {
			let (result, body) = result_and_body(written_body);
			viewed(head, DefinitionForm::Keyword(*op), result, body, written_body)
		}
		_ => None,
	}
}

fn viewed<'a>(
	head: &'a Node,
	form: DefinitionForm,
	result: Option<&'a Node>,
	body: &'a Node,
	written_body: &'a Node,
) -> Option<FunctionHead<'a>> {
	let Node::List(items, _, _) = head.drop_meta() else {
		return None;
	};
	let (name, written_parameters) = items.split_first()?;
	Some(FunctionHead {
		head,
		name: name.drop_meta().symbol_name()?,
		written_parameters,
		form,
		result,
		body,
		written_body,
	})
}

/// `(f x): int`, `(f x) as int`: the head and its result type
fn typed_head(target: &Node) -> (&Node, Option<&Node>) {
	match target.drop_meta() {
		Node::Key(head, Op::Colon | Op::As, result) if is_word_head(head) => (head, Some(result)),
		_ => (target, None),
	}
}

fn is_word_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|name| matches!(name.drop_meta(), Node::Symbol(_))))
}

/// `int {…}` or `int: body` after `def f(x):` or `->`: the result type and the body; anything else is the body
fn result_and_body(written_body: &Node) -> (Option<&Node>, &Node) {
	match written_body.drop_meta() {
		Node::List(parts, _, Separator::Space)
			if parts.len() == 2 && matches!(parts[1].drop_meta(), Node::List(_, Bracket::Curly, _)) =>
		{
			(Some(&parts[0]), &parts[1])
		}
		Node::Key(result, Op::Colon, body) => (Some(result), body),
		_ => (None, written_body),
	}
}
