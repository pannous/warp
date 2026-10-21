//! P179 (user, 2026-10-07): a variant without payload stays its symbol (`red`), and is of its sum type:
//! `red is Color` is true, `x is Color` tests the classes of the type and then the variant names.
//! P178: Rust's path `Shape::Circle(r)` of a variant is the variant `Circle(r)`, in a match its type test and binding.

use super::nodes::key;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::VARIANTS_MARK;
use std::collections::HashMap;

/// A sum type's variants: those without payload by name, the classes extending it
#[derive(Default, Clone)]
pub(crate) struct Variants {
	bare: Vec<Node>,
	classes: Vec<String>,
}

impl Variants {
	/// The names of the variants without payload, `Dot` of `enum Shape { Circle(r), Dot }`
	pub(crate) fn bare_names(&self) -> impl Iterator<Item = String> + '_ {
		self.bare.iter().map(|name| name.drop_meta().name())
	}

	/// The names of all variants, those with payload (classes) first, as declared
	pub(crate) fn names(&self) -> Vec<String> {
		self.classes.iter().cloned().chain(self.bare_names()).collect()
	}

	/// No variant carries a payload: a plain enum, whose case at an index is `n as Coin` (plain_enums.rs)
	pub(crate) fn is_plain(&self) -> bool {
		self.classes.is_empty()
	}
}

/// The sum type's declarations: the type itself, its variants without payload stay symbols, members of the type by name
/// (`red is Color`, P179), then the classes of the variants with payload
pub(crate) fn sum_type(type_name: String, bare_variants: Vec<Node>, declarations: Vec<Node>) -> Node {
	let name = Node::Symbol(type_name)
		.with_attribute(VARIANTS_MARK, Node::List(bare_variants, Bracket::None, Separator::Space));
	let sum_type =
		Node::Type { name: Box::new(name), body: Box::new(Node::List(vec![], Bracket::Curly, Separator::None)) };
	Node::List([vec![sum_type], declarations].concat(), Bracket::None, Separator::Semicolon)
}

/// The cases of an enum body, a line of several cases `Add, Sub, Mul` among lines of cases each a case
pub(crate) fn case_rows(cases: &[Node]) -> Vec<Node> {
	cases
		.iter()
		.flat_map(|case| match case.drop_meta() {
			Node::List(row, Bracket::None, Separator::Comma) => row.clone(),
			_ => vec![case.clone()],
		})
		.collect()
}

/// `type @variants(…) Color`, an enum's sum type
pub(crate) fn is_sum_type(statement: &Node) -> bool {
	matches!(statement.drop_meta(), Node::Type { name, .. } if name.attribute(VARIANTS_MARK).is_some())
}

/// The program with the sum types these declarations declare lowered in it, those of a used module
pub(crate) fn with_types_of(program: Node, declarations: &[Node]) -> Node {
	let sum_types = sum_types(&Node::List(declarations.to_vec(), Bracket::None, Separator::Newline));
	match sum_types.is_empty() {
		true => program,
		false => with_variant_tests(program, &sum_types),
	}
}

pub fn lower(program: Node) -> Node {
	let mut sum_types = sum_types(&program);
	if sum_types.is_empty() {
		return program;
	}
	// `type Paint = Color`: `c is Paint` tests the variants of Color too
	for (alias, target) in super::type_aliases::resolved_aliases(&program) {
		if let Some(variants) = sum_types.get(&target).cloned() {
			sum_types.insert(alias, variants);
		}
	}
	with_variant_tests(program, &sum_types)
}

/// The sum types the program declares by name, `enum Shape { Circle(r), Dot }` read as `type @variants(Dot) Shape`
/// and the classes extending it
pub(crate) fn sum_types(program: &Node) -> HashMap<String, Variants> {
	let mut sum_types: HashMap<String, Variants> = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			if let Some(Node::List(names, _, _)) = name.attribute(VARIANTS_MARK).map(Node::drop_meta) {
				sum_types.entry(name.drop_meta().name()).or_default().bare = names.clone();
			}
		}
	});
	if sum_types.is_empty() {
		return sum_types;
	}
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			let parent = name.attribute(crate::warp_parser::EXTENDS_KEYWORD).map(|parent| parent.drop_meta().name());
			if let Some(variants) = parent.and_then(|parent| sum_types.get_mut(&parent)) {
				variants.classes.push(name.drop_meta().name());
			}
		}
	});
	sum_types
}

/// `x is Color` → `x is Color or x == red or x == green`; a literal variant `red is Color` is true; `Shape::Circle(2)` and
/// `Shape.Circle(2)` → `Circle(2)`
fn with_variant_tests(node: Node, sum_types: &HashMap<String, Variants>) -> Node {
	let bare_of = |right: &Node| match right.drop_meta() {
		Node::Symbol(type_name) if !crate::type_tests::is_equality_operand(right) => {
			sum_types.get(type_name).map(|variants| &variants.bare).filter(|names| !names.is_empty())
		}
		_ => None,
	};
	match node {
		Node::Key(sum_type, Op::Scope | Op::Dot, variant) if is_variant_of(&sum_type, &variant, sum_types) => {
			with_variant_tests(*variant, sum_types)
		}
		// `Node.Empty == x`: the variant by name first, so the comparison below sees it
		Node::Key(left, Op::Eq, right) if is_qualified_variant(&left, sum_types) => {
			with_variant_tests(Node::Key(Box::new(with_variant_tests(*left, sum_types)), Op::Eq, right), sum_types)
		}
		Node::Key(subject, Op::Eq, right)
			if bare_of(&right).is_some() && matches!(subject.drop_meta(), Node::Symbol(_)) =>
		{
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
		// `Dot == x`: a variant is a value on either side, so it compares from the right, where an unbound name on the
		// left would be taken for a definition written with `is` (P61); two variants compare by name
		Node::Key(variant, Op::Eq, other) if is_bare_variant(&variant, sum_types) => {
			let other = with_variant_tests(*other, sum_types);
			// `Dot == Shape`: a value never equals a type, only `is` tests one (user decision #30)
			if crate::type_tests::is_equality_operand(&other) {
				return Node::False;
			}
			match is_bare_variant(&other, sum_types) {
				true if variant.drop_meta() == other.drop_meta() => Node::True,
				true => Node::False,
				false => Node::Key(Box::new(other), Op::Eq, variant),
			}
		}
		other => other.map_children(|child| with_variant_tests(child, sum_types)),
	}
}

fn is_bare_variant(node: &Node, sum_types: &HashMap<String, Variants>) -> bool {
	matches!(node.drop_meta(), Node::Symbol(_))
		&& sum_types.values().any(|variants| variants.bare.iter().any(|bare| bare.drop_meta() == node.drop_meta()))
}

/// `Shape::Circle(2)`, `Shape::Dot`: a variant of the sum type Shape, by its name or the head of its call
fn is_qualified_variant(node: &Node, sum_types: &HashMap<String, Variants>) -> bool {
	matches!(node.drop_meta(), Node::Key(sum_type, Op::Scope | Op::Dot, variant) if is_variant_of(sum_type, variant, sum_types))
}

fn is_variant_of(sum_type: &Node, variant: &Node, sum_types: &HashMap<String, Variants>) -> bool {
	let Some(variants) = sum_types.get(&sum_type.drop_meta().name()) else {
		return false;
	};
	let name = match variant.drop_meta() {
		Node::List(items, _, _) => items.first().map(|head| head.drop_meta().name()).unwrap_or_default(),
		other => other.name(),
	};
	variants.classes.contains(&name) || variants.bare.iter().any(|bare| bare.drop_meta().name() == name)
}
