//! Memoization (wiki/charged.md §3, P72: the compiler decides alone, no syntax): a pure recursive function of one Int
//! parameter that returns an Int and calls itself more than once (fib-shaped, its calls overlap) keeps the results of
//! arguments 0 to MEMO_SIZE-1: `f(n) := body` becomes `f(n) := if memo_known(id, n) then memo_value(id, n) else
//! memo_store(id, n, body)` (one function, so it stays plainly recursive for folding and effects); the three words are
//! the emitter's (wasm_emitter/text_builtins.rs), one cache per function id. Only an expression body (no statements, no
//! `return`); effects, divergence aside, are never cached.

use super::nodes::{is_call_head, key};
use crate::analyzer::effects::{Effect, EffectReport, EffectSet};
use crate::analyzer::{captured_variables, collect_variables, param_kind, Scope};
use crate::compiler::context::UserFunctionDef;
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;

pub const MEMO_KNOWN: &str = "memo_known";
pub const MEMO_VALUE: &str = "memo_value";
pub const MEMO_STORE: &str = "memo_store";
/// The arguments 0 … MEMO_SIZE-1 are cached; any other is computed as before
pub const MEMO_SIZE: i32 = 4096;
const ARGUMENT_PLACEHOLDER: &str = "memo_argument";
const BODY_PLACEHOLDER: &str = "memo_body";

pub fn lower(program: Node) -> Node {
	let context = crate::analyzer::function_context(&program);
	// the shape first: the effects and variables of the whole program only when some function has it
	let candidates: Vec<&UserFunctionDef> =
		context.user_functions.values().filter(|function| has_memoizable_shape(function)).collect();
	if candidates.is_empty() {
		return program;
	}
	let report = EffectReport::of(&program);
	let mut main = Scope::new();
	collect_variables(&program, &mut main);
	let mut memoized: Vec<String> = candidates
		.into_iter()
		.filter(|function| is_memoizable(function, &report, &main))
		.map(|function| function.name.clone())
		.collect();
	memoized.sort();
	if memoized.is_empty() {
		return program;
	}
	rewritten(program, &memoized)
}

/// One Int parameter, an Int result, an expression body calling the function itself at least twice
fn has_memoizable_shape(function: &UserFunctionDef) -> bool {
	let [param] = function.params.as_slice() else {
		return false;
	};
	let mut self_calls = 0;
	function.body.visit(&mut |part| {
		if let Node::List(items, _, _) = part.drop_meta() {
			self_calls += matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if *name == function.name)
				as usize;
		}
	});
	let mut expression = true;
	function.body.visit(&mut |part| {
		expression &= !matches!(
			part.drop_meta(),
			Node::List(_, Bracket::Curly, _) | Node::List(_, _, Separator::Semicolon | Separator::Newline)
		) && part.drop_meta().name() != "return"
	});
	expression
		&& self_calls >= 2
		&& param_kind(param) == Kind::Int
		&& function.return_kind == Kind::Int
		&& function.tuple_kinds.is_empty()
}

/// A function of that shape that is pure (but for dividing) and reads no variable of main
fn is_memoizable(function: &UserFunctionDef, report: &EffectReport, main: &Scope) -> bool {
	report.effects_of(&function.name).is_some_and(|effects| effects.is_subset_of(EffectSet::of(&[Effect::Div])))
		&& captured_variables(function, main).is_empty()
}

/// A constructor of a definition of the same form for another head and body
pub(crate) type Rebuild = Box<dyn Fn(Node, Node) -> Node>;

/// A definition `f(n) := body`, `def f(n) = body` (`(f n) = body`) or `def f(n): body` (the keyword form, not yet
/// lowered here): its head, body and a constructor of the same form for another head and body, which keeps the
/// definition's meta (its comment, which `f.doc` reads)
pub(crate) fn definition_parts(node: &Node) -> Option<(Node, Node, Rebuild)> {
	match node {
		Node::Meta { node: inner, data } => {
			let data = data.clone();
			wrapped(definition_parts(inner), move |node| Node::Meta { node: Box::new(node), data: data.clone() })
		}
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if *op == Op::Define || is_call_head(head) => {
			key_parts(head, *op, body)
		}
		Node::List(items, bracket, separator) => match items.as_slice() {
			[keyword, definition] if crate::operators::is_function_keyword(&keyword.drop_meta().name()) => {
				let (keyword, bracket, separator) = (keyword.clone(), bracket.clone(), separator.clone());
				wrapped(keyword_definition_parts(definition.drop_meta()), move |definition| {
					Node::List(vec![keyword.clone(), definition], bracket.clone(), separator.clone())
				})
			}
			_ => None,
		},
		_ => None,
	}
}

/// What follows the keyword of `def f(n): body`, `def f(n) := body` and `def f() { … }` (as parsed: the head and the
/// block)
fn keyword_definition_parts(definition: &Node) -> Option<(Node, Node, Rebuild)> {
	match definition {
		Node::Key(head, op @ (Op::Colon | Op::Define), body) => key_parts(head, *op, body),
		Node::List(parts, bracket, separator) => match parts.as_slice() {
			[head, body] if is_call_head(head) && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
				let (bracket, separator) = (bracket.clone(), separator.clone());
				Some((
					head.clone(),
					body.clone(),
					Box::new(move |head, body| Node::List(vec![head, body], bracket.clone(), separator.clone())),
				))
			}
			_ => None,
		},
		_ => None,
	}
}

fn key_parts(head: &Node, op: Op, body: &Node) -> Option<(Node, Node, Rebuild)> {
	Some((head.clone(), body.clone(), Box::new(move |head, body| key(head, op, body))))
}

/// The parts of a definition rebuilt inside what `wrap` makes of it
fn wrapped(
	parts: Option<(Node, Node, Rebuild)>,
	wrap: impl Fn(Node) -> Node + 'static,
) -> Option<(Node, Node, Rebuild)> {
	let (head, body, rebuild) = parts?;
	Some((head, body, Box::new(move |head, body| wrap(rebuild(head, body)))))
}

/// The parameter names of a definition's head `(f a b)`
pub(crate) fn head_parameters(head: &Node) -> Vec<String> {
	match head.drop_meta() {
		Node::List(items, _, _) => items.iter().skip(1).filter_map(super::nodes::parameter_name).collect(),
		_ => vec![],
	}
}

/// Every definition of a memoized function with its body behind the cache
fn rewritten(node: Node, memoized: &[String]) -> Node {
	if let Some(cached) = cached(&node, memoized) {
		return cached;
	}
	node.map_children(|child| rewritten(child, memoized))
}

/// The definition of a memoized function with its body behind the cache
fn cached(node: &Node, memoized: &[String]) -> Option<Node> {
	if let Some((head, body, make)) = definition_parts(node) {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			let id = items.first().and_then(|name| memoized.iter().position(|memo| *memo == name.drop_meta().name()));
			if let ([_, param], Some(id)) = (items.as_slice(), id) {
				let template = crate::warp_parser::parse(&format!(
					"if {MEMO_KNOWN}({id}, {ARGUMENT_PLACEHOLDER}) then {MEMO_VALUE}({id}, {ARGUMENT_PLACEHOLDER}) else {MEMO_STORE}({id}, {ARGUMENT_PLACEHOLDER}, {BODY_PLACEHOLDER})"));
				let argument = Node::Symbol(match param.drop_meta() {
					Node::Key(name, _, _) => name.drop_meta().name(), // `n: int`
					other => other.name(),
				});
				let cached_body =
					substitute(substitute(template, ARGUMENT_PLACEHOLDER, &argument), BODY_PLACEHOLDER, &body);
				return Some(make(head.clone(), cached_body));
			}
		}
	}
	None
}
