//! Charged bodies read their free variables like Python functions (wiki/charged.md section 3, revising D7): at call
//! time. A main-level variable a function reads and the program changes after the definition must be declared
//! `global y` in that function; otherwise the change is a compile error where it is written, once a later call could
//! observe it. `name := expr` is a getter, evaluated at every use: it becomes the function `name() := expr` when the
//! timing can be observed (its expression has effects, which also warns, or reads a `global`).
//!
//! Functions keep capturing by value at the definition (wasm_emitter `emit_closure_capture`): with this check every
//! accepted program reads the same value either way, except a variable first bound after the definition, which is
//! made a global so that the call reads it (`def f(x){x+k}; k=3; f(1)` → 4).

use crate::analyzer::{captured_variables, collect_variables, declare_global, extract_user_functions, find_assignments, is_list_mutating_method, Scope};
use crate::context::{Context, UserFunctionDef};
use crate::diagnostic::{ask, reading, Ask, Diagnostic, Fallback};
use crate::effects::EffectReport;
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use std::collections::{HashMap, HashSet};

const GLOBAL: &str = "global";
const EFFECTFUL_GETTER_TOPIC: &str = "effectful-getter";

pub fn lower(program: Node) -> Result<Node, Node> {
	let statements = statements_of(&program);
	let mut main = Scope::new();
	collect_variables(&program, &mut main);
	let declared = declared_global_reads(&statements, &main);
	let program = without_global_reads(program, &declared);
	let (program, getters) = charged_getters(program, &main, &declared)?;
	let statements = statements_of(&program);
	let mut late_bound: Vec<String> = declared.clone();
	let definitions = definitions(&statements, &getters);
	let callers = callers(&definitions);
	for definition in &definitions {
		let free = captured_variables(&definition.function, &main).into_iter().map(|(name, _)| name)
			.filter(|name| !main.is_global(name) && !declared.contains(name) && !callers.contains_key(name));
		for variable in free {
			let changes = changes_after(&statements, definition.index, &variable);
			let unbound_before = !binds(&statements[..definition.index], &variable);
			let checked = match (unbound_before, changes.split_first()) {
				(true, Some((_, later))) => {
					late_bound.push(variable.clone());
					later
				}
				_ => changes.as_slice(),
			};
			if let Some(change) = checked.iter().find(|change| observed_after(&statements, change, &callers[&definition.function.name])) {
				return Err(late_binding_error(definition, &variable, change.node));
			}
		}
	}
	late_bound.sort();
	late_bound.dedup();
	Ok(declare_global(program, &late_bound))
}

/// A function the program defines in its statement `index`
struct Definition {
	index: usize,
	function: UserFunctionDef,
	line: usize,
}

/// A change of a variable: the statement it is in and the assignment
struct Change<'a> {
	index: usize,
	node: &'a Node,
}

fn statements_of(program: &Node) -> Vec<Node> {
	match program.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items.clone(),
		_ => vec![program.clone()],
	}
}

fn map_statements(program: Node, transform: &mut dyn FnMut(Node) -> Node) -> Node {
	match program {
		Node::List(items, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) =>
			Node::List(items.into_iter().map(|item| transform(item)).collect(), Bracket::None, separator),
		Node::Meta { node, data } if matches!(node.drop_meta(), Node::List(_, Bracket::None, Separator::Semicolon | Separator::Newline)) =>
			Node::Meta { node: Box::new(map_statements(*node, transform)), data },
		single => transform(single),
	}
}

/// The functions a statement defines when it is a definition itself; one inside a loop or block keeps capturing the
/// values of that iteration (loop variables need no declaration)
fn functions_in(statement: &Node) -> Vec<UserFunctionDef> {
	let is_definition = match statement.drop_meta() {
		Node::Key(_, Op::Define | Op::Assign, _) => true,
		Node::List(items, _, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if is_function_keyword(word)),
		_ => false,
	};
	if !is_definition {
		return vec![];
	}
	let mut context = Context::new();
	extract_user_functions(&mut context, statement);
	context.user_functions.into_values().collect()
}

/// `global y` (no value) as a statement of a function body: the function reads main's current y
fn global_read(node: &Node) -> Option<&String> {
	match node.drop_meta() {
		Node::Key(keyword, Op::Colon, name) if matches!(keyword.drop_meta(), Node::Symbol(word) if word == GLOBAL) => match name.drop_meta() {
			Node::Symbol(name) => Some(name),
			_ => None,
		},
		_ => None,
	}
}

/// The main-level variables some function declares `global y` for
fn declared_global_reads(statements: &[Node], main: &Scope) -> Vec<String> {
	let mut names: Vec<String> = vec![];
	for function in statements.iter().flat_map(functions_in) {
		function.body.visit(&mut |node| {
			if let Some(name) = global_read(node).filter(|name| main.lookup(name).is_some() && !main.is_global(name)) {
				names.push(name.clone());
			}
		});
	}
	names.sort();
	names.dedup();
	names
}

/// The function bodies without their `global y` reads of the variables main declares global now
fn without_global_reads(program: Node, declared: &[String]) -> Node {
	if declared.is_empty() {
		return program;
	}
	fn strip(node: Node, declared: &[String]) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let kept = items.into_iter().filter(|item| !global_read(item).is_some_and(|name| declared.contains(name)));
				Node::List(kept.map(|item| strip(item, declared)).collect(), bracket, separator)
			}
			Node::Key(left, op, right) => Node::Key(Box::new(strip(*left, declared)), op, Box::new(strip(*right, declared))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(strip(*node, declared)), data },
			other => other,
		}
	}
	map_statements(program, &mut |statement| match functions_in(&statement).is_empty() {
		true => statement,
		false => strip(statement, declared),
	})
}

/// `name := expr` that is no function yet: its name and expression
fn getter(statement: &Node) -> Option<(&String, &Node)> {
	match statement.drop_meta() {
		Node::Key(name, Op::Define, expression) if functions_in(statement).is_empty() => match name.drop_meta() {
			Node::Symbol(name) => Some((name, expression)),
			_ => None,
		},
		_ => None,
	}
}

fn as_function(name: &str, expression: &Node) -> Node {
	let signature = Node::List(vec![Node::Symbol(name.to_string())], Bracket::Round, Separator::None);
	Node::Key(Box::new(signature), Op::Define, Box::new(expression.clone()))
}

fn reads_any(node: &Node, names: &dyn Fn(&str) -> bool) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if names(name)));
	found
}

/// The getters `name := expr` whose timing can be observed become `name() := expr`; an effectful one warns. Returns
/// the program and every getter that reads a main-level variable (all are definitions for the late-binding check)
fn charged_getters(program: Node, main: &Scope, declared: &[String]) -> Result<(Node, HashSet<String>), Node> {
	let candidates: Vec<(String, Node, Node)> = statements_of(&program).iter()
		.filter_map(|statement| getter(statement).map(|(name, expression)| (name.clone(), expression.clone(), statement.clone())))
		.collect();
	if candidates.is_empty() {
		return Ok((program, HashSet::new()));
	}
	let as_functions = Node::List(candidates.iter().map(|(name, expression, _)| as_function(name, expression)).collect(), Bracket::None, Separator::Newline);
	let report = EffectReport::of(&Node::List(vec![program.clone(), as_functions], Bracket::None, Separator::Newline));
	let mut charged: HashSet<String> = HashSet::new();
	let mut reading_variables: HashSet<String> = HashSet::new();
	for (name, expression, statement) in &candidates {
		let effectful = report.effects_of(name).is_some_and(|effects| !effects.is_pure());
		if effectful {
			warn_effectful_getter(name, expression, statement)?;
		}
		if effectful || reads_any(expression, &|word| main.is_global(word) || declared.iter().any(|name| name == word)) {
			charged.insert(name.clone());
		}
		if reads_any(expression, &|word| word != name && main.lookup(word).is_some()) {
			reading_variables.insert(name.clone());
		}
	}
	let program = map_statements(program, &mut |statement| match getter(&statement) {
		Some((name, expression)) if charged.contains(name) => as_function(name, expression),
		_ => called_getters(statement, &charged),
	});
	Ok((program, reading_variables))
}

/// Every read of a charged getter `t` is the call `t()`
fn called_getters(node: Node, charged: &HashSet<String>) -> Node {
	match node {
		Node::Symbol(name) if charged.contains(&name) => Node::List(vec![Node::Symbol(name)], Bracket::Round, Separator::None),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| called_getters(item, charged)).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(called_getters(*left, charged)), op, Box::new(called_getters(*right, charged))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(called_getters(*node, charged)), data },
		other => other,
	}
}

/// `clock()` for the call `(clock)`, as it was written
fn written_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
			let arguments: Vec<String> = items[1..].iter().map(written_text).collect();
			format!("{}({})", items[0].drop_meta().serialize().trim(), arguments.join(", "))
		}
		other => other.serialize().trim().to_string(),
	}
}

fn warn_effectful_getter(name: &str, expression: &Node, statement: &Node) -> Result<(), Node> {
	let written = format!("{name} := {}", written_text(expression));
	let question = format!("{name} runs {} at every read; write {name} = … for one value", written_text(expression));
	ask(&Ask::new(EFFECTFUL_GETTER_TOPIC, question, vec![reading("run it at every read", &written)], Fallback::Warning).written(&written).at_node(statement))?;
	Ok(())
}

/// The functions of the main-level statements (and the getters reading variables), with their statement index
fn definitions(statements: &[Node], getters: &HashSet<String>) -> Vec<Definition> {
	let line_of = |statement: &Node| Diagnostic::at(statement, "").line;
	let mut found = vec![];
	for (index, statement) in statements.iter().enumerate() {
		let getter_function = getter(statement).filter(|(name, _)| getters.contains(*name))
			.map(|(name, expression)| functions_in(&as_function(name, expression)));
		for function in getter_function.unwrap_or_else(|| functions_in(statement)) {
			found.push(Definition { index, function, line: line_of(statement) });
		}
	}
	found
}

/// Each function's name with the names of the functions that call it, transitively, itself included: a use of any of
/// them is a call that may read the variable
fn callers(definitions: &[Definition]) -> HashMap<String, HashSet<String>> {
	let names: HashSet<&String> = definitions.iter().map(|definition| &definition.function.name).collect();
	let mut callers: HashMap<String, HashSet<String>> = names.iter().map(|name| ((*name).clone(), HashSet::from([(*name).clone()]))).collect();
	loop {
		let mut changed = false;
		for definition in definitions {
			let caller = &definition.function.name;
			for callee in names.iter().filter(|callee| *callee != &caller && reads_any(&definition.function.body, &|word| word == callee.as_str())) {
				let known: Vec<String> = callers[caller].iter().cloned().collect();
				let entry = callers.get_mut(*callee).expect("a defined function");
				for name in known {
					changed |= entry.insert(name);
				}
			}
		}
		if !changed {
			return callers;
		}
	}
}

/// The changes of `variable` in the statements after `index`, outside function definitions
fn changes_after<'a>(statements: &'a [Node], index: usize, variable: &str) -> Vec<Change<'a>> {
	statements.iter().enumerate().skip(index + 1)
		.filter(|(_, statement)| functions_in(statement).is_empty())
		.flat_map(|(index, statement)| changes_in(statement, variable).into_iter().map(move |node| Change { index, node }))
		.collect()
}

/// Assignments of `variable` (`y = …`, `y += …`, `xs#i = …`) and in-place mutations (`xs.add(v)`, `xs.pop()`)
fn changes_in<'a>(node: &'a Node, variable: &str) -> Vec<&'a Node> {
	let mut found: Vec<&Node> = find_assignments(node, &|name| name == variable).into_iter().map(|(node, _)| node).collect();
	found.extend(mutating_calls(node, variable));
	found
}

fn mutating_calls<'a>(node: &'a Node, variable: &str) -> Vec<&'a Node> {
	match node.drop_meta() {
		Node::Key(target, Op::Dot, call) if matches!(target.drop_meta(), Node::Symbol(name) if name == variable) && is_mutating_call(call) => vec![node],
		Node::Key(left, _, right) => [mutating_calls(left, variable), mutating_calls(right, variable)].concat(),
		Node::List(items, _, _) => items.iter().flat_map(|item| mutating_calls(item, variable)).collect(),
		_ => vec![],
	}
}

fn is_mutating_call(call: &Node) -> bool {
	let method = match call.drop_meta() {
		Node::List(items, _, _) => items.first().map(Node::drop_meta),
		other => Some(other),
	};
	matches!(method, Some(Node::Symbol(name)) if is_list_mutating_method(name))
}

fn binds(statements: &[Node], variable: &str) -> bool {
	let mut scope = Scope::new();
	collect_variables(&Node::List(statements.to_vec(), Bracket::None, Separator::Newline), &mut scope);
	scope.lookup(variable).is_some()
}

/// Can a call of one of `users` run after the change? In a later statement, or in the change's own statement outside
/// the assigned value (a loop that changes the variable and calls the function)
fn observed_after(statements: &[Node], change: &Change, users: &HashSet<String>) -> bool {
	let used = |node: &Node| reads_any(node, &|word| users.contains(word));
	let statement = &statements[change.index];
	let whole_statement = std::ptr::eq(statement.drop_meta(), change.node.drop_meta());
	statements[change.index + 1..].iter().any(used) || (!whole_statement && used(statement))
}

fn late_binding_error(definition: &Definition, variable: &str, change: &Node) -> Node {
	let function = &definition.function.name;
	Diagnostic::at(change, format!("{function} reads {variable} (line {}): declare `global {variable}` in {function} to read its current value, or pass {variable} as a parameter", definition.line))
		.fix(format!("global {variable}")).into_error()
}
