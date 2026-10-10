//! First-class functions, resolved at compile time. A function that takes a function as a parameter (`apply(f, x) := f(x)`) is
//! specialised for every function it is called with: `apply(double, 3)` calls `apply__double(3)`, whose body has `double` where
//! the parameter was. The functions passed are named functions, `&name`, aliases (`g = double`), operators (`+`) and lambdas that
//! capture no variable. A function that is not known at the call (chosen at run time) or a lambda that captures a variable is passed
//! as a closure to a generic version of the function (closures.rs).

use super::nodes::{call, children_rewritten, key, parameter_name};
use crate::closures::may_be_function_value;
use crate::lambdas::lambda_definition;
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const SPECIALISATION_SEPARATOR: &str = "__";
/// The argument left out of a partial application: `add(1, _)`
const PARTIAL_HOLE: &str = "_";
/// `lambda_value_1`: the function a lambda argument becomes; no text the user wrote
pub(crate) const LAMBDA_PREFIX: &str = "lambda_value_";
/// Words whose last argument is a function
pub(crate) const ITERATION_WORDS: [&str; 5] = ["map", "filter", "each", "fold", "reduce"];
/// Words between two values that make a spaced list no call of its first item: `p in xs`
const INFIX_WORDS: [&str; 6] = ["in", "is", "of", "and", "or", "as"];

/// A user function `name(params) := body` as found in the program
#[derive(Clone)]
pub(crate) struct Definition {
	pub(crate) name: String,
	pub(crate) params: Vec<Node>,
	pub(crate) body: Node,
}

impl Definition {
	fn from(node: &Node) -> Option<Definition> {
		let Node::Key(head, Op::Define, body) = node.drop_meta() else { return None };
		let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else { return None };
		let (name, params) = items.split_first()?;
		let Node::Symbol(name) = name.drop_meta() else { return None };
		Some(Definition { name: name.clone(), params: params.to_vec(), body: body.as_ref().clone() })
	}

	pub(crate) fn param_names(&self) -> Vec<String> {
		self.params.iter().filter_map(parameter_name).collect()
	}

	fn node(&self) -> Node {
		let head = call(&self.name.clone(), self.params.clone());
		key(head, Op::Define, self.body.clone())
	}
}

/// `def make(){ def inc(){ 1 }; inc }; c = make(); c()`: make returns what inc returns (P82, a bare name is a call), so
/// calling that result can never work: a compile error that names the reference `function inc`
fn refuse_called_bare_results(program: &Node, found: &[Definition]) -> Result<(), Node> {
	let returning: HashMap<String, String> = found.iter().filter_map(|definition| Some((definition.name.clone(), nested_tail(definition, found)?))).collect();
	if returning.is_empty() {
		return Ok(());
	}
	let mut results: HashMap<String, String> = HashMap::new();
	collect_assigned_results(program, &returning, &mut results);
	let Some(call) = called_variable(program, &results) else { return Ok(()) };
	let variable = call_head(call);
	let function = &results[&variable];
	let nested = &returning[function];
	let message = format!("{variable} holds no function: {function} ends with `{nested}`, which calls {nested} (a bare name is a call); fix: return the function with `function {nested}`");
	Err(crate::diagnostic::Diagnostic::at(call, message).into_error())
}

/// `inc := add 1` of `add(a, b)`: a definition leaving out an argument is no partial application, which needs the hole
/// `inc := add(1, _)` (card arctan-allow, user 2026-10-10)
fn refused_partial_definition(program: &Node, found: &[Definition]) -> Option<Node> {
	let mut refused = None;
	program.visit(&mut |node| {
		let Node::Key(name, Op::Define, value) = node.drop_meta() else { return };
		let (Some(name), Node::List(items, Bracket::None | Bracket::Round, _)) = (name.symbol_name(), value.drop_meta()) else { return };
		let Some((function, arguments)) = items.split_first() else { return };
		let Some(definition) = found.iter().find(|definition| function.symbol_name() == Some(definition.name.as_str())) else { return };
		let has_hole = arguments.iter().any(|argument| argument.is_symbol(PARTIAL_HOLE));
		if refused.is_some() || has_hole || arguments.is_empty() || arguments.len() >= required_parameters(definition) {
			return;
		}
		let written: Vec<String> = arguments.iter().map(|argument| argument.serialize()).collect();
		let holes = vec![PARTIAL_HOLE.to_string(); definition.params.len() - arguments.len()];
		let partial = format!("{}({})", definition.name, [written.clone(), holes].concat().join(", "));
		let message = format!("{} needs {} arguments, {name} := {} {} leaves some out", definition.name, definition.params.len(), definition.name, written.join(" "));
		refused = Some(crate::diagnostic::Diagnostic::at(value, message).fix(format!("{name} := {partial}")).into_error());
	});
	refused
}

/// The parameters without a default: `add(a, b = 1)` needs only a
fn required_parameters(definition: &Definition) -> usize {
	definition.params.iter().filter(|parameter| !matches!(parameter.drop_meta(), Node::Key(_, Op::Assign, _))).count()
}

fn call_head(call: &Node) -> String {
	match call.drop_meta() {
		Node::List(items, _, _) => items[0].drop_meta().name(),
		other => other.name(),
	}
}

/// The function without parameters defined in the body whose bare name ends it, and that returns no function itself:
/// `def make(){ def inc(){ 1 }; inc }` → inc
fn nested_tail(definition: &Definition, found: &[Definition]) -> Option<String> {
	let Node::Symbol(name) = crate::closures::tail(&definition.body) else { return None };
	if references(&definition.body, name) {
		return None; // `function inc`: tail drops the Meta that marks the reference
	}
	let mut nested = Vec::new();
	definitions(&definition.body, &mut nested);
	let inner = nested.iter().find(|inner| inner.name == *name && inner.params.is_empty())?;
	let inner_tail = crate::closures::tail(&inner.body);
	let returns_function = crate::lambdas::arrow_lambda(inner_tail).is_some() || crate::closures::referenced_function(inner_tail).is_some()
		|| found.iter().any(|other| matches!(inner_tail.drop_meta(), Node::Symbol(called) if *called == other.name));
	(!returns_function).then(|| name.clone())
}

/// `function name` or `&name` somewhere in node
fn references(node: &Node, name: &str) -> bool {
	if crate::closures::referenced_function(node).is_some_and(|referenced| referenced == name) {
		return true;
	}
	match node {
		Node::Meta { node, .. } => references(node, name),
		Node::Key(left, _, right) => references(left, name) || references(right, name),
		Node::List(items, _, _) => items.iter().any(|item| references(item, name)),
		_ => false,
	}
}

/// `c = make()` of a function in `returning`: c → make
fn collect_assigned_results(node: &Node, returning: &HashMap<String, String>, results: &mut HashMap<String, String>) {
	if let Node::Key(target, Op::Assign, value) = node.drop_meta() {
		if let (Node::Symbol(variable), Node::List(items, Bracket::Round, Separator::None)) = (target.drop_meta(), value.drop_meta()) {
			if let [function] = items.as_slice() {
				let function = function.drop_meta().name();
				if returning.contains_key(&function) {
					results.insert(variable.clone(), function);
				}
			}
		}
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => {
			collect_assigned_results(left, returning, results);
			collect_assigned_results(right, returning, results);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_assigned_results(item, returning, results)),
		_ => {}
	}
}

/// The call of a variable of `results`: `c()`, `c(1)`
fn called_variable<'a>(node: &'a Node, results: &HashMap<String, String>) -> Option<&'a Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if results.contains_key(name)) => Some(node),
		Node::Key(left, _, right) => called_variable(left, results).or_else(|| called_variable(right, results)),
		Node::List(items, _, _) => items.iter().find_map(|item| called_variable(item, results)),
		_ => None,
	}
}

pub(crate) fn definitions(node: &Node, found: &mut Vec<Definition>) {
	if let Some(definition) = Definition::from(node) {
		found.push(definition);
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => {
			definitions(left, found);
			definitions(right, found);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| definitions(item, found)),
		_ => {}
	}
}

/// The parameters a body uses as functions: called (`f(x)`, `f x`), given to an iteration word, or passed to a parameter of another function that is one
fn function_parameters(definition: &Definition, higher_order: &HashMap<String, Vec<usize>>) -> Vec<usize> {
	let names = definition.param_names();
	let mut used: HashSet<String> = HashSet::new();
	collect_function_uses(&definition.body, &names, higher_order, &mut used);
	names.iter().enumerate().filter(|(_, name)| used.contains(*name)).map(|(index, _)| index).collect()
}

fn collect_function_uses(node: &Node, names: &[String], higher_order: &HashMap<String, Vec<usize>>, used: &mut HashSet<String>) {
	let is_param = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) if names.contains(name) => Some(name.clone()),
		_ => None,
	};
	match node.drop_meta() {
		// `switch c {cases}` parses as [switch, (c {cases})]: the subject and its cases, no call of c
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::switch::SWITCH_WORDS.contains(&word.as_str())) => {
			for item in &items[1..] {
				match item.drop_meta() {
					Node::List(pair, _, _) if pair.len() == 2 => pair.iter().for_each(|part| collect_function_uses(part, names, higher_order, used)),
					other => collect_function_uses(other, names, higher_order, used),
				}
			}
		}
		Node::List(items, bracket, separator) => {
			if let Some((head, arguments)) = items.split_first() {
				// f(x), f x
				if let Some(name) = is_param(head) {
					let is_call = (*bracket == Bracket::Round && *separator == Separator::None) || (*bracket == Bracket::None && *separator == Separator::Space);
					// `p in xs`, `p is int`: a word joining two values, no call of p
					let infix = matches!(arguments.first().map(Node::drop_meta), Some(Node::Symbol(word)) if INFIX_WORDS.contains(&word.as_str()));
					if !arguments.is_empty() && is_call && !infix {
						used.insert(name);
					}
				}
				if let Node::Symbol(word) = head.drop_meta() {
					// map xs f, fold xs 0 f
					if ITERATION_WORDS.contains(&word.as_str()) {
						if let Some(name) = arguments.last().and_then(is_param) {
							used.insert(name);
						}
					}
					// apply(f, x) where apply takes a function first
					if let Some(indexes) = higher_order.get(word) {
						for index in indexes {
							if let Some(name) = arguments.get(*index).and_then(is_param) {
								used.insert(name);
							}
						}
					}
				}
			}
			items.iter().for_each(|item| collect_function_uses(item, names, higher_order, used));
		}
		Node::Key(receiver, Op::Dot, method) => {
			if let Node::List(items, _, _) = method.drop_meta() {
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if ITERATION_WORDS.contains(&word.as_str())) {
					if let Some(name) = items.last().and_then(is_param) {
						used.insert(name);
					}
				}
			}
			collect_function_uses(receiver, names, higher_order, used);
			collect_function_uses(method, names, higher_order, used);
		}
		Node::Key(left, _, right) => {
			collect_function_uses(left, names, higher_order, used);
			collect_function_uses(right, names, higher_order, used);
		}
		_ => {}
	}
}

/// `f = &double`, `f = function double` and the definition `f := double` name a function again: the aliases and what
/// they name
fn aliases(node: &Node, functions: &HashSet<String>, assigned: &mut HashMap<String, usize>, found: &mut HashMap<String, String>) {
	match node.drop_meta() {
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
			if let Some(name) = aliased_name(target, op) {
				*assigned.entry(name.to_string()).or_default() += 1;
				// `g = function add` / `g = &add` (P83: a bare `g = add` needs add's arguments), `h = g` of an alias
				let function = match (crate::closures::referenced_function(value), bare_name(value, op)) {
					(Some(function), _) => Some(function),
					(None, Some(alias)) if found.contains_key(alias) => Some(alias.to_string()),
					// `arctan := arc_tangent` defines a name for the function (card arctan-allow)
					// a plain name only: `compute() := emit ask` reads `(on·ask)`, a call (card arctan-allow)
					(None, Some(function)) if *op == Op::Define && functions.contains(function) && matches!(value.drop_meta(), Node::Symbol(_)) => Some(function.to_string()),
					_ => None,
				};
				if let Some(function) = function {
					let named = found.get(&function).cloned().unwrap_or(function);
					if functions.contains(&named) && named != name {
						found.insert(name.to_string(), named);
					}
				}
			}
			aliases(value, functions, assigned, found);
		}
		Node::Key(left, _, right) => {
			aliases(left, functions, assigned, found);
			aliases(right, functions, assigned, found);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| aliases(item, functions, assigned, found)),
		_ => {}
	}
}

/// The name an assignment or definition binds: `g = …`, `g := …`, and `(g) := …`, as an earlier pass writes the
/// definition `g := f` of a name used later
fn aliased_name<'a>(target: &'a Node, op: &Op) -> Option<&'a str> {
	match target.drop_meta() {
		Node::Symbol(name) => Some(name),
		Node::List(items, Bracket::Round, _) if *op == Op::Define && items.len() == 1 => items[0].symbol_name(),
		_ => None,
	}
}

/// The function a definition names bare: `f`, or `(f)` as an earlier pass writes it in `h := g` (card alias-chain)
fn bare_name<'a>(value: &'a Node, op: &Op) -> Option<&'a str> {
	match value.drop_meta() {
		Node::Symbol(name) => Some(name),
		Node::List(items, Bracket::Round, _) if *op == Op::Define && items.len() == 1 => items[0].symbol_name(),
		_ => None,
	}
}

/// The alias assignments become ø and every use of the alias is the function
fn replace_aliases(node: Node, found: &HashMap<String, String>) -> Node {
	match node {
		Node::Symbol(name) => Node::Symbol(found.get(&name).cloned().unwrap_or(name)),
		Node::Key(target, op @ (Op::Assign | Op::Define), value) if aliased_name(&target, &op).is_some_and(|name| found.contains_key(name)) && bare_name(&value, &op).is_some() => {
			Node::Empty
		}
		// `(h)`, as an earlier pass calls the defined name h: the function's name, a call of it only without parameters
		Node::List(items, Bracket::Round, _) if items.len() == 1 && items[0].symbol_name().is_some_and(|name| found.contains_key(name)) => {
			items.into_iter().next().map_or(Node::Empty, |item| replace_aliases(item, found))
		}
		// `h 4` of the alias h: the call of its function, which the earlier passes didn't know h for (card alias-chain)
		Node::List(items, Bracket::None, Separator::Space) if items.len() > 1 && items[0].symbol_name().is_some_and(|name| found.contains_key(name)) => {
			let mut items = items.into_iter().map(|item| replace_aliases(item, found));
			let function = items.next().and_then(|function| function.symbol_name().map(String::from)).unwrap_or_default();
			call(&function, items.collect())
		}
		other => children_rewritten(other, |child| replace_aliases(child, found)),
	}
}

struct Specialising {
	definitions: HashMap<String, Definition>,
	/// Function name → the indexes of its parameters that are functions
	higher_order: HashMap<String, Vec<usize>>,
	variables: HashSet<String>,
	/// Specialisations made, by original function: (specialised name, definition)
	specialised: HashMap<String, Vec<(String, Definition)>>,
	names: HashMap<(String, Vec<String>), String>,
	lambda_definitions: Vec<Node>,
	/// Functions that are also called with closures: their generic version
	generic: HashMap<String, Definition>,
	counter: usize,
}

impl Specialising {
	/// The name of the function an argument stands for: a defined function, or a lambda or operator that captures no variable
	fn function_name(&mut self, argument: &Node) -> Option<String> {
		match argument.drop_meta() {
			Node::Symbol(name) if self.definitions.contains_key(name) && !self.higher_order.contains_key(name) => Some(name.clone()),
			_ => {
				self.counter += 1;
				let name = format!("{LAMBDA_PREFIX}{}", self.counter);
				let definition = lambda_definition(&name, argument)?;
				let parsed = Definition::from(&definition)?;
				let free = free_variables(&parsed);
				if free.iter().any(|variable| self.variables.contains(variable)) {
					return None;
				}
				self.lambda_definitions.push(definition);
				self.definitions.insert(name.clone(), parsed);
				Some(name)
			}
		}
	}

	fn rewrite(&mut self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.rewrite(item)).collect();
				self.specialised_call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Key(left, op, right) => key(self.rewrite(*left), op, self.rewrite(*right)),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	/// `apply(double, 3)` and `apply double 3`: the call of `apply__double` without the function
	fn specialised_call(&mut self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let (head, arguments) = items.split_first()?;
		let Node::Symbol(name) = head.drop_meta() else { return None };
		let indexes = self.higher_order.get(name)?.clone();
		if !(*bracket == Bracket::Round && *separator == Separator::None || *bracket == Bracket::None && *separator == Separator::Space) {
			return None;
		}
		let mut targets = Vec::new();
		for index in &indexes {
			let argument = arguments.get(*index).map(|argument| crate::lambdas::it_function(argument).unwrap_or_else(|| argument.clone()));
			let argument = argument.as_ref();
			let Some(target) = argument.and_then(|argument| self.function_name(argument)) else {
				// a function known only at run time is passed as a closure to the generic version (closures.rs)
				if argument.is_some_and(may_be_function_value) {
					self.make_generic(name);
					return None;
				}
				let param = self.definitions[name].param_names().get(*index).cloned().unwrap_or_default();
				let given = argument.cloned().unwrap_or(Node::Empty);
				return Some(crate::closures::needs_a_function(name, Some(&param), &given, "a lambda like x => …"));
			};
			targets.push(target);
		}
		let specialised = self.specialise(name, &targets);
		let remaining: Vec<Node> = arguments.iter().enumerate().filter(|(index, _)| !indexes.contains(index)).map(|(_, argument)| argument.clone()).collect();
		Some(call(&specialised, remaining))
	}

	/// The version of `name` that takes its functions as closures, made once: `f x` with a function parameter `f` is the call `f(x)`,
	/// which closures.rs makes a closure call
	fn make_generic(&mut self, name: &str) {
		if self.generic.contains_key(name) {
			return;
		}
		let original = self.definitions[name].clone();
		self.generic.insert(name.to_string(), original.clone());
		let function_params: Vec<String> = self.higher_order[name].iter().filter_map(|index| original.param_names().get(*index).cloned()).collect();
		let body = self.rewrite(round_calls(original.body.clone(), &function_params));
		self.generic.insert(name.to_string(), Definition { body, ..original });
	}

	/// The specialisation of `name` for the functions `targets` (one per function parameter), made once
	fn specialise(&mut self, name: &str, targets: &[String]) -> String {
		let key = (name.to_string(), targets.to_vec());
		if let Some(existing) = self.names.get(&key) {
			return existing.clone();
		}
		let specialised_name = format!("{name}{SPECIALISATION_SEPARATOR}{}", targets.join(SPECIALISATION_SEPARATOR));
		self.names.insert(key, specialised_name.clone());
		let original = self.definitions[name].clone();
		let indexes = self.higher_order[name].clone();
		let mut body = original.body.clone();
		for (index, target) in indexes.iter().zip(targets) {
			if let Some(param) = original.param_names().get(*index) {
				body = substitute(body, param, &Node::Symbol(target.clone()));
			}
		}
		let params: Vec<Node> = original.params.iter().enumerate().filter(|(index, _)| !indexes.contains(index)).map(|(_, param)| param.clone()).collect();
		// the body may call more functions with functions: rewritten in turn
		let body = self.rewrite(body);
		let definition = Definition { name: specialised_name.clone(), params, body };
		self.specialised.entry(name.to_string()).or_default().push((specialised_name.clone(), definition));
		specialised_name
	}
}

/// `f x` with a parameter `f` that is a function: the call `f(x)`
fn round_calls(node: Node, function_params: &[String]) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if items.len() > 1 && matches!(items[0].drop_meta(), Node::Symbol(name) if function_params.contains(name)) => {
			Node::List(items.into_iter().map(|item| round_calls(item, function_params)).collect(), Bracket::Round, Separator::None)
		}
		other => other.map_children(|child| round_calls(child, function_params)),
	}
}

/// Variables a function body reads that are not its parameters or a function of the program
fn free_variables(definition: &Definition) -> Vec<String> {
	let params = definition.param_names();
	let mut names = Vec::new();
	fn collect(node: &Node, names: &mut Vec<String>) {
		match node.drop_meta() {
			Node::Symbol(name) => names.push(name.clone()),
			Node::Key(left, _, right) => {
				collect(left, names);
				collect(right, names);
			}
			Node::List(items, _, _) => items.iter().for_each(|item| collect(item, names)),
			_ => {}
		}
	}
	collect(&definition.body, &mut names);
	names.retain(|name| !params.contains(name));
	names
}

/// Replace the definition of a function that takes functions by its specialisations; lambdas that became functions go first
fn assemble(node: Node, specialising: &Specialising) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items = items
				.into_iter()
				.flat_map(|item| match Definition::from(&item) {
					Some(definition) if specialising.higher_order.contains_key(&definition.name) => {
						let specialised = specialising.specialised.get(&definition.name).into_iter().flatten().map(|(_, specialised)| specialised.node());
						specialised.chain(specialising.generic.get(&definition.name).map(Definition::node)).collect()
					}
					_ => vec![assemble(item, specialising)],
				})
				.collect();
			Node::List(items, bracket, separator)
		}
		Node::Key(left, op, right) => key(assemble(*left, specialising), op, assemble(*right, specialising)),
		Node::Meta { node, data } => Node::Meta { node: Box::new(assemble(*node, specialising)), data },
		other => other,
	}
}

pub fn lower(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
	if let Err(error) = refuse_called_bare_results(&program, &found) {
		return error;
	}
	if let Some(error) = refused_partial_definition(&program, &found) {
		return error;
	}
	let functions: HashSet<String> = found.iter().map(|definition| definition.name.clone()).collect();
	let mut assigned_count = HashMap::new();
	let mut alias_of = HashMap::new();
	aliases(&program, &functions, &mut assigned_count, &mut alias_of);
	// an alias assigned once only
	alias_of.retain(|name, _| assigned_count.get(name) == Some(&1));
	let program = if alias_of.is_empty() { program } else { replace_aliases(program, &alias_of) };

	let mut found = Vec::new();
	definitions(&program, &mut found);
	let mut higher_order: HashMap<String, Vec<usize>> = HashMap::new();
	loop {
		let mut changed = false;
		for definition in &found {
			let indexes = function_parameters(definition, &higher_order);
			if !indexes.is_empty() && higher_order.get(&definition.name) != Some(&indexes) {
				higher_order.insert(definition.name.clone(), indexes);
				changed = true;
			}
		}
		if !changed {
			break;
		}
	}
	if higher_order.is_empty() {
		return program;
	}
	// a lambda reading one of these (a loop variable too) is a closure, not a function of its own
	let variables = crate::closures::captured_variables_of(&program);
	let definitions_by_name: HashMap<String, Definition> = found.into_iter().map(|definition| (definition.name.clone(), definition)).collect();
	let mut specialising = Specialising {
		definitions: definitions_by_name,
		higher_order,
		variables,
		specialised: HashMap::new(),
		names: HashMap::new(),
		lambda_definitions: Vec::new(),
		generic: HashMap::new(),
		counter: 0,
	};
	// calls in the code that is not the body of a function taking functions
	let program = rewrite_outside(program, &mut specialising);
	let assembled = assemble(program, &specialising);
	// the lambdas' definitions go first among the program's statements: nesting the program in a list of its own hid
	// its main level from the later passes (a handler's write to a main-level variable was refused)
	let program = match (specialising.lambda_definitions.is_empty(), assembled.drop_meta()) {
		(true, _) => assembled,
		(false, Node::List(statements, bracket, separator)) if crate::variable_signals::is_statement_list(bracket, separator) => {
			Node::List([specialising.lambda_definitions.clone(), statements.clone()].concat(), bracket.clone(), separator.clone())
		}
		(false, _) => Node::List([specialising.lambda_definitions.clone(), vec![assembled]].concat(), Bracket::Round, Separator::Semicolon),
	};
	// `f(*args)` of a parameter f spreads into the function now known (variadic.rs kept the spread)
	crate::variadic::lower(program)
}

/// Rewrite every call except inside the definitions of functions that take functions (those are only rewritten once specialised)
fn rewrite_outside(node: Node, specialising: &mut Specialising) -> Node {
	if let Some(definition) = Definition::from(&node) {
		if specialising.higher_order.contains_key(&definition.name) {
			return node;
		}
	}
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| rewrite_outside(item, specialising)).collect();
			match specialising.specialised_call(&items, &bracket, &separator) {
				Some(call) => call,
				None => Node::List(items, bracket, separator),
			}
		}
		Node::Key(left, op, right) => key(rewrite_outside(*left, specialising), op, rewrite_outside(*right, specialising)),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite_outside(*node, specialising)), data },
		other => other,
	}
}
