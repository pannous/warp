//! What the program takes for itself, one answer for every pass (card one-dispatch, notes/dispatch.md "The rule",
//! levels 0 and 2): a word is the program's where one of its bindings is in scope. Everywhere: its functions, at the
//! numbers of arguments they take, and the variables it assigns at the top level. Inside a function: also its
//! parameters and the names its body assigns. A field of an object literal takes no bare word.
//! Bindings: every name the program gives anywhere and how (twin: lib/compiler/scope.warp), one walk that knows every
//! binding form: `x = v` a variable (a function when v is one), `f(a) := …` / `def f(a) {…}` a function and its
//! parameters, `x => …` a parameter, `for x in …` a loop variable, `a, b = …` destructured variables, `{k: v}` a field.

use super::nodes::{for_in_variable, function_head, parameter_name, FunctionHead};
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Default)]
pub struct ProgramNames {
	/// Each function and the numbers of arguments its definitions take, fewest and most
	functions: HashMap<String, Vec<(usize, usize)>>,
	/// The variables bound where the names are asked: the top level's, a function's inside it
	bound: HashSet<String>,
}

impl ProgramNames {
	pub fn of(program: &Node) -> Self {
		let mut functions: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
		program.visit(&mut |part| {
			if let Some(head) = function_head(part) {
				functions.entry(head.name.to_string()).or_default().push(head.arity());
			}
		});
		let mut bound = HashSet::new();
		crate::type_tests::collect_outer_assigned_names(program, &mut bound);
		bound.extend(outer_loop_variables(program));
		bound.retain(|name| !functions.contains_key(name));
		ProgramNames { functions, bound }
	}

	/// Does the program take `word` here, as a function or a variable
	pub fn takes(&self, word: &str) -> bool {
		self.bound.contains(word) || self.functions.contains_key(word)
	}

	/// Does a variable or parameter here take `word`
	pub fn binds(&self, word: &str) -> bool {
		self.bound.contains(word)
	}

	/// Does the program take the call of `word` with that many arguments: a variable of that name, or a function
	/// taking them; `first(x) := …` leaves `first(xs, 3)` to the library
	pub fn takes_call(&self, word: &str, arguments: usize) -> bool {
		self.binds(word) || self.function_takes(word, arguments) == Some(true)
	}

	/// The uniform call: is `x.word(args)` the program's `word(x, args)`, a function of it taking args + 1 values
	pub fn takes_method_call(&self, word: &str, arguments: usize) -> bool {
		self.function_takes(word, arguments + 1) == Some(true)
	}

	/// Does the program's function `word` take that many arguments; None when the program defines no function `word`
	pub fn function_takes(&self, word: &str, arguments: usize) -> Option<bool> {
		let arities = self.functions.get(word)?;
		Some(arities.iter().any(|(fewest, most)| (*fewest..=*most).contains(&arguments)))
	}

	/// The names in scope inside `node` when it defines a function: its parameters and the names its body assigns
	pub fn inside(&self, node: &Node) -> Option<ProgramNames> {
		let head = function_head(node)?;
		let mut inner = self.clone();
		inner.bound.extend(bound_in_function(&head));
		Some(inner)
	}
}

/// The names any function of the program binds inside (bound_in_function)
pub fn bound_in_functions(program: &Node) -> HashSet<String> {
	let mut bound = HashSet::new();
	program.visit(&mut |part| bound.extend(function_head(part).iter().flat_map(bound_in_function)));
	bound
}

/// The names a function binds inside: its parameters, the names its body assigns and its loop variables
fn bound_in_function(head: &FunctionHead) -> HashSet<String> {
	let mut bound: HashSet<String> = head.parameters().into_iter().filter_map(parameter_name).collect();
	crate::library_words::collect_assigned_names(head.body, &mut bound);
	bound.extend(crate::for_loop::loop_variables(head.body));
	bound
}

/// `rewrite` of a pass giving `word` its library or host meaning, applied where the program leaves the word free:
/// nowhere when the program takes it everywhere, and not inside a function binding it (`f(x) := { random = 4; … }`
/// keeps its own random, while the rest of the program has the host's)
pub fn rewritten_where_free(program: Node, word: &str, rewrite: &dyn Fn(Node) -> Node) -> Node {
	let names = ProgramNames::of(&program);
	where_free(program, &names, word, rewrite)
}

/// `rewrite` of every node bottom-up, given the names the program takes where the node stands
pub fn rewritten_by_scope(program: Node, rewrite: &dyn Fn(Node, &ProgramNames) -> Node) -> Node {
	let names = ProgramNames::of(&program);
	by_scope(program, &names, rewrite)
}

fn by_scope(node: Node, names: &ProgramNames, rewrite: &dyn Fn(Node, &ProgramNames) -> Node) -> Node {
	let inner = names.inside(&node);
	let names = inner.as_ref().unwrap_or(names);
	rewrite(node.map_children(|child| by_scope(child, names, rewrite)), names)
}

fn where_free(node: Node, names: &ProgramNames, word: &str, rewrite: &dyn Fn(Node) -> Node) -> Node {
	if names.takes(word) {
		return node;
	}
	if !binds_inside(&node, word) {
		return rewrite(node);
	}
	let inner = names.inside(&node);
	let names = inner.as_ref().unwrap_or(names);
	match inner.as_ref().map(|inner| inner.takes(word)) {
		Some(true) => node,
		_ => node.map_children(|child| where_free(child, names, word, rewrite)),
	}
}

/// Does a function inside `node` (itself too) bind `word`, which the names outside leave free
fn binds_inside(node: &Node, word: &str) -> bool {
	let mut binds = false;
	node.visit(&mut |part| binds |= function_head(part).is_some_and(|head| bound_in_function(&head).contains(word)));
	binds
}

/// `x = 1` of a parameter `x = 1`: a default value, which may be left out
/// The loop variables outside function bodies
fn outer_loop_variables(node: &Node) -> Vec<String> {
	if function_head(node).is_some() {
		return vec![];
	}
	let mut variables = match node.drop_meta() {
		Node::List(items, _, _) => super::nodes::for_in_variable(items).into_iter().collect(),
		_ => vec![],
	};
	variables.extend(node.drop_meta().parts().into_iter().flat_map(outer_loop_variables));
	variables
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Meaning {
	Variable,
	Function,
	Parameter,
	Loop,
	Field,
}

impl Meaning {
	/// How messages name it: "variable", "loop variable", …
	pub fn word(self) -> &'static str {
		match self {
			Meaning::Variable => "variable",
			Meaning::Function => "function",
			Meaning::Parameter => "parameter",
			Meaning::Loop => "loop variable",
			Meaning::Field => "field",
		}
	}
}

/// Each name with its meanings, in the order the program first gives them
#[derive(Default)]
pub struct Bindings {
	meanings: BTreeMap<String, Vec<Meaning>>,
}

impl Bindings {
	pub fn of(program: &Node) -> Bindings {
		let mut bindings = Bindings::default();
		program.visit(&mut |node| bindings.remember_bindings(node));
		bindings
	}

	/// Bound as anything but a field: a field binds its name only inside its object
	pub fn binds(&self, name: &str) -> bool {
		self.meaning_of(name).is_some()
	}

	pub fn binds_as(&self, name: &str, meaning: Meaning) -> bool {
		self.meanings.get(name).is_some_and(|meanings| meanings.contains(&meaning))
	}

	/// The names bound as anything but a field
	pub fn bound(&self) -> HashSet<String> {
		self.meanings.keys().filter(|name| self.binds(name)).cloned().collect()
	}

	/// Every name the program gives, fields too
	pub fn given(&self) -> HashSet<String> {
		self.meanings.keys().cloned().collect()
	}

	/// The first meaning a name is bound with, a field's aside
	pub fn meaning_of(&self, name: &str) -> Option<Meaning> {
		self.meanings.get(name)?.iter().copied().find(|meaning| *meaning != Meaning::Field)
	}

	/// Each name with each of its meanings, in the order the program gives them
	pub fn meanings(&self) -> impl Iterator<Item = (&str, Meaning)> {
		self.meanings.iter().flat_map(|(name, meanings)| meanings.iter().map(move |meaning| (name.as_str(), *meaning)))
	}

	fn remember(&mut self, name: impl Into<String>, meaning: Meaning) {
		let meanings = self.meanings.entry(name.into()).or_default();
		if !meanings.contains(&meaning) {
			meanings.push(meaning);
		}
	}

	fn remember_bindings(&mut self, node: &Node) {
		if let Some(head) = function_head(node).filter(|head| !is_data_head(head.head)) {
			self.remember(head.name, Meaning::Function);
			for parameter in head.parameters() {
				self.remember_all(destructured_parameter(parameter), Meaning::Parameter);
				self.remember_all(parameter_name(parameter), Meaning::Parameter);
			}
		}
		match node {
			// `x = v`, `x: int = v`, `x += 1`
			Node::Key(target, op, value) if op.is_write() || *op == Op::Define => {
				if let Some(name) = typed_name(target) {
					let holds_function = crate::lowering::lambdas::arrow_lambda(value).is_some()
						|| crate::closures::referenced_function(value).is_some();
					self.remember(name, if holds_function { Meaning::Function } else { Meaning::Variable });
				}
			}
			// `Some(v) => …`, a match arm's call pattern, binds no parameter named Some
			Node::Key(parameters, Op::Arrow, _) if !is_call_pattern(parameters) => {
				let names: Vec<String> = match parameters.drop_meta() {
					Node::List(items, _, _) => items.iter().filter_map(parameter_name).collect(),
					other => parameter_name(other).into_iter().collect(),
				};
				self.remember_all(names, Meaning::Parameter);
			}
			Node::List(items, bracket, _) => {
				self.remember_all(for_in_variable(items), Meaning::Loop);
				// `for x` of a loop still written as a phrase
				let written_loops = items.windows(2).filter(|pair| pair[0].is_symbol(crate::words::FOR_WORD));
				self.remember_all(
					written_loops.filter_map(|pair| pair[1].symbol_name().map(String::from)),
					Meaning::Loop,
				);
				self.remember_all(crate::lowering::for_loop::step_variables(items), Meaning::Loop);
				if *bracket == Bracket::Curly {
					self.remember_all(items.iter().filter_map(field_name), Meaning::Field);
				}
			}
			_ => {}
		}
		self.remember_all(crate::tuples::destructured_names(node), Meaning::Variable);
	}

	fn remember_all(&mut self, names: impl IntoIterator<Item = String>, meaning: Meaning) {
		names.into_iter().for_each(|name| self.remember(name, meaning));
	}
}

/// `[a, b] = …` and `{a: 1} = …` are no function heads
fn is_data_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(_, Bracket::Square | Bracket::Curly, _))
}

/// `Some(v)`: a name glued to its group
fn is_call_pattern(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, crate::node::Separator::None) if items.len() > 1)
}

/// `x` of `x` and of `x: int`
fn typed_name(target: &Node) -> Option<&str> {
	match target.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.drop_meta().symbol_name(),
		other => other.symbol_name(),
	}
}

/// `k` of the entry `k: v`
fn field_name(entry: &Node) -> Option<String> {
	match entry.drop_meta() {
		Node::Key(key, Op::Colon, _) => key.symbol_name().map(String::from),
		_ => None,
	}
}

/// `width` and `h` of a destructuring parameter `{width, height: h}`
fn destructured_parameter(parameter: &Node) -> Vec<String> {
	let Node::List(entries, Bracket::Curly, _) = parameter.drop_meta() else { return vec![] };
	let name = |entry: &Node| match entry.drop_meta() {
		Node::Key(_, Op::Colon, name) => name.drop_meta().symbol_name().map(String::from),
		other => other.symbol_name().map(String::from),
	};
	entries.iter().filter_map(name).collect()
}
