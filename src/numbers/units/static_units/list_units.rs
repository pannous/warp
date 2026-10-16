//! Lists of quantities (card dimensions-units Q3, user 2026-10-15): a list's signature is its elements' with a depth
//! (static_units.rs `Signature`). A unit after a list is each element's: `[3, 4] cm` is `[3 cm, 4 cm]`, `xs cm` maps xs,
//! nested lists (matrices) alike; a whole list shows as the text of its elements.
use super::*;

/// The element of a list a computation maps: `map(xs, quantity_item1 => quantity_item1 * 0.01)`, one name per depth
const ITEM: &str = "quantity_item";
/// Where the list goes in a mapped list and in the text of a whole list, and the body in a mapped one
const LIST_PLACEHOLDER: &str = "quantity_list";
const BODY_PLACEHOLDER: &str = "quantity_body";
/// How many calls deep a plain list is followed into function bodies (`g() := h()`, `h() := [1, 2]`)
pub(super) const CALLS_FOLLOWED: usize = 4;
/// One level of a mapped list and of a list's text
const MAPPED: &str = "map(quantity_list, quantity_item => quantity_body)";
const SHOWN: &str = r#"("[" + join(map(quantity_list, quantity_item => quantity_body), " ") + "]")"#;

impl Inference {
	/// How deep a list of plain values nests: `[3, 4]` and a variable holding one 1, a matrix 2; 0 for any other value
	pub(super) fn plain_list_depth(&self, value: &Node) -> usize {
		self.plain_list_depth_within(value, CALLS_FOLLOWED)
	}

	/// `g()` of `g() := [1, 2]` is a list too: the calls followed into function bodies are at most `calls_left` deep
	fn plain_list_depth_within(&self, value: &Node, calls_left: usize) -> usize {
		let depth_of = |node: &Node| self.plain_list_depth_within(node, calls_left);
		match value.drop_meta() {
			Node::Symbol(name) => self.plain_lists.get(name).copied().unwrap_or(0),
			Node::List(items, Bracket::Square, _) => 1 + items.iter().map(depth_of).max().unwrap_or(0),
			Node::List(items, Bracket::Round, _) => match self.returned_by_call(items, calls_left) {
				Some(last) => self.plain_list_depth_within(last, calls_left - 1),
				None if items.len() == 1 => depth_of(&items[0]),
				None => 0,
			},
			_ => 0,
		}
	}

	/// What a call `(f a b)` of a user function gives, while `calls_left` allows following one more call
	pub(super) fn returned_by_call(&self, call: &[Node], calls_left: usize) -> Option<&Node> {
		if calls_left == 0 {
			return None;
		}
		let (head, arguments) = call.split_first()?;
		let definition = self.definitions.get(&(head.drop_meta().symbol_name()?.to_string(), arguments.len()))?;
		returned_value(&definition.body)
	}

	/// `cm`, `km/h`: a unit or an expression of units, as written after a value
	pub(super) fn is_unit_after_value(&self, unit: &Node) -> bool {
		match unit.drop_meta() {
			Node::Symbol(_) => self.unit_word(unit).is_some(),
			other => super::super::unit_expression(other).is_some(),
		}
	}
}

/// The one signature of the items of a list: items of different units are a DimensionError
pub(super) fn one_signature(signatures: &[Signature]) -> Result<Signature, Stop> {
	let Some(first) = signatures.first() else {
		return Ok(Signature::plain());
	};
	match signatures.iter().find(|signature| *signature != first) {
		// `[1 m, [2 m]]`: lists of different depths
		Some(other) if **other == **first => Err(Stop::Unsupported),
		Some(other) => Err(Stop::Error(format!("DimensionError: a list of {} and {}", shown(first), shown(other)))),
		None => Ok(first.clone()),
	}
}

/// `[3, 4] cm` → `[3 cm, 4 cm]`, nested lists alike
pub(super) fn each_in_unit(amount: &Node, op: Op, unit: &Node) -> Node {
	match amount.drop_meta() {
		Node::List(items, Bracket::Square, separator) => Node::List(
			items.iter().map(|item| each_in_unit(item, op, unit)).collect(),
			Bracket::Square,
			separator.clone(),
		),
		_ => Node::Key(Box::new(amount.clone()), op, Box::new(unit.clone())),
	}
}

/// `compute` applied to each element of a list `depth` deep, at run time:
/// `map(xs, quantity_item1 => map(quantity_item1, quantity_item2 => compute(quantity_item2)))`
pub(super) fn each_element(list: Node, depth: usize, compute: &dyn Fn(Node) -> Node) -> Node {
	each_level(list, depth, MAPPED, compute)
}

/// A whole list of quantities (`depth` deep) as text, built at run time:
/// `"[" + join(map(xs, item => str(item / (1/100)) + "cm"), " ") + "]"`, a list of lists in brackets each
pub(super) fn list_text(list: Node, depth: usize, units: &[Factor], amount_text: &str) -> Result<Node, Stop> {
	let element = format!("{ITEM}0");
	let shown = quantity_text(Node::Symbol(element.clone()), units, amount_text).ok_or(Stop::Unsupported)?;
	let shown_element = |item: Node| crate::lowering::library_words::substitute(shown.clone(), &element, &item);
	Ok(each_level(list, depth, SHOWN, &shown_element))
}

/// `template` around each level of a list `depth` deep, `leaf` of each element
fn each_level(list: Node, depth: usize, template: &str, leaf: &dyn Fn(Node) -> Node) -> Node {
	if depth == 0 {
		return leaf(list);
	}
	let item = format!("{ITEM}{depth}");
	let body = each_level(Node::Symbol(item.clone()), depth - 1, template, leaf);
	let level = crate::warp_parser::parse(&template.replace(ITEM, &item));
	let level = crate::lowering::library_words::substitute(level, BODY_PLACEHOLDER, &body);
	crate::lowering::library_words::substitute(level, LIST_PLACEHOLDER, &list)
}
