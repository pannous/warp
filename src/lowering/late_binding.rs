//! Charged bodies read their free variables like Python functions (wiki/charged.md section 3, revising D7): at call
//! time. A main-level variable a function reads and the program changes after the definition must be declared
//! `global y` in that function; otherwise the change is a compile error where it is written, once a later call could
//! observe it. A function defined in a function body reads that function's parameters and variables the same way: a
//! change after its definition that a later call in the body could observe is an error, unless the nested function
//! declares `nonlocal y` (card g-qUkY): then each call from the enclosing body reads its current value.
//! A `def` without parameters that always gives the same value gets the "needless charging" note.
//!
//! Functions keep capturing by value at the definition (wasm_emitter `emit_closure_capture`): with this check every
//! accepted program reads the same value either way, except a variable first bound after the definition, which is
//! made a global so that the call reads it (`def f(x){x+k}; k=3; f(1)` → 4). A nested function `outer·inner` reads
//! its captures as they are at each call the enclosing body makes (directly, through a sibling or passed as a value:
//! wasm_emitter `refresh_enclosing_captures`), so `nonlocal y`
//! needs no code of its own: the declaration only lifts the check and is dropped here. Writing y from inner waits.

use super::nodes::key;
use crate::analyzer::{captured_variables, collect_variables, declare_global, find_assignments, is_list_mutating_method, param_kind, Scope};
use crate::context::{Context, UserFunctionDef};
use crate::diagnostic::{ask, reading, Ask, Diagnostic, Fallback};
use crate::effects::EffectReport;
use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use std::collections::{HashMap, HashSet};

pub(crate) const GLOBAL: &str = "global";
pub(crate) const NONLOCAL: &str = "nonlocal";
const NEEDLESS_CHARGING_TOPIC: &str = "needless-charging";
const EFFECTFUL_GETTER_TOPIC: &str = "effectful-getter";
/// A getter body up to this length is quoted in its warning (`t runs clock() at every read`); a longer one is "its code"
const QUOTED_BODY_LENGTH: usize = 24;
/// names the compiler makes join their parts with it (`closure_lambda_1·captured·0`); the parser reads it as `*`
const GENERATED_NAME_MARK: char = '·';

pub fn lower(program: Node) -> Result<Node, Node> {
	let statements = statements_of(&program);
	if let Some(declaration) = statements.iter().find(|statement| is_nonlocal_declaration(statement)) {
		return Err(nonlocal_at_main_level_error(declaration));
	}
	let mut main = Scope::new();
	collect_variables(&program, &mut main);
	let declared = declared_global_reads(&statements, &main, false);
	// a `global y` read of a variable main declares global already goes too (a lambda's, made global by nonlocal_cells)
	let program = without_global_reads(program, &declared_global_reads(&statements, &main, true));
	let statements = statements_of(&program);
	let definitions = definitions(&statements);
	let report = EffectReport::of(&program);
	for definition in &definitions {
		note_charging(definition, &statements, &main, &report)?;
	}
	let mut late_bound = check_definitions(&statements, &definitions, &main, &|_, name| declared.iter().any(|known| known == name), None)?;
	for definition in &definitions {
		check_function_body(&definition.function)?;
	}
	late_bound.extend(declared);
	late_bound.sort();
	late_bound.dedup();
	Ok(declare_global(without_nonlocal_declarations(crate::nonlocal_cells::lower(program)), &late_bound))
}

/// The late-binding check of the functions defined in `statements` whose free variables `scope` binds: main's
/// statements, or the body of the function `enclosing`. Returns the variables first bound after a definition, which
/// main makes global so that the call reads them; in a function body the call reads its captures as they are then.
fn check_definitions(statements: &[Node], definitions: &[Definition], scope: &Scope, skip: &dyn Fn(&Definition, &str) -> bool, enclosing: Option<&UserFunctionDef>) -> Result<Vec<String>, Node> {
	let callers = callers(definitions, &|name| written_names(enclosing, name));
	// asked for every free variable of every definition: worked out once per statement and per definition
	let defines_functions: Vec<bool> = statements.iter().map(|statement| !functions_in(statement).is_empty()).collect();
	let mut late_bound: Vec<String> = vec![];
	for definition in definitions {
		let free: Vec<String> = captured_variables(&definition.function, scope).into_iter().map(|(name, _)| name)
			.filter(|name| !scope.is_global(name) && !skip(definition, name) && !callers.contains_key(name))
			.collect();
		if free.is_empty() {
			continue;
		}
		let bound_before = variables_of(&statements[..definition.index]);
		// a call before the definition runs before its capture (`x = f(); def f(){ k }`): it reads main's variable as it is
		let called_before = statements[..definition.index].iter().any(|statement| reads_any(statement, &|word| callers[&definition.function.name].contains(word)));
		for variable in free {
			if called_before && enclosing.is_none() {
				late_bound.push(variable.clone());
			}
			let changes = changes_after(statements, &defines_functions, definition.index, &variable);
			let parameter = enclosing.is_some_and(|function| function.params.iter().any(|param| param.name == variable));
			let unbound_before = !parameter && bound_before.lookup(&variable).is_none();
			let checked = match (unbound_before, changes.split_first()) {
				(true, Some((_, later))) => {
					if enclosing.is_none() {
						late_bound.push(variable.clone());
					}
					later
				}
				_ => changes.as_slice(),
			};
			let Some(change) = checked.iter().find(|change| observed_after(statements, change, &callers[&definition.function.name])) else { continue };
			// a getter (P71) reads the value as it is at each use: main's variable becomes global, a function's
			// captures are refreshed at each call anyway
			match (definition.getter, enclosing) {
				(true, None) => late_bound.push(variable.clone()),
				(true, Some(_)) => {}
				(false, _) => return Err(late_binding_error(definition, &variable, change.node, enclosing)),
			}
		}
	}
	Ok(late_bound)
}

/// The functions defined directly in the body of `function` read its parameters and variables at call time too: a
/// change of one after such a definition is an error once a later call in the body could observe it (card g-qUrk)
fn check_function_body(function: &UserFunctionDef) -> Result<(), Node> {
	let statements = body_statements(&function.body);
	let nested = function.name.contains(NESTED_DEF_SEPARATOR); // `outer·inner`: outer checks its nonlocal declarations
	if let Some(name) = declared_nonlocals(&function.body).first().filter(|_| !nested) {
		return Err(nonlocal_without_enclosing_error(function, name));
	}
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
	for definition in &definitions {
		let names = declared_nonlocals(&definition.function.body);
		let Some(first) = names.first() else { continue };
		if !definition.function.name.contains(NESTED_DEF_SEPARATOR) {
			return Err(nonlocal_in_block_error(definition, first, function));
		}
		if let Some(name) = names.iter().find(|name| enclosing.lookup(name).is_none()) {
			return Err(unbound_nonlocal_error(definition, name, function));
		}
	}
	let declared = |definition: &Definition, name: &str| {
		let mut found = false;
		definition.function.body.visit(&mut |node| found |= global_read(node).is_some_and(|declared| declared == name));
		found || declared_nonlocals(&definition.function.body).iter().any(|declared| declared == name)
	};
	check_definitions(&statements, &definitions, &enclosing, &declared, Some(function)).map(|_| ())
}

/// The names a function body declares `nonlocal` in its own statements (`nonlocal y`, `nonlocal a, b`), not those of
/// the functions defined in it
pub(crate) fn declared_nonlocals(body: &Node) -> Vec<String> {
	body_statements(body).iter().flat_map(nonlocal_names).collect()
}

/// `nonlocal y` → [y]; `nonlocal a, b` → [a, b]; any other node → []
fn nonlocal_names(node: &Node) -> Vec<String> {
	let Node::Key(keyword, Op::Colon, names) = node.drop_meta() else { return vec![] };
	if !keyword.is_symbol(NONLOCAL) {
		return vec![];
	}
	match names.drop_meta() {
		Node::Symbol(name) => vec![name.clone()],
		Node::List(items, _, _) => items.iter().filter_map(|item| item.symbol_name().map(String::from)).collect(),
		_ => vec![],
	}
}

fn is_nonlocal_declaration(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(keyword, Op::Colon, _) if keyword.is_symbol(NONLOCAL))
}

/// The program without its `nonlocal y` declarations: once checked they have no code
fn without_nonlocal_declarations(program: Node) -> Node {
	let mut found = false;
	program.visit(&mut |node| found |= is_nonlocal_declaration(node));
	if !found {
		return program;
	}
	without_statements(program, &is_nonlocal_declaration)
}

/// The statement `nonlocal name` of the body, for the position of an error
fn declaration_of(body: &Node, name: &str) -> Node {
	body_statements(body).into_iter().find(|statement| nonlocal_names(statement).iter().any(|declared| declared == name)).unwrap_or_else(|| body.clone())
}

/// `nonlocal y` among main's statements
fn nonlocal_at_main_level_error(declaration: &Node) -> Node {
	let name = nonlocal_names(declaration).into_iter().next().unwrap_or_default();
	Diagnostic::at(declaration, format!("`nonlocal {name}` outside a function: main reads its own {name}; `nonlocal` belongs in a function defined in a function"))
		.into_error()
}

/// `nonlocal y` in a function that no function encloses (Python: "nonlocal declaration not allowed at module level")
fn nonlocal_without_enclosing_error(function: &UserFunctionDef, name: &str) -> Node {
	Diagnostic::at(&declaration_of(&function.body, name), format!("`nonlocal {name}` in {}: no function encloses {}; declare `global {name}` to read main's {name}", function.name, function.name))
		.into_error()
}

/// `inner := {nonlocal y; …}`: a block made a closure captures y when it is made; only `def inner(){…}` reads y anew
fn nonlocal_in_block_error(definition: &Definition, name: &str, enclosing: &UserFunctionDef) -> Node {
	let inner = short_name_in(enclosing, &definition.function.name);
	Diagnostic::at(&declaration_of(&definition.function.body, name), format!("`nonlocal {name}` in {inner}: only a function defined with `def {inner}(){{…}}` reads the current {name} so far"))
		.into_error()
}

/// `nonlocal y` in inner where the enclosing function has no y
fn unbound_nonlocal_error(definition: &Definition, name: &str, enclosing: &UserFunctionDef) -> Node {
	let inner = short_name_in(enclosing, &definition.function.name);
	let outer = enclosing.name.rsplit(NESTED_DEF_SEPARATOR).next().unwrap_or(&enclosing.name);
	Diagnostic::at(&declaration_of(&definition.function.body, name), format!("`nonlocal {name}` in {inner}: {outer} has no parameter or variable {name}"))
		.into_error()
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
	/// `z() := y*y` from `z := y*y` (getters.rs): reads its free variables as they are at each use
	getter: bool,
	/// a getter written `z := y*y`, without the parentheses of a function: read like a value
	bare: bool,
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

pub(crate) fn map_statements(program: Node, transform: &mut dyn FnMut(Node) -> Node) -> Node {
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
	// `o·s() := e; o = {…}`: an object with function entries (blocks.rs) is one statement of definitions
	if let Node::List(items, Bracket::None, Separator::Semicolon) = statement.drop_meta() {
		return items.iter().flat_map(functions_in).collect();
	}
	let is_definition = match statement.drop_meta() {
		Node::Key(_, Op::Define | Op::Assign, _) => true,
		Node::List(items, _, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if is_function_keyword(word)),
		_ => false,
	};
	if !is_definition {
		return vec![];
	}
	// names, parameters and bodies only: the kinds the full inference adds are read by no caller
	let mut context = Context::new();
	crate::analyzer::defined_functions(&mut context, statement);
	context.user_functions.into_values().collect()
}

/// `global n = 5` as a statement of a block (a function body) is `global n; n = 5`: n is main's, then set
/// (card global-function-inside: the passes that read `global n` saw no name and n stayed the function's own)
pub fn split_global_assignments(node: Node) -> Node {
	let node = node.map_children(split_global_assignments);
	let Node::List(items, Bracket::Curly, separator) = node else { return node };
	let split = |item: Node| match item.drop_meta() {
		Node::Key(keyword, Op::Colon, assignment) if keyword.is_symbol(GLOBAL) => match assignment.drop_meta() {
			Node::Key(name, Op::Assign, _) if matches!(name.drop_meta(), Node::Symbol(_)) => {
				vec![Node::Key(keyword.clone(), Op::Colon, name.clone()), assignment.as_ref().clone()]
			}
			_ => vec![item],
		},
		_ => vec![item],
	};
	let count = items.len();
	let items: Vec<Node> = items.into_iter().flat_map(split).collect();
	let separator = if items.len() > count && separator == Separator::None { Separator::Semicolon } else { separator };
	Node::List(items, Bracket::Curly, separator)
}

/// `global y` (no value) as a statement of a function body: the function reads main's current y
fn global_read(node: &Node) -> Option<&String> {
	match node.drop_meta() {
		Node::Key(keyword, Op::Colon, name) if keyword.is_symbol(GLOBAL) => match name.drop_meta() {
			Node::Symbol(name) => Some(name),
			_ => None,
		},
		_ => None,
	}
}

/// The main-level variables some function declares `global y` for (`with_globals`: also those main declares global)
fn declared_global_reads(statements: &[Node], main: &Scope, with_globals: bool) -> Vec<String> {
	let mut names: Vec<String> = vec![];
	for function in statements.iter().flat_map(functions_in) {
		function.body.visit(&mut |node| {
			if let Some(name) = global_read(node).filter(|name| if with_globals { main.lookup(name).is_some() || main.is_global(name) } else { main.lookup(name).is_some() && !main.is_global(name) }) {
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
	without_statements(program, &|item| global_read(item).is_some_and(|name| declared.contains(name)))
}

/// The function definitions among the program's statements without their statements that are `dropped`, at any depth
fn without_statements(program: Node, dropped: &dyn Fn(&Node) -> bool) -> Node {
	fn strip(node: Node, dropped: &dyn Fn(&Node) -> bool) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let kept = items.into_iter().filter(|item| !dropped(item));
				Node::List(kept.map(|item| strip(item, dropped)).collect(), bracket, separator)
			}
			Node::Key(left, op, right) => key(strip(*left, dropped), op, strip(*right, dropped)),
			Node::Meta { node, data } => Node::Meta { node: Box::new(strip(*node, dropped)), data },
			other => other,
		}
	}
	map_statements(program, &mut |statement| match functions_in(&statement).is_empty() {
		true => statement,
		false => strip(statement, dropped),
	})
}

/// Every symbol in `node`
fn symbols_read(node: &Node) -> HashSet<&str> {
	let mut symbols = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Symbol(name) = part {
			symbols.insert(name.as_str());
		}
	});
	symbols
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

/// A `def` or getter without parameters whose body is pure and reads only constants (`def area(): 3*4`,
/// `area := 3*4`) always gives the same value: educate toward `area = 3*4` (wiki/charged.md "Needless charging").
/// A getter whose body has effects (`t := clock()`) gets a got-it warning: it runs them at every read.
fn note_charging(definition: &Definition, statements: &[Node], main: &Scope, report: &EffectReport) -> Result<(), Node> {
	let function = &definition.function;
	let statement = &statements[definition.index];
	let is_def = matches!(statement.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if is_function_keyword(word)));
	let nested = function.name.contains(NESTED_DEF_SEPARATOR); // `outer·inner` may read outer's variables, `o·s` is an entry
	if nested || !(is_def || definition.getter) || !function.params.is_empty() {
		return Ok(());
	}
	// a body the compiler already rewrote (a returned lambda is `closure_new(closure_lambda_1)`) is no text the user
	// wrote: a hint never shows it
	let generated = |word: &str| word == crate::closures::CLOSURE_NEW || word.contains(GENERATED_NAME_MARK) || word.contains(crate::function_values::LAMBDA_PREFIX);
	if generated(&function.name) || reads_any(&function.body, &generated) {
		return Ok(());
	}
	let value = written_text(&function.body);
	let name = &function.name;
	let written = match (is_def, definition.bare) {
		(true, _) => format!("def {name}(): {value}"),
		(false, true) => format!("{name} := {value}"),
		(false, false) => format!("{name}() := {value}"),
	};
	let pure = report.effects_of(name).is_some_and(|effects| effects.is_pure());
	if !pure {
		// `w() := random()` says by its parentheses that it runs at every call
		return match definition.bare {
			true => warn_effectful_getter(name, &value, &written, statement),
			false => Ok(()),
		};
	}
	// a getter reading a variable reads its current value (P71); a def's free variables are constants unless global
	let reads_variables = reads_any(&function.body, &|word| if definition.getter { main.lookup(word).is_some() } else { main.is_global(word) });
	if !reads_variables {
		crate::normalize::set_position_of(statement);
		crate::diagnostic::educate_once(NEEDLESS_CHARGING_TOPIC, &written, &format!("{name} = {value}"), &format!("{name} never changes"));
	}
	Ok(())
}

/// `t := clock()`: the clock is read at every `t`, which may be meant, or `t = clock()` for one reading
fn warn_effectful_getter(name: &str, value: &str, written: &str, at: &Node) -> Result<(), Node> {
	let quoted = value.chars().count() <= QUOTED_BODY_LENGTH && !value.contains('\n');
	let message = match quoted {
		true => format!("{name} runs {value} at every read; write {name} = {value} for one value"),
		false => format!("{name} runs its code at every read; write {name} = … for one value"),
	};
	let question = Ask::new(EFFECTFUL_GETTER_TOPIC, message,
		vec![reading("at every read", written), reading("one value", &format!("{name} = {value}"))], Fallback::Warning)
		.written(written).at_node(at);
	ask(&question).map(|_| ())
}

/// The functions of the main-level statements, with their statement index
fn definitions(statements: &[Node]) -> Vec<Definition> {
	let line_of = |statement: &Node| Diagnostic::at(statement, "").line;
	statements.iter().enumerate()
		.flat_map(|(index, statement)| functions_in(statement).into_iter().map(move |function| {
			let getter_left = getter_definition_left(statement, &function.name).filter(|_| function.params.is_empty());
			let bare = getter_left.is_some_and(crate::getters::is_written_bare);
			Definition { index, function, line: line_of(statement), getter: getter_left.is_some(), bare }
		}))
		.collect()
}

/// `name() := expr` with expr not in braces, in the statement: a getter, whether written so or as `name := expr` (P71);
/// `def name(){…}` keeps its braces. An object's getter entry is one of the definitions before the object (blocks.rs).
/// Its `name()` as written, with the mark of a bare getter
fn getter_definition_left<'a>(statement: &'a Node, name: &str) -> Option<&'a Node> {
	match statement.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon) => items.iter().find_map(|item| getter_definition_left(item, name)),
		Node::Key(left, Op::Define, body) if !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => match left.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() == 1 && matches!(items[0].drop_meta(), Node::Symbol(head) if head == name) => Some(left),
			_ => None,
		},
		_ => None,
	}
}

/// Each function's name with the names of the functions that call it, transitively, itself included: a use of any of
/// them is a call that may read the variable
fn callers(definitions: &[Definition], written: &dyn Fn(&str) -> Vec<String>) -> HashMap<String, HashSet<String>> {
	let names: HashSet<&String> = definitions.iter().map(|definition| &definition.function.name).collect();
	let spellings: HashMap<&String, Vec<String>> = names.iter().map(|name| (*name, written(name))).collect();
	let mut callers: HashMap<String, HashSet<String>> = names.iter().map(|name| ((*name).clone(), HashSet::from([(*name).clone()]))).collect();
	// each body's callees once: the loop below runs until nothing changes
	let callees: Vec<Vec<&String>> = definitions.iter().map(|definition| {
		let read = symbols_read(&definition.function.body);
		names.iter().copied().filter(|callee| *callee != &definition.function.name && spellings[*callee].iter().any(|name| read.contains(name.as_str()))).collect()
	}).collect();
	loop {
		let mut changed = false;
		for (definition, callees) in definitions.iter().zip(&callees) {
			let caller = &definition.function.name;
			for callee in callees {
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

/// The changes of `variable` in the statements after `index`, outside the statements that define functions
fn changes_after<'a>(statements: &'a [Node], defines_functions: &[bool], index: usize, variable: &str) -> Vec<Change<'a>> {
	statements.iter().enumerate().skip(index + 1)
		.filter(|(index, _)| !defines_functions[*index])
		.flat_map(|(index, statement)| changes_in(statement, variable).into_iter().map(move |node| Change { index, node }))
		.collect()
}

/// Assignments of `variable` (`y = …`, `y += …`, `xs#i = …`) and in-place mutations (`xs.add(v)`, `xs.pop()`)
pub(crate) fn changes_in<'a>(node: &'a Node, variable: &str) -> Vec<&'a Node> {
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

/// The variables `statements` bind
fn variables_of(statements: &[Node]) -> Scope {
	let mut scope = Scope::new();
	collect_variables(&Node::List(statements.to_vec(), Bracket::None, Separator::Newline), &mut scope);
	scope
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
	Diagnostic::at(change, format!("{inner} reads {variable} of {outer} (line {}): declare `nonlocal {variable}` in {inner} to read its current value, or pass {variable} as a parameter", definition.line))
		.fix(format!("nonlocal {variable}")).into_error()
}
