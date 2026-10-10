//! Data parallelism with the task keyword (user, 2026-10-06: "a dual use for the go keyword … later running on the
//! GPU"): `go f(x)` and `go { … }` start one task, `go xs.map(f)` and `go for x in xs { … }` one task per chunk of xs.
//! `xs.map(f) @parallel` and `@parallel f all xs` are the same map; `@parallel` on another form warns. The items are
//! split into PARALLEL_CHUNKS slices, each mapped (or looped) in a go block; the map's results come back in order, the
//! loop ends when every chunk is done. A loop body updating an outer variable (`t += x`) updates its task's copy: a
//! warning names `shared` (P106). notes/go_blocks.md

use super::words::{FOR_WORD, IN_WORD, MAP_WORD};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;

/// How many slices a parallel map or loop splits its items into: tasks are threads with an instance each, so never one
/// per item; a later backend (SIMD, GPU) may choose its own split
const PARALLEL_CHUNKS: i64 = 8;
const GO_WORD: &str = "go";
const PARALLEL_ATTRIBUTE: &str = "parallel";
/// The temporaries of a template: `parallel_part` becomes `parallel·part·3` for the third parallel map or loop
const TEMPORARY_PREFIX: &str = "parallel_";
const TEMPORARY_BASE: &str = "parallel";
const LIST_PLACEHOLDER: &str = "go_map_list";
const FUNCTION_PLACEHOLDER: &str = "go_map_function";
const LOOP_PLACEHOLDER: &str = "go_map_loop";

/// The slices of `go_map_list` as tasks in `parallel_jobs`; `{work}` is the go block's body for `parallel_part`
fn chunked(work: &str) -> String {
	format!("parallel_list = {LIST_PLACEHOLDER}; parallel_size = #parallel_list; parallel_chunk = (parallel_size + {PARALLEL_CHUNKS} - 1)//{PARALLEL_CHUNKS}; \
		parallel_jobs = []; parallel_start = 0; \
		while parallel_start < parallel_size {{ parallel_part = parallel_list[parallel_start:parallel_start + parallel_chunk]; parallel_jobs.add({GO_WORD} {{ {work} }}); parallel_start += parallel_chunk }}; \
		parallel_parts = await all parallel_jobs")
}

/// `go xs.map(f)` among a list's items, `xs.map(f) @parallel`, `@parallel xs.map(f)`, `@parallel f all xs`: xs and f
pub(crate) fn parallel_map(node: &Node) -> Option<(Node, Node)> {
	if let Node::List(items, _, _) = node.drop_meta() {
		if let [go, map] = items.as_slice() {
			if go.drop_meta().name() == GO_WORD {
				return map_call(map);
			}
		}
	}
	is_annotated(node, PARALLEL_ATTRIBUTE).then(|| map_call(node).or_else(|| all_call(node))).flatten()
}

/// `@parallel` on a form parallel_map can't split into tasks (`@parallel square xs`): a warning (an error under strict)
/// instead of running it in sequence without a word; said once, by the innermost form the annotation is on
pub(crate) fn sequential_warning(node: &Node) -> Option<Node> {
	if !annotated_here(node, PARALLEL_ATTRIBUTE) {
		return None;
	}
	let message = "@parallel runs this in sequence: only `xs.map(f)`, `f all xs` and `go for x in xs { … }` run in parallel";
	crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(node, message.to_string())]).err()
}

/// `@parallel` (or another attribute, `@gpu`) on a form or on the start of its first part: `@parallel xs.map(f)`
/// annotates xs, `@parallel square all xs` (read as `(square all) xs`) square
pub(crate) fn is_annotated(node: &Node, attribute: &str) -> bool {
	annotated_here(node, attribute) || head(node).is_some_and(|head| is_form(head) && is_annotated(head, attribute))
}

/// The attribute on a form or on its first part when that is no form itself
pub(crate) fn annotated_here(node: &Node, attribute: &str) -> bool {
	head(node).is_some_and(|head| node.attribute(attribute).is_some() || (!is_form(head) && head.attribute(attribute).is_some()))
}

/// The first part of a form: the left of a key, the first item of a list
fn head(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Key(left, _, _) => Some(left.as_ref()),
		Node::List(items, _, _) => items.first(),
		_ => None,
	}
}

fn is_form(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(..) | Node::List(_, Bracket::None | Bracket::Round, _))
}

/// `f all xs` (wiki/all.md): xs and f
fn all_call(node: &Node) -> Option<(Node, Node)> {
	let Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) = node.drop_meta() else { return None };
	let (function, list) = crate::broadcasting::all_call_parts(items)?;
	Some((list.drop_meta().clone(), function.clone()))
}

/// `xs.map(f)`: xs and f
pub(crate) fn map_call(node: &Node) -> Option<(Node, Node)> {
	let Node::Key(list, Op::Dot, call) = node.drop_meta() else { return None };
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let [map, function] = items.as_slice() else { return None };
	(map.drop_meta().name() == MAP_WORD).then(|| (list.drop_meta().clone(), function.clone()))
}

/// `go for x in xs { body }`: x, xs and the body
pub(crate) fn parallel_loop(items: &[Node]) -> Option<(Node, Node, Node)> {
	match items {
		[go, word, variable, in_word, list, body] if go.drop_meta().name() == GO_WORD && word.drop_meta().name() == FOR_WORD && in_word.drop_meta().name() == IN_WORD => {
			Some((variable.clone(), list.clone(), body.clone()))
		}
		[go, rest] if go.drop_meta().name() == GO_WORD => match rest.drop_meta() {
			Node::List(inner, Bracket::None, Separator::Space) => parallel_loop(&[vec![go.clone()], inner.clone()].concat()),
			_ => None,
		},
		_ => None,
	}
}

/// The map over slices in tasks, its results joined in order; a function value written in place (`x => x + 1`) is
/// named first, so each task gets it as a value
pub(crate) fn map_in_tasks(list: Node, function: Node, number: usize) -> Node {
	let named = !matches!(function.drop_meta(), Node::Symbol(_));
	let mapped = if named { "parallel_function" } else { FUNCTION_PLACEHOLDER };
	let naming = if named { format!("parallel_function = {FUNCTION_PLACEHOLDER}; ") } else { String::new() };
	let program = format!("({naming}{}; parallel_results = []; for parallel_done in parallel_parts {{ parallel_results = parallel_results + parallel_done }}; parallel_results)",
		chunked(&format!("parallel_part.map({mapped})")));
	filled(&program, number, &[(LIST_PLACEHOLDER, list), (FUNCTION_PLACEHOLDER, function)])
}

/// The loop over slices in tasks; it ends when every slice is done and gives ø
pub(crate) fn loop_in_tasks(variable: Node, list: Node, body: Node, number: usize, shared: &[String]) -> Node {
	if let Some(error) = copied_updates(&body, shared) {
		return error;
	}
	let walk = Node::List(vec![symbol(FOR_WORD), variable, symbol(IN_WORD), temporary("part", number), body], Bracket::None, Separator::Space);
	let program = format!("({}; ø)", chunked(&format!("{LOOP_PLACEHOLDER}; 0")));
	filled(&program, number, &[(LIST_PLACEHOLDER, list), (LOOP_PLACEHOLDER, walk)])
}

/// The template with its temporaries named apart, then the placeholders filled (the program's own `parallel_…` names
/// stay as they are)
fn filled(program: &str, number: usize, fills: &[(&str, Node)]) -> Node {
	let template = crate::library_words::named_apart(crate::warp_parser::parse(program), TEMPORARY_PREFIX, TEMPORARY_BASE, number);
	fills.iter().fold(template, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, value))
}


/// `part` of the third parallel map or loop: `parallel·part·3`
fn temporary(name: &str, number: usize) -> Node {
	Node::Symbol(crate::library_words::temporary_name(&[TEMPORARY_BASE, name, &number.to_string()]))
}

/// `t += x`, `t++`, `t = t + x` in a parallel loop body of a variable that is not shared: each task would update its own
/// copy. A warning naming `shared` (an error under strict)
fn copied_updates(body: &Node, shared: &[String]) -> Option<Node> {
	let mut updated: Option<(String, Node)> = None;
	body.visit(&mut |part| if let Node::Key(target, op, value) = part {
		let Node::Symbol(name) = target.drop_meta() else { return };
		let updates = op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) || (*op == Op::Assign && crate::warp_parser::mentions(value, name));
		if updates && !shared.contains(name) && updated.is_none() {
			updated = Some((name.clone(), part.clone()));
		}
	});
	let (name, at) = updated?;
	let message = format!("{name} is updated in a parallel loop: every task updates its own copy; share it: `shared {name} = …` (P106)");
	crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(&at, message)]).err()
}
