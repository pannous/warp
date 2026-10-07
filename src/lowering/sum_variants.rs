//! P179 (user, 2026-10-07): a variant without payload stays its symbol (`red`), and is of its sum type:
//! `red is Color` is true, `x is Color` tests the classes of the type and then the variant names.

use crate::node::Node;
use crate::operators::Op;
use std::collections::HashMap;

/// The parser's attribute on a sum type's name: its variants without payload
pub const VARIANTS_MARK: &str = "variants";

pub fn lower(program: Node) -> Node {
	let mut variants: HashMap<String, Vec<Node>> = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			if let Some(Node::List(names, _, _)) = name.attribute(VARIANTS_MARK).map(Node::drop_meta) {
				variants.insert(name.drop_meta().name(), names.clone());
			}
		}
	});
	if variants.values().all(Vec::is_empty) {
		return program;
	}
	with_variant_tests(program, &variants)
}

/// `x is Color` → `x is Color or x == red or x == green`; a literal variant `red is Color` is true
fn with_variant_tests(node: Node, variants: &HashMap<String, Vec<Node>>) -> Node {
	let names_of = |right: &Node| match right.drop_meta() {
		Node::Symbol(type_name) if !crate::type_tests::is_equality_operand(right) => variants.get(type_name).filter(|names| !names.is_empty()),
		_ => None,
	};
	match node {
		Node::Key(subject, Op::Eq, right) if names_of(&right).is_some() && matches!(subject.drop_meta(), Node::Symbol(_)) => {
			let names = names_of(&right).expect("guarded");
			if names.iter().any(|name| name.drop_meta() == subject.drop_meta()) {
				return Node::True;
			}
			let class_test = Node::Key(subject.clone(), Op::Eq, right);
			names.iter().fold(class_test, |test, name| {
				let equals = Node::Key(subject.clone(), Op::Eq, Box::new(name.clone()));
				Node::Key(Box::new(test), Op::Or, Box::new(equals))
			})
		}
		other => other.map_children(|child| with_variant_tests(child, variants)),
	}
}
