//! `enum color {red green blue}` declares the object `color={red:0 green:1 blue:2}`: a case is its index, `color.green` is 1.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const ENUM_WORD: &str = "enum";
const FIRST_CASE_INDEX: i64 = 0;

pub fn lower(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(lower).collect();
			enum_object(&items).unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
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
