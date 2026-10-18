//! A module's definitions the program needs, those its calls reach, put in front of the program and renamed apart from its variables
use super::*;

/// A module word a program variable shadows is renamed apart: `words` of lib/text is `lib·words` in its definitions
pub(crate) const SHADOWED_WORD_PREFIX: &str = "lib·";
/// the warning kind of a program function replacing a module's, which "got it" silences
pub(crate) const REPLACED_MODULE_FUNCTION_TOPIC: &str = "replaced-module-function";

/// The names the program declares, but not its methods: `take()` of a class Box, `take(b:Box)` add to list's take
/// (notes/dispatch.md "Methods"), a plain `take(xs, n)` is the program's own take
pub(crate) fn own_functions(program: &Node) -> HashSet<String> {
	let classes = class_names(program);
	let mut own = classes.clone();
	collect_own_functions(program, &classes, &mut own);
	own
}

pub(crate) fn class_names(program: &Node) -> HashSet<String> {
	let mut classes = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Type { name, .. } = node {
			classes.extend(leftmost_symbol(name));
		}
	});
	classes
}

/// The program's variables and its own functions, which a module's words of the same name are renamed apart from
/// (shadowed_apart): a program's `added(x, y) := 7` leaves sound's mix its own added (card use-sound)
#[derive(Default)]
pub(crate) struct ProgramWords {
	variables: HashSet<String>,
	/// each function's fewest and most arguments: it shadows a module's definitions taking any of those only,
	/// `clamp(x)` leaves math's clamp(x, low, high) to the program too
	functions: HashMap<String, Vec<(usize, usize)>>,
	/// each function's first definition, where a warning about it points
	definitions: HashMap<String, Node>,
}

impl ProgramWords {
	pub(super) fn contains(&self, name: &str) -> bool {
		self.variables.contains(name) || self.functions.contains_key(name)
	}

	/// a module definition the program still sees under its own name: no number of arguments it takes is one a
	/// same-named program function takes
	fn leaves_to_program(&self, name: &str, (fewest, most): (usize, usize)) -> bool {
		let overlaps =
			|&(program_fewest, program_most): &(usize, usize)| program_fewest <= most && fewest <= program_most;
		!self.variables.contains(name) && self.functions.get(name).is_some_and(|ranges| !ranges.iter().any(overlaps))
	}

	/// The program's definition replacing a module definition of `name` taking these arguments, for the program's calls
	fn replacing(&self, name: &str, arities: (usize, usize)) -> Option<&Node> {
		let replaces = self.functions.contains_key(name) && !self.leaves_to_program(name, arities);
		self.definitions.get(name).filter(|_| replaces && !self.variables.contains(name))
	}
}

/// `use sound; def play(x) := …`: a program function taking a module function's arguments replaces it for the
/// program's calls, which is never silent (card use-sound-def); the module's own code keeps its own (shadowed_apart)
pub(crate) fn warn_replaced(
	definitions: &[Node],
	words: &ProgramWords,
	module: &str,
	level: crate::errors::diagnostic::Level,
) {
	let mut warned = HashSet::new();
	for definition in definitions {
		let Some((head, _, _)) = crate::type_tests::function_definition(definition) else { continue };
		let Some(name) = leftmost_symbol(&head) else { continue };
		let Some(replacing) = words.replacing(&name, head_arities(&head)) else { continue };
		if warned.insert(name.clone()) {
			let replaced = signature(&name, &head);
			let message = format!(
				"{name} replaces {module}'s {replaced} in this program ({module}'s own code keeps its {name}); rename yours to keep both"
			);
			crate::errors::diagnostic::say(
				crate::errors::diagnostic::Diagnostic::at(replacing, message)
					.level(level)
					.about(REPLACED_MODULE_FUNCTION_TOPIC, &name),
			);
		}
	}
}

/// Methods (`play(s: Song)` of a class Song) stay out: they add to the module's word
pub(crate) fn program_words(program: &Node) -> ProgramWords {
	let classes = class_names(program);
	let is_method = |head: &Node| {
		let mut typed_by_class = false;
		head.visit(&mut |node| {
			if let Node::Key(_, Op::Colon, class) = node {
				typed_by_class |= class.symbol_name().is_some_and(|class| classes.contains(class));
			}
		});
		typed_by_class
	};
	let mut functions: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
	let mut definitions = HashMap::new();
	for statement in statements_of(program) {
		let Some((head, _, _)) = crate::type_tests::function_definition(statement) else { continue };
		if let (false, Some(name)) = (is_method(&head), leftmost_symbol(&head)) {
			functions.entry(name.clone()).or_default().push(head_arities(&head));
			definitions.entry(name).or_insert_with(|| statement.clone());
		}
	}
	ProgramWords { variables: program_variables(program), functions, definitions }
}

/// The arguments a definition head takes, fewest and most: `mix(a, b, c = [])` takes 2 or 3
pub(super) fn head_arities(head: &Node) -> (usize, usize) {
	let parameters = parameters_of(head);
	let defaulted = parameters.iter().filter(|parameter| matches!(parameter.drop_meta(), Node::Key(_, Op::Assign, _)));
	(parameters.len() - defaulted.count(), parameters.len())
}

fn parameters_of(head: &Node) -> &[Node] {
	match head.drop_meta() {
		Node::List(items, _, _) => &items[1..],
		_ => &[],
	}
}

/// `play(tone)`, as a definition head reads in a message
fn signature(name: &str, head: &Node) -> String {
	let parameters: Vec<String> = parameters_of(head).iter().map(parameter_text).collect();
	format!("{name}({})", parameters.join(", "))
}

/// `tone`, `bus: Bus`: a parameter as written, without the `any` type lowering gave an untyped one
fn parameter_text(parameter: &Node) -> String {
	match parameter.drop_meta() {
		Node::Key(name, Op::Colon, kind) if kind.is_symbol(crate::words::ANY_TYPE) => {
			name.serialize().trim().to_string()
		}
		other => other.serialize().trim().to_string(),
	}
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
	collect_called_words(program, is_module_name, &mut names);
	names
}

/// In a function, its parameters and variables are no calls of a module's words (a local `items` beside map.warp's
/// items, lib/sound.warp)
fn collect_called_words(node: &Node, is_module_name: &dyn Fn(&Node) -> bool, names: &mut HashSet<String>) {
	if let Some((head, body, _)) = crate::type_tests::function_definition(node) {
		let mut called_inside = HashSet::new();
		collect_called_words(&body, is_module_name, &mut called_inside);
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
			collect_called_words(receiver, is_module_name, names);
			// `xs.product` is `xs.product()` (user 2026-10-14, card type-module); method_call reads a field of that name
			// first (`p.center` of an object)
			match member.drop_meta() {
				Node::List(call, _, _) => {
					if !is_module_name(receiver) {
						names.extend(called(call));
					}
					call.iter().skip(1).for_each(|argument| collect_called_words(argument, is_module_name, names));
				}
				Node::Symbol(word)
					if !is_module_name(receiver) && crate::library_words::type_module_defining(word).is_some() =>
				{
					names.insert(word.clone());
				}
				_ => {}
			}
		}
		Node::Key(left, _, right) => {
			collect_called_words(left, is_module_name, names);
			collect_called_words(right, is_module_name, names);
		}
		Node::List(items, bracket, _) => {
			if matches!(bracket, Bracket::Round | Bracket::None) && items.len() > 1 {
				names.extend(called(items));
			}
			items.iter().for_each(|item| collect_called_words(item, is_module_name, names));
		}
		Node::Type { body, .. } => collect_called_words(body, is_module_name, names),
		_ => {}
	}
}

pub(crate) fn with_needed_definitions(program: Node, definitions: Vec<Node>) -> Node {
	let needed = needed_definitions(&program, &definitions);
	crate::lowering::nodes::with_statements_first(program, needed)
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
			.filter(|definition| {
				name_declared_by(definition).is_some_and(|name| is_needed(&name, &mentioned))
					|| is_implicit_operation_of(definition, &mentioned)
			})
			.cloned()
			.collect();
		if now_needed.len() == needed.len() {
			return needed;
		}
		mentioned.extend(called_names(&now_needed));
		needed = now_needed;
	}
}

/// `call(b: Bus, signal)` of a type the statements mention: an operation the program uses without naming it
/// (`room(signal)`, traits IMPLICIT_OPERATIONS)
fn is_implicit_operation_of(definition: &Node, mentioned: &HashSet<String>) -> bool {
	let Some(head) = crate::lowering::function_head(definition) else {
		return false;
	};
	let Node::List(items, _, _) = head.head.drop_meta() else {
		return false;
	};
	let [name, first, ..] = items.as_slice() else {
		return false;
	};
	crate::lowering::traits::IMPLICIT_OPERATIONS.contains(&name.name().as_str())
		&& matches!(first.drop_meta(), Node::Key(_, Op::Colon, type_node) if mentioned.contains(&type_node.drop_meta().name()))
}

/// A mentioned name, or the companion f_as of a mentioned f: overloads lowering calls it where f gives an expected type;
/// or a word operator called by its word, `matches(s, p)` of `infix operator matches` (word_operators lower_call_forms)
pub(crate) fn is_needed(name: &str, mentioned: &HashSet<String>) -> bool {
	mentioned.contains(name)
		|| crate::lowering::overloads::companion_of(name).is_some_and(|function| mentioned.contains(function))
		|| name.strip_prefix(crate::operators::OPERATOR_FUNCTION_PREFIX).is_some_and(|word| mentioned.contains(word))
}

/// The forms a later pass or the emitter turns into calls of prelude words the program never names, one row each: the
/// prelude word a node needs, if any (card lib-from-runtime). A move of a runtime function into lib/prelude.warp adds
/// its row; a form missing here still compiles, the second time (pipeline::loading_compiler_words)
const IMPLIED_WORDS: [fn(&Node) -> Option<&'static str>; 6] = [
	crate::lowering::broadcasting::implied_word,
	crate::gpu_maps::implied_word,
	crate::wasm_emitter::numbers::radix::implied_word,
	crate::lowering::filled_lists::implied_word,
	crate::wasm_emitter::numbers::uncertain::implied_word,
	crate::wasm_emitter::numbers::exact::implied_word,
];

/// The prelude words IMPLIED_WORDS finds in the statements
fn implied_words(statements: &[Node]) -> Vec<String> {
	let mut words = Vec::new();
	for statement in statements {
		statement.visit(&mut |node| words.extend(IMPLIED_WORDS.iter().filter_map(|implied| implied(node))));
	}
	words.into_iter().map(str::to_string).collect()
}

/// The names the statements mention or call, as methods too (`"hé".to_utf8()`), in interpolations too, with the library
/// words they spell
pub(crate) fn called_names(statements: &[Node]) -> HashSet<String> {
	let implied = [
		implied_words(statements),
		crate::lowering::library_words::implied_count_words(statements),
		crate::compiler::compile_context::read(|context| context.compiler_words.clone()),
	]
	.concat();
	with_library_words(mentioned_names(statements).into_iter().chain(method_names(statements)).chain(implied).collect())
}

/// The names with the library words they spell (`isdigit` → is_digit, `round` → round_to) and the prelude names of
/// those (`prelude·is_digit`): library_words, which writes them so, runs after the modules are resolved
pub(crate) fn with_library_words(names: HashSet<String>) -> HashSet<String> {
	// `prelude·random_between` of a pass that wrote the call qualified already (host::lower_random_forms)
	let prelude_qualifier = format!("{PRELUDE_MODULE}{QUALIFIER}");
	let unqualified: Vec<String> =
		names.iter().filter_map(|name| name.strip_prefix(&prelude_qualifier)).map(String::from).collect();
	let names: HashSet<String> = names.into_iter().chain(unqualified).collect();
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
/// A module's `global` is a setting, which the program's `tempo = 90` sets (lib/sound.warp's tempo, note_seconds).
/// A program function shadows only its own arities: the module's other definitions of the word stay the program's too
pub(crate) fn shadowed_apart(definitions: Vec<Node>, words: &ProgramWords) -> Vec<Node> {
	let declared: Vec<String> = definitions.iter().filter_map(name_declared_by).collect();
	let settings: Vec<String> = definitions
		.iter()
		.filter(|definition| is_global_declaration(definition))
		.filter_map(name_declared_by)
		.collect();
	let shadowed: Vec<String> =
		declared.iter().filter(|name| words.contains(name) && !settings.contains(name)).cloned().collect();
	let left_to_program: Vec<Option<String>> = definitions
		.iter()
		.map(|definition| {
			let (head, _, _) = crate::type_tests::function_definition(definition)?;
			let name = leftmost_symbol(&head)?;
			words.leaves_to_program(&name, head_arities(&head)).then_some(name)
		})
		.collect();
	renamed_apart(definitions, &shadowed)
		.into_iter()
		.map(|definition| {
			let locals: Vec<String> = assigned_names(&definition)
				.into_iter()
				.filter(|name| words.contains(name) && !declared.contains(name))
				.collect();
			renamed_apart(vec![definition], &locals).remove(0)
		})
		.zip(left_to_program)
		.flat_map(|(definition, left)| {
			let under_own_name = left.and_then(|name| named_again(&definition, &name));
			std::iter::once(definition).chain(under_own_name)
		})
		.collect()
}

/// `lib·mix(a, b) := …` as `mix(a, b) := …` again, its body still calling the module's own words
fn named_again(definition: &Node, name: &str) -> Option<Node> {
	let (head, body, rebuild) = crate::type_tests::function_definition(definition)?;
	let Node::List(mut items, bracket, separator) = head.drop_meta().clone() else { return None };
	items[0] = Node::Symbol(name.to_string());
	Some(rebuild(Node::List(items, bracket, separator), body))
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

/// `from` as a word renamed `to`; a field `x.from` and a map's key `{from: …}` keep their name (card map-key)
pub(crate) fn renamed_word(node: Node, from: &str, to: &str) -> Node {
	match node {
		Node::Symbol(name) if name == from => Node::Symbol(to.to_string()),
		Node::Key(receiver, Op::Dot, field) => Node::Key(Box::new(renamed_word(*receiver, from, to)), Op::Dot, field),
		Node::List(items, Bracket::Curly, separator) => Node::List(
			items.into_iter().map(|item| renamed_entry_value(item, from, to)).collect(),
			Bracket::Curly,
			separator,
		),
		other => other.map_children(|child| renamed_word(child, from, to)),
	}
}

/// A map entry `from: value` with only its value renamed; any other item as a whole
fn renamed_entry_value(item: Node, from: &str, to: &str) -> Node {
	match item {
		Node::Meta { node, data } => Node::Meta { node: Box::new(renamed_entry_value(*node, from, to)), data },
		Node::Key(name, Op::Colon, value) if matches!(name.drop_meta(), Node::Symbol(_)) => {
			Node::Key(name, Op::Colon, Box::new(renamed_word(*value, from, to)))
		}
		other => renamed_word(other, from, to),
	}
}
