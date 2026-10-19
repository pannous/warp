//! A parameter of type Type (card random-should, decision random-forms): `f(t: type) := t == int` takes the type
//! itself, so `f int`, `f(text)` and `f Circle` pass the type value a bare type word is, never its constructor's call. The
//! type value is a node at run time, held as `any` holds it; a literal argument that is no type is a loud error.
//! A type variable of a generic signature (decision compiler-typed, card generic-signatures) is held as `any` too:
//! `id(x: T): T`, `first(xs: list of T): T`, `pairs(m: map of K): list`.

use super::memoization::definition_parts;
use super::nodes::{called_name, children_rewritten, function_head, key};
use crate::node::Node;
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The annotations naming the type of types: `t: type`, `t: Type`
const TYPE_ANNOTATIONS: [&str; 2] = [crate::type_tests::TYPE_WORD, "Type"];

pub fn lower(node: Node) -> Node {
	let typed = type_parameter_positions(&node);
	if typed.is_empty() {
		return node;
	}
	match misplaced_argument(&node, &typed) {
		Some(error) => error,
		None => held_as_any(node),
	}
}

/// `x: T`, `xs: list of T`, `xs: [T]`, `: T` of a definition's head is `x: any`, `xs: list`, `: any`
pub fn lower_type_variables(node: Node) -> Node {
	let declared = crate::modules::needed_definitions::class_names(&node);
	general(node, &|annotation| is_type_variable(annotation, &declared))
}

/// A single capital letter no class or type of the program declares: `T`, `K`, `V`
fn is_type_variable(annotation: &Node, declared: &HashSet<String>) -> bool {
	annotation.symbol_name().is_some_and(|name| {
		let mut letters = name.chars();
		matches!((letters.next(), letters.next()), (Some(letter), None) if letter.is_ascii_uppercase())
			&& !declared.contains(name)
	})
}

fn general(node: Node, is_variable: &dyn Fn(&Node) -> bool) -> Node {
	if let Some((head, body, rebuild)) = definition_parts(&node) {
		return rebuild(annotated_any(head, is_variable), general(body, is_variable));
	}
	children_rewritten(node, |child| general(child, is_variable))
}

fn is_type_annotation(node: &Node) -> bool {
	TYPE_ANNOTATIONS.iter().any(|word| node.is_symbol(word))
}

/// The positions of the parameters declared of type Type, by function name
fn type_parameter_positions(node: &Node) -> HashMap<String, Vec<usize>> {
	let mut typed = HashMap::new();
	node.visit(&mut |part| {
		let Some(head) = function_head(part) else { return };
		let positions: Vec<usize> = (head.parameters().into_iter().enumerate())
			.filter(|(_, parameter)| matches!(parameter.drop_meta(), Node::Key(_, Op::Colon, annotation) if is_type_annotation(annotation)))
			.map(|(position, _)| position)
			.collect();
		if !positions.is_empty() {
			typed.insert(head.name.to_string(), positions);
		}
	});
	typed
}

/// `f 3` of `f(t: type)`: the error naming what a type parameter takes
fn misplaced_argument(node: &Node, typed: &HashMap<String, Vec<usize>>) -> Option<Node> {
	let mut error = None;
	node.visit(&mut |part| {
		let Node::List(items, bracket, separator) = part.drop_meta() else { return };
		let Some(positions) = called_name(items, bracket, separator).and_then(|name| typed.get(name)) else { return };
		let literal =
			positions.iter().filter_map(|position| items.get(position + 1)).find(|argument| is_literal_value(argument));
		if let (None, Some(argument)) = (&error, literal) {
			error = Some(crate::node::error(&format!(
				"{}: the parameter of type type takes a type like int, text or a class name, not the value {}",
				items[0].drop_meta().name(),
				argument.drop_meta().serialize()
			)));
		}
	});
	error
}

fn is_literal_value(argument: &Node) -> bool {
	matches!(argument.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_))
}

/// `t: type` in a function head is `t: any`: the type value passes as the node it is
fn held_as_any(node: Node) -> Node {
	if let Some((head, body, rebuild)) = definition_parts(&node) {
		return rebuild(annotated_any(head, &is_type_annotation), held_as_any(body));
	}
	children_rewritten(node, held_as_any)
}

fn annotated_any(head: Node, is_held_as_any: &dyn Fn(&Node) -> bool) -> Node {
	let any = || Node::Symbol(crate::words::ANY_TYPE.to_string());
	match head {
		// also a result, `(f x:T): T`, whose head holds the parameters
		Node::Key(parameter, Op::Colon, annotation) if is_held_as_any(&annotation) => {
			key(annotated_any(*parameter, is_held_as_any), Op::Colon, any())
		}
		// `xs: list of T`, `m: map of K` as one type name: the collection of any elements
		Node::Key(parameter, Op::Colon, annotation)
			if collection_of_variable(&annotation, is_held_as_any).is_some() =>
		{
			let collection = collection_of_variable(&annotation, is_held_as_any).expect("guarded");
			key(annotated_any(*parameter, is_held_as_any), Op::Colon, Node::Symbol(collection))
		}
		// `xs: [T]`: a list of any elements
		Node::Key(parameter, Op::Colon, annotation) if matches!(annotation.drop_meta(), Node::List(items, crate::node::Bracket::Square, _) if matches!(items.as_slice(), [element] if is_held_as_any(element))) => {
			key(*parameter, Op::Colon, Node::Symbol(crate::words::LIST_WORD.to_string()))
		}
		// `xs: list of T` as the parser keeps it in a head, the items `xs: list`, `of`, `T`: of any elements
		Node::List(items, bracket, separator) => {
			let mut kept = vec![];
			let mut items = items.into_iter().peekable();
			while let Some(item) = items.next() {
				if item.is_symbol(crate::words::OF_WORD) && items.peek().is_some_and(is_held_as_any) {
					items.next();
					continue;
				}
				kept.push(annotated_any(item, is_held_as_any));
			}
			Node::List(kept, bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(annotated_any(*node, is_held_as_any)), data },
		other => children_rewritten(other, |child| annotated_any(child, is_held_as_any)),
	}
}

/// `list` of the type name `list of T`, `map` of `map of K`: a collection of a type variable's elements
fn collection_of_variable(annotation: &Node, is_held_as_any: &dyn Fn(&Node) -> bool) -> Option<String> {
	let name = annotation.symbol_name()?;
	let (collection, element) = name.split_once(&format!(" {} ", crate::words::OF_WORD))?;
	is_held_as_any(&Node::Symbol(element.to_string())).then(|| collection.to_string())
}
