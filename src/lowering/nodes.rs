//! The small node forms many lowering passes build and take apart: calls, keys, blocks, conditions, words, the parts
//! of a function definition

use crate::analyzer::call_name;
use crate::extensions::numbers::Number;
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{EACH_WORD, FOR_WORD, IN_WORD, MATCH_WORD};
use std::cell::Cell;

pub(crate) use crate::node::{call, children_rewritten, int, prelude_call, symbol};

pub(crate) fn key(left: Node, op: Op, right: Node) -> Node {
	Node::Key(Box::new(left), op, Box::new(right))
}

/// `room.reverb = 0.6` of a function reverb(room) read as `room.reverb`: `room = reverb(room, 0.6)`, the call's value
/// with the new setting stored back (decision shared-buses), never a definition of `reverb(room)`
pub(crate) fn set_by_call(receiver: &Node, function: Node, value: Node) -> Node {
	key(receiver.clone(), Op::Assign, parens(vec![function, receiver.clone(), value]))
}

/// `match subject { arms }`
pub(crate) fn matched(subject: Node, arms: Vec<Node>) -> Node {
	Node::List(
		vec![Node::Symbol(MATCH_WORD.into()), subject, Node::List(arms, Bracket::Curly, Separator::Semicolon)],
		Bracket::None,
		Separator::Space,
	)
}

/// The code with each placeholder word as its node: `filled("f(X)", [("X", x)])`
pub(crate) fn filled<'a>(code: &str, bindings: impl IntoIterator<Item = (&'a str, Node)>) -> Node {
	let bindings = bindings.into_iter().map(|(placeholder, node)| (placeholder.to_string(), node)).collect();
	crate::law::substitute(crate::warp_parser::parse_generated(code).drop_meta(), &bindings)
}

/// The words of a phrase, the groups the parser makes of it undone: `cheapest = items sorted by price` reads as
/// `[[items, [sorted, by]], price]`
pub(crate) fn phrase_words(items: &[Node]) -> Vec<Node> {
	items
		.iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(inner, Bracket::None, Separator::Space) => phrase_words(inner),
			_ => vec![item.clone()],
		})
		.collect()
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

/// `name:type`, a typed parameter or declaration
pub(crate) fn typed(name: &str, type_name: &str) -> Node {
	key(symbol(name), Op::Colon, symbol(type_name))
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
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_parts_rewritten(node, rewrite)),
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

/// A place a value can be written to: a variable `s`, a field of one `box.shape`, `p#…` (a declared type's field
/// index), `shapes#1`, in parentheses or not
pub(crate) fn is_place(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(_) => true,
		Node::Key(object, Op::Dot, field) => matches!(field.drop_meta(), Node::Symbol(_)) && is_place(object),
		Node::Key(object, Op::Hash, _) => is_place(object),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_place(&items[0]),
		_ => false,
	}
}

pub(crate) fn is_block(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _))
}

/// The words `for i in …` or `for each i in …` of a loop: i
pub(crate) fn for_in_variable(items: &[Node]) -> Option<String> {
	match items {
		[keyword, each, variable, within, ..]
			if keyword.is_symbol(FOR_WORD) && each.is_symbol(EACH_WORD) && within.is_symbol(IN_WORD) =>
		{
			variable.symbol_name().map(String::from)
		}
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

/// `if c then a else b` and `c ? a : b`: the condition and both branches
pub(crate) fn branches(node: &Node) -> Option<(&Node, &Node, &Node)> {
	match node.drop_meta() {
		Node::Key(if_then, Op::Else, otherwise) => {
			let (condition, then) = if_then_parts(if_then)?;
			Some((condition, then, otherwise))
		}
		Node::Key(condition, Op::Question, branches) => match branches.drop_meta() {
			Node::Key(then, Op::Colon, otherwise) => Some((condition, then, otherwise)),
			_ => None,
		},
		_ => None,
	}
}

/// `if c then a`: the condition and the branch
pub(crate) fn if_then_parts(node: &Node) -> Option<(&Node, &Node)> {
	let Node::Key(condition, Op::Then, then) = node.drop_meta() else {
		return None;
	};
	match condition.drop_meta() {
		Node::Key(empty, Op::If, condition) if empty.is_empty_node() => Some((condition, then)),
		_ => None,
	}
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

/// `int`, `float`, `text`, `u8`, `number` …: a word naming a type with a value kind (analyzer::type_word_kind)
pub(crate) fn is_kinded_type_word(node: &Node) -> bool {
	node.symbol_name().is_some_and(|word| crate::analyzer::type_word_kind(word).is_some())
}

/// C#'s and Java's one parameter `int x`, `int[] xs` (`[int] xs` as parsed): a type, then the name
fn is_typed_parameter(words: &[Node]) -> bool {
	let is_type = |word: &Node| match word.drop_meta() {
		Node::List(element, Bracket::Square, _) => matches!(element.as_slice(), [only] if is_kinded_type_word(only)),
		_ => is_kinded_type_word(word),
	};
	matches!(words, [kind, name] if is_type(kind) && matches!(name.drop_meta(), Node::Symbol(_)))
}

/// An operand `&` and `|` may combine bit by bit (card operator-shall): a whole number, a name, a call or an
/// arithmetic; a truth value, comparison, text, list, float or type is no whole number, where they are and/or
pub(crate) fn may_be_whole_number(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(number) => matches!(number, Number::Int(_) | Number::BigInt(_)),
		Node::Symbol(name) => !is_kinded_type_word(node) && !name.starts_with(char::is_uppercase),
		Node::Key(_, op, _) => !(op.is_comparison() || op.is_logical() || *op == Op::Not),
		// a call `f(4)`, never `[1 2]` or the type-headed `text { … }`
		Node::List(items, bracket, _) => *bracket != Bracket::Square && items.first().is_none_or(may_be_whole_number),
		_ => false,
	}
}

/// The name a parameter declares: `x`, `x:int`, `x=1`
pub(crate) fn parameter_name(parameter: &Node) -> Option<String> {
	parameter_head(parameter).symbol_name().map(String::from)
}

/// The word a parameter declares, also when it is no symbol
pub(crate) fn parameter_word(parameter: &Node) -> String {
	parameter_head(parameter).name()
}

/// `x` of `x`, `x:int`, `x:int=1`, `for x` (preposition_calls.rs): the innermost left side
fn parameter_head(parameter: &Node) -> &Node {
	match parameter.drop_meta() {
		Node::Key(name, _, _) => parameter_head(name),
		Node::List(words, Bracket::None, Separator::Space)
			if words.len() == 2 && crate::type_name_matching::is_label_word(&words[0]) =>
		{
			parameter_head(&words[1])
		}
		other => other,
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
	let words =
		middle.iter().chain(std::iter::once(last_word.as_ref())).map(Node::symbol_name).collect::<Option<_>>()?;
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

/// The next number for a temporary that has to differ from all others of the compile
pub(crate) fn fresh_number() -> usize {
	crate::compiler::compile_context::change(|context| {
		context.fresh_number += 1;
		context.fresh_number - 1
	})
}

/// Run a compile whose fresh numbers start from 0 when it is the outermost one, so a program lowers alike every time;
/// a nested compile continues the numbers of the one around it, so their names never meet
pub(crate) fn numbering_fresh_names<R>(compile: impl FnOnce() -> R) -> R {
	let depth = crate::compiler::compile_context::change(|context| {
		if context.compile_depth == 0 {
			context.fresh_number = 0;
		}
		context.compile_depth
	});
	crate::compiler::compile_context::scoped(|context| &mut context.compile_depth, depth + 1, compile)
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
	/// `def (f x) { body }`: keyword, head and body as three words
	KeywordSpaced,
}

/// One view of a function definition in any of its forms (DefinitionForm): what each pass takes apart
#[derive(Clone)]
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
	/// The bracket around head and block of `def f(x) { body }`
	pub block_bracket: Option<&'a Bracket>,
}

impl<'a> FunctionHead<'a> {
	/// The parameters: none for `f()`, the grouped ones of `def f(x, y) {…}`
	pub fn parameters(&self) -> Vec<&'a Node> {
		match self.written_parameters {
			[only] if only.is_nothing() => vec![],
			[group @ Node::List(typed, Bracket::Round, Separator::Space)] if is_typed_parameter(typed) => vec![group],
			// `got(int[] xs)` arrives as `got [int] xs`: one parameter, by its name
			[_, name] if is_typed_parameter(self.written_parameters) => vec![name],
			[Node::List(grouped, Bracket::Round, _)] => grouped.iter().collect(),
			parameters => parameters.iter().collect(),
		}
	}

	/// The fewest and most values it takes: `f(a, b = 2)` (1, 2), `f(a, xs...)` (1, unbounded)
	pub fn arity(&self) -> (usize, usize) {
		let parameters = self.parameters();
		let is_rest = |parameter: &&Node| crate::lowering::variadic::is_rest_parameter(parameter);
		let has_default = |parameter: &&Node| matches!(parameter.drop_meta(), Node::Key(_, Op::Assign, _));
		let required = parameters.iter().filter(|parameter| !has_default(parameter) && !is_rest(parameter)).count();
		match parameters.iter().any(is_rest) {
			true => (required, usize::MAX),
			false => (required, parameters.len()),
		}
	}

	/// `f(x) := body`, `f(x) = body` with a round head and no result type: its operator
	pub fn plain_operator(&self) -> Option<Op> {
		match self.form {
			DefinitionForm::Operator(op) if self.is_untyped_call() => Some(op),
			_ => None,
		}
	}

	/// `f(x) := body`, `def f(x) = body`, `def f(x) { body }`: a call head without a declared result type, in any form
	pub fn is_untyped_call(&self) -> bool {
		self.result.is_none() && is_call_head(self.head)
	}

	/// `f(x)` written without a space before its parenthesis
	pub fn is_glued_head(&self) -> bool {
		matches!(self.head.drop_meta(), Node::List(_, _, Separator::None))
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
			[keyword, definition] if is_function_word(keyword) => keyword_function_head(definition),
			[keyword, head, body] if is_function_word(keyword) => {
				viewed(head, DefinitionForm::KeywordSpaced, None, body, body)
			}
			_ => None,
		},
		_ => None,
	}
}

fn is_function_word(word: &Node) -> bool {
	crate::operators::is_function_keyword(&word.drop_meta().name())
}

fn keyword_function_head(definition: &Node) -> Option<FunctionHead<'_>> {
	match definition.drop_meta() {
		Node::List(parts, bracket, _) if parts.len() == 2 => {
			let definition = viewed(&parts[0], DefinitionForm::KeywordBlock, None, &parts[1], &parts[1])?;
			Some(FunctionHead { block_bracket: Some(bracket), ..definition })
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
		block_bracket: None,
	})
}

/// `(f x): int`, `(f x) as int`, `(f x) -> int`: the head and its result type
fn typed_head(target: &Node) -> (&Node, Option<&Node>) {
	match result_typed(target) {
		Some((head, result)) if is_word_head(head) => (head, Some(result)),
		_ => (target, None),
	}
}

/// `head as T`, `head: T`, `head -> T`: the head and what its result operator holds, the one split of a typed head
/// (declarations::result_typed_head adds the unions `as int or text`)
pub(crate) fn result_typed(target: &Node) -> Option<(&Node, &Node)> {
	match target.drop_meta() {
		Node::Key(head, Op::As | Op::Colon | Op::Arrow, result) => Some((head, result)),
		_ => None,
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

/// Whether some part of the tree, as `Node::visit` walks it (class code, no metadata), is one `wanted` accepts
pub(crate) fn any_part(node: &Node, wanted: impl Fn(&Node) -> bool) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= wanted(part));
	found
}

/// `f a b` taken apart: the word heading the items and the items after it
pub(crate) fn word_headed(items: &[Node]) -> Option<(&str, &[Node])> {
	let (head, rest) = items.split_first()?;
	Some((head.symbol_name()?, rest))
}

/// The name a definition head starts with: `name` of `name` or `(name x)`
pub(crate) fn head_symbol(head: &Node) -> Option<&str> {
	match head.drop_meta() {
		Node::List(items, _, _) => items.first()?.symbol_name(),
		other => other.symbol_name(),
	}
}

/// `word x` or `word(x)`: the one subject the word is applied to
pub(crate) fn word_applied<'a>(node: &'a Node, word: &str) -> Option<&'a Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, _) => match items.as_slice() {
			[head, subject] if head.is_symbol(word) => Some(subject),
			_ => None,
		},
		_ => None,
	}
}

/// `f(a, b)` (round brackets, any separator) taken apart: the word called and its arguments
pub(crate) fn round_call(node: &Node) -> Option<(&str, &[Node])> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) => word_headed(items),
		_ => None,
	}
}

/// The function a list calls: `f(a, b)`, or spaced `f a b`
pub(crate) fn called_name<'a>(items: &'a [Node], bracket: &Bracket, separator: &Separator) -> Option<&'a str> {
	let (name, _) = word_headed(items)?;
	let spaced = *separator == Separator::Space && matches!(bracket, Bracket::None | Bracket::Round);
	(call_name(items, bracket, separator).is_some() || spaced).then_some(name)
}

/// The statements of a template with its placeholders filled in, its match arms in the form the passes read
pub(crate) fn template(text: &str, bindings: &[(&str, &Node)]) -> Vec<Node> {
	let parsed = crate::welcome_forms::lower_match_arms(crate::warp_parser::parse_generated(text));
	let filled = bindings
		.iter()
		.fold(parsed, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, value));
	filled.drop_meta().sequence_statements().to_vec()
}

/// The names of the classes the program declares (`class Point {…}`)
pub(crate) fn declared_class_names(node: &Node) -> std::collections::HashSet<String> {
	let mut names = std::collections::HashSet::new();
	node.visit(&mut |part| {
		if let Node::Type { name, .. } = part {
			names.insert(name.name());
		}
	});
	names
}

/// `receiver.method(arguments)`, a receiver in round brackets `(a)` unwrapped
pub(crate) fn method_call(receiver: Node, method: &str, arguments: Vec<Node>) -> Node {
	let receiver = match receiver.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => items[0].clone(),
		_ => receiver,
	};
	key(receiver, Op::Dot, call(method, arguments))
}

/// `return value`, as the items of a statement
pub(crate) fn is_return(items: &[Node]) -> bool {
	matches!(items, [word, _] if word.is_symbol(crate::words::RETURN_WORD))
}

/// The list with its head kept and each later item rewritten: a call's name stays, its arguments change
pub(crate) fn arguments_rewritten(
	items: Vec<Node>,
	bracket: Bracket,
	separator: Separator,
	mut rewrite: impl FnMut(Node) -> Node,
) -> Node {
	let mut items = items.into_iter();
	let head = items.next();
	Node::List(head.into_iter().chain(items.map(&mut rewrite)).collect(), bracket, separator)
}
