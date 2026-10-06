//! Data parallelism with the task keyword (user, 2026-10-06: "a dual use for the go keyword … later running on the
//! GPU"): `go f(x)` and `go { … }` start one task, `go xs.map(f)` and `go for x in xs { … }` one task per chunk of xs.
//! `xs.map(f) @parallel` is the same map. The items are split into PARALLEL_CHUNKS slices, each mapped (or looped) in
//! a go block; the map's results come back in order, the loop ends when every chunk is done. A loop body updating an
//! outer variable (`t += x`) updates its task's copy: a warning names `shared` (P106). notes/go_blocks.md

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

/// How many slices a parallel map or loop splits its items into: tasks are threads with an instance each, so never one
/// per item; a later backend (SIMD, GPU) may choose its own split
const PARALLEL_CHUNKS: i64 = 8;
const GO_WORD: &str = "go";
const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
const MAP_WORD: &str = "map";
const PARALLEL_ATTRIBUTE: &str = "parallel";
/// The temporaries of a template: `parallel_part` becomes `parallel·part·3` for the third parallel map or loop
const TEMPORARY_PREFIX: &str = "parallel_";
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

/// `go xs.map(f)` among a list's items, `xs.map(f) @parallel`, `@parallel xs.map(f)`: xs and f
pub(crate) fn parallel_map(node: &Node) -> Option<(Node, Node)> {
	if let Node::List(items, _, _) = node.drop_meta() {
		return match items.as_slice() {
			[go, map] if go.drop_meta().name() == GO_WORD => map_call(map),
			_ => None,
		};
	}
	let Node::Key(list, Op::Dot, _) = node.drop_meta() else { return None };
	let annotated = node.attribute(PARALLEL_ATTRIBUTE).is_some() || list.attribute(PARALLEL_ATTRIBUTE).is_some();
	annotated.then(|| map_call(node)).flatten()
}

/// `xs.map(f)`: xs and f
fn map_call(node: &Node) -> Option<(Node, Node)> {
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
	let walk = Node::List(vec![Node::Symbol(FOR_WORD.into()), variable, Node::Symbol(IN_WORD.into()), temporary("part", number), body], Bracket::None, Separator::Space);
	let program = format!("({}; ø)", chunked(&format!("{LOOP_PLACEHOLDER}; 0")));
	filled(&program, number, &[(LIST_PLACEHOLDER, list), (LOOP_PLACEHOLDER, walk)])
}

/// The template with its temporaries named apart, then the placeholders filled (the program's own `parallel_…` names
/// stay as they are)
fn filled(program: &str, number: usize, fills: &[(&str, Node)]) -> Node {
	let template = named_apart(crate::wasp_parser::parse(program), number);
	fills.iter().fold(template, |node, (placeholder, value)| crate::library_words::substitute(node, placeholder, value))
}

fn named_apart(node: Node, number: usize) -> Node {
	match node {
		Node::Symbol(name) if name.starts_with(TEMPORARY_PREFIX) => temporary(&name[TEMPORARY_PREFIX.len()..], number),
		other => other.map_children(|child| named_apart(child, number)),
	}
}

/// `part` of the third parallel map or loop: `parallel·part·3`
fn temporary(name: &str, number: usize) -> Node {
	Node::Symbol(format!("parallel·{name}·{number}"))
}

/// `t += x`, `t++`, `t = t + x` in a parallel loop body of a variable that is not shared: each task would update its own
/// copy. A warning naming `shared` (an error under strict)
fn copied_updates(body: &Node, shared: &[String]) -> Option<Node> {
	let mut updated: Option<(String, Node)> = None;
	body.visit(&mut |part| if let Node::Key(target, op, value) = part {
		let Node::Symbol(name) = target.drop_meta() else { return };
		let updates = op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) || (*op == Op::Assign && crate::wasp_parser::mentions(value, name));
		if updates && !shared.contains(name) && updated.is_none() {
			updated = Some((name.clone(), part.clone()));
		}
	});
	let (name, at) = updated?;
	let message = format!("{name} is updated in a parallel loop: every task updates its own copy; share it: `shared {name} = …` (P106)");
	crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(&at, message)]).err()
}
