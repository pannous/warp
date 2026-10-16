//! A library word is immutable for the types it is defined for (decisions library-words-typed and
//! dispatch-untyped-library-words, notes/dispatch.md). A program's untyped definition of one, `count(x) := 7`, becomes
//! `count·any` and covers the other values: a call is the library word for a value of the library's types and the
//! program's for any other, chosen at compile time when the value's kind is known, else at run time. A typed
//! redefinition for the library's types, `count(xs: list) := …`, is a loud error. Members of classes are free
//! (decision class-members-free): class_methods named them `count·Bag` before this pass.
//! Runs on the program only, before modules::resolve loads the library words it calls.

use super::library_signatures::{library_arity, library_types};
use super::nodes::{call, function_head, if_then_else, key};
use crate::analyzer::type_word_kind;
use crate::errors::diagnostic::{Diagnostic, Level};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::words::{ANY_TYPE, LIST_WORD, NUMBER_WORD, TEXT_WORD};
use std::collections::HashMap;

/// The "got it" topic of the hint that a library word keeps its types
const TOPIC: &str = "library-words-typed";
/// The temporary holding a value tested at run time, numbered
const TESTED_VALUE: &str = "library·value";

/// A library word the program defines untyped: the library's types and the numbers of arguments the program's takes
struct Redefined {
	types: Vec<String>,
	arities: Vec<(usize, usize)>,
}

pub fn lower(program: Node) -> Node {
	if let Some(error) = typed_redefinition(&program) {
		return error;
	}
	let redefined = untyped_redefinitions(&program);
	if redefined.is_empty() {
		return program;
	}
	let variables = crate::analyzer::agreeing_kinds(&program, HashMap::new(), |_, value| {
		Some(crate::lowering::overloads::argument_kind(value, &HashMap::new()))
	});
	Dispatch { redefined, variables }.rewrite(program)
}

/// Each definition: its name, arguments (fewest, most) and the type of its first parameter
fn definitions(program: &Node) -> Vec<(String, (usize, usize), Option<String>, Node)> {
	let mut found = vec![];
	program.visit(&mut |node| {
		if let Some(head) = function_head(node) {
			let parameters = head.parameters();
			let defaulted = parameters.iter().filter(|p| matches!(p.drop_meta(), Node::Key(_, Op::Assign, _))).count();
			let first_type = parameters.first().and_then(|first| crate::lowering::overloads::parameter_type(first));
			let arity = (parameters.len() - defaulted, parameters.len());
			found.push((head.name.to_string(), arity, first_type, node.clone()));
		}
	});
	found
}

/// Does the library word take that many arguments; a word without a signature takes any number
fn library_takes(word: &str, arity: (usize, usize)) -> bool {
	library_arity(word).is_none_or(|(fewest, most)| fewest <= arity.1 && arity.0 <= most)
}

/// `count(xs: list) := …`: the loud error for redefining a library word for its own types
fn typed_redefinition(program: &Node) -> Option<Node> {
	definitions(program).into_iter().find_map(|(name, arity, first_type, definition)| {
		let first_type = first_type?;
		let types = library_types(&name);
		let owned = types.iter().find(|library_type| covers(library_type, &first_type))?;
		library_takes(&name, arity).then(|| {
			let reason = format!(
				"{name} of a {owned} is the library's word, which stays as it is (decision library-words-typed): name yours differently, e.g. my_{name}"
			);
			Diagnostic::at(&definition, reason).into_error()
		})
	})
}

/// Does the library's type cover a parameter type: `number` covers int and float, `list` every list type
fn covers(library_type: &str, parameter_type: &str) -> bool {
	library_type == parameter_type
		|| match library_type {
			NUMBER_WORD => type_word_kind(parameter_type).is_some_and(|kind| kind.is_int() || kind.is_float()),
			LIST_WORD => crate::analyzer::names_list_type(parameter_type),
			_ => false,
		}
}

/// The library words the program defines only untyped, each with the hint that the library keeps its types
fn untyped_redefinitions(program: &Node) -> HashMap<String, Redefined> {
	let mut redefined: HashMap<String, Redefined> = HashMap::new();
	let mut typed = vec![];
	for (name, arity, first_type, definition) in definitions(program) {
		let types = library_types(&name);
		if types.is_empty() || !library_takes(&name, arity) {
			continue;
		}
		if first_type.is_some() {
			typed.push(name);
			continue;
		}
		let reason = format!("{name} of a {} stays the library word; yours takes the other values", types.join(" or "));
		crate::errors::diagnostic::say(Diagnostic::at(&definition, reason).level(Level::Hint).about(TOPIC, &name));
		redefined.entry(name).or_insert_with(|| Redefined { types, arities: vec![] }).arities.push(arity);
	}
	// a name with typed definitions too is an overload set of its own (overloads.rs)
	redefined.retain(|name, _| !typed.contains(name));
	redefined
}

struct Dispatch {
	redefined: HashMap<String, Redefined>,
	variables: HashMap<String, Option<Kind>>,
}

impl Dispatch {
	fn own_name(name: &str) -> String {
		crate::traits::witness_name(name, ANY_TYPE)
	}

	fn rewrite(&self, node: Node) -> Node {
		if let Some(renamed) = self.renamed_definition(&node) {
			return renamed;
		}
		if let Some(dispatched) = self.dispatched_call(&node) {
			return dispatched;
		}
		match node {
			Node::Symbol(name) if self.redefined.contains_key(&name) => Node::Symbol(Self::own_name(&name)),
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// `count(x) := 7` → `count·any(x) := 7`, its body rewritten
	fn renamed_definition(&self, node: &Node) -> Option<Node> {
		let head = function_head(node)?;
		let name = head.name.to_string();
		self.redefined.get(&name)?;
		let mut renamed = false;
		let node = renamed_first(node.clone(), &name, &Self::own_name(&name), &mut renamed);
		Some(node.map_children(|child| self.rewrite(child)))
	}

	/// `count(v)`, `count v`, `v.count`, `v.count()`: the library's call for a value of its types, else the program's
	fn dispatched_call(&self, node: &Node) -> Option<Node> {
		let (name, arguments) = self.call_of(node)?;
		let redefined = self.redefined.get(&name)?;
		let count = arguments.len();
		if !redefined.arities.iter().any(|(fewest, most)| (*fewest..=*most).contains(&count)) {
			return None;
		}
		let mut arguments: Vec<Node> = arguments.into_iter().map(|argument| self.rewrite(argument)).collect();
		let own = |arguments: Vec<Node>| call(&Self::own_name(&name), arguments);
		let library = |arguments: Vec<Node>| call(&name, arguments);
		match self.known_fit(&arguments[0], &redefined.types) {
			Some(true) => return Some(library(arguments)),
			Some(false) => return Some(own(arguments)),
			None => {}
		}
		let mut bindings = vec![];
		if !matches!(arguments[0].drop_meta(), Node::Symbol(_)) {
			let temporary = Node::Symbol(format!("{TESTED_VALUE}·{}", super::nodes::fresh_number()));
			bindings.push(key(temporary.clone(), Op::Assign, arguments[0].clone()));
			arguments[0] = temporary;
		}
		let test = (redefined.types.iter())
			.filter_map(|type_name| crate::lowering::type_tests::declared_type_test(arguments[0].clone(), type_name))
			.reduce(|either, test| key(either, Op::Or, test))?;
		let choice = if_then_else(test, library(arguments.clone()), own(arguments));
		Some(match bindings.is_empty() {
			true => choice,
			false => Node::List([bindings, vec![choice]].concat(), Bracket::Round, Separator::Semicolon),
		})
	}

	/// The word called and its arguments, the receiver first
	fn call_of(&self, node: &Node) -> Option<(String, Vec<Node>)> {
		match node.drop_meta() {
			Node::List(items, Bracket::Round, _) | Node::List(items, Bracket::None, Separator::Space) => {
				let (head, arguments) = items.split_first()?;
				let name = head.drop_meta().symbol_name()?;
				(!arguments.is_empty() && self.redefined.contains_key(name))
					.then(|| (name.to_string(), arguments.to_vec()))
			}
			Node::Key(receiver, Op::Dot, member) => {
				let (name, rest) = match member.drop_meta() {
					Node::Symbol(name) => (name.as_str(), vec![]),
					Node::List(items, Bracket::Round, _) => {
						(items.first()?.drop_meta().symbol_name()?, items[1..].to_vec())
					}
					_ => return None,
				};
				self.redefined
					.contains_key(name)
					.then(|| (name.to_string(), [vec![receiver.as_ref().clone()], rest].concat()))
			}
			_ => None,
		}
	}

	/// Whether a value of a kind known at compile time is of the library's types; None when only the run time knows
	fn known_fit(&self, value: &Node, types: &[String]) -> Option<bool> {
		let kind = crate::lowering::overloads::argument_kind(value, &self.variables)?;
		let type_name = match kind {
			kind if kind.is_int() || kind.is_float() => NUMBER_WORD,
			Kind::Text | Kind::Codepoint => TEXT_WORD,
			Kind::List => LIST_WORD,
			_ => return None,
		};
		Some(types.iter().any(|library_type| library_type == type_name))
	}
}

/// The node with its first symbol `from` (in reading order) named `to`: the name in a definition's head
fn renamed_first(node: Node, from: &str, to: &str, renamed: &mut bool) -> Node {
	if *renamed {
		return node;
	}
	match node {
		Node::Symbol(name) if name == from => {
			*renamed = true;
			Node::Symbol(to.to_string())
		}
		other => other.map_children(|child| renamed_first(child, from, to, renamed)),
	}
}
