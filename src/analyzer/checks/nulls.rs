//! Nulls and errors: a possibly null or failed variable needs an `if x {…}` check before arithmetic or member access
use super::*;

/// Why a variable cannot be used as a plain value before an `if x {…}` check
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Unchecked {
	Null,  // `x=ø` (wiki/null.md)
	Error, // `x=fetch …`: a remote call may fail (DESIGN.md "Effects": Error as Result<T, E>)
}

impl Unchecked {
	fn of(value: &Node) -> Option<Unchecked> {
		match value.drop_meta() {
			Node::Empty => Some(Unchecked::Null),
			_ if crate::host::fetch_call(value).is_some() => Some(Unchecked::Error),
			_ => None,
		}
	}

	fn describe(self, name: &str) -> String {
		match self {
			Unchecked::Null => format!("{name} may be ø (null)"),
			Unchecked::Error => format!("{name} may be an error (fetch can fail)"),
		}
	}
}

/// A list literal `[1, 2]` or a variable assigned one
pub(crate) fn holds_list(operand: &Node, lists: &HashSet<String>) -> bool {
	match operand.drop_meta() {
		Node::List(_, Bracket::Square, _) => true,
		Node::Symbol(name) => lists.contains(name),
		_ => false,
	}
}

/// The variables the program assigns a list literal: what `+` concatenates
pub(crate) fn list_variables(program: &Node) -> HashSet<String> {
	let mut lists = HashSet::new();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let (Node::Symbol(name), Node::List(_, Bracket::Square, _)) = (target.drop_meta(), value.drop_meta()) {
				lists.insert(name.clone());
			}
		}
	});
	lists
}

/// `x=ø` makes x possibly null, `x=fetch …` possibly an error: arithmetic or member access on it needs an
/// `if x {…}` check first (ø and errors are falsy). A fetch used directly as an operand is never checked.
pub(crate) fn check_null_use(
	node: &Node,
	nullable: &mut HashMap<String, Unchecked>,
	lists: &HashSet<String>,
) -> Option<Diagnostic> {
	let possibly_null = |operand: &Node, nullable: &HashMap<String, Unchecked>| match operand.drop_meta() {
		Node::Symbol(name) => nullable
			.get(name)
			.map(|reason| (name.clone(), reason.describe(name), None)),
		fetched if crate::host::fetch_call(fetched).is_some() => {
			let call = fetched.serialize();
			Some((
				"result".to_string(),
				format!("{call} may be an error (fetch can fail)"),
				Some(call),
			))
		}
		_ => None,
	};
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			// `out = out + xs` building a list from ø, the empty list: a concatenation, no use of a null
			let found = match value.drop_meta() {
				Node::Key(left, Op::Add, right)
					if left.drop_meta() == target.drop_meta()
						&& nullable.get(&target.name()) == Some(&Unchecked::Null) =>
				{
					check_null_use(right, nullable, lists)
				}
				_ => check_null_use(value, nullable, lists),
			};
			let target = match target.drop_meta() {
				Node::Key(name, Op::Colon, _) => &**name, // x:int?=ø
				other => other,
			};
			if let Node::Symbol(name) = target.drop_meta() {
				match Unchecked::of(value) {
					Some(reason) => nullable.insert(name.clone(), reason),
					None => nullable.remove(name),
				};
			}
			found
		}
		Node::Key(if_condition, Op::Then, then) => {
			let Node::Key(_, Op::If, condition) = if_condition.drop_meta() else {
				return check_null_use(if_condition, nullable, lists).or_else(|| check_null_use(then, nullable, lists));
			};
			let mut narrowed = nullable.clone();
			if let Node::Symbol(name) = condition.drop_meta() {
				narrowed.remove(name);
			}
			check_null_use(condition, nullable, lists).or_else(|| check_null_use(then, &mut narrowed, lists))
		}
		// ø is the empty list: appending to it or concatenating a list needs no check (`a=(); a.add(1)`), nor popping a
		// stack built in a loop (`st=[]; for t in ts { if … { st.pop() } else { st.add(t) } }`)
		Node::Key(list, Op::Dot, call)
			if (updates_list(list, call) || popped_list(list, call).is_some())
				&& nullable.get(&list.name()) != Some(&Unchecked::Error) =>
		{
			let found = check_null_use(call, nullable, lists);
			nullable.remove(&list.name()); // no longer empty
			found
		}
		// ø is the empty list: it counts 0 (`b=[]; b.count`)
		Node::Key(list, Op::Dot, property)
			if matches!(list.drop_meta(), Node::Symbol(_))
				&& is_counting_property(&property.name())
				&& nullable.get(&list.name()) != Some(&Unchecked::Error) =>
		{
			None
		}
		// `xs + ys` of a list xs and ys = []: concatenating the empty list
		Node::Key(left, Op::Add, right) if [left, right].iter().any(|side| holds_list(side, lists)) => {
			check_null_use(left, nullable, lists).or_else(|| check_null_use(right, nullable, lists))
		}
		Node::Key(left, op, right) if op.is_arithmetic() || *op == Op::Dot => {
			let operand = possibly_null(left, nullable).or_else(|| {
				if *op == Op::Dot {
					None
				} else {
					possibly_null(right, nullable)
				}
			});
			if let Some((name, reason, call)) = operand {
				let expression = node.drop_meta().serialize();
				let fix = match call {
					Some(call) => format!("{name} = {call}; if {name} {{ {} }}", expression.replace(&call, &name)),
					None => format!("if {name} {{ {expression} }}"),
				};
				return Some(Diagnostic::at(node, format!("{reason} in {expression}")).fix(fix));
			}
			check_null_use(left, nullable, lists).or_else(|| check_null_use(right, nullable, lists))
		}
		Node::Key(left, _, right) => {
			check_null_use(left, nullable, lists).or_else(|| check_null_use(right, nullable, lists))
		}
		Node::List(items, _, _) => items.iter().find_map(|item| check_null_use(item, nullable, lists)),
		_ => None,
	}
}
