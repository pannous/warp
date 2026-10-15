//! A module's definitions the program needs, those its calls reach, put in front of the program and renamed apart from its variables
use super::*;

/// A module word a program variable shadows is renamed apart: `words` of lib/text is `lib·words` in its definitions
pub(crate) const SHADOWED_WORD_PREFIX: &str = "lib·";

/// The names the program declares, but not its methods: `take()` of a class Box, `take(b:Box)` add to list's take
/// (notes/dispatch.md "Methods"), a plain `take(xs, n)` is the program's own take
pub(crate) fn own_functions(program: &Node) -> HashSet<String> {
	let mut classes = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			classes.extend(leftmost_symbol(name));
		}
	});
	let mut own = classes.clone();
	collect_own_functions(program, &classes, &mut own);
	own
}

/// The program's declarations; in a function body only the nested functions, its variables are its own (a local
/// `words = …` in one function leaves the text module's words to the others, card local-variable-named)
pub(crate) fn collect_own_functions(node: &Node, classes: &HashSet<String>, own: &mut HashSet<String>) {
	if let Some((head, body, _)) = crate::type_tests::function_definition(node) {
		own.extend(leftmost_symbol(&head));
		let mut locals = HashSet::new();
		collect_own_functions(&body, classes, &mut locals);
		let mut assigned = HashSet::new();
		crate::type_tests::collect_outer_assigned_names(&body, &mut assigned);
		own.extend(locals.into_iter().filter(|name| !assigned.contains(name)));
		return;
	}
	let is_method = |target: &Node| match target.drop_meta() {
		Node::List(items, _, _) => {
			matches!(items.get(1).map(Node::drop_meta), Some(Node::Key(_, Op::Colon, class)) if class.symbol_name().is_some_and(|class| classes.contains(class)))
		}
		_ => false,
	};
	match node.drop_meta() {
		Node::Type { .. } => {}
		Node::Key(target, Op::Assign | Op::Define, value) if is_method(target) => {
			collect_own_functions(value, classes, own)
		}
		Node::Key(left, _, right) => {
			own.extend(name_declared_by(node));
			collect_own_functions(left, classes, own);
			collect_own_functions(right, classes, own);
		}
		Node::List(items, _, _) => {
			own.extend(name_declared_by(node));
			items.iter().for_each(|item| collect_own_functions(item, classes, own));
		}
		other => own.extend(name_declared_by(other)),
	}
}

/// The words called with arguments: `unique` of `unique(xs)`, `unique xs`, `xs.unique()` and `take` of `xs.take(2)`; `book.title`
/// may be a field, `list.zip(a, b)` of a module name is a qualified word
pub(crate) fn called_words(program: &Node, is_module_name: &dyn Fn(&Node) -> bool) -> HashSet<String> {
	let mut names = HashSet::new();
	collect_called_words(program, program, is_module_name, &mut names);
	names
}

/// In a function, its parameters and variables are no calls of a module's words (a local `items` beside map.warp's
/// items, lib/sound.warp)
fn collect_called_words(
	node: &Node,
	program: &Node,
	is_module_name: &dyn Fn(&Node) -> bool,
	names: &mut HashSet<String>,
) {
	if let Some((head, body, _)) = crate::type_tests::function_definition(node) {
		let mut called_inside = HashSet::new();
		collect_called_words(&body, program, is_module_name, &mut called_inside);
		let mut locals: HashSet<String> = crate::lowering::memoization::head_parameters(&head).into_iter().collect();
		crate::library_words::collect_assigned_names(&body, &mut locals);
		names.extend(called_inside.into_iter().filter(|name| !locals.contains(name)));
		return;
	}
	let called = |call: &[Node]| call.first().and_then(Node::symbol_name).map(String::from);
	// `function capitalize`, an item of `methods("hi")`: the function itself needs its definition as a call does
	if let Some(name) = crate::closures::referenced_function(node) {
		names.insert(name);
		return;
	}
	match node.drop_meta() {
		Node::Key(receiver, Op::Dot | Op::SafeDot, member) => {
			collect_called_words(receiver, program, is_module_name, names);
			// `xs.product` of a list is `xs.product()` (user 2026-10-14, card type-module); `p.center` of an object a field
			match member.drop_meta() {
				Node::List(call, _, _) => {
					if !is_module_name(receiver) {
						names.extend(called(call));
					}
					call.iter()
						.skip(1)
						.for_each(|argument| collect_called_words(argument, program, is_module_name, names));
				}
				Node::Symbol(word) if crate::members::is_fieldless_value(receiver, program) => {
					names.insert(word.clone());
				}
				_ => {}
			}
		}
		Node::Key(left, _, right) => {
			collect_called_words(left, program, is_module_name, names);
			collect_called_words(right, program, is_module_name, names);
		}
		Node::List(items, bracket, _) => {
			if matches!(bracket, Bracket::Round | Bracket::None) && items.len() > 1 {
				names.extend(called(items));
			}
			items.iter().for_each(|item| collect_called_words(item, program, is_module_name, names));
		}
		Node::Type { body, .. } => collect_called_words(body, program, is_module_name, names),
		_ => {}
	}
}

pub(crate) fn with_statements_first(program: Node, first: Vec<Node>) -> Node {
	if first.is_empty() {
		return program;
	}
	match program {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List(first.into_iter().chain(statements).collect(), Bracket::None, separator)
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_statements_first(node, first)),
		single => {
			Node::List(first.into_iter().chain(std::iter::once(single)).collect(), Bracket::None, Separator::Newline)
		}
	}
}

pub(crate) fn with_needed_definitions(program: Node, definitions: Vec<Node>) -> Node {
	let needed = needed_definitions(&program, &definitions);
	with_statements_in_front(program, needed)
}

/// The definitions the program's calls reach, and those their calls reach in turn
pub(crate) fn needed_definitions(program: &Node, definitions: &[Node]) -> Vec<Node> {
	let mut needed: Vec<Node> = vec![];
	if definitions.is_empty() {
		return needed;
	}
	let mut mentioned = called_names(std::slice::from_ref(program));
	loop {
		let now_needed: Vec<Node> = definitions
			.iter()
			.filter(|definition| name_declared_by(definition).is_some_and(|name| is_needed(&name, &mentioned)))
			.cloned()
			.collect();
		if now_needed.len() == needed.len() {
			return needed;
		}
		mentioned.extend(called_names(&now_needed));
		needed = now_needed;
	}
}

/// The program with the statements before its own
pub(crate) fn with_statements_in_front(program: Node, needed: Vec<Node>) -> Node {
	if needed.is_empty() {
		return program;
	}
	let (statements, separator) = match program {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			(statements, separator)
		}
		single => (vec![single], Separator::Semicolon),
	};
	Node::List([needed, statements].concat(), Bracket::None, separator)
}

/// A mentioned name, or the companion f_as of a mentioned f: overloads lowering calls it where f gives an expected type
pub(crate) fn is_needed(name: &str, mentioned: &HashSet<String>) -> bool {
	mentioned.contains(name)
		|| crate::lowering::overloads::companion_of(name).is_some_and(|function| mentioned.contains(function))
}

/// The names the statements mention or call, as methods too (`"hé".to_utf8()`), in interpolations too, with the library
/// words they spell
pub(crate) fn called_names(statements: &[Node]) -> HashSet<String> {
	let implied =
		[crate::lowering::broadcasting::implied_words(statements), crate::gpu_maps::implied_words(statements)].concat();
	with_library_words(mentioned_names(statements).into_iter().chain(method_names(statements)).chain(implied).collect())
}

/// The names with the library words they spell (`isdigit` → is_digit, `round` → round_to) and the prelude names of
/// those (`prelude·is_digit`): library_words, which writes them so, runs after the modules are resolved
pub(crate) fn with_library_words(names: HashSet<String>) -> HashSet<String> {
	let spelled: Vec<&str> = names.iter().flat_map(|name| crate::library_words::words_spelled_by(name)).collect();
	let qualified: Vec<String> =
		names.iter().map(String::as_str).chain(spelled.iter().copied()).filter_map(prelude_name).collect();
	let spelled: Vec<String> = spelled.into_iter().map(str::to_string).collect();
	names.into_iter().chain(spelled).chain(qualified).collect()
}

/// The variables a program assigns at its top: `words = […]`, also declared global: `global hue = 0`, `global hue`
pub(crate) fn program_variables(program: &Node) -> HashSet<String> {
	statements_of(program)
		.iter()
		.filter_map(|statement| match statement.drop_meta() {
			Node::Key(variable, Op::Assign, _) => variable.drop_meta().symbol_name().map(String::from),
			Node::Key(keyword, Op::Colon, declared) if keyword.is_symbol(crate::words::GLOBAL_WORD) => {
				leftmost_symbol(declared)
			}
			_ => None,
		})
		.collect()
}

/// Module words a program variable shadows, renamed in the module's definitions (`words` → `lib·words`): the module's
/// own code calls its own words (lexical scope), the program's `words` is its variable. Likewise a local of a module
/// function named like a program variable: a program's `global hue` is not draw's `hue` in hsv (card program-global).
/// A module's `global` is a setting, which the program's `tempo = 90` sets (lib/sound.warp's tempo, note_seconds)
pub(crate) fn shadowed_apart(definitions: Vec<Node>, variables: &HashSet<String>) -> Vec<Node> {
	let declared: Vec<String> = definitions.iter().filter_map(name_declared_by).collect();
	let settings: Vec<String> = definitions
		.iter()
		.filter(|definition| is_global_declaration(definition))
		.filter_map(name_declared_by)
		.collect();
	let shadowed: Vec<String> =
		declared.iter().filter(|name| variables.contains(*name) && !settings.contains(name)).cloned().collect();
	renamed_apart(definitions, &shadowed)
		.into_iter()
		.map(|definition| {
			let locals: Vec<String> = assigned_names(&definition)
				.into_iter()
				.filter(|name| variables.contains(name) && !declared.contains(name))
				.collect();
			renamed_apart(vec![definition], &locals).remove(0)
		})
		.collect()
}

/// The names a function definition's body assigns: `hue = …` in `def hsv(h, s, v) {…}`; none for other declarations
pub(crate) fn assigned_names(definition: &Node) -> HashSet<String> {
	if matches!(definition.drop_meta(), Node::Key(_, Op::Assign, _)) || is_global_declaration(definition) {
		return HashSet::new();
	}
	definition.assigned_names(|op| *op == Op::Assign || op.is_compound_assign())
}

/// `global x = …`
pub(crate) fn is_global_declaration(statement: &Node) -> bool {
	matches!(statement.drop_meta(), Node::Key(keyword, Op::Colon, _) if keyword.is_symbol(crate::words::GLOBAL_WORD))
}

/// `names` renamed `lib·name` in the definitions: the definitions still call them, the program does not see them
pub(crate) fn renamed_apart(definitions: Vec<Node>, names: &[String]) -> Vec<Node> {
	definitions
		.into_iter()
		.map(|definition| {
			names
				.iter()
				.fold(definition, |node, name| renamed_word(node, name, &format!("{SHADOWED_WORD_PREFIX}{name}")))
		})
		.collect()
}

/// `from` as a word renamed `to`; a field `x.from` keeps its name
pub(crate) fn renamed_word(node: Node, from: &str, to: &str) -> Node {
	match node {
		Node::Symbol(name) if name == from => Node::Symbol(to.to_string()),
		Node::Key(receiver, Op::Dot, field) => Node::Key(Box::new(renamed_word(*receiver, from, to)), Op::Dot, field),
		other => other.map_children(|child| renamed_word(child, from, to)),
	}
}
