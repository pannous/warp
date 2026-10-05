//! Constant calls are folded (wiki/charged.md §3 "Precomputed and precompiled paths"): in a compiled module, a call
//! of a pure, terminating function with constant arguments is computed at compile time and replaced by its value
//! (`def fib(n): …; fib(10)` compiles to 55), also inside a function body or a loop (an invariant part hoisted to
//! compile time). Only `compile` folds: `eval` would do the same work twice.
//! The values come from running the definitions and the calls as one small module, so they are exactly what the
//! program computes; a call that fails or runs out of fuel stays a call.

use crate::analyzer::{captured_variables, collect_variables, Scope};
use crate::effects::EffectReport;
use crate::extensions::numbers::Number;
use crate::late_binding::{functions_in, statements_of};
use crate::node::{Bracket, Node, Separator};
use std::collections::HashSet;

pub fn fold_constant_calls(program: Node) -> Node {
	let statements = statements_of(&program);
	let foldable = foldable_functions(&program, &statements);
	let mut calls: Vec<Node> = vec![];
	// main-level statements and function bodies alike: a constant call in a body is computed once, not at every call
	for statement in &statements {
		statement.visit(&mut |node| {
			if is_constant_call(node, &foldable) && !calls.contains(node) {
				calls.push(node.clone());
			}
		});
	}
	if calls.is_empty() {
		return program;
	}
	let definitions = statements.iter().filter(|statement| functions_in(statement).iter().any(|function| foldable.contains(&function.name)));
	let probe = [definitions.cloned().collect(), vec![Node::List(calls.clone(), Bracket::Square, Separator::Colon)]].concat();
	let Ok(module) = crate::wasm_emitter::emit_module(&Node::List(probe, Bracket::None, Separator::Newline)) else { return program };
	let values = match crate::pipeline::run_module(module).drop_meta() {
		Node::List(values, _, _) if values.len() == calls.len() => values.clone(),
		_ => return program,
	};
	let folded: Vec<(Node, Node)> = calls.into_iter().zip(values).filter(|(_, value)| is_constant(value)).collect();
	replaced(program, &folded)
}

/// Pure functions (no effect, not even possible divergence) that read no main-level variable: their value depends
/// only on their arguments
fn foldable_functions(program: &Node, statements: &[Node]) -> HashSet<String> {
	let report = EffectReport::of(program);
	let mut main = Scope::new();
	collect_variables(program, &mut main);
	statements.iter().flat_map(functions_in)
		.filter(|function| report.effects_of(&function.name).is_some_and(|effects| effects.is_pure()))
		.filter(|function| captured_variables(function, &main).is_empty())
		.map(|function| function.name)
		.collect()
}

fn is_constant(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(Number::Int(_) | Number::Float(_)) | Node::Text(_) | Node::Char(_))
}

/// `f(10)`, `f(2, "a")`: a foldable function applied to constants only
fn is_constant_call(node: &Node, foldable: &HashSet<String>) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() > 1 =>
			matches!(items[0].drop_meta(), Node::Symbol(name) if foldable.contains(name)) && items[1..].iter().all(is_constant),
		_ => false,
	}
}

fn replaced(node: Node, folded: &[(Node, Node)]) -> Node {
	if let Some((_, value)) = folded.iter().find(|(call, _)| *call == node) {
		return value.drop_meta().clone();
	}
	match node {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| replaced(item, folded)).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(replaced(*left, folded)), op, Box::new(replaced(*right, folded))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(replaced(*node, folded)), data },
		other => other,
	}
}
