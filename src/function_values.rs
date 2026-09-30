//! First-class functions, resolved at compile time. A function that takes a function as a parameter (`apply(f, x) := f(x)`) is
//! specialised for every function it is called with: `apply(double, 3)` calls `apply__double(3)`, whose body has `double` where
//! the parameter was. The functions passed are named functions, `&name`, aliases (`g = double`), operators (`+`) and lambdas that
//! capture no variable. A function that is not known at the call (chosen at run time) or a lambda that captures a variable stays the
//! error `functions are not first-class values yet`: that needs a funcref table and closures.

use crate::diagnostic::Diagnostic;
use crate::lambdas::{lambda_definition, NOT_FIRST_CLASS};
use crate::library_words::{collect_assigned_names, substitute};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const SPECIALISATION_SEPARATOR: &str = "__";
/// Words whose last argument is a function
const ITERATION_WORDS: [&str; 5] = ["map", "filter", "each", "fold", "reduce"];

/// A user function `name(params) := body` as found in the program
#[derive(Clone)]
struct Definition {
	name: String,
	params: Vec<Node>,
	body: Node,
}

impl Definition {
	fn from(node: &Node) -> Option<Definition> {
		let Node::Key(head, Op::Define, body) = node.drop_meta() else { return None };
		let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else { return None };
		let (name, params) = items.split_first()?;
		let Node::Symbol(name) = name.drop_meta() else { return None };
		Some(Definition { name: name.clone(), params: params.to_vec(), body: body.as_ref().clone() })
	}

	fn param_names(&self) -> Vec<String> {
		self.params.iter().filter_map(parameter_name).collect()
	}

	fn node(&self) -> Node {
		let head = Node::List([vec![Node::Symbol(self.name.clone())], self.params.clone()].concat(), Bracket::Round, Separator::None);
		Node::Key(Box::new(head), Op::Define, Box::new(self.body.clone()))
	}
}

fn parameter_name(param: &Node) -> Option<String> {
	match param.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, _, _) => parameter_name(name),
		_ => None,
	}
}

fn definitions(node: &Node, found: &mut Vec<Definition>) {
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
		Node::List(items, bracket, separator) => {
			if let Some((head, arguments)) = items.split_first() {
				// f(x), f x
				if let Some(name) = is_param(head) {
					let is_call = (*bracket == Bracket::Round && *separator == Separator::None) || (*bracket == Bracket::None && *separator == Separator::Space);
					if !arguments.is_empty() && is_call {
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

/// `f = double` and `f = &double` name a function again: the aliases and what they name
fn aliases(node: &Node, functions: &HashSet<String>, assigned: &mut HashMap<String, usize>, found: &mut HashMap<String, String>) {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Node::Symbol(name) = target.drop_meta() {
				*assigned.entry(name.clone()).or_default() += 1;
				if let Node::Symbol(function) = value.drop_meta() {
					let named = found.get(function).unwrap_or(function);
					if functions.contains(named) {
						found.insert(name.clone(), named.clone());
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

/// The alias assignments become ø and every use of the alias is the function
fn replace_aliases(node: Node, found: &HashMap<String, String>) -> Node {
	match node {
		Node::Symbol(name) => Node::Symbol(found.get(&name).cloned().unwrap_or(name)),
		Node::Key(target, Op::Assign | Op::Define, value) if matches!(target.drop_meta(), Node::Symbol(name) if found.contains_key(name)) && matches!(value.drop_meta(), Node::Symbol(_)) => {
			Node::Empty
		}
		Node::Key(left, op, right) => Node::Key(Box::new(replace_aliases(*left, found)), op, Box::new(replace_aliases(*right, found))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| replace_aliases(item, found)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(replace_aliases(*node, found)), data },
		other => other,
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
	counter: usize,
}

impl Specialising {
	/// The name of the function an argument stands for: a defined function, or a lambda or operator that captures no variable
	fn function_name(&mut self, argument: &Node) -> Option<String> {
		match argument.drop_meta() {
			Node::Symbol(name) if self.definitions.contains_key(name) && !self.higher_order.contains_key(name) => Some(name.clone()),
			_ => {
				self.counter += 1;
				let name = format!("lambda_value_{}", self.counter);
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
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
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
			let Some(target) = arguments.get(*index).and_then(|argument| self.function_name(argument)) else {
				return Some(Diagnostic::at(head, NOT_FIRST_CLASS).into_error());
			};
			targets.push(target);
		}
		let specialised = self.specialise(name, &targets);
		let remaining: Vec<Node> = arguments.iter().enumerate().filter(|(index, _)| !indexes.contains(index)).map(|(_, argument)| argument.clone()).collect();
		Some(Node::List([vec![Node::Symbol(specialised)], remaining].concat(), Bracket::Round, Separator::None))
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
					Some(definition) if specialising.higher_order.contains_key(&definition.name) => match specialising.specialised.get(&definition.name) {
						Some(made) => made.iter().map(|(_, specialised)| specialised.node()).collect(),
						None => vec![],
					},
					_ => vec![assemble(item, specialising)],
				})
				.collect();
			Node::List(items, bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(assemble(*left, specialising)), op, Box::new(assemble(*right, specialising))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(assemble(*node, specialising)), data },
		other => other,
	}
}

pub fn lower(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
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
	let mut variables = HashSet::new();
	collect_assigned_names(&program, &mut variables);
	let definitions_by_name: HashMap<String, Definition> = found.into_iter().map(|definition| (definition.name.clone(), definition)).collect();
	let mut specialising = Specialising {
		definitions: definitions_by_name,
		higher_order,
		variables,
		specialised: HashMap::new(),
		names: HashMap::new(),
		lambda_definitions: Vec::new(),
		counter: 0,
	};
	// calls in the code that is not the body of a function taking functions
	let program = rewrite_outside(program, &mut specialising);
	let assembled = assemble(program, &specialising);
	if specialising.lambda_definitions.is_empty() {
		return assembled;
	}
	Node::List([specialising.lambda_definitions.clone(), vec![assembled]].concat(), Bracket::Round, Separator::Semicolon)
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
		Node::Key(left, op, right) => Node::Key(Box::new(rewrite_outside(*left, specialising)), op, Box::new(rewrite_outside(*right, specialising))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite_outside(*node, specialising)), data },
		other => other,
	}
}
