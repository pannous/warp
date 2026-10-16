//! A library word is immutable for the types it is defined for (decisions library-words-typed and
//! dispatch-untyped-library-words, notes/dispatch.md). A program's untyped definition of one, `count(x) := 7`, becomes
//! `count·any` and covers the other values: a call is the library word for a value of the library's types and the
//! program's for any other, chosen at compile time when the value's kind is known, else at run time. A typed
//! redefinition for the library's types, `count(xs: list) := …`, is a loud error. Members of classes are free
//! (decision class-members-free): class_methods named them `count·Bag` before this pass.
//! Runs on the program only, before modules::resolve loads the library words it calls.

use super::library_signatures::{library_arity, library_parameter_types, library_types, typed_first_parameters};
use super::nodes::{call, function_head, if_then_else, key};
use crate::analyzer::type_word_kind;
use crate::errors::diagnostic::{Diagnostic, Level};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::words::{ANY_TYPE, LIST_WORD, NUMBER_WORD, TEXT_WORD};
use std::collections::{HashMap, HashSet};

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
	let shared = crate::lowering::shared_arrays::shared_names(&program).into_iter().collect();
	Dispatch { redefined, variables, shared }.rewrite(program)
}

/// A function the program defines: its name, arguments (fewest, most) and the type of its first parameter
struct Definition {
	name: String,
	arity: (usize, usize),
	first_type: Option<String>,
	node: Node,
}

fn definitions(program: &Node) -> Vec<Definition> {
	let mut found = vec![];
	program.visit(&mut |node| {
		if let Some(head) = function_head(node) {
			let parameters = head.parameters();
			let defaulted = parameters.iter().filter(|p| matches!(p.drop_meta(), Node::Key(_, Op::Assign, _))).count();
			let first_type = parameters.first().and_then(|first| crate::lowering::overloads::parameter_type(first));
			let arity = (parameters.len() - defaulted, parameters.len());
			found.push(Definition { name: head.name.to_string(), arity, first_type, node: node.clone() });
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
	definitions(program).into_iter().find_map(|Definition { name, arity, first_type, node: definition }| {
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
	for Definition { name, arity, first_type, node: definition } in definitions(program) {
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
	/// `shared xs = float[n]`: an array of the host's, no list the library's words change (shared_arrays.rs)
	shared: HashSet<String>,
}

impl Dispatch {
	fn rewrite(&self, node: Node) -> Node {
		if let Some(renamed) = self.renamed_definition(&node) {
			return renamed;
		}
		if let Some(dispatched) = self.dispatched_call(&node) {
			return dispatched;
		}
		if self.call_of(&node).is_some() {
			return self.kept_call(node);
		}
		match node {
			Node::Symbol(name) if self.redefined.contains_key(&name) => Node::Symbol(own_name(&name)),
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// `first(xs, 2)`: a call of a number of values only the library's word takes stays the library's
	fn kept_call(&self, node: Node) -> Node {
		match node {
			meta @ Node::Meta { .. } => meta.under_meta(|node| self.kept_call(node)),
			Node::List(items, bracket, separator) => {
				let mut items = items.into_iter();
				let head = items.next().into_iter();
				Node::List(head.chain(items.map(|item| self.rewrite(item))).collect(), bracket, separator)
			}
			Node::Key(receiver, Op::Dot, member) => {
				let member = match member.drop_meta() {
					Node::List(..) => self.kept_call(*member),
					_ => *member,
				};
				key(self.rewrite(*receiver), Op::Dot, member)
			}
			other => other,
		}
	}

	/// `count(x) := 7` → `count·any(x) := 7`, its body rewritten
	fn renamed_definition(&self, node: &Node) -> Option<Node> {
		let head = function_head(node)?;
		let name = head.name.to_string();
		self.redefined.get(&name)?;
		let mut renamed = false;
		let node = renamed_first(node.clone(), &name, &own_name(&name), &mut renamed);
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
		let arguments: Vec<Node> = arguments.into_iter().map(|argument| self.rewrite(argument)).collect();
		if arguments[0].drop_meta().symbol_name().is_some_and(|receiver| self.shared.contains(receiver)) {
			return Some(call(&own_name(&name), arguments));
		}
		Some(chosen_call(&name, &redefined.types, arguments, &self.variables))
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
}

/// The program's own word for a library word it defines untyped: `count·any`
fn own_name(name: &str) -> String {
	crate::traits::witness_name(name, ANY_TYPE)
}

/// `count` of the program's `count·any`, the library word it stands beside
pub fn library_word_of(own: &str) -> Option<&str> {
	own.strip_suffix(ANY_TYPE)?.strip_suffix(crate::traits::WITNESS_SEPARATOR)
}

/// A call of the library word `name` the program also defines untyped (`4 trimmed` of `trim(x) := x-1`), chosen by
/// its values like every other call of it
pub fn dispatched(name: &str, arguments: Vec<Node>) -> Node {
	chosen_call(name, &library_types(name), arguments, &HashMap::new())
}

/// The library's call when the values are of its types, else the program's: known at compile time, or tested at run
/// time by the receiver's type only (testing a function would call it)
fn chosen_call(
	name: &str,
	types: &[String],
	mut arguments: Vec<Node>,
	variables: &HashMap<String, Option<Kind>>,
) -> Node {
	let own = |arguments: Vec<Node>| call(&own_name(name), arguments);
	let misfits = (arguments.iter().zip(library_parameter_types(name)).skip(1))
		.filter_map(|(argument, parameter_type)| Some((argument, parameter_type?)))
		.any(|(argument, parameter_type)| known_fit(argument, &[parameter_type], variables) == Some(false));
	if misfits {
		return own(arguments);
	}
	match known_fit(&arguments[0], types, variables) {
		Some(true) => return library_call(name, arguments),
		Some(false) => return own(arguments),
		None => {}
	}
	let mut bindings = vec![];
	if !matches!(arguments[0].drop_meta(), Node::Symbol(_)) {
		let temporary = Node::Symbol(format!("{TESTED_VALUE}·{}", super::nodes::fresh_number()));
		bindings.push(key(temporary.clone(), Op::Assign, arguments[0].clone()));
		arguments[0] = temporary;
	}
	let tested = arguments[0].clone();
	let test = (types.iter())
		.filter_map(|type_name| crate::lowering::type_tests::declared_type_test(tested.clone(), type_name))
		.reduce(|either, test| key(either, Op::Or, test));
	let Some(test) = test else {
		return own(arguments);
	};
	let choice = if_then_else(test, library_call(name, arguments.clone()), own(arguments));
	match bindings.is_empty() {
		true => choice,
		false => Node::List([bindings, vec![choice]].concat(), Bracket::Round, Separator::Semicolon),
	}
}

/// `floor(x)`; a builtin method of a type that is no library function is called on its receiver, `t.add(n)`
fn library_call(name: &str, mut arguments: Vec<Node>) -> Node {
	match typed_first_parameters(name).is_some() || crate::library_words::is_library_word(name) {
		true => call(name, arguments),
		false => {
			let receiver = arguments.remove(0);
			super::nodes::method_call(receiver, name, arguments)
		}
	}
}

/// Whether a value of a kind known at compile time is of one of the types; None when only the run time knows
fn known_fit(value: &Node, types: &[String], variables: &HashMap<String, Option<Kind>>) -> Option<bool> {
	let kind = crate::lowering::overloads::argument_kind(value, variables)?;
	let known =
		kind.is_int() || kind.is_float() || matches!(kind, Kind::Text | Kind::Codepoint | Kind::List | Kind::Function);
	known.then(|| types.iter().any(|type_name| kind_fits(kind, type_name)))
}

/// Is a value of the kind of the type: `number` takes ints and floats, `text` codepoints too, `list` every list type,
/// `function` only functions
fn kind_fits(kind: Kind, type_name: &str) -> bool {
	match type_name {
		NUMBER_WORD => kind.is_int() || kind.is_float(),
		TEXT_WORD => matches!(kind, Kind::Text | Kind::Codepoint),
		list_type if crate::analyzer::names_list_type(list_type) => kind == Kind::List,
		_ => type_word_kind(type_name) == Some(kind) || kind.to_string() == type_name,
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
