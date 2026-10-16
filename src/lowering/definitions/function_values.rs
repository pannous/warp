//! First-class function forms, made values closures.rs passes: a function that takes a function as a parameter
//! (`apply(f, x) := f(x)`) calls it round (`f x` is `f(x)`), and its calls give it a function value: a named function,
//! `&name`, an alias (`g := double`), a lambda, an `it` expression, an operator (`+`), a block or a library word (both
//! become functions of their own). A constant where a function is needed is a compile error. Every call passes a closure
//! (closures.rs; one that captures nothing is made once, wasm_emitter/functions/closures.rs).

use super::nodes::{call, children_rewritten, key, parameter_name, word_headed};
use crate::closures::may_be_function_value;
use crate::lambdas::lambda_definition;
use crate::library_words::{arity, canonical_spelling, is_standard_function};
use crate::modules::prelude_word_arity;
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The argument left out of a partial application: `add(1, _)`
const PARTIAL_HOLE: &str = "_";
/// `lambda_value_1`: the function a lambda argument becomes; no text the user wrote
pub(crate) const LAMBDA_PREFIX: &str = "lambda_value_";
/// Words whose last argument is a function
pub(crate) const ITERATION_WORDS: [&str; 5] = ["map", "filter", "each", "fold", "reduce"];
/// `argument·1`: a parameter of the function a library word given as a function becomes; no name the user writes
const LIBRARY_WORD_PARAMETER: &str = "argument·";

/// A user function `name(params) := body` as found in the program
#[derive(Clone)]
pub(crate) struct Definition {
	pub(crate) name: String,
	pub(crate) params: Vec<Node>,
	pub(crate) body: Node,
}

impl Definition {
	fn from(node: &Node) -> Option<Definition> {
		let Node::Key(head, Op::Define, body) = node.drop_meta() else {
			return None;
		};
		let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else {
			return None;
		};
		let (name, params) = word_headed(items)?;
		Some(Definition { name: name.to_string(), params: params.to_vec(), body: body.as_ref().clone() })
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
	let returning: HashMap<String, String> = found
		.iter()
		.filter_map(|definition| Some((definition.name.clone(), nested_tail(definition, found)?)))
		.collect();
	if returning.is_empty() {
		return Ok(());
	}
	let mut results: HashMap<String, String> = HashMap::new();
	collect_assigned_results(program, &returning, &mut results);
	let Some(call) = called_variable(program, &results) else {
		return Ok(());
	};
	let variable = call_head(call);
	let function = &results[&variable];
	let nested = &returning[function];
	let message = format!("{variable} holds no function: {function} ends with `{nested}`, which calls {nested} (a bare name is a call); fix: return the function with `function {nested}`");
	Err(crate::errors::diagnostic::Diagnostic::at(call, message).into_error())
}

/// `inc := add 1` of `add(a, b)`: a definition leaving out an argument is no partial application, which needs the hole
/// `inc := add(1, _)` (card arctan-allow, user 2026-10-10)
fn refused_partial_definition(program: &Node, found: &[Definition]) -> Option<Node> {
	let mut refused = None;
	program.visit(&mut |node| {
		let Node::Key(name, Op::Define, value) = node.drop_meta() else {
			return;
		};
		// an unused `inc := add 1` arrives as the parameterless function `(inc) := add 1`
		let parameterless = || {
			crate::lowering::nodes::function_head(node)
				.filter(|head| head.parameters().is_empty())
				.map(|head| head.name)
		};
		let (Some(name), Node::List(items, Bracket::None | Bracket::Round, _)) =
			(name.symbol_name().or_else(parameterless), value.drop_meta())
		else {
			return;
		};
		let Some((function, arguments)) = items.split_first() else {
			return;
		};
		let Some(definition) = found.iter().find(|definition| function.symbol_name() == Some(definition.name.as_str()))
		else {
			return;
		};
		let has_hole = arguments.iter().any(|argument| argument.is_symbol(PARTIAL_HOLE));
		if refused.is_some() || has_hole || arguments.is_empty() || arguments.len() >= required_parameters(definition) {
			return;
		}
		let written: Vec<String> = arguments.iter().map(|argument| argument.serialize()).collect();
		let holes = vec![PARTIAL_HOLE.to_string(); definition.params.len() - arguments.len()];
		let partial = format!("{}({})", definition.name, [written.clone(), holes].concat().join(", "));
		let message = format!(
			"{} needs {} arguments, {name} := {} {} leaves some out",
			definition.name,
			definition.params.len(),
			definition.name,
			written.join(" ")
		);
		refused = Some(
			crate::errors::diagnostic::Diagnostic::at(value, message).fix(format!("{name} := {partial}")).into_error(),
		);
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
	let Node::Symbol(name) = crate::closures::tail(&definition.body) else {
		return None;
	};
	if references(&definition.body, name) {
		return None; // `function inc`: tail drops the Meta that marks the reference
	}
	let mut nested = Vec::new();
	definitions(&definition.body, &mut nested);
	let inner = nested.iter().find(|inner| inner.name == *name && inner.params.is_empty())?;
	let inner_tail = crate::closures::tail(&inner.body);
	let returns_function = crate::lambdas::arrow_lambda(inner_tail).is_some()
		|| crate::closures::referenced_function(inner_tail).is_some()
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
		if let (Node::Symbol(variable), Node::List(items, Bracket::Round, Separator::None)) =
			(target.drop_meta(), value.drop_meta())
		{
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
		Node::List(items, Bracket::Round, Separator::None)
			if items.first().and_then(Node::symbol_name).is_some_and(|name| results.contains_key(name)) =>
		{
			Some(node)
		}
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

fn collect_function_uses(
	node: &Node,
	names: &[String],
	higher_order: &HashMap<String, Vec<usize>>,
	used: &mut HashSet<String>,
) {
	let is_param = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) if names.contains(name) => Some(name.clone()),
		_ => None,
	};
	match node.drop_meta() {
		// `switch c {cases}` parses as [switch, (c {cases})]: the subject and its cases, no call of c
		Node::List(items, _, _)
			if items
				.first()
				.and_then(Node::symbol_name)
				.is_some_and(|word| crate::words::SWITCH_WORDS.contains(&word)) =>
		{
			for item in &items[1..] {
				match item.drop_meta() {
					Node::List(pair, _, _) if pair.len() == 2 => {
						pair.iter().for_each(|part| collect_function_uses(part, names, higher_order, used))
					}
					other => collect_function_uses(other, names, higher_order, used),
				}
			}
		}
		Node::List(items, bracket, separator) => {
			if let Some((head, arguments)) = items.split_first() {
				// f(x), f x
				if let Some(name) = is_param(head) {
					let is_call = (*bracket == Bracket::Round && *separator == Separator::None)
						|| (*bracket == Bracket::None && *separator == Separator::Space);
					// `p in xs`, `p is int`: a word joining two values, no call of p
					let infix =
						arguments.first().and_then(Node::symbol_name).is_some_and(crate::warp_parser::joins_operands);
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
				if items.first().and_then(Node::symbol_name).is_some_and(|word| ITERATION_WORDS.contains(&word)) {
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
/// they name; `values` holds the aliases assigned with `=`, values that are the function itself
fn aliases(
	node: &Node,
	functions: &HashSet<String>,
	assigned: &mut HashMap<String, usize>,
	found: &mut HashMap<String, String>,
	values: &mut HashSet<String>,
) {
	match node.drop_meta() {
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
			if let Some(name) = aliased_name(target, op) {
				let mut reassigned = true;
				// `g = function add` / `g = &add` (P83: a bare `g = add` needs add's arguments), `h = g` of an alias
				let function = match (crate::closures::referenced_function(value), bare_name(value, op)) {
					(Some(function), _) => Some(function),
					(None, Some(alias)) if found.contains_key(alias) => Some(alias.to_string()),
					// `arctan := arc_tangent` defines a name for the function (card arctan-allow)
					// a plain name only: `compute() := emit ask` reads `(on·ask)`, a call (card arctan-allow)
					(None, Some(function))
						if *op == Op::Define
							&& functions.contains(function)
							&& matches!(value.drop_meta(), Node::Symbol(_)) =>
					{
						Some(function.to_string())
					}
					_ => None,
				};
				if let Some(function) = function {
					let named = found.get(&function).cloned().unwrap_or(function);
					// math.warp's `arctan=&arc_tangent` and the program's `arctan := arc_tangent` name one function
					reassigned = found.get(name) != Some(&named);
					if functions.contains(&named) && named != name {
						found.insert(name.to_string(), named);
						if *op == Op::Assign {
							values.insert(name.to_string());
						}
					}
				}
				if reassigned {
					*assigned.entry(name.to_string()).or_default() += 1;
				}
			}
			aliases(value, functions, assigned, found, values);
		}
		Node::Key(left, _, right) => {
			aliases(left, functions, assigned, found, values);
			aliases(right, functions, assigned, found, values);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| aliases(item, functions, assigned, found, values)),
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

/// The alias assignments become ø and every use of the alias is the function: called by its name, otherwise the
/// function itself when the alias was assigned with `=` (`print g` of `g = &f` prints f)
fn replace_aliases(node: Node, found: &HashMap<String, String>, values: &HashSet<String>) -> Node {
	let replaced = |node: Node| replace_aliases(node, found, values);
	let called = |head: &Node| {
		head.symbol_name().and_then(|name| found.get(name)).map(|function| Node::Symbol(function.clone()))
	};
	match node {
		Node::Symbol(name) if values.contains(&name) => crate::closures::function_reference(found[&name].clone()),
		Node::Symbol(name) => Node::Symbol(found.get(&name).cloned().unwrap_or(name)),
		Node::Key(target, op @ (Op::Assign | Op::Define), value)
			if aliased_name(&target, &op).is_some_and(|name| found.contains_key(name))
				&& bare_name(&value, &op).is_some() =>
		{
			Node::Empty
		}
		// `(h)`, as an earlier pass calls the defined name h: the function's name, a call of it only without parameters
		Node::List(items, Bracket::Round, _) if items.len() == 1 && called(&items[0]).is_some() => {
			called(&items[0]).expect("guarded")
		}
		// `h 4` of the alias h: the call of its function, which the earlier passes didn't know h for (card alias-chain)
		Node::List(items, Bracket::None, Separator::Space) if items.len() > 1 && called(&items[0]).is_some() => {
			let function = called(&items[0]).and_then(|function| function.symbol_name().map(String::from));
			call(&function.expect("guarded"), items.into_iter().skip(1).map(replaced).collect())
		}
		// `h(4)`, read `(h 4)`: the call of its function
		Node::List(items, Bracket::Round, separator) if items.len() > 1 && called(&items[0]).is_some() => {
			let head = called(&items[0]).expect("guarded");
			let arguments = items.into_iter().skip(1).map(replaced);
			Node::List(std::iter::once(head).chain(arguments).collect(), Bracket::Round, separator)
		}
		other => children_rewritten(other, replaced),
	}
}

/// The calls of functions that take functions, their function arguments made values closures.rs can pass
struct FunctionArguments {
	definitions: HashSet<String>,
	/// Function name → the indexes of its parameters that are functions
	higher_order: HashMap<String, Vec<usize>>,
	variables: HashSet<String>,
	lambda_definitions: Vec<Node>,
}

impl FunctionArguments {
	/// A function argument as closures.rs passes it: a lambda, a name or a variable as it is; an operator, a block or a
	/// library word as a function of its own (`lambda_value_1(argument·1) := is_letter(argument·1)`), named
	fn function_value(&mut self, name: &str, param: &str, argument: &Node) -> Result<Node, Node> {
		if let Some(lambda) = crate::lambdas::it_function(argument) {
			return Ok(lambda);
		}
		if crate::lambdas::arrow_lambda(argument).is_some() || self.definitions.contains(&argument.drop_meta().name()) {
			return Ok(argument.clone());
		}
		let lambda_name = format!("{LAMBDA_PREFIX}{}", self.lambda_definitions.len() + 1);
		let definition = self
			.library_word_lambda(&lambda_name, argument)
			.or_else(|| lambda_definition(&lambda_name, argument))
			.filter(|definition| {
				Definition::from(definition)
					.is_some_and(|parsed| !free_variables(&parsed).iter().any(|free| self.variables.contains(free)))
			});
		if let Some(definition) = definition {
			self.lambda_definitions.push(definition);
			return Ok(Node::Symbol(lambda_name));
		}
		if may_be_function_value(argument) || matches!(argument.drop_meta(), Node::List(_, Bracket::Curly, _)) {
			return Ok(argument.clone());
		}
		Err(crate::closures::needs_a_function(name, Some(param), argument, "a lambda like x => …"))
	}

	/// A library word given as a function (`f(is_letter)`, `twice(upper, t)`) is a function calling it with as many
	/// parameters as the word takes: `lambda_value_1(argument·1) := is_letter(argument·1)`
	fn library_word_lambda(&self, name: &str, argument: &Node) -> Option<Node> {
		let word = argument.symbol_name()?;
		// `sqrt`, `abs`: an operator word known by its signature
		let operator_arity = crate::lowering::library_signatures::library_signature(word)
			.filter(|_| crate::warp_parser::PREFIX_OPERATOR_WORDS.iter().any(|(spelled, _)| *spelled == word))
			.map(|(params, _)| params.len());
		let known = is_standard_function(word) || prelude_word_arity(word).is_some() || operator_arity.is_some();
		if !known || self.variables.contains(word) {
			return None;
		}
		let parameters: Vec<Node> = (1..=operator_arity.unwrap_or_else(|| arity(canonical_spelling(word))))
			.map(|index| Node::Symbol(format!("{LIBRARY_WORD_PARAMETER}{index}")))
			.collect();
		Some(key(call(name, parameters.clone()), Op::Define, crate::closures::word_applied(word, parameters)))
	}

	fn rewrite(&mut self, node: Node, params: &HashMap<String, Vec<String>>) -> Node {
		if let Some(definition) = Definition::from(&node) {
			let body = self.rewrite(definition.body.clone(), params);
			return Definition { body, ..definition }.node();
		}
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.rewrite(item, params)).collect();
				self.function_call(&items, &bracket, &separator, params)
					.unwrap_or(Node::List(items, bracket, separator))
			}
			other => children_rewritten(other, |child| self.rewrite(child, params)),
		}
	}

	/// `apply(double, 3)` and `apply double 3`: the call `apply(double, 3)` with its function arguments made values
	fn function_call(
		&mut self,
		items: &[Node],
		bracket: &Bracket,
		separator: &Separator,
		params: &HashMap<String, Vec<String>>,
	) -> Option<Node> {
		let (name, arguments) = word_headed(items)?;
		let indexes = self.higher_order.get(name)?.clone();
		if !(*bracket == Bracket::Round && *separator == Separator::None
			|| *bracket == Bracket::None && *separator == Separator::Space)
		{
			return None;
		}
		let mut arguments = arguments.to_vec();
		for index in indexes {
			// `apply sqrt 9`: the parser applied sqrt, apply takes it as its function
			arguments = crate::closures::with_operator_word_unfolded(arguments, index);
			let Some(argument) = arguments.get(index) else { continue };
			let param = params[name].get(index).cloned().unwrap_or_default();
			match self.function_value(name, &param, argument) {
				Ok(value) => arguments[index] = value,
				Err(error) => return Some(error),
			}
		}
		Some(call(name, arguments))
	}
}

/// `f x` with a parameter `f` that is a function: the call `f(x)`
fn round_calls(node: Node, function_params: &[String]) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space)
			if items.len() > 1
				&& matches!(items[0].drop_meta(), Node::Symbol(name) if function_params.contains(name)) =>
		{
			parens(items.into_iter().map(|item| round_calls(item, function_params)).collect())
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

/// The definitions of functions taking functions call their function parameters round: `f x` is `f(x)`
fn with_round_calls(node: Node, higher_order: &HashMap<String, Vec<usize>>) -> Node {
	if let Some(definition) = Definition::from(&node) {
		if let Some(indexes) = higher_order.get(&definition.name) {
			let function_params: Vec<String> =
				indexes.iter().filter_map(|index| definition.param_names().get(*index).cloned()).collect();
			let body = round_calls(definition.body.clone(), &function_params);
			return Definition { body, ..definition }.node();
		}
	}
	children_rewritten(node, |child| with_round_calls(child, higher_order))
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
	let program = aliases_replaced(program, &found);
	let mut found = Vec::new();
	definitions(&program, &mut found);
	let higher_order = higher_order_parameters(&found);
	if higher_order.is_empty() {
		return program;
	}
	let params: HashMap<String, Vec<String>> =
		found.iter().map(|definition| (definition.name.clone(), definition.param_names())).collect();
	let mut arguments = FunctionArguments {
		definitions: found.into_iter().map(|definition| definition.name).collect(),
		higher_order,
		// a lambda reading one of these (a loop variable too) is a closure, not a function of its own
		variables: crate::closures::captured_variables_of(&program),
		lambda_definitions: Vec::new(),
	};
	let program = arguments.rewrite(with_round_calls(program, &arguments.higher_order), &params);
	let program = with_lambda_definitions_first(program, &arguments.lambda_definitions);
	// `f(*args)` of a parameter f spreads into the function now known (variadic.rs kept the spread)
	crate::variadic::lower(program)
}

/// `g = f` of a function f, assigned once only: g is f
fn aliases_replaced(program: Node, found: &[Definition]) -> Node {
	let functions: HashSet<String> = found.iter().map(|definition| definition.name.clone()).collect();
	let mut assigned_count = HashMap::new();
	let mut alias_of = HashMap::new();
	let mut alias_values = HashSet::new();
	aliases(&program, &functions, &mut assigned_count, &mut alias_of, &mut alias_values);
	// an alias assigned once only
	alias_of.retain(|name, _| assigned_count.get(name) == Some(&1));
	if alias_of.is_empty() {
		program
	} else {
		replace_aliases(program, &alias_of, &alias_values)
	}
}

/// The indexes of the parameters each function calls, until passing a function on to another one adds no more
pub(crate) fn higher_order_parameters(found: &[Definition]) -> HashMap<String, Vec<usize>> {
	let mut higher_order: HashMap<String, Vec<usize>> = HashMap::new();
	loop {
		let mut changed = false;
		for definition in found {
			let indexes = function_parameters(definition, &higher_order);
			if !indexes.is_empty() && higher_order.get(&definition.name) != Some(&indexes) {
				higher_order.insert(definition.name.clone(), indexes);
				changed = true;
			}
		}
		if !changed {
			return higher_order;
		}
	}
}

/// The lambdas' definitions go first among the program's statements: nesting the program in a list of its own hid its
/// main level from the later passes (a handler's write to a main-level variable was refused)
fn with_lambda_definitions_first(program: Node, lambda_definitions: &[Node]) -> Node {
	match (lambda_definitions.is_empty(), program.drop_meta()) {
		(true, _) => program,
		(false, Node::List(statements, bracket, separator))
			if crate::variable_signals::is_statement_list(bracket, separator) =>
		{
			Node::List([lambda_definitions, statements].concat(), bracket.clone(), separator.clone())
		}
		(false, _) => Node::List([lambda_definitions, &[program]].concat(), Bracket::Round, Separator::Semicolon),
	}
}
