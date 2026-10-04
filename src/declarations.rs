//! `enum color {red green blue}` declares the object `color={red:0 green:1 blue:2}`: a case is its index, `color.green` is 1.
//! `real f(real x, int n) { … }`, the C way, defines `f(x:real, n:int) := { … }`.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const ENUM_WORD: &str = "enum";
const FIRST_CASE_INDEX: i64 = 0;

pub fn lower(node: Node) -> Node {
	lower_lists(node, enum_object)
}

/// C definitions, before any pass reads `real f(…)` as a conversion of a call; and keyword definitions
/// (`def f(x) {…}`, `fun`, `function`) as `f(x) := {…}`, so every pass reads one definition form
pub fn lower_c_functions(node: Node) -> Node {
	lower_lists(node, |items| c_function(items).or_else(|| keyword_definition(items)))
}

/// `def f(a, b) { body }`, `def f(x) := body`, `function g() { … }`: the definition `f(a, b) := body`
fn keyword_definition(items: &[Node]) -> Option<Node> {
	let (keyword, definition) = match items {
		[keyword, definition] => (keyword, definition.drop_meta().clone()),
		[keyword, head, body] => (keyword, Node::List(vec![head.clone(), body.clone()], Bracket::Round, Separator::None)),
		_ => return None,
	};
	if !matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)) {
		return None;
	}
	let (head, op, body) = match definition {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) => (*head, op, *body),
		Node::List(parts, Bracket::Round, _) if parts.len() == 2 && matches!(parts[1].drop_meta(), Node::List(_, Bracket::Curly, _)) => (parts[0].clone(), Op::Define, parts[1].clone()),
		_ => return None,
	};
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	// the parameters may come as one group: `f (a, b)`, `f (m)`, `f ø`
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![argument.clone()],
	});
	let head = Node::List(std::iter::once(name.clone()).chain(parameters).collect(), Bracket::Round, Separator::None);
	Some(Node::Key(Box::new(head), op, Box::new(body)))
}

fn lower_lists(node: Node, lowering: impl Fn(&[Node]) -> Option<Node> + Copy) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| lower_lists(item, lowering)).collect();
			lowering(&items).unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower_lists(*left, lowering)), op, Box::new(lower_lists(*right, lowering))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_lists(*node, lowering)), data },
		other => other,
	}
}

/// The assignment `name={case:index …}` of the items `enum name {case …}`
fn enum_object(items: &[Node]) -> Option<Node> {
	let [word, name, cases] = items else { return None };
	let is_enum = matches!(word.drop_meta(), Node::Symbol(word) if word == ENUM_WORD);
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let Node::List(case_names, Bracket::Curly, _) = cases.drop_meta() else { return None };
	if !is_enum || case_names.iter().any(|case| !matches!(case.drop_meta(), Node::Symbol(_))) {
		return None;
	}
	let entries = case_names
		.iter()
		.zip(FIRST_CASE_INDEX..)
		.map(|(case, index)| Node::Key(Box::new(case.clone()), Op::Colon, Box::new(Node::int(index))))
		.collect();
	let object = Node::List(entries, Bracket::Curly, Separator::Space);
	Some(Node::Key(Box::new(name.clone()), Op::Assign, Box::new(object)))
}


/// `real f(real x) { … }`: the definition `f(x:real) := { … }` (the parser reads the type word, then the call and its
/// block); the result kind is inferred as for any definition
fn c_function(items: &[Node]) -> Option<Node> {
	let [result_type, definition] = items else { return None };
	let Node::Symbol(result_type) = result_type.drop_meta() else { return None };
	if crate::analyzer::type_word_kind(result_type).is_none() && result_type != "void" {
		return None;
	}
	let Node::List(parts, Bracket::Round, _) = definition.drop_meta() else { return None };
	let [head, body] = parts.as_slice() else { return None };
	let Node::List(_, Bracket::Curly, _) = body.drop_meta() else { return None };
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, Separator::Colon) => group.clone(), // `(real a, int b)`
		_ => vec![argument.clone()],
	});
	let parameters: Option<Vec<Node>> = parameters.map(|parameter| c_parameter(&parameter)).collect();
	let head = Node::List([vec![name.clone()], parameters?].concat(), Bracket::Round, Separator::Colon);
	Some(Node::Key(Box::new(head), Op::Define, Box::new(body.clone())))
}

/// `real x` is `x:real`; a bare name stays untyped
fn c_parameter(parameter: &Node) -> Option<Node> {
	match parameter.drop_meta() {
		Node::Symbol(_) => Some(parameter.clone()),
		Node::List(words, _, Separator::Space) => match words.as_slice() {
			[kind, name] if matches!((kind.drop_meta(), name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) => {
				Some(Node::Key(Box::new(name.clone()), Op::Colon, Box::new(kind.clone())))
			}
			_ => None,
		},
		_ => None,
	}
}
