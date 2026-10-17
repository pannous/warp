//! Reading function definitions: nested ones lifted as `outer·inner` and their calls renamed, parameters with their annotations and defaults, `it` and `$n`, the `def`/`fun`/`fn` forms
use super::*;
use crate::words::{LIST_WORD, OF_WORD};

pub(crate) fn extract_user_functions_inner(ctx: &mut Context, node: &Node) {
	extract_user_functions_in(ctx, node, None);
}

/// Nested `def` / `f() = …` inside a function body becomes a top-level UserFunctionDef named `outer·inner`, with call
/// sites in the enclosing body rewritten to that mangled name (wiki/charged.md §3 nested defs; card g-qUkY step 1).
pub(in crate::analyzer) fn extract_user_functions_in(ctx: &mut Context, node: &Node, enclosing: Option<&str>) {
	match node.drop_meta() {
		Node::Key(left, Op::Assign, body) if let Some((name, items, bracket)) = signature(left) => {
			register_user_function(ctx, name, extract_params(items, bracket), body, enclosing);
		}
		Node::Key(left, Op::Define, body) if let Some((name, params)) = defined_function(left, body) => {
			register_user_function(ctx, name, params, body, enclosing);
		}
		Node::List(items, _, _) if let Some(function) = keyword_definition(items) => {
			register_user_function(ctx, &function.name, function.params, &function.body, enclosing);
		}
		Node::List(items, _, _) => {
			for item in items {
				extract_user_functions_in(ctx, item, enclosing);
			}
		}
		Node::Key(left, _, right) => {
			extract_user_functions_in(ctx, left, enclosing);
			extract_user_functions_in(ctx, right, enclosing);
		}
		_ => {}
	}
}

/// `f(a, b)`, `f a b`: the name a signature list starts with, its items (the name first) and its bracket
fn signature(node: &Node) -> Option<(&str, &[Node], &Bracket)> {
	let Node::List(items, bracket, _) = node.drop_meta() else {
		return None;
	};
	Some((items.first()?.symbol_name()?, items, bracket))
}

/// `def f(x) {…}`, `fn f …`: the function a statement headed by a function keyword defines
fn keyword_definition(items: &[Node]) -> Option<UserFunctionDef> {
	let keyword = items.first()?.symbol_name()?;
	(items.len() >= 2 && is_function_keyword(keyword)).then(|| extract_def_function(&items[1..])).flatten()
}

/// The name and parameters of the function `left := body` defines: `f x := …`, `f() := [1, 2]` (none), `f := …it…`
/// (`it`), `f := { statements }` (`it` when read outside its own loops); None for a value, `x := 5`
fn defined_function<'a>(left: &'a Node, body: &Node) -> Option<(&'a str, Vec<Param>)> {
	let implicit_param = uses_dollar_param(body) || uses_it(body);
	let it = || vec![Param::untyped(crate::words::IMPLICIT_PARAMETER)];
	if let Some((name, items, bracket)) = signature(left) {
		let params = extract_params(items, bracket);
		let written_call = *bracket == Bracket::Round;
		return match params.is_empty() {
			false => Some((name, params)),
			true if implicit_param => Some((name, it())),
			true => written_call.then_some((name, params)),
		};
	}
	let name = left.symbol_name()?;
	let statement_block = is_statement_block(body);
	if !implicit_param && !statement_block {
		return None;
	}
	// a block of statements takes `it` when it reads it outside its own `for 1..n {…it…}` loops
	let takes_it = !statement_block || uses_it_outside_loops(body) || uses_dollar_param(body);
	Some((name, if takes_it { it() } else { vec![] }))
}

/// Between an enclosing function and a nested `def` lifted out of its body: `outer·inner`
pub(crate) const NESTED_DEF_SEPARATOR: &str = "·";

pub(crate) fn qualify_nested_name(enclosing: Option<&str>, name: &str) -> String {
	match enclosing {
		Some(parent) => format!("{parent}{NESTED_DEF_SEPARATOR}{name}"),
		None => name.to_string(),
	}
}

pub(in crate::analyzer) fn register_user_function(
	ctx: &mut Context,
	short_name: &str,
	params: Vec<Param>,
	body: &Node,
	enclosing: Option<&str>,
) {
	let name = qualify_nested_name(enclosing, short_name);
	let (body, renames) = lift_nested_defs_from_body(ctx, body.clone(), &name);
	let body = rename_nested_calls(body, &renames);
	// a sibling calls a nested function too: `def sibling(){ inner() }` beside `def inner()` in outer
	let nested_prefix = format!("{name}{NESTED_DEF_SEPARATOR}");
	for function in ctx.user_functions.values_mut().filter(|function| function.name.starts_with(&nested_prefix)) {
		*function.body = rename_nested_calls(function.body.as_ref().clone(), &renames);
	}
	if let Some(parent) = enclosing {
		ctx.enclosing_functions.insert(name.clone(), parent.to_string());
	}
	ctx.user_functions.insert(name.clone(), user_function(&name, params, body));
}

/// Nested function definitions in `body` become UserFunctionDefs named `parent·short`; their statements stay (emit skips
/// them) and call sites are renamed via the returned map.
pub(in crate::analyzer) fn lift_nested_defs_from_body(
	ctx: &mut Context,
	body: Node,
	parent: &str,
) -> (Node, HashMap<String, String>) {
	let mut renames = HashMap::new();
	let body = lift_nested_defs_walk(ctx, body, parent, &mut renames);
	(body, renames)
}

pub(in crate::analyzer) fn lift_nested_defs_walk(
	ctx: &mut Context,
	node: Node,
	parent: &str,
	renames: &mut HashMap<String, String>,
) -> Node {
	match node {
		meta @ Node::Meta { .. } => meta.under_meta(|node| lift_nested_defs_walk(ctx, node, parent, renames)),
		Node::Key(ref left, op @ (Op::Assign | Op::Define), body)
			if let Some((short, items, bracket)) = signature(left) =>
		{
			let mangled = qualify_nested_name(Some(parent), short);
			renames.insert(short.to_string(), mangled.clone());
			register_user_function(ctx, short, extract_params(items, bracket), &body, Some(parent));
			// LHS must use the mangled name so emit's defined_function_name finds it in user_functions
			let mut signature = left.drop_meta().clone();
			if let Node::List(items, _, _) = &mut signature {
				items[0] = Node::Symbol(mangled);
			}
			Node::Key(Box::new(signature), op, body)
		}
		Node::Key(left, op, right) => Node::Key(
			Box::new(lift_nested_defs_walk(ctx, *left, parent, renames)),
			op,
			Box::new(lift_nested_defs_walk(ctx, *right, parent, renames)),
		),
		// quoted data (the program a run-time block carries) is not code of this body
		quoted if crate::run_time_blocks::is_data(&quoted) => quoted,
		Node::List(items, bracket, separator) if let Some(function) = keyword_definition(&items) => {
			renames.insert(function.name.clone(), qualify_nested_name(Some(parent), &function.name));
			register_user_function(ctx, &function.name, function.params, &function.body, Some(parent));
			Node::List(items, bracket, separator)
		}
		Node::List(items, bracket, separator) => Node::List(
			items.into_iter().map(|item| lift_nested_defs_walk(ctx, item, parent, renames)).collect(),
			bracket,
			separator,
		),
		other => other,
	}
}

/// `inner()` / `inner()+1` / `apply(inner)` inside the enclosing body: the name becomes `outer·inner`
pub(in crate::analyzer) fn rename_nested_calls(node: Node, renames: &HashMap<String, String>) -> Node {
	if renames.is_empty() {
		return node;
	}
	match node {
		Node::Symbol(name) if renames.contains_key(&name) => Node::Symbol(renames[&name].clone()),
		Node::List(items, bracket, separator) => {
			let mut items: Vec<Node> = items.into_iter().map(|item| rename_nested_calls(item, renames)).collect();
			if bracket == Bracket::Round {
				if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
					if let Some(mangled) = renames.get(name) {
						items[0] = Node::Symbol(mangled.clone());
					}
				}
			}
			Node::List(items, bracket, separator)
		}
		other => children_rewritten(other, |child| rename_nested_calls(child, renames)),
	}
}

/// The parameters after the function name in a signature list. Only a spaced signature `f int x` pairs a type word with
/// the next name; in the call form `f(T, y)` every item is a parameter (the parser made `f(int x)` the item `x:int`)
pub(in crate::analyzer) fn extract_params(signature: &[Node], bracket: &Bracket) -> Vec<Param> {
	// `f(xs: list of int, y)`: the comma groups `xs:list of int` as one spaced item
	let flat: Vec<&Node> = signature
		.iter()
		.skip(1)
		.flat_map(|item| match item.drop_meta() {
			Node::List(parts, _, Separator::Space | Separator::None)
				if matches!(parts.first().map(Node::drop_meta), Some(Node::Key(_, Op::Colon, _))) =>
			{
				parts.iter().collect()
			}
			_ => vec![item],
		})
		.collect();
	let mut items = flat.into_iter().peekable();
	let mut params = vec![];
	while let Some(item) = items.next() {
		let type_first_name = match (item.drop_meta(), items.peek().map(|next| next.drop_meta())) {
			(Node::Symbol(type_name), Some(Node::Symbol(name)))
				if *bracket != Bracket::Round && type_word_kind(type_name).is_some() =>
			{
				Some(name)
			}
			_ => None,
		};
		match type_first_name {
			Some(name) => {
				params.push(Param { annotation: Some(item.clone()), ..Param::untyped(name) });
				items.next();
			}
			None => params.extend(extract_param(item).map(|param| with_list_annotation(param, &mut items))),
		}
	}
	params
}

/// `xs: T list` and `xs: list of T` (the parser leaves `list`, or `of T`, as the next items) annotate xs as a `list of T`:
/// T a type word, a declared type or a trait (`sort(xs: Comparable list)`)
pub(in crate::analyzer) fn with_list_annotation<'a>(
	param: Param,
	items: &mut std::iter::Peekable<impl Iterator<Item = &'a Node>>,
) -> Param {
	let Some(annotation) = param.annotation.as_ref().map(Node::name) else {
		return param;
	};
	let next_word = |items: &mut std::iter::Peekable<_>| match items.peek().map(|next: &&Node| next.drop_meta()) {
		Some(Node::Symbol(word)) => Some(word.clone()),
		_ => None,
	};
	let element = match next_word(items).as_deref() {
		Some(LIST_WORD) => annotation,
		Some(OF_WORD) if crate::warp_parser::list_type_head(&annotation) == LIST_WORD => {
			items.next();
			match next_word(items) {
				Some(element) => element,
				None => return param,
			}
		}
		_ => return param,
	};
	items.next();
	Param { annotation: Some(Node::Symbol(format!("{LIST_OF_PREFIX}{element}"))), ..param }
}

/// Extract parameter name, annotated kind and optional default value from a parameter node
pub(in crate::analyzer) fn extract_param(item: &Node) -> Option<Param> {
	match item.drop_meta() {
		Node::Symbol(s) => Some(Param::untyped(s)),
		Node::Key(name, Op::Colon, type_name) => {
			Some(Param { annotation: Some(named_list_annotation(type_name)), ..Param::untyped(name.symbol_name()?) })
		}
		// `f: (Int) -> Int` parses as `(f: (Int)) -> Int`: a function
		Node::Key(typed, Op::Arrow, _) if matches!(typed.drop_meta(), Node::Key(_, Op::Colon, _)) => {
			let param = extract_param(typed)?;
			Some(Param { annotation: Some(Node::Symbol(Kind::Function.to_string())), ..param })
		}
		// `b=2` and the typed `b:int=2`
		Node::Key(declared, Op::Assign, default) => {
			let param = extract_param(declared)?;
			Some(Param { default: Some(default.as_ref().clone()), ..param })
		}
		_ => None,
	}
}

/// Whether a body reads the implicit parameter `it`
pub(crate) fn uses_it(node: &Node) -> bool {
	node.mentions(crate::words::IMPLICIT_PARAMETER)
}

/// A body of `name := body` that reads `it` or `$0`: a function of that parameter
pub(crate) fn takes_implicit_parameter(body: &Node) -> bool {
	uses_it(body) || uses_dollar_param(body)
}

/// Check if a node uses $n parameter references (e.g., $0, $1)
pub(in crate::analyzer) fn uses_dollar_param(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Symbol(s) if s.starts_with('$') && s[1..].parse::<u32>().is_ok() => true,
		Node::Key(left, _, right) => uses_dollar_param(left) || uses_dollar_param(right),
		Node::List(items, _, _) => items.iter().any(uses_dollar_param),
		_ => false,
	}
}

/// Extract function from def/fun/fn syntax
pub(crate) fn extract_def_function(items: &[Node]) -> Option<UserFunctionDef> {
	if items.is_empty() {
		return None;
	}
	let first = items[0].drop_meta();
	// `def add(a:int) -> int { a }`, before the declaration passes take its result type off
	if let Some((untyped, _)) = crate::declarations::typed_result(first) {
		return extract_def_function(&[untyped]);
	}

	// def (name params...): body, or := body
	if let Node::Key(sig, Op::Colon | Op::Define, body) = first {
		if let Some((name, sig_items, bracket)) = signature(sig) {
			return Some(user_function(name, extract_params(sig_items, bracket), body.as_ref().clone()));
		}
	}
	// def ((name params...) {body})
	let Node::List(inner_items, _, _) = first else {
		return None;
	};
	let (name, sig_items, _) = signature(inner_items.first()?)?;
	let body = inner_items.get(1)?;
	let params: Vec<Param> = sig_items
		.iter()
		.skip(1)
		.flat_map(|item| match item.drop_meta() {
			Node::List(param_items, _, _) => param_items.iter().filter_map(extract_param).collect::<Vec<_>>(),
			_ => extract_param(item).into_iter().collect(),
		})
		.collect();
	Some(user_function(name, params, body.clone()))
}
