//! The standard modules a program needs without `use`: a file URL, a page, routes, units, agent, sound, wagi, and the modules named after a type
use super::*;

pub(crate) const FILE_URL_PREFIX: &str = "file://";
/// lib/units.warp: quantities whose unit is known only at run time, `quantity("5 km")` (notes/units_runtime.md)
pub(crate) const UNITS_MODULE: &str = "units";
pub(crate) const QUANTITY: &str = "quantity";
/// A served route's path parameters (`post "/todos/:id:int/toggle"`, lowering/web/serve.rs) are read by the router's words
pub(crate) const ROUTE_PARTS: [&str; 3] = ["route_segment", "route_parameter", "route_matches"];
/// lib/agent.warp: `agent "prompt"` asks Claude (card g_X_F0)
pub(crate) const AGENT_MODULE: &str = "agent";
/// lib/sound.warp: `play(440Hz, 0.5s)`, `beep` (card basic-sound), loaded when a program calls one of its words
pub(crate) const SOUND_MODULE: &str = "sound";
pub(crate) const SOUND_WORDS: [&str; 20] = [
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
/// routes or a server route's path parameters → router, a route's regular expression → regex (lowering/web/routes.rs), a call of quantity → units (run-time units),
/// a call of agent → agent, a call of play, beep, stop_sound or wait_sound → sound, a WAGI build (`warp build --wagi`) → wagi
pub(crate) const IMPLICIT_MODULES: [(&str, NeededBy); 8] = [
	("file", mentions_file_url),
	("markup", |program| {
		let mode = crate::pipeline::compile_mode();
		mode.renders_itself() || mode.for_hosting && calls_undefined(program, crate::page_html::TO_HTML)
	}),
	("router", |program| {
		defined(program, crate::routes::PAGE_ROUTES).is_some() || {
			let called = called_names(statements_of(program));
			ROUTE_PARTS.iter().any(|word| called.contains(*word))
		}
	}),
	("regex", |program| {
		defined(program, crate::routes::PAGE_ROUTE_INDEX)
			.is_some_and(|index| called_names(&[index]).contains(crate::routes::REGEX_MATCH))
	}),
	(UNITS_MODULE, |program| calls_undefined(program, QUANTITY)),
	(AGENT_MODULE, |program| calls_undefined(program, AGENT_MODULE)),
	(SOUND_MODULE, |program| first_called_undefined(program, SOUND_SETUP_FREE_WORDS).is_some()),
	("wagi", |_| crate::pipeline::compile_mode().for_wagi),
];

/// The program with the standard modules' definitions it calls, and those they call, in front: a word nobody calls
/// is never compiled (its parameters would have no kinds)
/// An implicit module with classes (units: `quantity("5 km")` gives a Quantity) in front of the program, the definitions
/// it needs only, before class_methods: the instances its functions give then take their class's operators and methods.
/// The program defining them now, implicit_std_modules loads it no more, nor its own `use units`. A class of fields
/// only (sound's Sample) needs no such move: the module stays a used module, its words lowered as before (note names)
pub(crate) fn with_implicit_class_modules(mut program: Node) -> Node {
	for (module, needed) in IMPLICIT_MODULES {
		if !needed(&program) {
			continue;
		}
		let source = std_module(module).expect("an implicit module is embedded");
		let definitions = statements(crate::normalize::without_hints(|| WarpParser::parse(source)));
		if definitions.iter().any(crate::node::is_class_with_methods) {
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

/// `xs.unique()`, `unique(xs)`: the words of the standard modules named after a type (lib/list.warp, text.warp,
/// map.warp) are standard functions of every program, called either way (user decision type-library-words, card
/// dispatch: "just provide the standard functions for list and map by default"), so a program calling one uses that
/// module. A library word stays the builtin, a word the program defines is its own, a word of two such modules needs
/// its `use`, `list.zip(a, b)` names the module (test_std_qualified)
/// The standard modules named after a type with their words, the methods of that type: list, map, text
pub fn type_modules() -> Vec<(&'static str, &'static [String])> {
	std_module_names()
		.filter(|module| crate::type_tests::is_builtin_type_word(module))
		.filter_map(|module| Some((module, std_module_definitions(module)?)))
		.collect()
}

pub(crate) fn with_type_module_uses(program: Node) -> Node {
	let defined = own_functions(&program);
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
		.filter(|module| !used.contains(*module))
		.collect();
	modules.sort();
	modules.dedup();
	with_statements_first(
		program,
		modules.into_iter().map(|module| WarpParser::parse(&format!("use {module}"))).collect(),
	)
}

/// Whether the program plays sound: it calls a sound word or says `use sound` (lowering/words/music_words.rs reads its note names)
pub fn uses_sound(program: &Node) -> bool {
	first_called_undefined(program, SOUND_WORDS).is_some()
		|| statements_of(program).iter().filter_map(used_module).any(|used| used.name == SOUND_MODULE)
}

/// A rare sound word the program calls: without `use sound` its error names the `use` (card bizarre-global)
pub(crate) fn rare_sound_word(program: &Node) -> Option<&'static str> {
	first_called_undefined(program, SOUND_WORDS.into_iter().filter(|word| !SOUND_SETUP_FREE_WORDS.contains(word)))
}

/// Whether the program calls `word` without defining it
pub(crate) fn calls_undefined(program: &Node, word: &str) -> bool {
	first_called_undefined(program, [word]).is_some()
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
