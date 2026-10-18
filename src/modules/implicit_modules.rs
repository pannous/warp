//! The standard modules a program needs without `use`: a file URL, a page, routes, units, agent, sound, wagi, and the modules named after a type
use super::*;

pub(crate) const FILE_URL_PREFIX: &str = "file://";
/// lib/units.warp: quantities whose unit is known only at run time, `quantity("5 km")` (notes/units_runtime.md)
pub(crate) const UNITS_MODULE: &str = "units";
pub(crate) const QUANTITY: &str = "quantity";
/// A served route's path parameters (`post "/todos/:id:int/toggle"`, lowering/web/serve.rs) are read by the router's words
pub(crate) const ROUTE_PARTS: [&str; 3] = ["route_segment", "route_parameter", "route_matches"];
/// lib/router.warp: a route's `status 418` and `header "name" "value"`, and the table of routes without a server
const ANSWER_PARTS: [&str; 3] = ["status", "header", crate::serve::ROUTE_ADD];
/// lib/markup.warp: `form post "/todos" { … }`, a form named by its route
const FORM_ROUTE: &str = "form";
/// lib/agent.warp: `agent "prompt"` asks Claude (card g_X_F0)
pub(crate) const AGENT_MODULE: &str = "agent";
/// lib/fast_math.warp: sin cos exp pow atan2 as wasm polynomials, `use fast_math` or for every program `warp fast <file>`
pub(crate) const FAST_MODULE: &str = "fast_math";
/// lib/sound.warp: `play(440Hz, 0.5s)`, `beep` (card basic-sound), loaded when a program calls one of its words
pub(crate) const SOUND_MODULE: &str = "sound";
pub(crate) const SOUND_WORDS: [&str; 21] = [
	"pattern",
	"live_loop",
	"play",
	"melody",
	"tone",
	"beep",
	"play_file",
	"play_sample",
	"stop_sound",
	"sound_queued",
	"wait_sound",
	"render_sound",
	"osc",
	"midi_send",
	"midi_outputs",
	"midi_inputs",
	"midi_receive",
	"midi_on",
	"midi_note",
	"write_midi",
	"read_midi",
];
/// The sound words every program plays without `use sound` (card basic-sound); the module's rarer words need it (card
/// bizarre-global, P171), also when these words bring the module along
pub(crate) const SOUND_SETUP_FREE_WORDS: [&str; 4] = ["play", "beep", "stop_sound", "wait_sound"];
/// Whether a program needs a module
pub(crate) type NeededBy = fn(&Node) -> bool;
/// The standard modules a program needs without `use`: P183 a file URL → file, a page or a hosted page route → markup (lowering/web/page_html.rs, serve.rs answered_page),
/// routes, a server route's path parameters or its status and headers → router, a route's regular expression → regex (lowering/web/routes.rs), a call of quantity → units (run-time units),
/// a call of agent → agent, a call of play, beep, stop_sound or wait_sound → sound, a WAGI build (`warp build --wagi`) → wagi
pub(crate) const IMPLICIT_MODULES: [(&str, NeededBy); 9] = [
	("file", mentions_file_url),
	("markup", |program| {
		let mode = crate::pipeline::compile_mode();
		mode.renders_itself()
			|| mode.for_hosting && calls_undefined(program, crate::page_html::TO_HTML)
			|| calls_undefined(program, FORM_ROUTE)
	}),
	("router", |program| {
		defined(program, crate::routes::PAGE_ROUTES).is_some()
			|| {
				let called = called_names(statements_of(program));
				ROUTE_PARTS.iter().any(|word| called.contains(*word))
			} || first_called_undefined(program, ANSWER_PARTS).is_some()
	}),
	("regex", |program| {
		defined(program, crate::routes::PAGE_ROUTE_INDEX)
			.is_some_and(|index| called_names(&[index]).contains(crate::routes::REGEX_MATCH))
	}),
	(UNITS_MODULE, |program| calls_undefined(program, QUANTITY)),
	(AGENT_MODULE, |program| calls_undefined(program, AGENT_MODULE)),
	(SOUND_MODULE, |program| first_called_undefined(program, SOUND_SETUP_FREE_WORDS).is_some()),
	("wagi", |_| crate::pipeline::compile_mode().for_wagi),
	(FAST_MODULE, |_| crate::pipeline::fast_math()),
];

/// units: `quantity("5 km")` gives a Quantity whose methods read the module's own number words (bound_of), which work
/// only compiled as program code; the classes of the other implicit modules go in front alone (insert_module_classes)
pub(crate) const MODULES_COMPILED_AS_PROGRAM: [&str; 1] = [UNITS_MODULE];

/// The program with the standard modules' definitions it calls, and those they call, in front: a word nobody calls
/// is never compiled (its parameters would have no kinds). For MODULES_COMPILED_AS_PROGRAM, before class_methods: the
/// instances their functions give then take their class's operators and methods. The program defining them now,
/// implicit_std_modules loads such a module no more, nor its own `use units`
pub(crate) fn with_implicit_class_modules(mut program: Node) -> Node {
	for (module, needed) in IMPLICIT_MODULES {
		if MODULES_COMPILED_AS_PROGRAM.contains(&module) && needed(&program) {
			// lib/units.warp's unit_definitions block as the tables its functions read, before they are picked by name
			let parsed = parsed_std_module(module).expect("an implicit module is embedded");
			let definitions = statements(crate::numbers::units::with_run_time_unit_tables(parsed));
			program = with_needed_definitions(without_use_of(program, module), definitions);
		}
	}
	program
}

/// The program without its `use <module>`
pub(crate) fn without_use_of(program: Node, module: &str) -> Node {
	match program {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let kept = statements
				.into_iter()
				.filter(|statement| used_module(statement).is_none_or(|used| used.name != module))
				.collect();
			Node::List(kept, Bracket::None, separator)
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| without_use_of(node, module)),
		single => single,
	}
}

/// The standard modules named after a type with their words, the methods of that type: list, map, text
pub fn type_modules() -> Vec<(&'static str, &'static [String])> {
	std_module_names()
		.filter(|module| crate::type_tests::is_builtin_type_word(module))
		.filter_map(|module| Some((module, std_module_definitions(module)?)))
		.collect()
}

/// `xs.unique()`, `unique(xs)`: the words of the standard modules named after a type (lib/list.warp, text.warp,
/// map.warp) are standard functions of every program, called either way (user decision type-library-words, card
/// dispatch: "just provide the standard functions for list and map by default"), so a program calling one uses that
/// module. A library word stays the builtin, a word the program defines is its own, a word of two such modules needs
/// its `use`, `list.zip(a, b)` names the module (test_std_qualified)
pub(crate) fn with_type_module_uses(program: Node) -> Node {
	let mut defined = own_functions(&program);
	defined.extend(imported_module_exports(&program));
	// `from list import zip` brings only zip: the module's other words stay out (test_from_import)
	let imported =
		|statement: &Node| used_module(statement).or_else(|| imported_words(statement).map(|(used, _, _)| used));
	let used: HashSet<String> = statements_of(&program).iter().filter_map(imported).map(|used| used.name).collect();
	let type_modules = type_modules();
	let is_module_name = |receiver: &Node| {
		receiver.symbol_name().is_some_and(|name| std_module(name).is_some() && !defined.contains(name))
	};
	let only_provider = |method: &String| match type_modules
		.iter()
		.filter(|(_, words)| words.contains(method))
		.collect::<Vec<_>>()
		.as_slice()
	{
		[(module, _)] => Some(*module),
		_ => None,
	};
	let mut modules: Vec<&str> = called_words(&program, &is_module_name)
		.iter()
		.filter(|method| !defined.contains(*method) && !crate::library_words::is_library_word(method))
		.filter_map(only_provider)
		.chain(crate::reflection::value_methods_modules(&program))
		.filter(|module| !used.contains(*module))
		.collect();
	modules.sort();
	modules.dedup();
	with_statements_first(
		program,
		modules.into_iter().map(|module| crate::warp_parser::parse_generated(&format!("use {module}"))).collect(),
	)
}

/// Whether the program plays sound: it calls a sound word or says `use sound`
pub fn uses_sound(program: &Node) -> bool {
	// `use sound` first: it reads the statements, the calls walk the whole program
	says_use(program, SOUND_MODULE) || first_called_undefined(program, SOUND_WORDS).is_some()
}

/// Whether the program says `use <module>`, also first in a line of several (`use json; m = …` before other lines)
pub(crate) fn says_use(program: &Node, module: &str) -> bool {
	statements_of(program)
		.iter()
		.flat_map(Node::sequence_statements)
		.filter_map(used_module)
		.any(|used| used.name == module)
}

/// A rare sound word the program calls: without `use sound` its error names the `use` (card bizarre-global)
pub(crate) fn rare_sound_word(program: &Node) -> Option<&'static str> {
	first_called_undefined(program, SOUND_WORDS.into_iter().filter(|word| !SOUND_SETUP_FREE_WORDS.contains(word)))
}

/// Whether the program calls `word` without defining it
pub(crate) fn calls_undefined(program: &Node, word: &str) -> bool {
	first_called_undefined(program, [word]).is_some()
}

/// Whether the program calls `word` or writes it as a tag with a block: `style{ … }`
pub(crate) fn tags_or_calls(program: &Node, word: &str) -> bool {
	let mut found = false;
	program.visit(&mut |node| {
		found |= matches!(node, Node::Key(tag, Op::Colon, block) if tag.is_symbol(word) && crate::lowering::nodes::is_block(block))
	});
	found || called_names(statements_of(program)).contains(word)
}

/// The first of `words` the program calls without defining it
pub(crate) fn first_called_undefined<'a>(program: &Node, words: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
	let called = called_names(statements_of(program));
	words.into_iter().find(|word| called.contains(*word) && defined(program, word).is_none())
}

pub(crate) fn mentions_file_url(program: &Node) -> bool {
	let mut file_url = false;
	program.visit(&mut |node| {
		if let Node::Text(text) = node {
			file_url |= text.starts_with(FILE_URL_PREFIX)
		}
	});
	file_url
}
