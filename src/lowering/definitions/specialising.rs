//! A function passed by name to a higher-order function gets a copy of that function made for it, which calls it by
//! name: `each(f, n) := { … f(i, i + 1) … }; each(ink, 3000000)` adds `each·ink(n) := { … ink(i, i + 1) … }` and calls
//! `each·ink(3000000)`. Called as a value its parameters would lose their types (a closure call, 2–3× slower).
//! Only top-level functions defined once, whose parameter is only called or passed on, and only names no assignment,
//! parameter or one-parameter lambda binds too (a loop variable or a destructured name of the same name is not looked
//! for: a call's head is the function anyway); a copy's own calls are specialised in turn (`fold(f, xs) := … fold(f,
//! rest)`), up to a cap.

use super::inlining::renamed_names;
use super::nodes::{children_rewritten, key};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The most copies one program gets
const MAX_COPIES: usize = 64;
const NAME_SEPARATOR: &str = "·";

#[derive(Clone)]
struct Definition {
	name: String,
	params: Vec<String>,
	body: Node,
	op: Op,
}

impl Definition {
	fn from(node: &Node) -> Option<Definition> {
		let Node::Key(head, op @ (Op::Define | Op::Assign), body) = node.drop_meta() else {
			return None;
		};
		let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else {
			return None;
		};
		let (name, params) = items.split_first()?;
		let params = params.iter().map(|param| param.symbol_name().map(String::from)).collect::<Option<_>>()?;
		Some(Definition { name: name.symbol_name()?.to_string(), params, body: body.as_ref().clone(), op: *op })
	}

	fn node(&self) -> Node {
		let head = [Node::Symbol(self.name.clone())].into_iter().chain(self.params.iter().cloned().map(Node::Symbol));
		key(Node::List(head.collect(), Bracket::Round, Separator::None), self.op, self.body.clone())
	}

	/// The copy of this function for the function `passed` given as its parameter at `index`
	fn specialised(&self, index: usize, passed: &str) -> Definition {
		let mut params = self.params.clone();
		let param = params.remove(index);
		Definition {
			name: copy_name(&self.name, passed),
			params,
			body: renamed_names(self.body.clone(), &HashSet::from([param]), &|_| passed.to_string()),
			op: self.op,
		}
	}

	/// Is the parameter at `index` one the body only calls or passes on
	fn takes_function_at(&self, index: usize) -> bool {
		let mut used = false;
		let mut other_use = false;
		visit_uses(&self.body, &self.params[index], &mut |use_| match use_ {
			Use::Called | Use::Passed => used = true,
			Use::Other => other_use = true,
		});
		used && !other_use
	}
}

fn copy_name(function: &str, passed: &str) -> String {
	// the program's `each·any` beside the library's each copies to `each·ink`, the name as written
	format!("{}{NAME_SEPARATOR}{passed}", crate::lowering::typed_library_words::written(function))
}

enum Use {
	Called,
	Passed,
	Other,
}

fn visit_uses(node: &Node, name: &str, found: &mut dyn FnMut(Use)) {
	match node.drop_meta() {
		Node::Symbol(word) if word == name => found(Use::Other),
		Node::List(items, Bracket::Round, Separator::None) if items.first().and_then(Node::symbol_name).is_some() => {
			if items[0].symbol_name() == Some(name) {
				found(Use::Called);
			}
			for argument in &items[1..] {
				match argument.drop_meta() {
					Node::Symbol(word) if word == name => found(Use::Passed),
					_ => visit_uses(argument, name, found),
				}
			}
		}
		Node::Key(left, _, right) => {
			visit_uses(left, name, found);
			visit_uses(right, name, found);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| visit_uses(item, name, found)),
		_ => {}
	}
}

/// Bound other than by a definition: assigned a value, a parameter, a loop variable
fn bound_as_value(bindings: &crate::program_names::Bindings, name: &str) -> bool {
	use crate::program_names::Meaning::{Loop, Parameter, Variable};
	[Variable, Parameter, Loop].into_iter().any(|meaning| bindings.binds_as(name, meaning))
}

struct Specialiser {
	definitions: HashMap<String, Definition>,
	/// The functions a call may pass by name: defined once at the top level, their names bound nowhere else
	passable: HashSet<String>,
	/// The copies made, in order, each after the function it copies
	copies: Vec<(String, Definition)>,
}

impl Specialiser {
	fn rewrite(&mut self, node: Node) -> Node {
		match node {
			Node::List(items, Bracket::Round, Separator::None) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.rewrite(item)).collect();
				self.specialised_call(items)
			}
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}

	/// `each(ink, n)` as `each·ink(n)`, making that copy once
	fn specialised_call(&mut self, mut items: Vec<Node>) -> Node {
		loop {
			let Some((index, copy)) = self.copy_for(&items) else {
				return Node::List(items, Bracket::Round, Separator::None);
			};
			items.remove(index + 1);
			items[0] = Node::Symbol(copy);
		}
	}

	fn copy_for(&mut self, items: &[Node]) -> Option<(usize, String)> {
		let (head, arguments) = items.split_first()?;
		let function = self.definitions.get(head.symbol_name()?)?.clone();
		if arguments.len() != function.params.len() {
			return None;
		}
		let (index, passed) = arguments.iter().enumerate().find_map(|(index, argument)| {
			let passed = argument.symbol_name().filter(|name| self.passable.contains(*name))?;
			let free = !function.params.iter().any(|param| param == passed);
			(free && function.takes_function_at(index)).then(|| (index, passed.to_string()))
		})?;
		let name = copy_name(&function.name, &passed);
		if !self.definitions.contains_key(&name) {
			if self.copies.len() >= MAX_COPIES {
				return None;
			}
			let copy = function.specialised(index, &passed);
			let original = self.original_of(&function.name);
			self.definitions.insert(name.clone(), copy.clone());
			self.copies.push((original, copy));
		}
		Some((index, name))
	}

	/// The top-level function a copy (of a copy) was made from: its copies follow it in the program
	fn original_of(&self, name: &str) -> String {
		self.copies
			.iter()
			.find(|(_, copy)| copy.name == name)
			.map_or(name.to_string(), |(original, _)| original.clone())
	}
}

/// The statements of a statement list, those of the lists nested in it too (`f(x) := …; g(x) := …` on one line); a
/// block's own definitions stay its own
fn statements_of(node: &Node) -> Vec<&Node> {
	match node.drop_meta() {
		Node::List(items, bracket, Separator::Semicolon | Separator::Newline) if *bracket != Bracket::Curly => {
			items.iter().flat_map(statements_of).collect()
		}
		_ => vec![node],
	}
}

/// Each copy right after the definition of the function it copies
fn with_copies(node: Node, copies: &[(String, Definition)]) -> Node {
	match node {
		Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline))
			if bracket != Bracket::Curly =>
		{
			let mut statements = vec![];
			for item in items {
				let defined = Definition::from(&item).map(|definition| definition.name);
				statements.push(with_copies(item, copies));
				if let Some(name) = defined {
					let made = copies.iter().filter(|(original, _)| *original == name);
					statements.extend(made.map(|(_, copy)| copy.node()));
				}
			}
			Node::List(statements, bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_copies(*node, copies)), data },
		other => other,
	}
}

pub fn lower(program: Node) -> Node {
	let found: Vec<Definition> = statements_of(&program).into_iter().filter_map(Definition::from).collect();
	let mut counts: HashMap<&str, usize> = HashMap::new();
	for definition in &found {
		*counts.entry(&definition.name).or_default() += 1;
	}
	let bindings = crate::program_names::Bindings::of(&program);
	let once: Vec<&Definition> = found.iter().filter(|definition| counts[definition.name.as_str()] == 1).collect();
	let passable =
		once.iter().map(|definition| definition.name.clone()).filter(|name| !bound_as_value(&bindings, name)).collect();
	let higher_order: HashMap<String, Definition> = once
		.into_iter()
		.filter(|definition| (0..definition.params.len()).any(|index| definition.takes_function_at(index)))
		.map(|definition| (definition.name.clone(), definition.clone()))
		.collect();
	if higher_order.is_empty() {
		return program;
	}
	let mut specialiser = Specialiser { definitions: higher_order, passable, copies: vec![] };
	let rewritten = specialiser.rewrite(program.clone());
	if specialiser.copies.is_empty() {
		return program;
	}
	// the copies' bodies are rewritten until no new copy appears (a copy calling itself, or another function by name)
	let mut done = 0;
	while done < specialiser.copies.len() {
		let (original, copy) = specialiser.copies[done].clone();
		let body = specialiser.rewrite(copy.body);
		specialiser.copies[done] = (original, Definition { body, ..copy });
		done += 1;
	}
	with_copies(rewritten, &specialiser.copies)
}
