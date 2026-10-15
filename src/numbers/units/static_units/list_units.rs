//! Lists of quantities (card dimensions-units Q3, user 2026-10-15): a list literal `[3 cm, 4 cm]`, a variable holding
//! one, and a unit after a list, which is each element's: `[3, 4] cm` is `[3 cm, 4 cm]`, `xs cm` maps xs, nested lists
//! (matrices) alike. A list value has one element signature and a depth (a matrix is 2); it shows whole as text.
use super::*;

/// The element of a list a unit scales: `map(xs, quantity_item => quantity_item * 0.01)`
const ITEM: &str = "quantity_item";
/// Where the list goes in the text of a whole list, and the unit's SI amount in a scaled one
const LIST_PLACEHOLDER: &str = "quantity_list";
const SCALE: &str = "quantity_scale";

/// A list value: its lowered node, the signature of its elements and how deep they are nested
pub(super) struct ListValue {
	pub(super) node: Node,
	pub(super) element: Signature,
	pub(super) depth: usize,
}

impl Inference {
	/// The list a value is: a literal, a list variable, a unit after a list, a mapped list; None for any other value
	pub(super) fn list_value(&mut self, value: &Node) -> Option<Result<ListValue, Stop>> {
		match value.drop_meta() {
			Node::Symbol(name) => {
				let element =
					self.lists.get(name).cloned().or_else(|| self.plain_lists.contains(name).then(Vec::new))?;
				let depth = self.list_depths.get(name).copied().unwrap_or(1);
				Some(Ok(ListValue { node: value.clone(), element, depth }))
			}
			Node::List(items, Bracket::Square, separator) => Some(self.list_literal(items.clone(), separator.clone())),
			Node::List(items, Bracket::None, Separator::Space) => match items.as_slice() {
				[list, unit] => self.list_in_unit(list, unit),
				_ => None,
			},
			Node::Key(list, Op::Mul, unit) => self.list_in_unit(list, unit),
			_ => self
				.mapped_list(value)
				.map(|mapped| mapped.map(|(node, element)| ListValue { node, element, depth: 1 })),
		}
	}

	/// A list of quantities, not of plain values; None for any other value
	pub(super) fn quantity_list(&mut self, value: &Node) -> Option<Result<ListValue, Stop>> {
		match self.list_value(value)? {
			Ok(list) if list.element.is_empty() => None,
			found => Some(found),
		}
	}

	/// `[3, 4]`, `xs`: a value that is a list
	pub(super) fn is_list(&self, value: &Node) -> bool {
		match value.drop_meta() {
			Node::List(_, Bracket::Square, _) => true,
			Node::Symbol(name) => self.lists.contains_key(name) || self.plain_lists.contains(name),
			_ => false,
		}
	}

	/// The items of a list literal, all of one signature and depth: items of different units are a DimensionError
	pub(super) fn list_literal(&mut self, items: Vec<Node>, separator: Separator) -> Result<ListValue, Stop> {
		let mut lowered = vec![];
		let mut shape: Option<(Signature, usize)> = None;
		for item in items {
			let (item, element, depth) = match self.list_value(&item) {
				Some(inner) => {
					let inner = inner?;
					(inner.node, inner.element, inner.depth + 1)
				}
				None => {
					let (item, signature) = self.infer(item)?;
					(item, signature, 1)
				}
			};
			match &shape {
				Some((earlier, _)) if *earlier != element => {
					return Err(Stop::Error(format!(
						"DimensionError: a list of {} and {}",
						shown(earlier),
						shown(&element)
					)));
				}
				Some((_, earlier)) if *earlier != depth => return Err(Stop::Unsupported),
				_ => shape = Some((element, depth)),
			}
			lowered.push(item);
		}
		let (element, depth) = shape.unwrap_or((vec![], 1));
		Ok(ListValue { node: Node::List(lowered, Bracket::Square, separator), element, depth })
	}

	/// `cm`, `km/h`: a unit or an expression of units, as written after a value
	pub(super) fn is_unit_after_value(&self, unit: &Node) -> bool {
		match unit.drop_meta() {
			Node::Symbol(_) => self.unit_word(unit).is_some(),
			other => super::super::unit_expression(other).is_some(),
		}
	}

	/// `[3, 4] cm` is `[3 cm, 4 cm]`; `xs km/h` maps xs; None when no list comes before the unit
	pub(super) fn list_in_unit(&mut self, list: &Node, unit: &Node) -> Option<Result<ListValue, Stop>> {
		if !self.is_unit_after_value(unit) {
			return None;
		}
		if matches!(list.drop_meta(), Node::List(_, Bracket::Square, _)) {
			return self.list_value(&each_in_unit(list, unit));
		}
		if !matches!(list.drop_meta(), Node::Symbol(_)) {
			return None;
		}
		let list = match self.list_value(list)? {
			Ok(list) if list.depth == 1 => list,
			Ok(_) => return Some(Err(Stop::Unsupported)),
			Err(stop) => return Some(Err(stop)),
		};
		Some(self.infer(unit.clone()).map(|(scale, unit_signature)| {
			let mapped = crate::warp_parser::parse(&format!("map({LIST_PLACEHOLDER}, {ITEM} => {ITEM} * {SCALE})"));
			let mapped = crate::lowering::library_words::substitute(mapped, SCALE, &scale);
			ListValue {
				node: crate::lowering::library_words::substitute(mapped, LIST_PLACEHOLDER, &list.node),
				element: combined(&list.element, &unit_signature, 1),
				depth: 1,
			}
		}))
	}

	/// A whole list of quantities as text, built at run time: `"[" + join(map(xs, item => str(item / 1/100) + " cm"), " ") + "]"`,
	/// a list of lists in brackets each
	pub(super) fn list_text(&self, list: ListValue, amount_text: &str) -> Result<Node, Stop> {
		let mut text = format!("{QUANTITY_SHOWN}0");
		for level in 1..=list.depth {
			let source = match level == list.depth {
				true => LIST_PLACEHOLDER.to_string(),
				false => format!("{ITEM}{}", level + 1),
			};
			text = format!("(\"[\" + join(map({source}, {ITEM}{level} => {text}), \" \") + \"]\")");
		}
		let text = self.with_quantity_texts(&text, &[(format!("{ITEM}1"), &list.element)], amount_text)?;
		Ok(crate::lowering::library_words::substitute(text, LIST_PLACEHOLDER, &list.node))
	}
}

/// `[3, 4] cm` → `[3 cm, 4 cm]`, nested lists alike
fn each_in_unit(amount: &Node, unit: &Node) -> Node {
	match amount.drop_meta() {
		Node::List(items, Bracket::Square, separator) => {
			Node::List(items.iter().map(|item| each_in_unit(item, unit)).collect(), Bracket::Square, separator.clone())
		}
		_ => Node::Key(Box::new(amount.clone()), Op::Mul, Box::new(unit.clone())),
	}
}
