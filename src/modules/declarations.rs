//! The declarations of a module or program: the names they bind, their headers, methods and template holes
use super::*;

/// `stored x = v` too: a used module's persisted signal is the program's (card web-stores)
pub(crate) const DECLARATION_KEYWORDS: [&str; 4] =
	["let", "var", crate::words::GLOBAL_WORD, crate::stored_values::STORED_WORD];

const ALIAS_KEYWORD: &str = "alias";

pub(crate) fn is_declaration_keyword(keyword: &str) -> bool {
	DECLARATION_KEYWORDS.contains(&keyword)
		|| is_import_keyword(keyword)
		|| crate::words::CONSTANT_KEYWORDS.contains(&keyword)
}

pub(crate) fn is_class(statement: &Node) -> bool {
	matches!(statement.drop_meta(), Node::Type { .. })
}

/// The program's main-level definition of `name`
pub(crate) fn defined(program: &Node, name: &str) -> Option<Node> {
	statements_of(program).iter().find(|statement| name_declared_by(statement).as_deref() == Some(name)).cloned()
}

/// The statements of a module, borrowed (statements takes them)
pub(crate) fn statements_of(module: &Node) -> &[Node] {
	match module.drop_meta() {
		Node::Empty => &[],
		plain => plain.sequence_statements(),
	}
}

pub(crate) fn statements(module: Node) -> Vec<Node> {
	statements_of(&module).to_vec()
}

/// A used file's statements with its enums lowered (plain_enums.rs, sum_variants.rs): the sum type and the classes of
/// its variants each a statement, a class insert_module_classes puts in front of the program (card node-enum)
pub(crate) fn module_statements(module: Node) -> Vec<Node> {
	let module = crate::lowering::sum_variants::lower(crate::lowering::plain_enums::lower(module));
	let enum_parts = |statement: &Node| match statement.drop_meta() {
		Node::List(parts, Bracket::None, Separator::Semicolon)
			if parts.first().is_some_and(crate::lowering::sum_variants::is_sum_type) =>
		{
			parts.clone()
		}
		_ => vec![statement.clone()],
	};
	statements_of(&module).iter().flat_map(enum_parts).collect()
}

pub(crate) fn is_declaration(statement: &Node) -> bool {
	match statement.drop_meta() {
		Node::Type { .. } => true,
		Node::Key(_, Op::Assign | Op::Define, _) => true,
		Node::Key(scope, Op::Colon, _) => scope.symbol_name().is_some_and(is_declaration_keyword),
		Node::List(items, _, _) if items.len() >= 2 => items[0]
			.symbol_name()
			.is_some_and(|keyword| is_function_keyword(keyword) || is_declaration_keyword(keyword)),
		_ => false,
	}
}

/// The name a declaration defines: `f(x) := …`, `x = …`, `x:int = 1`, `fun f(x) {…}`, `global x = …`, `class T {…}`
pub(crate) fn name_declared_by(statement: &Node) -> Option<String> {
	if !is_declaration(statement) {
		return None; // a list `[a, b]` declares nothing
	}
	match statement.drop_meta() {
		Node::Type { name, .. } => leftmost_symbol(name),
		Node::Key(target, Op::Assign | Op::Define, _) => leftmost_symbol(target),
		// `global x = 0`, as the parser reads it: `global: (x = 0)`
		Node::Key(keyword, Op::Colon, declared) if keyword.symbol_name().is_some_and(is_declaration_keyword) => {
			name_declared_by(declared)
		}
		Node::List(items, _, _) if items.len() >= 2 => leftmost_symbol(&items[1]),
		_ => None,
	}
}

/// What tells declarations of one name apart: a function's parameter types, `half(int)` and `half(float)` are
/// overloads; anything else by its name alone
pub(crate) fn declared_signature(statement: &Node) -> Option<String> {
	let name = name_declared_by(statement)?;
	let Some(head) = crate::lowering::nodes::function_head(statement) else {
		return Some(name);
	};
	let types: Vec<String> = head.parameters().into_iter().map(parameter_type).collect();
	Some(format!("{name}({})", types.join(", ")))
}

/// `x: int` → int, `x: int = 1` → int, an untyped `x` → any
fn parameter_type(parameter: &Node) -> String {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, kind) => kind.serialize(),
		Node::Key(typed, Op::Assign, _) => parameter_type(typed),
		_ => "any".to_string(),
	}
}

/// `alias pow = power`: another name of a module's word (wiki/alias.md), as (alias, word); it declares nothing
pub(crate) fn word_alias(statement: &Node) -> Option<(String, String)> {
	let Node::List(items, _, _) = statement.drop_meta() else {
		return None;
	};
	let [keyword, alias] = items.as_slice() else {
		return None;
	};
	let Node::Key(alias, Op::Assign, word) = alias.drop_meta() else {
		return None;
	};
	(keyword.symbol_name() == Some(ALIAS_KEYWORD))
		.then(|| Some((alias.symbol_name()?.to_string(), word.symbol_name()?.to_string())))
		.flatten()
}

pub(crate) fn leftmost_symbol(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::List(items, _, _) => leftmost_symbol(items.first()?),
		Node::Key(left, Op::Colon | Op::Assign | Op::Define, _) => leftmost_symbol(left),
		_ => None,
	}
}

/// Words that bind the names after them: `fun f(x)`, `for i in`, `import f from`, `global x`
pub(crate) fn binds_names(keyword: &str) -> bool {
	is_function_keyword(keyword) || is_declaration_keyword(keyword) || keyword == crate::words::FOR_WORD
}

/// The signature part of a definition: `f(x)` of `f(x): body` and of `fun f(x) {body}`
pub(crate) fn header(definition: &Node) -> &Node {
	match definition.drop_meta() {
		Node::Key(header, Op::Colon | Op::Assign | Op::Define, _) => header,
		Node::List(items, _, _) if items.first().is_some_and(|first| matches!(first.drop_meta(), Node::List(..))) => {
			&items[0]
		}
		_ => definition,
	}
}

/// Every name the program binds anywhere: variables, functions, parameters, loop variables, imports.
/// Generous on purpose: a bound name never comes from a sibling
pub(crate) fn program_bound_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(target, op, value)
			if matches!(op, Op::Assign | Op::Define | Op::Arrow) || op.is_compound_assign() =>
		{
			names.extend(mentioned_names(std::slice::from_ref(target.as_ref())));
			program_bound_names(value, names);
		}
		Node::Key(left, _, right) => {
			program_bound_names(left, names);
			program_bound_names(right, names);
		}
		Node::List(items, _, _) => {
			if let [keyword, bound, ..] = items.as_slice() {
				if keyword.symbol_name().is_some_and(binds_names) {
					names.extend(mentioned_names(std::slice::from_ref(header(bound))));
				}
			}
			items.iter().for_each(|item| program_bound_names(item, names));
		}
		Node::Type { name, body } => {
			program_bound_names(name, names);
			names.extend(leftmost_symbol(name));
			program_bound_names(body, names);
		}
		_ => {}
	}
}

pub(crate) fn same_definitions(first: &[Node], second: &[Node]) -> bool {
	first.len() == second.len() && first.iter().zip(second).all(|(a, b)| a.serialize() == b.serialize())
}

/// The methods called in statements: `f` of `x.f(…)` and of `x.f`
pub(crate) fn method_names(statements: &[Node]) -> Vec<String> {
	let mut names = Vec::new();
	for statement in statements {
		statement.visit(&mut |node| {
			if let Node::Key(_, Op::Dot | Op::SafeDot, member) = node {
				match member.drop_meta() {
					Node::Symbol(name) => names.push(name.clone()),
					Node::List(call, _, _) => names.extend(call.first().map(|name| name.drop_meta().name())),
					_ => {}
				}
			}
		});
	}
	let holes = template_holes(statements);
	if !holes.is_empty() {
		names.extend(method_names(&holes));
	}
	names
}

/// The expressions in the interpolations of the statements (`"\(x.round(2))"`), which are lowered after the modules
/// are resolved
pub(crate) fn template_holes(statements: &[Node]) -> Vec<Node> {
	fn collect(node: &Node, holes: &mut Vec<Node>) {
		match node {
			Node::Meta { node: inner, .. } => {
				holes.extend(crate::text::interpolation::holes(node));
				collect(inner, holes);
			}
			Node::Key(left, _, right) => {
				collect(left, holes);
				collect(right, holes);
			}
			Node::List(items, _, _) => items.iter().for_each(|item| collect(item, holes)),
			Node::Type { body, .. } => collect(body, holes),
			_ => {}
		}
	}
	let mut holes = vec![];
	statements.iter().for_each(|statement| collect(statement, &mut holes));
	holes
}

/// Every word the statements mention
pub(crate) fn mentioned_names(statements: &[Node]) -> Vec<String> {
	// a function's parameters and variables are its own: `f(x, melody) := x + melody` mentions no module word melody,
	// prelude's `dict(pairs) := { out = {}; … }` no sound word out
	fn collect(node: &Node, names: &mut Vec<String>) {
		// `f(x): int := …` too: function_head reads a typed head
		let function = crate::lowering::nodes::function_head(node);
		let mut locals: HashSet<String> = function
			.as_ref()
			.map(|function| function.parameters())
			.unwrap_or_default()
			.into_iter()
			.filter_map(crate::lowering::nodes::parameter_name)
			.collect();
		if let Some(function) = &function {
			crate::library_words::collect_assigned_names(function.body, &mut locals);
		}
		if locals.is_empty() {
			return collect_parts(node, names);
		}
		let mut mentioned = vec![];
		collect_parts(node, &mut mentioned);
		names.extend(mentioned.into_iter().filter(|name| !locals.contains(name)));
	}
	fn collect_parts(node: &Node, names: &mut Vec<String>) {
		match node {
			Node::Symbol(name) => names.push(name.clone()),
			Node::Key(receiver, Op::Dot | Op::SafeDot, member) => {
				collect(receiver, names); // `xs.add(x)` names a method of xs, no definition of the folder
				if let Node::List(call, _, _) = member.drop_meta() {
					call.iter().skip(1).for_each(|argument| collect(argument, names));
				}
			}
			Node::Key(left, _, right) => {
				collect(left, names);
				collect(right, names);
			}
			// `play tone:440Hz for:0.5s`: a named argument's name mentions no word (preposition_calls.rs)
			Node::List(items, bracket, separator)
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))
					&& (*bracket == Bracket::Round || *separator == Separator::Space) =>
			{
				collect(&items[0], names);
				items[1..].iter().for_each(|item| match crate::lowering::named_arguments::named_argument(item) {
					Some((_, value)) => collect(value, names),
					None => collect(item, names),
				})
			}
			Node::List(items, _, _) => items.iter().for_each(|item| collect(item, names)),
			// an interpolation is lowered after the modules are resolved: `"\(zip(a, b))"` calls zip
			Node::Meta { node: inner, .. } => crate::text::interpolation::holes(node)
				.iter()
				.chain([inner.as_ref()])
				.for_each(|part| collect(part, names)),
			Node::Type { name, body } => {
				collect(name, names);
				collect(body, names);
			}
			_ => {}
		}
	}
	let mut names = Vec::new();
	statements.iter().for_each(|statement| collect(statement, &mut names));
	names
}
