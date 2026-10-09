//! Small helper functions are inlined where they are called, so a list they index or update by index stays the caller's
//! array (list_dispatch.rs `$NodeList`) instead of a Node list passed and rebuilt on every call:
//! `arr = swap(arr, i, j)` with `swap(arr, i, j) := {temp = arr[i]; arr[i] = arr[j]; arr[j] = temp; return arr}` becomes
//! `arr = (swap·arr·1 = arr; swap·i·1 = i; swap·j·1 = j; swap·temp·1 = …; …; swap·arr·1)`.
//! Only functions whose meaning cannot change by moving their body: not recursive, no free variables (every name is a
//! parameter or a local), no nested definitions, lambdas, globals or early returns, and a few statements at most.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

/// The most statements a body may have to be inlined
const MAX_STATEMENTS: usize = 8;
const RETURN: &str = "return";
const NAME_SEPARATOR: &str = "·";

struct Inlinable {
	parameters: Vec<String>,
	statements: Vec<Node>,
	result: Node,
	/// The parameters and locals, renamed per inlined call
	names: HashSet<String>,
}

pub fn lower(node: Node) -> Node {
	let mut functions = HashMap::new();
	collect(&node, &mut functions);
	if functions.is_empty() {
		return node;
	}
	let counter = Cell::new(0);
	inline(node, &functions, &counter)
}

fn collect(node: &Node, functions: &mut HashMap<String, Inlinable>) {
	node.visit(&mut |part| {
		if let Some((name, function)) = inlinable(part) {
			let outside = names_outside(node, part);
			let locals_shared = function.names.iter().filter(|local| !function.parameters.contains(local)).any(|local| outside.contains(local));
			if !locals_shared {
				functions.insert(name, function);
			}
		}
	});
}

fn symbol_name(node: &Node) -> Option<&String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name),
		_ => None,
	}
}

fn inlinable(node: &Node) -> Option<(String, Inlinable)> {
	let Node::Key(head, Op::Define | Op::Assign, body) = node.drop_meta() else { return None };
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, parameters) = head_items.split_first()?;
	let name = symbol_name(name)?.clone();
	let parameters: Vec<String> = parameters.iter().map(|parameter| symbol_name(parameter).cloned()).collect::<Option<_>>()?;
	let mut statements = match body.drop_meta() {
		Node::List(items, Bracket::Curly, Separator::Semicolon | Separator::Newline) => items.clone(),
		_ => return None,
	};
	if statements.is_empty() || statements.len() > MAX_STATEMENTS {
		return None;
	}
	let result = match statements.pop()?.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && symbol_name(&items[0]).is_some_and(|word| word == RETURN) => items[1].clone(),
		_ => return None,
	};
	let mut names: HashSet<String> = parameters.iter().cloned().collect();
	let mut movable = true;
	for statement in statements.iter().chain(std::iter::once(&result)) {
		statement.visit(&mut |part| match part {
			Node::Key(target, Op::Assign, _) => match target.drop_meta() {
				Node::Symbol(local) => {
					names.insert(local.clone());
				}
				Node::Key(list, Op::Hash, _) if symbol_name(list).is_some() => {}
				_ => movable = false,
			},
			Node::Key(_, Op::Define | Op::FatArrow | Op::Arrow, _) => movable = false,
			Node::Key(_, op, _) if op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) => movable = false,
			Node::Symbol(word) if word == RETURN || *word == name || word == "global" => movable = false,
			_ => {}
		});
	}
	// every word is a parameter or a local, or a call's function name: a free variable would mean something else where
	// the body moves
	let mut called = HashSet::new();
	for statement in statements.iter().chain(std::iter::once(&result)) {
		statement.visit(&mut |part| {
			if let Node::List(items, Bracket::Round, _) = part {
				if let Some(function) = items.first().and_then(symbol_name) {
					called.insert(function.clone());
				}
			}
		});
	}
	for statement in statements.iter().chain(std::iter::once(&result)) {
		movable &= words(statement).iter().all(|word| names.contains(word) || called.contains(word));
	}
	movable.then_some((name, Inlinable { parameters, statements, result, names }))
}

/// The names a statement reads or writes, not the keys of its entries or the fields it reads
fn words(node: &Node) -> Vec<String> {
	match node.drop_meta() {
		Node::Symbol(word) => vec![word.clone()],
		Node::Key(_, Op::Colon, right) => words(right),
		Node::Key(left, Op::Dot, _) => words(left),
		Node::Key(left, _, right) => [words(left), words(right)].concat(),
		Node::List(items, _, _) => items.iter().flat_map(words).collect(),
		_ => vec![],
	}
}

/// The names the program uses outside the definition `except`: a local of an inlined body that is also one of them may
/// be a main-level variable or a global the body changes
fn names_outside(node: &Node, except: &Node) -> HashSet<String> {
	if std::ptr::eq(node, except) {
		return HashSet::new();
	}
	match node {
		Node::Symbol(word) => HashSet::from([word.clone()]),
		Node::Key(left, _, right) => names_outside(left, except).union(&names_outside(right, except)).cloned().collect(),
		Node::List(items, _, _) => items.iter().flat_map(|item| names_outside(item, except)).collect(),
		Node::Meta { node, .. } => names_outside(node, except),
		_ => HashSet::new(),
	}
}

fn inline(node: Node, functions: &HashMap<String, Inlinable>, counter: &Cell<usize>) -> Node {
	match node {
		// a definition keeps its own body
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)) => {
			Node::Key(head, op, Box::new(inline(*body, functions, counter)))
		}
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| inline(item, functions, counter)).collect();
			match inlined_call(&items, &bracket, &separator, functions, counter) {
				Some(inlined) => inlined,
				None => Node::List(items, bracket, separator),
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(inline(*left, functions, counter)), op, Box::new(inline(*right, functions, counter))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(inline(*node, functions, counter)), data },
		other => other,
	}
}

fn inlined_call(items: &[Node], bracket: &Bracket, separator: &Separator, functions: &HashMap<String, Inlinable>, counter: &Cell<usize>) -> Option<Node> {
	if *bracket != Bracket::Round || *separator != Separator::None {
		return None;
	}
	let (name, arguments) = items.split_first()?;
	let name = symbol_name(name)?;
	let function = functions.get(name)?;
	if arguments.len() != function.parameters.len() {
		return None;
	}
	counter.set(counter.get() + 1);
	let rename = |word: &str| format!("{name}{NAME_SEPARATOR}{word}{NAME_SEPARATOR}{}", counter.get());
	let renamed = |node: &Node| renamed_names(node.clone(), &function.names, &rename);
	let bindings = function.parameters.iter().zip(arguments).map(|(parameter, argument)| {
		Node::Key(Box::new(Node::Symbol(rename(parameter))), Op::Assign, Box::new(argument.clone()))
	});
	let body = bindings.chain(function.statements.iter().map(renamed)).chain(std::iter::once(renamed(&function.result)));
	Some(Node::List(body.collect(), Bracket::Round, Separator::Semicolon))
}

pub(crate) fn renamed_names(node: Node, names: &HashSet<String>, rename: &dyn Fn(&str) -> String) -> Node {
	match node {
		Node::Symbol(word) if names.contains(&word) => Node::Symbol(rename(&word)),
		// a key `{x: x}` and a field `p.x` keep their names
		Node::Key(left, op @ (Op::Colon | Op::Dot), right) if op == Op::Colon => Node::Key(left, op, Box::new(renamed_names(*right, names, rename))),
		Node::Key(left, Op::Dot, right) => Node::Key(Box::new(renamed_names(*left, names, rename)), Op::Dot, right),
		Node::Key(left, op, right) => Node::Key(Box::new(renamed_names(*left, names, rename)), op, Box::new(renamed_names(*right, names, rename))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| renamed_names(item, names, rename)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(renamed_names(*node, names, rename)), data },
		other => other,
	}
}

/// Is the name one of the inliner's renamed parameters or locals (`swap·arr·1`), which only its inlined body reads
pub fn is_temporary(name: &str) -> bool {
	let mut parts = name.rsplit(NAME_SEPARATOR);
	parts.next().is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())) && name.matches(NAME_SEPARATOR).count() >= 2
}
