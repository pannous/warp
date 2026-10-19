//! Function heads: result types (`-> int`, `as int or text`, curried, function and tuple results), parameter labels and types, Ruby's `end` and bodiless definitions
use super::*;
use crate::lowering::nodes::{assign, is_call_head, result_typed, round_call, word_headed};
use crate::node::parens;
use crate::words::ANY_TYPE;

/// `f(x) -> int { body }`, `f(x): int { body }` (Rust/Swift/Python, TypeScript/Kotlin) and `f(x) -> int: body`
/// (Python): the definition `(f(x) { body })` without its result type, and the type
pub(crate) fn typed_result(definition: &Node) -> Option<(Node, Node)> {
	let (head, typed_body) = result_typed_head(definition).filter(|(head, _)| is_call_head(head))?;
	let (result_type, body) = match typed_body.drop_meta() {
		Node::List(parts, _, Separator::Space)
			if parts.len() == 2 && is_result_type(&parts[0]) && parts[1].is_block() =>
		{
			(parts[0].clone(), parts[1].clone())
		}
		Node::Key(result_type, Op::Colon, body) if is_result_type(result_type) => {
			(result_type.as_ref().clone(), body.as_ref().clone())
		}
		union @ Node::Key(_, Op::Or, _) => {
			let (union, body) = union_result(union)?;
			// named as lower_inline_unions names a union, which has run; the analyzer refuses a part naming no type
			(Node::Symbol(crate::analyzer::union_of_types(&union, &|_| true)?), body)
		}
		_ => return None,
	};
	Some((key(head, Op::Define, body), result_type))
}

/// `f(x) as int`, `f(x): int`, `f(x) -> int` and `f(x) as int or text`: the head and what its result operator holds,
/// the one reader of a typed definition head (nodes::result_typed borrows the forms without a union)
pub(crate) fn result_typed_head(target: &Node) -> Option<(Node, Node)> {
	as_union_result(target).or_else(|| result_typed(target).map(|(head, result)| (head.clone(), result.clone())))
}

/// A built-in type, a collection, a class (`def g(): G {…}`, card def-int), or `any`: a Node of whatever kind the
/// paths return (`def f(w) as any`, card def)
fn is_result_type(result_type: &Node) -> bool {
	result_type.symbol_name().is_some_and(crate::warp_parser::names_a_type) || result_type.is_symbol(ANY_TYPE)
}

/// `f(x) as int or text` parses as `(f(x) as int) or text`: the head and its result type `int or text`, which `as`
/// spells canonically (user 2026-10-12, return-as)
fn as_union_result(definition: &Node) -> Option<(Node, Node)> {
	let Node::Key(left, Op::Or, rest) = definition.drop_meta() else {
		return None;
	};
	let (head, first) = match left.drop_meta() {
		Node::Key(head, Op::As, first) => (head.as_ref().clone(), first.as_ref().clone()),
		left => as_union_result(left)?,
	};
	Some((head, key(first, Op::Or, rest.as_ref().clone())))
}

/// `def f(x) -> int` keeps working; warp writes a result type with `as`, unions too (user 2026-10-12, return-as)
pub(super) fn hint_as_result(definition: &Node, result_type: &Node) {
	crate::normalize::set_position_of(definition);
	let written = result_type.serialize();
	crate::normalize::hint(
		&format!("-> {written}"),
		&format!("as {written}"),
		&format!("prefer `def f(x) as {written}`"),
	);
}

/// `-> int or text { body }` arrives as `int or (text { body })`: the parts `int or text` and the body
pub(super) fn union_result(typed_body: &Node) -> Option<(Node, Node)> {
	match typed_body.drop_meta() {
		Node::Key(part, Op::Or, rest) => {
			let (rest, body) = union_result(rest)?;
			Some((key(part.as_ref().clone(), Op::Or, rest), body))
		}
		Node::List(parts, _, Separator::Space)
			if parts.len() == 2 && matches!(parts[0].drop_meta(), Node::Symbol(_)) && parts[1].is_block() =>
		{
			Some((parts[0].clone(), parts[1].clone()))
		}
		_ => None,
	}
}

/// Scala's curried `def add(x: Int)(y: Int): Int = x + y`: `add(x: Int) := (y: Int) => x + y`, so `add(1)(2)` calls the
/// function add(1) returns
pub(super) fn curried_definition(head: &Node, rest: &Node) -> Option<Node> {
	let Node::Key(second, Op::Assign, body) = rest.drop_meta() else {
		return None;
	};
	let second = match second.drop_meta() {
		Node::Key(group, Op::Colon, result_type) if is_kinded_type_word(result_type) => group.as_ref(),
		other => other,
	};
	let Node::List(_, Bracket::Round, _) = second.drop_meta() else {
		return None;
	};
	round_call(head)?;
	let lambda = Node::Key(Box::new(second.clone()), Op::Arrow, body.clone());
	Some(key(head.clone(), Op::Define, lambda))
}

/// The glued groups `add(x)(y)` arrive as one operand (the parser's curried call): `def add(x)(y) = x + y` and
/// `def add(x: Int)(y: Int): Int = …` as curried_definition takes them
pub(super) fn glued_curried_definition(definition: &Node) -> Option<Node> {
	let Node::Key(left, Op::Assign, body) = definition.drop_meta() else {
		return None;
	};
	let groups = |node: &Node| match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 => {
			Some((items[0].clone(), items[1].clone()))
		}
		_ => None,
	};
	let (head, second) = match left.drop_meta() {
		Node::Key(signature, Op::Colon, result_type) => {
			let (head, second) = groups(signature)?;
			(head, Node::Key(Box::new(second), Op::Colon, result_type.clone()))
		}
		other => groups(other)?,
	};
	curried_definition(&head, &Node::Key(Box::new(second), Op::Assign, body.clone()))
}

/// Swift's `f(k) -> (Int) -> Int { body }`, a function as the result: the definition `f(k) { body }`, the closure it
/// returns carries its own types
pub(super) fn function_typed_result(definition: Node) -> Node {
	let Node::Key(head, Op::Arrow, result) = &definition else {
		return definition;
	};
	let Node::Key(parameter_types, Op::Arrow, typed_body) = result.drop_meta() else {
		return definition;
	};
	let Node::List(types, Bracket::Round, _) = parameter_types.drop_meta() else {
		return definition;
	};
	let Node::List(parts, _, Separator::Space) = typed_body.drop_meta() else {
		return definition;
	};
	match parts.as_slice() {
		[result_type, body]
			if types.iter().all(is_kinded_type_word) && is_kinded_type_word(result_type) && body.is_block() =>
		{
			parens(vec![head.as_ref().clone(), body.clone()])
		}
		_ => definition,
	}
}

/// `f(a, b) -> (Int, Int)`: the head of a function whose result type is a tuple of types
pub(super) fn tuple_result(head: &Node) -> Option<&Node> {
	let Node::Key(head, Op::Arrow, result) = head.drop_meta() else {
		return None;
	};
	is_type_tuple(result).then_some(head.as_ref())
}

/// `return a / b, a % b` of a function returning `(int, int)`: `return (a / b) as int, (a % b) as int`
pub(super) fn typed_returns(body: Node, result_types: &Node) -> Node {
	let Node::List(types, _, _) = result_types.drop_meta() else {
		return body;
	};
	let converted = |values: &[Node]| -> Option<Vec<Node>> {
		(values.len() == types.len()).then(|| {
			values
				.iter()
				.zip(types)
				.map(|(value, kind)| {
					let grouped = parens(vec![value.clone()]);
					key(grouped, Op::As, kind.clone())
				})
				.collect()
		})
	};
	let is_return = |node: &Node| matches!(node.drop_meta(), Node::List(words, Bracket::None, Separator::Space) if words.len() == 2 && words[0].is_symbol(RETURN_WORD));
	match body {
		// `return a, b` parses as `(return a), b`
		Node::List(items, bracket, Separator::Comma) if items.first().is_some_and(is_return) => {
			let Node::List(words, _, _) = items[0].drop_meta() else { unreachable!("guarded") };
			let values: Vec<Node> = std::iter::once(words[1].clone()).chain(items[1..].iter().cloned()).collect();
			match converted(&values) {
				Some(mut values) => {
					let first = Node::List(vec![words[0].clone(), values.remove(0)], Bracket::None, Separator::Space);
					Node::List([vec![first], values].concat(), bracket, Separator::Comma)
				}
				None => Node::List(items, bracket, Separator::Comma),
			}
		}
		other => other.map_children(|child| typed_returns(child, result_types)),
	}
}

/// `(Int, Int)`
pub(super) fn is_type_tuple(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(types, Bracket::Round, _) if types.len() > 1 && types.iter().all(is_kinded_type_word))
}

/// Swift's `label name: T`: the label (`_` for none)
pub(super) fn argument_label<'a>(label: &'a Node, typed: &Node) -> Option<&'a str> {
	let label = label.symbol_name()?;
	typed_parameter_name(typed).is_some().then_some(label)
}

/// The name of a typed parameter `name: T`, also of a function type `name: (Int) -> Int`
pub(super) fn typed_parameter_name(typed: &Node) -> Option<&Node> {
	match typed.drop_meta() {
		Node::Key(name, Op::Colon, _) if matches!(name.drop_meta(), Node::Symbol(_)) => Some(name),
		Node::Key(parameter, Op::Arrow, _) => typed_parameter_name(parameter),
		_ => None,
	}
}

/// The typed parameter under another name: `name: T` → `label: T`
pub(super) fn renamed_parameter(typed: &Node, label: &str) -> Node {
	match typed.drop_meta() {
		Node::Key(_, Op::Colon, type_node) => Node::Key(Box::new(symbol(label)), Op::Colon, type_node.clone()),
		Node::Key(parameter, op, result) => {
			Node::Key(Box::new(renamed_parameter(parameter, label)), *op, result.clone())
		}
		other => other.clone(),
	}
}

/// `person name: String` is the parameter `person: String` whose value the body reads as name (`labeled_names` gets
/// `name = person`); `_ name: String` is `name: String`
pub(super) fn labeled_parameter(parameter: Node, labeled_names: &mut Vec<Node>) -> Node {
	let Node::List(words, _, _) = parameter.drop_meta() else {
		return parameter;
	};
	let [label, typed] = words.as_slice() else {
		return parameter;
	};
	let Some(label) = argument_label(label, typed) else {
		return parameter;
	};
	let name = typed_parameter_name(typed).expect("a labeled parameter has a name");
	// user 2026-10-06: warp names a parameter once, Swift's label and name are redundant; ported code still compiles
	let written = format!("{label} {}", typed.serialize());
	let preferred = if label == WILDCARD_LABEL { typed.clone() } else { renamed_parameter(typed, label) };
	crate::normalize::set_position_of(&parameter);
	crate::normalize::hint(&written, &preferred.serialize(), "warp names a parameter once, no label");
	if label == WILDCARD_LABEL {
		return preferred;
	}
	labeled_names.push(assign(name.clone(), symbol(label)));
	preferred
}

/// The body reading every labeled parameter by its label: `name` → `person` (an alias `name = person` would hide that
/// the parameter is a function, `f(x)` of `using f: (Int) -> Int`)
pub(super) fn with_label_names(body: Node, labeled_names: Vec<Node>) -> Node {
	let bindings: std::collections::HashMap<String, Node> = labeled_names
		.into_iter()
		.filter_map(|binding| match binding {
			Node::Key(name, Op::Assign, label) => Some((name.name(), *label)),
			_ => None,
		})
		.collect();
	if bindings.is_empty() {
		body
	} else {
		crate::law::substitute(&body, &bindings)
	}
}

/// Go's parameter `x int` (the name, then its type) is `x:int`
pub(super) fn name_then_type(parameter: Node) -> Node {
	let is_function_type = |node: &Node| matches!(node.drop_meta(), Node::List(words, _, _) if words.first().and_then(Node::symbol_name).is_some_and(is_function_keyword));
	match parameter.drop_meta() {
		// Go's function parameter `f func(int) int`: the name, a function (its calls tell the rest)
		Node::List(parts, _, Separator::Space)
			if parts.len() >= 2 && matches!(parts[0].drop_meta(), Node::Symbol(_)) && is_function_type(&parts[1]) =>
		{
			parts[0].clone()
		}
		Node::List(parts, _, Separator::Space)
			if parts.len() == 2
				&& matches!(parts[0].drop_meta(), Node::Symbol(_))
				&& is_kinded_type_word(&parts[1]) =>
		{
			key(parts[0].clone(), Op::Colon, parts[1].clone())
		}
		_ => parameter,
	}
}

/// Ruby's `def f(a, b: 2) a * b end` and `def f(a)` ⏎ statements ⏎ `end`: the definition `def f(a, b: 2) {…}`
pub(super) fn end_definitions(node: Node) -> Node {
	let node = children_rewritten(node, end_definitions);
	let Node::List(items, bracket, separator) = node else {
		return node;
	};
	// one line: `def head body… end`
	if let ([keyword, head, body @ .., end], Separator::Space) = (items.as_slice(), &separator) {
		if is_function_keyword(&keyword.drop_meta().name()) && is_end(end) && !body.is_empty() {
			return Node::List(vec![keyword.clone(), function_head(head), block(body.to_vec())], bracket, separator);
		}
	}
	if !matches!(separator, Separator::Newline | Separator::Semicolon) || !items.iter().any(is_end) {
		return Node::List(items, bracket, separator);
	}
	// statements: `def head`, the body's statements, `end`
	let mut statements: Vec<Node> = vec![];
	let mut rest = items.into_iter();
	while let Some(statement) = rest.next() {
		let Some((keyword, head)) = bodiless_definition(&statement) else {
			statements.push(statement);
			continue;
		};
		let body = block(rest.by_ref().take_while(|statement| !is_end(statement)).collect());
		let definition = match head.drop_meta() {
			// Crystal's `def f(a) : Int`: the head as Kotlin's `fun f(a): Int { … }` has it
			Node::Key(inner, Op::Colon, result) => {
				let typed_body = Node::List(vec![result.as_ref().clone(), body], Bracket::None, Separator::Space);
				vec![keyword, key(function_head(inner), Op::Colon, typed_body)]
			}
			_ => vec![keyword, function_head(&head), body],
		};
		statements.push(Node::List(definition, Bracket::None, Separator::Space));
	}
	Node::List(statements, bracket, separator)
}

pub(super) fn is_end(node: &Node) -> bool {
	node.is_symbol(END_WORD)
}

/// `def f(a)` or `def h` without a body, also with Crystal's result type `def f(a) : Int`
pub(super) fn bodiless_definition(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else {
		return None;
	};
	let [keyword, head] = items.as_slice() else { return None };
	let is_head = |head: &Node| {
		matches!(head.drop_meta(), Node::Symbol(_))
			|| matches!(head.drop_meta(), Node::List(parts, Bracket::Round, _) if word_headed(parts).is_some())
	};
	let is_typed_head = matches!(head.drop_meta(), Node::Key(inner, Op::Colon, _) if is_head(inner));
	(is_function_keyword(&keyword.drop_meta().name()) && (is_head(head) || is_typed_head))
		.then(|| (keyword.clone(), head.clone()))
}

/// `h` → `h()`; `(f a b:2)` → `(f (a, b:2))`, the parameters as one comma group as `def f(a, b: 2) {…}` has them (two
/// loose words would read as Swift's labeled parameter)
pub(super) fn function_head(head: &Node) -> Node {
	match head.drop_meta() {
		Node::Symbol(_) => parens(vec![head.clone()]),
		Node::List(items, Bracket::Round, separator) if items.len() > 2 => {
			let parameters = Node::List(items[1..].to_vec(), Bracket::Round, Separator::Comma);
			Node::List(vec![items[0].clone(), parameters], Bracket::Round, separator.clone())
		}
		_ => head.clone(),
	}
}
