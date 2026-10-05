//! Charged bodies read their free variables like Python functions (wiki/charged.md section 3, revising D7): at call
//! time. A main-level variable a function reads and the program changes after the definition must be declared
//! `global y` in that function; otherwise the change is a compile error where it is written, once a later call could
//! observe it. A function defined in a function body reads that function's parameters and variables the same way: a
//! change after its definition that a later call in the body could observe is an error (`nonlocal` comes later).
//! A `def` without parameters that always gives the same value gets the "needless charging" note.
//!
//! Functions keep capturing by value at the definition (wasm_emitter `emit_closure_capture`): with this check every
//! accepted program reads the same value either way, except a variable first bound after the definition, which is
//! made a global so that the call reads it (`def f(x){x+k}; k=3; f(1)` → 4).

use crate::analyzer::{captured_variables, collect_variables, declare_global, extract_user_functions, find_assignments, is_list_mutating_method, param_kind, Scope};
use crate::context::{Context, UserFunctionDef};
use crate::diagnostic::Diagnostic;
use crate::effects::EffectReport;
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use std::collections::{HashMap, HashSet};

const GLOBAL: &str = "global";
const NEEDLESS_CHARGING_TOPIC: &str = "needless-charging";

pub fn lower(program: Node) -> Result<Node, Node> {
	let statements = statements_of(&program);
	let mut main = Scope::new();
	collect_variables(&program, &mut main);
	let declared = declared_global_reads(&statements, &main);
	let program = without_global_reads(program, &declared);
	let statements = statements_of(&program);
	let definitions = definitions(&statements);
	let report = EffectReport::of(&program);
	for definition in &definitions {
		note_needless_charging(definition, &statements, &main, &report);
	}
	let mut late_bound = check_definitions(&statements, &definitions, &main, &|_, name| declared.iter().any(|known| known == name), None)?;
	for definition in &definitions {
		check_function_body(&definition.function)?;
	}
	late_bound.extend(declared);
	late_bound.sort();
	late_bound.dedup();
	Ok(declare_global(program, &late_bound))
}

/// The late-binding check of the functions defined in `statements` whose free variables `scope` binds: main's
/// statements, or the body of the function `enclosing`. Returns the variables first bound after a definition, which
/// main makes global so that the call reads them; a function body does not (that waits for `nonlocal`).
fn check_definitions(statements: &[Node], definitions: &[Definition], scope: &Scope, skip: &dyn Fn(&Definition, &str) -> bool, enclosing: Option<&UserFunctionDef>) -> Result<Vec<String>, Node> {
	let callers = callers(definitions, &|name| written_names(enclosing, name));
	let mut late_bound: Vec<String> = vec![];
	for definition in definitions {
		let free = captured_variables(&definition.function, scope).into_iter().map(|(name, _)| name)
			.filter(|name| !scope.is_global(name) && !skip(definition, name) && !callers.contains_key(name));
		for variable in free {
			let changes = changes_after(statements, definition.index, &variable);
			let parameter = enclosing.is_some_and(|function| function.params.iter().any(|param| param.name == variable));
			let unbound_before = !parameter && !binds(&statements[..definition.index], &variable);
			let checked = match (unbound_before, changes.split_first()) {
				(true, Some((_, later))) => {
					if enclosing.is_none() {
						late_bound.push(variable.clone());
					}
					later
				}
				_ => changes.as_slice(),
			};
			if let Some(change) = checked.iter().find(|change| observed_after(statements, change, &callers[&definition.function.name])) {
				return Err(late_binding_error(definition, &variable, change.node, enclosing));
			}
		}
	}
	Ok(late_bound)
}

/// The functions defined directly in the body of `function` read its parameters and variables at call time too: a
/// change of one after such a definition is an error once a later call in the body could observe it (card g-qUrk)
fn check_function_body(function: &UserFunctionDef) -> Result<(), Node> {
	let statements = body_statements(&function.body);
	let definitions: Vec<Definition> = definitions(&statements).into_iter()
		.filter(|definition| !short_name_in(function, &definition.function.name).contains(NESTED_DEF_SEPARATOR))
		.collect();
	if definitions.is_empty() {
		return Ok(());
	}
	let mut enclosing = Scope::new();
	for param in &function.params {
		enclosing.define(param.name.clone(), None, param_kind(param));
	}
	collect_variables(&Node::List(statements.clone(), Bracket::None, Separator::Newline), &mut enclosing);
	let declares_global = |definition: &Definition, name: &str| {
		let mut found = false;
		definition.function.body.visit(&mut |node| found |= global_read(node).is_some_and(|declared| declared == name));
		found
	};
	check_definitions(&statements, &definitions, &enclosing, &declares_global, Some(function)).map(|_| ())
}

/// Between an enclosing function and a definition lifted out of its body: `outer·inner` (analyzer, card g-qUkY)
const NESTED_DEF_SEPARATOR: &str = "·";

/// `inner` for `outer·inner` defined in the body of `outer`
fn short_name_in<'a>(enclosing: &UserFunctionDef, name: &'a str) -> &'a str {
	name.strip_prefix(enclosing.name.as_str()).and_then(|rest| rest.strip_prefix(NESTED_DEF_SEPARATOR)).unwrap_or(name)
}

/// The names a call of the function `name` may be written with: in a function body both `inner` and the lifted
/// `outer·inner`
fn written_names(enclosing: Option<&UserFunctionDef>, name: &str) -> Vec<String> {
	match enclosing {
		None => vec![name.to_string()],
		Some(outer) => {
			let short = short_name_in(outer, name);
			vec![short.to_string(), format!("{}{NESTED_DEF_SEPARATOR}{short}", outer.name)]
		}
	}
}

/// The statements of a function body: `{a; b}`, `a\nb` or the single expression
fn body_statements(body: &Node) -> Vec<Node> {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly | Bracket::None, Separator::Semicolon | Separator::Newline) => items.clone(),
		Node::List(items, Bracket::Curly, _) if items.len() == 1 => body_statements(&items[0]),
		_ => vec![body.clone()],
	}
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

pub(crate) fn statements_of(program: &Node) -> Vec<Node> {
	match program.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items.clone(),
		_ => vec![program.clone()],
	}
}

fn map_statements(program: Node, transform: &mut dyn FnMut(Node) -> Node) -> Node {
	match program {
		Node::List(items, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) =>
			Node::List(items.into_iter().map(transform).collect(), Bracket::None, separator),
		Node::Meta { node, data } if matches!(node.drop_meta(), Node::List(_, Bracket::None, Separator::Semicolon | Separator::Newline)) =>
			Node::Meta { node: Box::new(map_statements(*node, transform)), data },
		single => transform(single),
	}
}

/// The functions a statement defines when it is a definition itself; one inside a loop or block keeps capturing the
/// values of that iteration (loop variables need no declaration)
pub(crate) fn functions_in(statement: &Node) -> Vec<UserFunctionDef> {
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

fn reads_any(node: &Node, names: &dyn Fn(&str) -> bool) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if names(name)));
	found
}

/// `3*4` for the body, `clock()` for the call `(clock)`, as it was written
fn written_text(node: &Node) -> String {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
			let arguments: Vec<String> = items[1..].iter().map(written_text).collect();
			format!("{}({})", items[0].drop_meta().serialize().trim(), arguments.join(", "))
		}
		other => other.serialize().trim().to_string(),
	}
}

/// A `def` without parameters whose body is pure and reads only constants (`def area(): 3*4`) always gives the same
/// value: educate toward `area = 3*4` (wiki/charged.md "Needless charging"). `:=` without parameters waits for P71.
fn note_needless_charging(definition: &Definition, statements: &[Node], main: &Scope, report: &EffectReport) {
	let function = &definition.function;
	let statement = &statements[definition.index];
	let is_def = matches!(statement.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if is_function_keyword(word)));
	let pure = report.effects_of(&function.name).is_some_and(|effects| effects.is_pure());
	if !is_def || !function.params.is_empty() || !pure || reads_any(&function.body, &|word| main.is_global(word)) {
		return;
	}
	crate::normalize::set_position_of(statement);
	let value = written_text(&function.body);
	crate::diagnostic::educate_once(NEEDLESS_CHARGING_TOPIC, &format!("def {}(): {value}", function.name), &format!("{} = {value}", function.name), &format!("{} never changes", function.name));
}

/// The functions of the main-level statements, with their statement index
fn definitions(statements: &[Node]) -> Vec<Definition> {
	let line_of = |statement: &Node| Diagnostic::at(statement, "").line;
	statements.iter().enumerate()
		.flat_map(|(index, statement)| functions_in(statement).into_iter().map(move |function| Definition { index, function, line: line_of(statement) }))
		.collect()
}

/// Each function's name with the names of the functions that call it, transitively, itself included: a use of any of
/// them is a call that may read the variable
fn callers(definitions: &[Definition], written: &dyn Fn(&str) -> Vec<String>) -> HashMap<String, HashSet<String>> {
	let names: HashSet<&String> = definitions.iter().map(|definition| &definition.function.name).collect();
	let spellings: HashMap<&String, Vec<String>> = names.iter().map(|name| (*name, written(name))).collect();
	let mut callers: HashMap<String, HashSet<String>> = names.iter().map(|name| ((*name).clone(), HashSet::from([(*name).clone()]))).collect();
	loop {
		let mut changed = false;
		for definition in definitions {
			let caller = &definition.function.name;
			for callee in names.iter().filter(|callee| *callee != &caller && reads_any(&definition.function.body, &|word| spellings[*callee].iter().any(|name| name == word))) {
				let known: Vec<String> = callers[caller].iter().cloned().collect();
				let entry = callers.get_mut(*callee).expect("a defined function");
				for name in known {
					changed |= entry.insert(name);
				}
			}
		}
		if !changed {
			return callers.into_iter().map(|(callee, users)| (callee, users.iter().flat_map(|user| written(user)).collect())).collect();
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

fn late_binding_error(definition: &Definition, variable: &str, change: &Node, enclosing: Option<&UserFunctionDef>) -> Node {
	let function = &definition.function.name;
	let Some(outer) = enclosing else {
		return Diagnostic::at(change, format!("{function} reads {variable} (line {}): declare `global {variable}` in {function} to read its current value, or pass {variable} as a parameter", definition.line))
			.fix(format!("global {variable}")).into_error();
	};
	let inner = short_name_in(outer, function);
	let outer = outer.name.rsplit(NESTED_DEF_SEPARATOR).next().unwrap_or(&outer.name);
	Diagnostic::at(change, format!("{inner} reads {variable} of {outer} (line {}): pass {variable} as a parameter, or define {inner} after the last change of {variable} (`nonlocal {variable}` is not supported yet)", definition.line))
		.into_error()
}
