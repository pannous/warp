//! Objects and their fields: `object.name`, `name of object`, `object["name"]`, field assignments and `?.`, the variables only ever holding object literals
use super::*;

/// `object.name`, `name of object`, `object["name"]`: the entry named `name`, a subscript by a text key
pub(super) fn field_lookup(object: &Node, name: &str, position: &Node) -> Node {
	let key = match position {
		Node::Meta { data, .. } => Node::Meta { node: Box::new(text(name)), data: data.clone() },
		_ => text(name),
	};
	crate::warp_parser::subscript(object.clone(), key)
}

/// A field lookup by a name, as `field_lookup` builds it: its value is an object whenever the field holds one
/// A field read by name (`p.x`, `p["x"]`), an element of one (`company.employees#2`), or either in parentheses: the
/// value may be an object
pub(super) fn is_field_lookup(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Hash, index) if crate::warp_parser::subscript_key(index).and_then(field_name).is_some() => {
			true
		}
		Node::Key(list, Op::Hash, _) => is_field_lookup(list),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => is_field_lookup(&items[0]),
		_ => false,
	}
}

/// The entries of an object literal `{a:1 b:2}` (or `{a:1}`): every item is a `key:value`
pub(crate) fn object_entries(node: &Node) -> Option<Vec<(String, Node)>> {
	let single;
	let items: &[Node] = match node.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items, // `{}` is the empty object, grown by `d["k"] = v`
		// a named tuple `(x: 10, y: 20)`: its entries are fields too
		Node::List(items, Bracket::Round, _)
			if items.len() > 1 && items.iter().all(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			items
		}
		Node::Key(_, Op::Colon, _) => {
			single = [node.clone()];
			&single
		}
		_ => return None,
	};
	items
		.iter()
		.map(|item| match item.drop_meta() {
			Node::Key(key, Op::Colon, value) => Some((field_name(key)?, value.as_ref().clone())),
			_ => None,
		})
		.collect()
}

/// The field name of a subscript `p["name"]`, as `p.name` lowers: None for an element `xs#i`
pub(super) fn subscript_field(index: &Node) -> Option<String> {
	match crate::warp_parser::subscript_key(index)?.drop_meta() {
		Node::Text(name) => Some(name.clone()),
		Node::Char(letter) => Some(letter.to_string()),
		_ => None,
	}
}

pub(super) fn field_name(key: &Node) -> Option<String> {
	match key.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		Node::Char(letter) => Some(letter.to_string()), // `{"A": 1}`: a quoted one-letter key
		_ => None,
	}
}

/// Variables that are only ever assigned an object literal, with that literal (`None` for a variable assigned anything
/// else too), and the variables assigned a plain value (`plain`: a number, text or character, or a list of them)
#[derive(Default)]
pub(super) struct AssignedObjects {
	pub(super) objects: HashMap<String, Option<Node>>,
	pub(super) plain: HashSet<String>,
	/// variables also bound to what may hold an object, so not plain: a loop's `for item in basket`, `p = people#1`
	pub(super) rebound: HashSet<String>,
}

pub(super) fn collect_assigned_objects(node: &Node, assigned: &mut AssignedObjects) {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Node::Symbol(name) = target.drop_meta() {
				if is_plain_literal(value) {
					assigned.plain.insert(name.clone());
				}
				if may_hold_object(value) {
					assigned.rebound.insert(name.clone());
				}
				let copied = match value.drop_meta() {
					Node::Symbol(other) => assigned.objects.get(other).cloned().flatten(), // `q=p` is the same object
					_ => None,
				};
				let literal = copied.or_else(|| object_entries(value).map(|_| value.as_ref().clone()));
				let both_objects =
					literal.is_some() && assigned.objects.get(name).is_none_or(|earlier| earlier.is_some());
				assigned.objects.insert(name.clone(), if both_objects { literal } else { None });
			}
			collect_assigned_objects(value, assigned);
		}
		Node::Key(left, _, right) => {
			collect_assigned_objects(left, assigned);
			collect_assigned_objects(right, assigned);
		}
		Node::List(items, _, _) => {
			if let Some(variable) = loop_variable(items) {
				assigned.rebound.insert(variable);
			}
			items.iter().for_each(|item| collect_assigned_objects(item, assigned))
		}
		_ => {}
	}
}

/// An element, a field, a call's result or an object literal
pub(super) fn may_hold_object(value: &Node) -> bool {
	match value.drop_meta() {
		Node::Key(_, Op::Hash | Op::Dot, _) => true,
		Node::List(items, Bracket::Round, Separator::None) => {
			matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))
		}
		other => object_entries(other).is_some(),
	}
}

/// The variable `item` of a loop `for item in basket {…}`, `for each item in basket {…}`
pub(super) fn loop_variable(words: &[Node]) -> Option<String> {
	let words: Vec<String> = words.iter().take(4).map(|word| word.drop_meta().name()).collect();
	match words.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
		["for", "each", variable, "in"] | ["for", variable, "in", ..] => Some(variable.to_string()),
		_ => None,
	}
}

/// A number, text or character literal, a list literal of them, arithmetic or a range of them
pub(super) fn is_plain_literal(value: &Node) -> bool {
	match value.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False => true,
		Node::List(items, Bracket::Square, _) => !items.is_empty() && items.iter().all(is_plain_literal),
		// `"ab" + "c"`, `2 * 3`, the range `1..4`: arithmetic of plain values, or a range of them, is no object either
		Node::Key(left, op, right) if op.is_arithmetic() || matches!(op, Op::Range | Op::To) => {
			is_plain_literal(left) && is_plain_literal(right)
		}
		_ => false,
	}
}

/// `person: {name: "Alice"}` among the statements of a program that reads `person` later binds it like
/// `person = {…}`: `person.name` is then a field of a variable (a data key nothing reads stays data)
pub(super) fn bind_read_data_objects(program: Node) -> Node {
	let Node::List(items, bracket @ (Bracket::None | Bracket::Curly), separator) = program else {
		return program;
	};
	let mut read = HashSet::new();
	for item in &items {
		item.visit(&mut |part| {
			if let Node::Key(receiver, Op::Dot | Op::Hash, _) = part {
				if let Node::Symbol(name) = receiver.drop_meta() {
					read.insert(name.clone());
				}
			}
		});
	}
	let items = items
		.into_iter()
		.map(|item| match item.drop_meta() {
			Node::Key(name, Op::Colon, value)
				if matches!(name.drop_meta(), Node::Symbol(symbol) if read.contains(symbol))
					&& object_entries(value).is_some() =>
			{
				Node::Key(name.clone(), Op::Assign, value.clone())
			}
			_ => item,
		})
		.collect();
	Node::List(items, bracket, separator)
}

/// The names the program defines, as variables or functions: an alias word never shadows one (P142)
pub(crate) fn defined_names(node: &Node) -> HashSet<String> {
	let mut defined = HashSet::new();
	collect_assigned_names(node, &mut defined);
	let context = crate::analyzer::function_definitions(node);
	defined.extend(context.user_functions.into_keys());
	// a method lowered from a class or Go's `func (p Point) Sum()`: `(Sum self:Point) := …`
	node.visit(&mut |part| {
		if let Node::Key(head, Op::Define, _) = part {
			defined.insert(crate::node::leading_name(head));
		}
	});
	defined
}

pub(crate) fn collect_assigned_names(node: &Node, names: &mut HashSet<String>) {
	names.extend(node.assigned_names::<Vec<String>>(|op| matches!(op, Op::Assign | Op::Define)));
}

impl Lowering {
	/// `object.name = value` is `object = field_with(object, "name", value)`; a nested path updates the objects on the way
	/// (`p.b.c = 4` is `p = field_with(p, "b", field_with(p.b, "c", 4))`), an element on the way is stored back
	/// (`bags#1.n = 5` is `bags#1 = field_with(bags#1, "n", 5)`)
	pub(super) fn field_assignment(&self, target: &Node, value: &Node) -> Option<Node> {
		let Node::Key(_, Op::Hash, index) = target.drop_meta() else {
			return None;
		};
		subscript_field(index)?;
		self.stored(target, value.clone())
	}

	/// `a = p.x = 5` is `p.x = 5; a = p.x`: the value of a field assignment is the field's new value, not the object
	/// field_with gives (card chain-field)
	pub(super) fn chained_field_assignment(&self, outer: &Node, value: &Node) -> Option<Node> {
		let Node::Key(inner, Op::Assign, _) = value.drop_meta() else {
			return None;
		};
		let Node::Key(_, Op::Hash, index) = self.expand(inner.as_ref().clone()).drop_meta().clone() else {
			return None;
		};
		subscript_field(&index)?;
		let read_back = Node::Key(Box::new(outer.clone()), Op::Assign, inner.clone());
		Some(self.expand(Node::List(vec![value.clone(), read_back], Bracket::None, Separator::Semicolon)))
	}

	/// `place = value` for any place: a variable, an element `xs#i` of one (as written), a field (field_with on its
	/// object, stored back), an element of anything else through a temporary (`p.items#1 = v` is
	/// `place·0 = p.items; place·0#1 = v; p.items = place·0`)
	pub(super) fn stored(&self, place: &Node, value: Node) -> Option<Node> {
		let assigned = |place: &Node, value: Node| key(place.clone(), Op::Assign, value);
		let Node::Key(base, Op::Hash, index) = place.drop_meta() else {
			return matches!(place.drop_meta(), Node::Symbol(_)).then(|| assigned(place, value));
		};
		if let Some(name) = subscript_field(index) {
			let updated = call(FIELD_WITH, vec![base.as_ref().clone(), Node::Text(name), value]);
			return self.stored(base, updated);
		}
		if matches!(base.drop_meta(), Node::Symbol(_)) {
			return Some(assigned(place, value));
		}
		let held = Node::Symbol(format!("{PLACE_TEMPORARY}{}", self.next_temporary()));
		let element = Node::Key(Box::new(held.clone()), Op::Hash, index.clone());
		let statements =
			vec![assigned(&held, base.as_ref().clone()), assigned(&element, value), self.stored(base, held)?];
		Some(Node::List(statements, Bracket::None, Separator::Semicolon))
	}

	pub(super) fn next_temporary(&self) -> usize {
		self.temporaries.next_number()
	}

	/// `receiver?.name`: ø when the receiver is ø, else the field
	pub(super) fn safe_lookup(&self, receiver: Node, word: Node) -> Node {
		let Node::Symbol(name) = word.drop_meta() else {
			return Diagnostic::at(&word, "?. needs a field name after it").into_error();
		};
		let number = self.next_temporary();
		let program = named_apart(parse_template(&format!("({SAFE_PREFIX}value={RECEIVER_PLACEHOLDER}; if {SAFE_PREFIX}value == ø then ø else {LOOKUP_PLACEHOLDER})"), &receiver), SAFE_PREFIX, SAFE_BASE, number);
		let lookup =
			field_lookup(&Node::Symbol(temporary_name(&[SAFE_BASE, "value", &number.to_string()])), name, &word);
		substitute(substitute(program, RECEIVER_PLACEHOLDER, &receiver), LOOKUP_PLACEHOLDER, &lookup)
	}

	/// The object literal a node stands for: the literal itself, a variable only ever assigned one, or a field of such an object
	pub(super) fn object_literal(&self, node: &Node) -> Option<Node> {
		self.written_object(node).or_else(|| self.instances.fields_template(node))
	}

	pub(super) fn written_object(&self, node: &Node) -> Option<Node> {
		match node.drop_meta() {
			Node::Symbol(name) => self.objects.get(name).cloned(),
			Node::Key(base, Op::Hash, index) => {
				let field = field_name(crate::warp_parser::subscript_key(index)?)?;
				let (_, value) =
					object_entries(&self.object_literal(base)?)?.into_iter().find(|(name, _)| *name == field)?;
				object_entries(&value).map(|_| value)
			}
			other => object_entries(other).map(|_| other.clone()),
		}
	}
}
