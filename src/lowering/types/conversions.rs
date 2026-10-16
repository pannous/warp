//! A program's conversions (decision general-conversions): `Person as text := it.name` defines how a Person turns into
//! text, `p as text` then calls it. Each target B gets one function `converted_to_B` with a variant per source type A, and a
//! fallback for every other value that does the built-in `it as B`; the variants dispatch on their parameter's type
//! (traits.rs), so a program's conversions are looked up before the built-in ones. Unit lines are its unit cases
//! (src/numbers/units/definitions.rs).

use crate::lowering::declaration_forms::is_declaration_type;
use crate::node::{call, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::BTreeSet;

/// The name of the conversion to `target`: `converted_to_text` (`·` would read as a trait witness, traits.rs)
fn conversion_name(target: &str) -> String {
	format!("converted_to_{target}")
}

/// The parameter every conversion body reads its value from
const CONVERTED: &str = "it";

pub fn lower(node: Node) -> Node {
	let classes = crate::analyzer::declared_classes(&node);
	let is_type = |node: &Node| {
		is_declaration_type(node) || matches!(node.drop_meta(), Node::Symbol(name) if classes.contains(name))
	};
	let mut targets = BTreeSet::new();
	let node = defined(node, &is_type, &mut targets);
	if targets.is_empty() {
		return node;
	}
	let node = converted(node, &targets);
	// before the program's statements: its last one stays the program's value
	let fallbacks = targets.iter().map(|target| fallback(target));
	match node {
		Node::List(items, Bracket::None, separator) if separator.separates_statements() => {
			Node::List(fallbacks.chain(items).collect(), Bracket::None, separator)
		}
		program => Node::List(fallbacks.chain([program]).collect(), Bracket::None, Separator::Newline),
	}
}

/// Each `A as B := body` as the variant `converted_to_B(it: A) := body`, its target B kept
fn defined(node: Node, is_type: &dyn Fn(&Node) -> bool, targets: &mut BTreeSet<String>) -> Node {
	match node {
		Node::Key(head, Op::Define, body) if let Some((source, target)) = conversion_head(&head, is_type) => {
			if source.symbol_name() == Some(target.as_str()) {
				return crate::node::error(&format!(
					"{target} as {target}: a value is already of its own type, no conversion to define"
				));
			}
			targets.insert(target.clone());
			let parameter = Node::Key(Box::new(Node::Symbol(CONVERTED.into())), Op::Colon, Box::new(source));
			Node::Key(Box::new(call(&conversion_name(&target), vec![parameter])), Op::Define, body)
		}
		other => other.map_children(|child| defined(child, is_type, targets)),
	}
}

/// `Person as text`: the source type and the target's name
fn conversion_head(head: &Node, is_type: &dyn Fn(&Node) -> bool) -> Option<(Node, String)> {
	let Node::Key(source, Op::As, target) = head.drop_meta() else {
		return None;
	};
	let target_name = target.drop_meta().symbol_name()?.to_string();
	(is_type(source) && is_type(target)).then(|| (source.drop_meta().clone(), target_name))
}

/// The words whose `as` names an alias, `use math as m`, not a conversion
const ALIASING_WORDS: [&str; 3] = ["use", "import", "include"];

/// Each `x as B` of a defined target B as the call `converted_to_B(x)`; written declarations (`x as text = …`) and
/// aliases (`use math as text`) stay
fn converted(node: Node, targets: &BTreeSet<String>) -> Node {
	match node {
		Node::List(ref items, ..)
			if items
				.first()
				.and_then(|word| word.drop_meta().symbol_name())
				.is_some_and(|word| ALIASING_WORDS.contains(&word)) =>
		{
			node
		}
		Node::Key(head, op @ (Op::Define | Op::Assign), body) => {
			Node::Key(head, op, Box::new(converted(*body, targets)))
		}
		Node::Key(value, Op::As, ref target) if let Some(name) = defined_target(target, targets) => {
			call(&conversion_name(&name), vec![converted(*value, targets)])
		}
		other => other.map_children(|child| converted(child, targets)),
	}
}

fn defined_target(target: &Node, targets: &BTreeSet<String>) -> Option<String> {
	target.drop_meta().symbol_name().filter(|name| targets.contains(*name)).map(str::to_string)
}

/// `converted_to_B(it) := it as B`: every value without a conversion of the program's converts the built-in way
fn fallback(target: &str) -> Node {
	let converted = Node::Symbol(CONVERTED.into());
	let builtin = Node::Key(Box::new(converted.clone()), Op::As, Box::new(Node::Symbol(target.into())));
	Node::Key(Box::new(call(&conversion_name(target), vec![converted])), Op::Define, Box::new(builtin))
}
