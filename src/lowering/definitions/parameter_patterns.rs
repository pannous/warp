//! Patterns in parameters (card parameter-patterns): a parameter may be a pattern, as a match arm is.
//! `add_pair([a, b]) := a + b` takes a list of two and names its items; `sum(Point{x, y}) := x + y` takes a Point and
//! names its fields, `greet({name}) := "hi " + name` takes a map with a field name (card param-map-pattern). Each
//! pattern becomes a parameter of its own name, declared of the class for a class pattern, and the body a match on it:
//! `sum(point·1: Point) := match point·1 { Point{x, y}: x + y }`. A value of another shape is the match's "no case"
//! error.

use super::nodes::{arguments_rewritten, is_call_head, key, matched};
use super::switch::is_map_pattern;
use crate::node::{Bracket, Node};
use crate::operators::Op;

/// The name of a list pattern's parameter, numbered by its pattern
const LIST_PARAMETER: &str = "list";
const MAP_PARAMETER: &str = "map";
const NUMBER_MARK: &str = "·";

pub fn lower(node: Node) -> Node {
	match node.map_children(lower) {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if is_call_head(&head) => {
			let mut patterns = vec![];
			let named_head = parameters_named(head.as_ref().clone(), &mut patterns);
			if patterns.is_empty() {
				return Node::Key(head, op, body);
			}
			let body = patterns
				.into_iter()
				.rev()
				.fold(*body, |body, (name, pattern)| matched(Node::Symbol(name), vec![key(pattern, Op::Colon, body)]));
			key(named_head, op, body)
		}
		other => other,
	}
}

/// The head `f(…)` with each pattern parameter named, the patterns collected with their names
fn parameters_named(head: Node, patterns: &mut Vec<(String, Node)>) -> Node {
	let Node::List(items, bracket, separator) = head.drop_meta().clone() else {
		return head;
	};
	arguments_rewritten(items, bracket, separator, |parameter| parameter_named(parameter, patterns))
}

fn parameter_named(parameter: Node, patterns: &mut Vec<(String, Node)>) -> Node {
	match parameter.drop_meta() {
		// `f(n, [a, b])` may hold its parameters as one comma list
		Node::List(items, Bracket::None, separator) => Node::List(
			items.iter().map(|item| parameter_named(item.clone(), patterns)).collect(),
			Bracket::None,
			separator.clone(),
		),
		Node::List(_, Bracket::Square, _) => Node::Symbol(named(LIST_PARAMETER, parameter, patterns)),
		_ if is_map_pattern(&parameter) => Node::Symbol(named(MAP_PARAMETER, parameter, patterns)),
		Node::Key(class, Op::None, fields) if matches!(fields.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			let class = class.drop_meta().clone();
			let name = named(&class.name().to_lowercase(), parameter, patterns);
			key(Node::Symbol(name), Op::Colon, class)
		}
		_ => parameter,
	}
}

/// The name a pattern is passed by, numbered so that it clashes with no name of the program
fn named(word: &str, pattern: Node, patterns: &mut Vec<(String, Node)>) -> String {
	let name = format!("{word}{NUMBER_MARK}{}", patterns.len() + 1);
	patterns.push((name.clone(), pattern));
	name
}
