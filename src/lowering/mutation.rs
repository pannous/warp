//! `fn!` mutates in place (user decision D2 2026-10-03, "by position", wiki/mutable.md): a `!` after a function or method
//! assigns its result back to the variable it was called on. `x.upper!`, `x.upper()!` and `upper(x)!` become
//! `x = x.upper` in the parser; in `upper x!` the parser only sees `x!`, so it marks x and `lower` turns the call into
//! `x = upper x`. A lone `x!`, `f!` or `{…}!` keeps evaluating.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::diagnostic::Diagnostic;

/// Meta key on a name written `x!`
const MUTATED_MARK: &str = "mutated";
/// `x!` that changes nothing unwraps x (library_words: ø is a loud error)
pub const UNWRAP: &str = "unwrap";

/// Pure words whose result is a changed copy of their argument: dropping it changes nothing
const COPY_WORDS: [&str; 11] = ["upper", "uppercase", "lower", "lowercase", "trim", "strip", "reverse", "sort", "sorted", "replace", "capitalize"];

/// wiki/mutable.md: a statement dropping a copy word's result (`uppercase x`), and an assignment of a mutating call
/// (`y = x.upper!`, which changes x too), warn with the forms meant
pub fn warn_discarded(program: Node) -> Node {
	let mut warnings = vec![];
	program.visit(&mut |node| match node.drop_meta() {
		Node::List(statements, _, Separator::Semicolon | Separator::Newline) => {
			for statement in &statements[..statements.len().saturating_sub(1)] {
				if let Some((word, variable)) = copy_call(statement) {
					let message = format!("unused value of {word} {variable}: assign it (y = {word} {variable}) or write {variable}.{word}! to change {variable}");
					warnings.push(Diagnostic::at(statement, message).offer(format!("change {variable}"), format!("{word} {variable}"), format!("{variable}.{word}!")));
				}
			}
		}
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Some(variable) = mutating_call(value) {
				let target = crate::normalize::operand_text(target);
				warnings.push(Diagnostic::at(node, format!("{target} = …! changes {variable} too: assign the copy without `!`, or mutate {variable} first and then write {target} = {variable}")));
			}
		}
		_ => {}
	});
	match crate::diagnostic::report(&warnings) {
		Err(error) => error,
		Ok(()) => program,
	}
}

/// A copy word called on a variable, as written (`upper x`, `upper(x)`, `x.upper`, `x.upper()`), and whether a `!`
/// marks it as mutating (`upper x!`, `x.upper!`)
fn copy_word_call(call: &Node) -> Option<(&str, &str, bool)> {
	let (word, variable) = match call.drop_meta() {
		Node::Key(receiver, Op::Dot, method) => (method.as_ref(), receiver.as_ref()),
		Node::List(items, _, _) if items.len() == 2 => (&items[0], &items[1]),
		_ => return None,
	};
	let called = match word.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => items[0].drop_meta(),
		other => other,
	};
	match (called, variable.drop_meta()) {
		(Node::Symbol(called), Node::Symbol(name)) if COPY_WORDS.contains(&called.as_str()) => Some((called, name, is_marked(call) || is_marked(variable))),
		_ => None,
	}
}

fn copy_call(call: &Node) -> Option<(&str, &str)> {
	copy_word_call(call).and_then(|(word, variable, mutating)| (!mutating).then_some((word, variable)))
}

/// The variable a mutating call assigns: `x.upper!` and `upper x!` as marked, `x = x.upper` as the parser wrote `upper(x)!`
fn mutating_call(value: &Node) -> Option<&str> {
	match value.drop_meta() {
		Node::Key(variable, Op::Assign, call) => match (variable.drop_meta(), copy_call(call)) {
			(Node::Symbol(name), Some((_, called))) if name == called => Some(name),
			_ => None,
		},
		_ => copy_word_call(value).and_then(|(_, variable, mutating)| mutating.then_some(variable)),
	}
}

/// The variable a call written before `!` changes: x in `x.upper`, `x.upper()` and `upper(x)`
pub fn mutated_variable(call: &Node) -> Option<&Node> {
	match call.drop_meta() {
		Node::Key(receiver, Op::Dot, method) if is_name(receiver) && is_method(method) => Some(receiver),
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[function, argument] if is_name(function) && is_name(argument) => Some(argument),
			_ => None,
		},
		_ => None,
	}
}

/// `x!`: evaluated alone, assigned the result of the call it is the argument of
pub fn marked(name: Node) -> Node {
	Node::Meta { node: Box::new(name), data: Box::new(Node::key(MUTATED_MARK, Node::True)) }
}

/// `x!!`: like `x!`, and a block runs fully (blocks.rs)
pub fn marked_fully(name: Node) -> Node {
	Node::Meta { node: Box::new(name), data: Box::new(Node::key(MUTATED_MARK, Node::int(FULLY))) }
}
const FULLY: i64 = 2;

/// What a `!` or `!!` follows (a name or a field `o.s1`), and whether it was `!!`
pub(crate) fn bang_target(node: &Node) -> Option<(Node, bool)> {
	let Node::Meta { node: inner, data } = node else { return None };
	match data.as_ref() {
		Node::Key(key, _, value) if key.name() == MUTATED_MARK => Some((inner.drop_meta().clone(), matches!(value.drop_meta(), Node::Number(n) if *n == FULLY))),
		_ => bang_target(inner),
	}
}

/// `upper x!` → `x = upper x`; every other marked name is just the name
pub fn lower(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(lower_inside).collect();
			let call = Node::List(items, bracket, separator);
			match &call {
				Node::List(items, _, separator) if items.len() == 2 && is_name(&items[0]) && is_marked(&items[1]) && is_name(&unmarked(items[1].clone()))
					&& !matches!(separator, Separator::Semicolon | Separator::Newline) => {
					let variable = unmarked(items[1].clone());
					Node::Key(Box::new(variable), Op::Assign, Box::new(strip_marks(call, false)))
				}
				_ => strip_marks(call, true),
			}
		}
		// `x.upper!`: the parser marked the method it was parsing when it met the `!`
		Node::Key(receiver, Op::Dot, method) if is_name(&receiver) && is_marked(&method) => {
			let call = Node::Key(receiver.clone(), Op::Dot, Box::new(unmarked(*method)));
			Node::Key(receiver, Op::Assign, Box::new(call))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::Meta { .. } if is_marked(&node) => unwrapped(unmarked(node)),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}

/// Items keep their marks until their own list decides; deeper lists are lowered on their own
fn lower_inside(item: Node) -> Node {
	if is_marked(&item) { item } else { lower(item) }
}

/// The marked names as plain names (the variable a mutation assigns) or, `unwrap`, as unwraps (`x!` alone)
fn strip_marks(node: Node, unwrap: bool) -> Node {
	let plain = |item: Node| if unwrap { unwrapped(unmarked(item)) } else { unmarked(item) };
	match node {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| if is_marked(&item) { plain(item) } else { item }).collect(), bracket, separator),
		other => other,
	}
}

/// `x!` that blocks.rs did not run: a name unwraps (row 27, `unwrap`), any other expression would be a block known only at
/// run time, which needs the run-time compiler (wiki/charged.md section 5): a loud error until it exists
fn unwrapped(name: Node) -> Node {
	// `x.upper!` (no block field, blocks.rs ran those): x = x.upper (D2)
	if let Node::Key(receiver, Op::Dot, method) = name.drop_meta() {
		if is_name(receiver) && is_method(method) {
			return Node::Key(receiver.clone(), Op::Assign, Box::new(name.clone()));
		}
	}
	// `ø!`: P73 force — Empty unwraps like a name holding ø
	if matches!(name.drop_meta(), Node::Empty) {
		return Node::List(vec![Node::Symbol(UNWRAP.to_string()), name], Bracket::Round, Separator::None);
	}
	if !is_name(&name) {
		let written = crate::normalize::operand_text(&name);
		return crate::node::error(&format!("{written} is only known at run time: `!` needs a constant block"));
	}
	Node::List(vec![Node::Symbol(UNWRAP.to_string()), name], Bracket::Round, Separator::None)
}

/// The mark may sit under other meta information, such as the position the parser records
fn is_marked(node: &Node) -> bool {
	match node {
		Node::Meta { node, data } => is_mark(data) || is_marked(node),
		_ => false,
	}
}

fn is_mark(data: &Node) -> bool {
	matches!(data, Node::Key(key, _, _) if key.name() == MUTATED_MARK)
}

fn unmarked(node: Node) -> Node {
	match node {
		Node::Meta { node, data } if is_mark(&data) => *node,
		Node::Meta { node, data } => Node::Meta { node: Box::new(unmarked(*node)), data },
		other => other,
	}
}

fn is_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(_))
}

fn is_method(method: &Node) -> bool {
	match method.drop_meta() {
		Node::Symbol(_) => true,
		Node::List(items, _, _) => items.first().is_some_and(is_name),
		_ => false,
	}
}
