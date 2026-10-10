//! P179 (user, 2026-10-07): a variant without payload stays its symbol (`red`), and is of its sum type:
//! `red is Color` is true, `x is Color` tests the classes of the type and then the variant names.
//! P178: Rust's path `Shape::Circle(r)` of a variant is the variant `Circle(r)`, in a match its type test and binding.

use super::nodes::key;
use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;

/// The parser's attribute on a sum type's name: its variants without payload
pub const VARIANTS_MARK: &str = "variants";

/// A sum type's variants: those without payload by name, the classes extending it
#[derive(Default)]
struct Variants {
	bare: Vec<Node>,
	classes: Vec<String>,
}

pub fn lower(program: Node) -> Node {
	let mut sum_types: HashMap<String, Variants> = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			if let Some(Node::List(names, _, _)) = name.attribute(VARIANTS_MARK).map(Node::drop_meta) {
				sum_types.entry(name.drop_meta().name()).or_default().bare = names.clone();
			}
		}
	});
	if sum_types.is_empty() {
		return program;
	}
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			let parent = name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
			if let Some(variants) = parent.and_then(|parent| sum_types.get_mut(&parent)) {
				variants.classes.push(name.drop_meta().name());
			}
		}
	});
	with_variant_tests(program, &sum_types)
}

/// `x is Color` → `x is Color or x == red or x == green`; a literal variant `red is Color` is true; `Shape::Circle(2)` →
/// `Circle(2)`
fn with_variant_tests(node: Node, sum_types: &HashMap<String, Variants>) -> Node {
	let bare_of = |right: &Node| match right.drop_meta() {
		Node::Symbol(type_name) if !crate::type_tests::is_equality_operand(right) => sum_types.get(type_name).map(|variants| &variants.bare).filter(|names| !names.is_empty()),
		_ => None,
	};
	match node {
		Node::Key(sum_type, Op::Scope, variant) if is_variant_of(&sum_type, &variant, sum_types) => with_variant_tests(*variant, sum_types),
		Node::Key(subject, Op::Eq, right) if bare_of(&right).is_some() && matches!(subject.drop_meta(), Node::Symbol(_)) => {
			let names = bare_of(&right).expect("guarded");
			if names.iter().any(|name| name.drop_meta() == subject.drop_meta()) {
				return Node::True;
			}
			let class_test = Node::Key(subject.clone(), Op::Eq, right);
			names.iter().fold(class_test, |test, name| {
				let equals = Node::Key(subject.clone(), Op::Eq, Box::new(name.clone()));
				key(test, Op::Or, equals)
			})
		}
		other => other.map_children(|child| with_variant_tests(child, sum_types)),
	}
}

/// `Shape::Circle(2)`, `Shape::Dot`: a variant of the sum type Shape, by its name or the head of its call
fn is_variant_of(sum_type: &Node, variant: &Node, sum_types: &HashMap<String, Variants>) -> bool {
	let Some(variants) = sum_types.get(&sum_type.drop_meta().name()) else { return false };
	let name = match variant.drop_meta() {
		Node::List(items, _, _) => items.first().map(|head| head.drop_meta().name()).unwrap_or_default(),
		other => other.name(),
	};
	variants.classes.contains(&name) || variants.bare.iter().any(|bare| bare.drop_meta().name() == name)
}
