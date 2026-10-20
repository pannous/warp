//! Main-level variables: assignments from functions to main's variables, `global` declarations, local-or-global

use super::*;
use crate::lowering::nodes::children_rewritten;
use std::collections::HashSet;

pub fn resolve_main_variable_assignments(program: Node) -> Result<Node, Node> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, &program);
	let mut main = Scope::new();
	// `f := (y = 1; y * it)`: y is f's own, no main-level variable
	let defined: HashSet<String> = ctx.user_functions.keys().cloned().collect();
	collect_variables(&without_block_bodies(program.clone(), &defined), &mut main);
	let mut functions: Vec<&UserFunctionDef> = ctx.user_functions.values().collect();
	functions.sort_by(|a, b| a.name.cmp(&b.name));
	let mut meant_global: Vec<String> = vec![];
	let blocks = block_function_names(&program);
	let mut outside_blocks = Scope::new(); // a variable a block binds for itself is no outer variable
	collect_variables(&without_block_bodies(program.clone(), &blocks), &mut outside_blocks);
	// a used module's functions never see the program's variables (card module-locals)
	for function in functions.into_iter().filter(|function| {
		!crate::compiler::compile_context::read(|context| context.module_definitions.contains(&function.name))
	}) {
		let looped = crate::lowering::for_loop::loop_variables(&function.body);
		let is_main_variable = |name: &String| {
			!is_compiler_temporary(name)
				&& main.lookup(name).is_some()
				&& !main.is_global(name)
				&& !function.params.iter().any(|param| param.name == *name)
				&& !declares_local(&function.body, name)
				&& !looped.contains(name)
		};
		let mut decided: HashSet<&String> = HashSet::new();
		for (node, name) in find_changes(&function.body, &is_main_variable) {
			if !decided.insert(name) {
				continue;
			}
			if blocks.contains(&function.name) {
				if outside_blocks.lookup(name).is_some() {
					educate_block_assignment(node, name, &function.name, main_assignment(&program, name));
				}
				continue;
			}
			if !starts_with_fresh_binding(&function.body, name) {
				return Err(Diagnostic::at(node, format!(
					"{name} is a main-level variable: declare it `global {name}` to change it from a function, or use a new local name"))
					.fix(format!("global {name}")).into_error());
			}
			if ask_local_or_global(node, name, &function.name, main_assignment(&program, name))? == MAIN_LEVEL_READING
				&& !meant_global.contains(name)
			{
				meant_global.push(name.clone());
			}
		}
	}
	Ok(declare_global(program, &meant_global))
}

/// The functions defined as a block of statements, `inc := {x = x+1}`
pub(super) fn block_function_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Key(name, Op::Define, body) = node {
			if let (Node::Symbol(name), true) = (name.drop_meta(), is_statement_block(body)) {
				names.insert(name.clone());
			}
		}
	});
	names
}

pub(super) fn without_block_bodies(node: Node, blocks: &HashSet<String>) -> Node {
	match node {
		Node::Key(name, Op::Define, _) if name.symbol_name().is_some_and(|name| blocks.contains(name)) => Node::Empty,
		other => children_rewritten(other, |child| without_block_bodies(child, blocks)),
	}
}

/// A block captures the outer variables by value (user decision D7): its assignment changes the block's own copy,
/// so `x=1; inc:={x=x+1}; do inc; x` stays 1. Educate toward the forms that change x: `global x`, or returning the value.
pub(super) fn educate_block_assignment(assignment: &Node, name: &str, block: &str, main_assignment: Option<&Node>) {
	crate::normalize::set_position_of(assignment);
	let written = crate::normalize::operand_text(assignment);
	let reason = format!("the block {block} captures {name} by value: its change stays inside the block");
	crate::normalize::advise(
		&written,
		&format!("global {name}"),
		&reason,
		global_at_main(&reason, name, main_assignment),
	);
	crate::normalize::advise(
		&written,
		&format!("{name} = {block}()"),
		"or return the value from the block and assign it",
		None,
	);
}

pub(super) const LOCAL_OR_GLOBAL: &str = "local-or-global";
pub(crate) const MAIN_LEVEL_READING: usize = 1;

/// The main-level statement that first assigns `name`: `n=0` of `n=0; def f(x){n=5;x}`
pub(super) fn main_assignment<'a>(program: &'a Node, name: &str) -> Option<&'a Node> {
	let statements = match program.drop_meta() {
		Node::List(items, _, _) => items.as_slice(),
		_ => std::slice::from_ref(program),
	};
	statements
		.iter()
		.find(|statement| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, _) if target.is_symbol(name)))
}

/// `n = …` inside f where main has an n: a new local of f (the default, as in Python) or main's n? The fix of main's n
/// declares it where main assigns it: `global n=0`
pub(crate) fn ask_local_or_global(
	assignment: &Node,
	name: &str,
	function: &str,
	main_assignment: Option<&Node>,
) -> Result<usize, Node> {
	use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
	let question =
		format!("does `{name} = …` inside {function} make a new local of {function}, or change the main-level {name}?");
	let main_level = reading(&format!("the main-level {name}"), &format!("global {name}"));
	let main_level = match global_at_main(&main_level.meaning, name, main_assignment) {
		Some(fix) => main_level.fixed_by(fix),
		None => main_level.replacing("", ""), // no main-level assignment to declare: no edit
	};
	let readings = vec![
		reading(&format!("a new local of {function}"), &format!("let {name} = …"))
			.replacing(format!("{name} ="), format!("let {name} =")),
		main_level,
	];
	ask(&Ask::new(LOCAL_OR_GLOBAL, question, readings, Fallback::Warning)
		.written(&format!("{name} = …"))
		.at_node(assignment))
}

/// `global n=0` where main assigns n: the fix that makes every `n = …` change main's n; none without such an assignment
fn global_at_main(meaning: &str, name: &str, main_assignment: Option<&Node>) -> Option<crate::errors::fixits::Fix> {
	let (line, column) = main_assignment.and_then(crate::errors::diagnostic::position)?;
	Some(crate::errors::fixits::fix(meaning, format!("{name} ="), format!("global {name} =")).at(line, column))
}

/// `let n = …`, `var n = …`, `const n = …` in `body`: n is explicitly the function's own
pub(crate) fn declares_local(body: &Node, name: &str) -> bool {
	let mut declared = false;
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if let [keyword, declaration] = items.as_slice() {
				declared |= keyword.symbol_name().is_some_and(is_declaration_word)
					&& global_binding(declaration, &Scope::new()).is_some_and(|binding| binding.name == name);
			}
		}
	});
	declared
}

/// The program's first main-level assignment of each name becomes its `global` declaration (`global n` if none)
pub(crate) fn declare_global(program: Node, names: &[String]) -> Node {
	if names.is_empty() {
		return program;
	}
	let as_global =
		|declaration: Node| Node::Key(Box::new(Node::Symbol("global".to_string())), Op::Colon, Box::new(declaration));
	let (mut items, bracket, separator) = match program {
		Node::List(items, bracket, separator) => (items, bracket, separator),
		single => (vec![single], Bracket::None, Separator::Newline),
	};
	for name in names {
		let assigns = |assignment: &&Node| matches!(assignment, Node::Key(target, Op::Assign, _) if name_before_type(target) == Some(name));
		let found = items.iter().enumerate().find_map(|(index, item)| {
			statement_assignment(item).filter(assigns).map(|assignment| (index, assignment.clone()))
		});
		match found {
			Some((index, assignment)) => items[index] = as_global(assignment),
			None => items.insert(0, as_global(Node::Symbol(name.clone()))),
		}
	}
	Node::List(items, bracket, separator)
}

/// The assignment a main-level statement makes: `k = v`, and of the constant `const k = v` (as `global const k = v`)
fn statement_assignment(item: &Node) -> Option<&Node> {
	match item.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[keyword, assignment] if is_constant_keyword(keyword) => Some(assignment.drop_meta()),
			_ => None,
		},
		assignment => Some(assignment),
	}
}

/// The variable a declaration binds: `k` of `k` and of the typed `k: int`
fn name_before_type(target: &Node) -> Option<&String> {
	match target.drop_meta() {
		Node::Symbol(name) => Some(name),
		Node::Key(name, Op::Colon, _) => name_before_type(name),
		_ => None,
	}
}

/// Is the first mention of `name` in `body` a binding not reading it (`primes = []`, `a, b = 1, 2`, `for x in xs`)?
/// Then the body binds its own local, as in Python; reading or updating it first (`n += 1`, `xs#i = v`) means main's
/// variable.
pub(crate) fn starts_with_fresh_binding(body: &Node, name: &str) -> bool {
	let mut first_mention = None;
	body.visit(&mut |node| {
		if first_mention.is_some() {
			return;
		}
		match node {
			Node::Key(target, Op::Assign, value) if target.is_symbol(name) => {
				first_mention = Some(!mentions(value, name))
			}
			Node::List(items, _, _) => {
				if let Some((names, values)) =
					list_binder(items).filter(|(names, _)| names.iter().any(|n| n.is_symbol(name)))
				{
					first_mention = Some(!values.iter().any(|value| mentions(value, name)));
				}
			}
			Node::Symbol(symbol) if symbol == name => first_mention = Some(false),
			_ => {}
		}
	});
	first_mention.unwrap_or(false)
}

fn mentions(node: &Node, name: &str) -> bool {
	let mut mentioned = false;
	node.visit(&mut |part| mentioned |= matches!(part, Node::Symbol(symbol) if symbol == name));
	mentioned
}

/// The variables a binding list binds and the values it reads before binding them: `$destructure (a, b) v w` (the
/// lowered `a, b = v, w`) and `for x in xs {…}`
fn list_binder(items: &[Node]) -> Option<(&[Node], &[Node])> {
	match items {
		[head, names, values @ ..] if head.is_symbol(crate::tuples::DESTRUCTURE) => match names.drop_meta() {
			Node::List(names, _, _) => Some((names.as_slice(), values)),
			_ => None,
		},
		_ if crate::lowering::nodes::for_in_variable(items).is_some() => {
			let within = items.iter().position(|item| item.is_symbol(crate::words::IN_WORD))?;
			Some((&items[within - 1..within], items.get(within + 1..items.len() - 1).unwrap_or(&[])))
		}
		_ => None,
	}
}

/// Is `node` a loop whose variable it binds (`for x in xs {…}`)?
pub(crate) fn is_loop_binding(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if crate::lowering::nodes::for_in_variable(items).is_some())
}

/// The assignments (kept with their positions) whose changed variable satisfies `wanted`, in source order
pub(crate) fn find_assignments<'a>(node: &'a Node, wanted: &dyn Fn(&String) -> bool) -> Vec<(&'a Node, &'a String)> {
	find_changes_by(node, wanted, |node| assignment_target_root(node).into_iter().collect())
}

/// The bindings (kept with their positions) of a variable satisfying `wanted`, in source order: its assignments,
/// `a, b = …` and `for x in xs`
pub(crate) fn find_bindings<'a>(node: &'a Node, wanted: &dyn Fn(&String) -> bool) -> Vec<(&'a Node, &'a String)> {
	find_changes_by(node, wanted, |node| {
		let bound = match node {
			Node::List(items, _, _) => list_binder(items).map(|(names, _)| names).unwrap_or(&[]),
			_ => &[],
		};
		assignment_target_root(node)
			.into_iter()
			.chain(bound.iter().filter_map(|name| match name.drop_meta() {
				Node::Symbol(name) => Some(name),
				_ => None,
			}))
			.collect()
	})
}

/// The assignments and list updates (`xs.add(x)`, lowered to an assignment later) whose variable satisfies `wanted`
fn find_changes<'a>(node: &'a Node, wanted: &dyn Fn(&String) -> bool) -> Vec<(&'a Node, &'a String)> {
	find_changes_by(node, wanted, |node| {
		assignment_target_root(node).or_else(|| updated_list_root(node)).into_iter().collect()
	})
}

fn find_changes_by<'a>(
	node: &'a Node,
	wanted: &dyn Fn(&String) -> bool,
	changed: fn(&Node) -> Vec<&String>,
) -> Vec<(&'a Node, &'a String)> {
	let mut found: Vec<(&'a Node, &'a String)> =
		changed(node.drop_meta()).into_iter().filter(|name| wanted(name)).map(|name| (node, name)).collect();
	match node.drop_meta() {
		Node::Key(left, _, right) => found
			.extend(find_changes_by(left, wanted, changed).into_iter().chain(find_changes_by(right, wanted, changed))),
		Node::List(items, _, _) => found.extend(items.iter().flat_map(|item| find_changes_by(item, wanted, changed))),
		_ => {}
	}
	found
}

/// The list variable a list method changes: `xs` in `xs.add(x)`, `xs.pop()`
fn updated_list_root(node: &Node) -> Option<&String> {
	let Node::Key(target, Op::Dot, call) = node else {
		return None;
	};
	let Node::List(items, _, _) = call.drop_meta() else {
		return None;
	};
	let is_update =
		matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(method)) if is_list_mutating_method(method));
	match target.drop_meta() {
		Node::Symbol(name) if is_update => Some(name),
		_ => None,
	}
}

/// The variable an assignment changes: `n` in `n = …`, `n += …`, `n++` and `xs#i = …`
pub(super) fn assignment_target_root(node: &Node) -> Option<&String> {
	let target = match node {
		Node::Key(target, Op::Assign | Op::Inc | Op::Dec, _) => target,
		Node::Key(target, op, _) if op.is_compound_assign() => target,
		_ => return None,
	};
	let mut target = target.drop_meta();
	// `xs#i`, `p.x`, and the declared `k: int`
	while let Node::Key(container, Op::Hash | Op::Dot | Op::Colon, _) = target {
		target = container.drop_meta();
	}
	match target {
		Node::Symbol(name) => Some(name),
		_ => None,
	}
}
