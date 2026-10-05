//! Constant calls are folded (wiki/charged.md §3 "Precomputed and precompiled paths"): in a compiled module, a call
//! of a pure, terminating function with constant arguments is computed at compile time and replaced by its value
//! (`def fib(n): …; fib(10)` compiles to 55). Only `compile` folds: `eval` would do the same work twice.
//! The values come from running the definitions and the calls as one small module, so they are exactly what the
//! program computes; a call that fails or runs out of fuel stays a call.

use crate::analyzer::{captured_variables, collect_variables, Scope};
use crate::effects::EffectReport;
use crate::extensions::numbers::Number;
use crate::late_binding::{functions_in, statements_of};
use crate::node::{Bracket, Node, Separator};
use crate::function_values::{definitions, Definition};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The longest chain of variants one constant argument may unroll (`power(x, 3)` → power·n3 → … → power·n0); a deeper
/// one (or one that never ends) keeps the original call
const MAX_VARIANTS: usize = 32;
/// Between a function's name and the constant arguments of its variant: `power·n3`
const VARIANT_SEPARATOR: &str = "·";

/// The precomputed paths of a compiled module: constant arguments specialised, the variants inlined where small,
/// constant calls folded
pub fn precompute(program: Node) -> Node {
	fold_constant_calls(specialise_constant_arguments(program))
}

pub fn fold_constant_calls(program: Node) -> Node {
	let statements = statements_of(&program);
	let foldable = foldable_functions(&program, &statements);
	let mut calls: Vec<Node> = vec![];
	for statement in statements.iter().filter(|statement| functions_in(statement).is_empty()) {
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
	functions_within(program, statements, crate::effects::EffectSet::PURE)
}

/// The functions whose effects stay within `allowed` and that read no main-level variable
fn functions_within(program: &Node, statements: &[Node], allowed: crate::effects::EffectSet) -> HashSet<String> {
	let report = EffectReport::of(program);
	let mut main = Scope::new();
	collect_variables(program, &mut main);
	statements.iter().flat_map(functions_in)
		.filter(|function| report.effects_of(&function.name).is_some_and(|effects| effects.is_subset_of(allowed)))
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


/// Specialised paths (wiki/charged.md §3): a call of a pure function (as folding chooses them) with some constant Int
/// arguments calls a variant with those constants in place and every condition on them decided, recursively:
/// `power(y, 3)` calls power·n3, whose body is `y * power·n2(y)`, down to power·n0 = 1; inlining then removes the chain
pub fn specialise_constant_arguments(program: Node) -> Node {
	let statements = statements_of(&program);
	// possible divergence is allowed: specialising is bounded (MAX_VARIANTS), and the call keeps its meaning
	let pure = functions_within(&program, &statements, crate::effects::EffectSet::of(&[crate::effects::Effect::Div]));
	let mut found = vec![];
	definitions(&program, &mut found);
	let definitions: HashMap<String, Definition> = found.into_iter().filter(|definition| pure.contains(&definition.name)).map(|definition| (definition.name.clone(), definition)).collect();
	if definitions.is_empty() {
		return program;
	}
	let mut specialiser = Specialiser { definitions, variants: vec![], names: vec![] };
	let rewritten = specialiser.rewrite(program);
	if specialiser.variants.is_empty() {
		return rewritten;
	}
	let rewritten = specialiser.inlined(rewritten, 0);
	let variants: Vec<Node> = specialiser.variants.iter().filter(|(name, _)| calls(&rewritten, name) || specialiser.variants.iter().any(|(other, definition)| other != name && calls(definition, name)))
		.map(|(_, definition)| definition.clone()).collect();
	match rewritten {
		Node::List(items, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => Node::List([variants, items].concat(), Bracket::None, separator),
		single => Node::List([variants, vec![single]].concat(), Bracket::None, Separator::Semicolon),
	}
}

struct Specialiser {
	definitions: HashMap<String, Definition>,
	/// the variants made, in order: (key, definition)
	variants: Vec<(String, Node)>,
	/// the variant names given so far, in order (also those still being made, so a cycle calls itself)
	names: Vec<String>,
}

impl Specialiser {
	/// Calls with some constant arguments of a pure function replaced by calls of their variants
	fn rewrite(&mut self, node: Node) -> Node {
		let node = match node {
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator),
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		};
		self.specialised_call(&node, 0).unwrap_or(node)
	}

	/// `f(y, 3)` → `f·n3(y)`, or None when f is no pure function, no argument or every argument is constant (folding
	/// takes that), or the chain of variants would be too deep
	fn specialised_call(&mut self, call: &Node, depth: usize) -> Option<Node> {
		let Node::List(items, Bracket::Round, _) = call.drop_meta() else { return None };
		let (head, arguments) = items.split_first()?;
		let Node::Symbol(name) = head.drop_meta() else { return None };
		let definition = self.definitions.get(name)?.clone();
		let parameters = definition.param_names();
		if parameters.len() != arguments.len() {
			return None;
		}
		let constant = |argument: &Node| matches!(argument.drop_meta(), Node::Number(Number::Int(_)));
		let constants: Vec<(String, Node)> = parameters.iter().zip(arguments).filter(|(_, argument)| constant(argument)).map(|(parameter, argument)| (parameter.clone(), argument.drop_meta().clone())).collect();
		if constants.is_empty() || constants.len() == arguments.len() {
			return None;
		}
		let variant = self.variant(&definition, &constants, depth)?;
		let kept: Vec<Node> = arguments.iter().filter(|argument| !constant(argument)).cloned().collect();
		Some(Node::List([vec![Node::Symbol(variant)], kept].concat(), Bracket::Round, Separator::None))
	}

	/// The variant of `definition` with `constants` in place, made once per key; None (and nothing kept of the attempt)
	/// when its chain reaches MAX_VARIANTS
	fn variant(&mut self, definition: &Definition, constants: &[(String, Node)], depth: usize) -> Option<String> {
		let suffix: Vec<String> = constants.iter().map(|(parameter, value)| format!("{parameter}{}", value.serialize())).collect();
		let key = format!("{}{VARIANT_SEPARATOR}{}", definition.name, suffix.join(VARIANT_SEPARATOR));
		if self.names.contains(&key) {
			return Some(key);
		}
		if depth >= MAX_VARIANTS {
			return None;
		}
		let checkpoint = (self.variants.len(), self.names.len());
		self.names.push(key.clone());
		let mut body = definition.body.clone();
		for (parameter, value) in constants {
			body = crate::library_words::substitute(body, parameter, value);
		}
		match self.specialise_body(decided(body), depth + 1) {
			Some(body) => {
				let parameters: Vec<Node> = definition.params.iter().zip(definition.param_names())
					.filter(|(_, name)| !constants.iter().any(|(constant, _)| constant == name)).map(|(parameter, _)| parameter.clone()).collect();
				let head = Node::List([vec![Node::Symbol(key.clone())], parameters].concat(), Bracket::Round, Separator::None);
				self.variants.push((key.clone(), Node::Key(Box::new(head), Op::Define, Box::new(body))));
				Some(key)
			}
			None => {
				self.variants.truncate(checkpoint.0);
				self.names.truncate(checkpoint.1);
				None
			}
		}
	}

	/// Calls of variants with plain arguments (names, numbers) replaced by the variant's body, the arguments in place of
	/// its parameters, as long as that body calls no variant whose arguments are not plain: power·n3(y) → y*(y*(y*1))
	fn inlined(&self, node: Node, depth: usize) -> Node {
		let node = match node {
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.inlined(item, depth)).collect(), bracket, separator),
			Node::Key(left, op, right) => Node::Key(Box::new(self.inlined(*left, depth)), op, Box::new(self.inlined(*right, depth))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.inlined(*node, depth)), data },
			other => other,
		};
		let Node::List(items, Bracket::Round, _) = node.drop_meta() else { return node };
		let Some((head, arguments)) = items.split_first() else { return node };
		let plain = arguments.iter().all(|argument| matches!(argument.drop_meta(), Node::Symbol(_) | Node::Number(_)));
		let Some((_, Node::Key(variant_head, Op::Define, body))) = self.variants.iter().find(|(name, _)| matches!(head.drop_meta(), Node::Symbol(called) if called == name)) else { return node };
		let Node::List(head_items, _, _) = variant_head.drop_meta() else { return node };
		if !plain || depth >= MAX_VARIANTS || !is_expression(body) {
			return node;
		}
		let mut inlined = body.as_ref().clone();
		for (parameter, argument) in head_items[1..].iter().zip(arguments) {
			inlined = crate::library_words::substitute(inlined, &parameter.drop_meta().name(), argument);
		}
		self.inlined(inlined, depth + 1)
	}

	/// A variant's body with its own calls specialised in turn; None when one of them is too deep
	fn specialise_body(&mut self, node: Node, depth: usize) -> Option<Node> {
		let node = match node {
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.specialise_body(item, depth)).collect::<Option<_>>()?, bracket, separator),
			Node::Key(left, op, right) => Node::Key(Box::new(self.specialise_body(*left, depth)?), op, Box::new(self.specialise_body(*right, depth)?)),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.specialise_body(*node, depth)?), data },
			other => other,
		};
		let is_specialisable = |node: &Node| matches!(node.drop_meta(), Node::List(items, Bracket::Round, _)
			if items.len() > 1 && matches!(items[0].drop_meta(), Node::Symbol(name) if self.definitions.contains_key(name))
			&& items[1..].iter().any(|argument| matches!(argument.drop_meta(), Node::Number(Number::Int(_)))));
		if !is_specialisable(&node) {
			return Some(node);
		}
		let all_constant = matches!(node.drop_meta(), Node::List(items, _, _) if items[1..].iter().all(|argument| matches!(argument.drop_meta(), Node::Number(Number::Int(_)))));
		if all_constant {
			return Some(node); // folding computes it
		}
		self.specialised_call(&node, depth)
	}
}

/// Arithmetic and comparisons of Int constants computed, and a condition that is a constant decided: what is left of a
/// body once a parameter is a constant
fn decided(node: Node) -> Node {
	let node = match node {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(decided).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(decided(*left)), op, Box::new(decided(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(decided(*node)), data },
		other => other,
	};
	let int = |node: &Node| match node.drop_meta() {
		Node::Number(Number::Int(value)) => Some(*value),
		_ => None,
	};
	match node.drop_meta() {
		Node::Key(left, op, right) if int(left).is_some() && int(right).is_some() => {
			let (a, b) = (int(left).expect("guarded"), int(right).expect("guarded"));
			let value = match op {
				Op::Add => a.checked_add(b),
				Op::Sub => a.checked_sub(b),
				Op::Mul => a.checked_mul(b),
				Op::Eq => Some((a == b) as i64),
				Op::Ne => Some((a != b) as i64),
				Op::Lt => Some((a < b) as i64),
				Op::Le => Some((a <= b) as i64),
				Op::Gt => Some((a > b) as i64),
				Op::Ge => Some((a >= b) as i64),
				_ => None,
			};
			value.map_or(node.clone(), |value| Node::Number(Number::Int(value)))
		}
		// `if c then a else b` with c decided
		Node::Key(if_then, Op::Else, otherwise) => match if_then.drop_meta() {
			Node::Key(head, Op::Then, chosen) => match head.drop_meta() {
				Node::Key(empty, Op::If, condition) if matches!(empty.drop_meta(), Node::Empty) && int(condition).is_some() => {
					if int(condition) != Some(0) { chosen.as_ref().clone() } else { otherwise.as_ref().clone() }
				}
				_ => node,
			},
			_ => node,
		},
		// `c ? a : b` with c decided
		Node::Key(condition, Op::Question, branches) if int(condition).is_some() => match branches.drop_meta() {
			Node::Key(chosen, Op::Colon, otherwise) => if int(condition) != Some(0) { chosen.as_ref().clone() } else { otherwise.as_ref().clone() },
			_ => node,
		},
		_ => node,
	}
}

/// An expression without statements, definitions or blocks: its value is all it does, so it can stand where it is called
fn is_expression(body: &Node) -> bool {
	let mut plain = true;
	body.visit(&mut |part| plain &= !matches!(part, Node::Key(_, Op::Define | Op::Assign, _) | Node::List(_, Bracket::Curly, _) | Node::List(_, _, Separator::Semicolon | Separator::Newline)));
	plain
}

/// Does `node` call the function `name`
fn calls(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(called)) if called == name)));
	found
}
