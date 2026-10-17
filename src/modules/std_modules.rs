//! The standard modules: embedded lib/<module>.warp sources, their names and words, and the prelude words with their parameters
use super::*;

/// The standard library's modules written in warp (notes/stdlib.md), embedded so `use list` needs no files
pub(crate) const STD_MODULES: [(&str, &str); 29] = [
	(PRELUDE_MODULE, include_str!("../../lib/prelude.warp")),
	("memory", include_str!("../../lib/memory.warp")),
	("net", include_str!("../../lib/net.warp")),
	("collections", include_str!("../../lib/collections.warp")),
	("hash", include_str!("../../lib/hash.warp")),
	("regex", include_str!("../../lib/regex.warp")),
	("file", include_str!("../../lib/file.warp")),
	("json", include_str!("../../lib/json.warp")),
	("os", include_str!("../../lib/os.warp")),
	("shell", include_str!("../../lib/shell.warp")),
	("list", include_str!("../../lib/list.warp")),
	("math", include_str!("../../lib/math.warp")),
	("fast_math", include_str!("../../lib/fast_math.warp")),
	("text", include_str!("../../lib/text.warp")),
	("random", include_str!("../../lib/random.warp")),
	("map", include_str!("../../lib/map.warp")),
	("time", include_str!("../../lib/time.warp")),
	("matrix", include_str!("../../lib/matrix.warp")),
	("draw", include_str!("../../lib/draw.warp")),
	("compute_toys", include_str!("../../lib/compute_toys.warp")),
	("markup", include_str!("../../lib/markup.warp")),
	("router", include_str!("../../lib/router.warp")),
	("wagi", include_str!("../../lib/wagi.warp")),
	("i18n", include_str!("../../lib/i18n.warp")),
	(UNITS_MODULE, include_str!("../../lib/units.warp")),
	(AGENT_MODULE, include_str!("../../lib/agent.warp")),
	(SOUND_MODULE, include_str!("../../lib/sound.warp")),
	("R", include_str!("../../lib/R.warp")),
	("PHP", include_str!("../../lib/PHP.warp")),
];
/// Other names of standard modules (word choices are aliases): `use graphics` is `use draw`
pub(crate) const STD_MODULE_ALIASES: [(&str, &str); 1] = [("graphics", "draw")];
/// The standard modules' folder (P194: std/ merged into lib/), embedded in the binary
pub(crate) const STD_FOLDER: &str = "lib";
/// lib/prelude.warp: the warp-written words of every program, loaded when mentioned (Loader::use_prelude)
pub(crate) const PRELUDE_MODULE: &str = "prelude";
/// Between a module's name and its word in a qualified definition name, as between a nested function's outer and own
pub(crate) const QUALIFIER: char = '·';
/// P171: module words a program calls without their `use` (the prelude); only these definitions come along
pub(crate) const PRELUDE_WORDS: [(&str, &[&str]); 2] =
	[("file", &["write", "exists", "write_bytes", "read_bytes"]), ("os", &["exec"])];
/// Other languages' names of the standard modules' classes and words (Java, Python, Rust, C#), each read as warp's with
/// a note, when a used module defines that word
pub(crate) const STD_ALIASES: [(&str, &str); 27] = [
	("HashSet", "Set"),
	("TreeSet", "Set"),
	("LinkedHashSet", "Set"),
	("frozenset", "Set"),
	("ArrayDeque", "Deque"),
	("VecDeque", "Deque"),
	("deque", "Deque"),
	("OrderedDict", "OrderedMap"),
	("LinkedHashMap", "OrderedMap"),
	// P169: the C library's historic names of `use math`'s words (atan2 takes y first, angle(x, y) does not: no alias)
	// sqrt and cbrt are the operators √ and ∛ already
	("pow", "power"),
	("exp", "exponential"),
	("log2", "binary_log"),
	("log10", "decimal_log"),
	("sin", "sine"),
	("cos", "cosine"),
	("tan", "tangent"),
	("asin", "arc_sine"),
	("acos", "arc_cosine"),
	("atan", "arc_tangent"),
	("sinh", "hyperbolic_sine"),
	("cosh", "hyperbolic_cosine"),
	("tanh", "hyperbolic_tangent"),
	("hypot", "hypotenuse"),
	("ceil", "ceiling"),
	("trunc", "whole_part"),
	("fmod", "remainder"),
	("ln", "natural_log"),
];

/// warp's own lib/<module>.warp of a standard module: the source the binary embeds (natively the lib folder of the
/// repository it was built from, or any file of the same text: lib/ of another warp checkout, card local-lib; in the
/// browser lib/ of the served repository, the page's file root)
pub(super) fn is_embedded_std_file(path: &Path) -> bool {
	let Some(source) = path.file_stem().and_then(|stem| stem.to_str()).and_then(std_module) else {
		return false;
	};
	let folder = path.parent().unwrap_or(Path::new(""));
	if !cfg!(feature = "native") {
		return folder == Path::new(STD_FOLDER);
	}
	let standard_folder = Path::new(env!("CARGO_MANIFEST_DIR")).join(STD_FOLDER).canonicalize().ok();
	standard_folder.is_some() && folder.canonicalize().ok() == standard_folder
		|| std::fs::read_to_string(path).is_ok_and(|text| text == source)
}

/// A file changed after the running warp was built: a standard module being edited in a checkout, not a stale copy
pub(super) fn edited_since_build(path: &Path) -> bool {
	let modified = |file: &Path| std::fs::metadata(file).and_then(|metadata| metadata.modified()).ok();
	let built = std::env::current_exe().ok().and_then(|binary| modified(&binary));
	matches!((modified(path), built), (Some(edited), Some(built)) if edited > built)
}

/// The name an embedded module is loaded under, once per program
pub(super) fn std_path(name: &str) -> PathBuf {
	PathBuf::from(format!("{STD_FOLDER}/{name}.warp"))
}

/// Is `file` an embedded standard module as loaded (std_path): code the program's writer does not write
pub fn is_std_source(file: &str) -> bool {
	let name = file.strip_prefix(STD_FOLDER).and_then(|rest| rest.strip_prefix('/')?.strip_suffix(".warp"));
	name.is_some_and(|name| std_module(name).is_some())
}

/// The source of the standard module `name` (lib/<name>.warp, embedded)
pub fn std_module_source(name: &str) -> Option<&'static str> {
	std_module(name)
}

/// The standard modules' names: `list`, `math`, …
pub fn std_module_names() -> impl Iterator<Item = &'static str> {
	STD_MODULES.iter().map(|(module, _)| *module)
}

pub(super) fn std_module(name: &str) -> Option<&'static str> {
	let name = std_module_name(name);
	STD_MODULES.iter().find(|(module, _)| *module == name).map(|(_, source)| *source)
}

/// A standard module's own name for one of its aliases (`graphics` → draw), so each is loaded once
pub(super) fn std_module_name(name: &str) -> &str {
	STD_MODULE_ALIASES.iter().find(|(alias, _)| *alias == name).map_or(name, |(_, module)| *module)
}

/// The names the standard module at `index` of STD_MODULES defines, in their order: kept from an earlier run
/// (std_words_cache.rs), else every module parsed for them once, when first asked, and kept for the next runs
pub(super) fn std_module_words(index: usize) -> &'static [String] {
	static DEFINED: std::sync::OnceLock<Vec<Vec<String>>> = std::sync::OnceLock::new();
	&DEFINED.get_or_init(|| match kept_words() {
		Some(kept) => kept.words.clone(),
		None => {
			let words: Vec<Vec<String>> = STD_MODULES.iter().map(|(_, source)| parsed_words(source)).collect();
			if let Some(heads) = prelude_heads() {
				keep_words(&words, heads);
			}
			words
		}
	})[index]
}

fn parsed_words(source: &str) -> Vec<String> {
	statements(crate::normalize::without_hints(|| WarpParser::parse(source)))
		.iter()
		.filter(|statement| used_module(statement).is_none())
		.filter_map(name_declared_by)
		.collect()
}

/// The names the standard modules define with parameters (`ls(folder:any = ".") := …`), read from their text: the
/// parser asks while it may be parsing one of them, so std_module_words, which parses them, can't answer
pub fn std_function_names() -> &'static std::collections::HashSet<String> {
	static NAMES: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
	NAMES.get_or_init(|| {
		let defined = |line: &str| {
			let name: String = line.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
			(!name.is_empty() && line[name.len()..].starts_with('(') && line.contains(":=")).then_some(name)
		};
		STD_MODULES.iter().flat_map(|(_, source)| source.lines().filter_map(defined)).collect()
	})
}

/// The standard module that defines `word` (`zip` → list), for the error of a word used without its `use`; the module
/// named after a type first: `unique` is list's, shell only adds a method for paths (card cross-module-overload)
pub fn std_module_defining(word: &str) -> Option<&'static str> {
	let defining: Vec<&'static str> = (0..STD_MODULES.len())
		.filter(|index| std_module_words(*index).iter().any(|name| name == word))
		.map(|index| STD_MODULES[index].0)
		.collect();
	defining.iter().find(|module| crate::type_tests::is_builtin_type_word(module)).or(defining.first()).copied()
}

/// The classes a standard module declares (text's Cursor): no methods of the type the module is named after
pub fn std_module_classes(module: &str) -> Vec<String> {
	let Some(source) = std_module(module) else { return vec![] };
	let statements = statements(crate::normalize::without_hints(|| WarpParser::parse(source)));
	statements.iter().filter(|statement| is_class(statement)).filter_map(name_declared_by).collect()
}

/// The names a standard module defines, in their order (`dir(time)`); None for no standard module
pub fn std_module_definitions(module: &str) -> Option<&'static [String]> {
	STD_MODULES.iter().position(|(name, _)| *name == module).map(std_module_words)
}

/// The number of parameters of a prelude word (`replace(t, old, new) := …` → 3); None for no prelude word
pub fn prelude_word_arity(word: &str) -> Option<usize> {
	prelude_word_parameters(word).map(|(arity, _)| arity)
}

/// The number of parameters of a prelude word a call may leave out, those with a default (`limit = 1000000`)
pub fn prelude_word_defaults(word: &str) -> usize {
	prelude_word_parameters(word).map_or(0, |(_, defaults)| defaults)
}

/// The arguments of a call of a prelude word in parameter order: one may be named (`nearest(x, rational, within:
/// 1e-6)`), one left out is its default (`limit = 0`); unchanged for no prelude word
pub fn prelude_arguments(word: &str, arguments: Vec<Node>) -> Vec<Node> {
	let parameters = prelude_definition(word).map(|(parameters, _)| {
		parameters
			.iter()
			.map(|parameter| match parameter.drop_meta() {
				Node::Key(name, Op::Assign, default) => (name.drop_meta().name(), Some(default.as_ref().clone())),
				other => (other.name(), None),
			})
			.collect::<Vec<_>>()
	});
	let Some(parameters) = parameters else { return arguments };
	let mut slots: Vec<Option<Node>> = vec![None; parameters.len()];
	let named_slot = |argument: &Node| match argument.drop_meta() {
		Node::Key(name, Op::Colon | Op::Assign, value) => parameters
			.iter()
			.position(|(parameter, _)| *parameter == name.drop_meta().name())
			.map(|slot| (slot, value.as_ref().clone())),
		_ => None,
	};
	for argument in arguments.iter() {
		let (slot, value) = named_slot(argument)
			.unwrap_or_else(|| (slots.iter().position(Option::is_none).unwrap_or(slots.len()), argument.clone()));
		match slots.get_mut(slot) {
			Some(empty @ None) => *empty = Some(value),
			_ => return arguments, // too many, or one twice: the arity check names it
		}
	}
	let filled: Option<Vec<Node>> =
		slots.into_iter().zip(&parameters).map(|(slot, (_, default))| slot.or_else(|| default.clone())).collect();
	filled.unwrap_or(arguments)
}

/// The parameters and the body of the prelude word's definition (`paint_each(f, width, height) := …`)
pub fn prelude_definition(word: &str) -> Option<(Vec<Node>, Node)> {
	let source = std_module(PRELUDE_MODULE).unwrap_or_default();
	statements(crate::normalize::without_hints(|| WarpParser::parse(source))).into_iter().find_map(|statement| {
		match statement.drop_meta() {
			Node::Key(head, Op::Define, body) if leftmost_symbol(head).as_deref() == Some(word) => {
				match head.drop_meta() {
					Node::List(items, _, _) => Some((items[1..].to_vec(), body.as_ref().clone())),
					_ => None,
				}
			}
			_ => None,
		}
	})
}

pub(super) fn prelude_word_parameters(word: &str) -> Option<(usize, usize)> {
	prelude_heads()?.iter().find(|(name, _, _)| name == word).map(|(_, arity, defaults)| (*arity, *defaults))
}

/// The prelude word `word` names, spelled as the prelude defines it; None for no prelude word
pub fn prelude_word(word: &str) -> Option<&'static str> {
	prelude_heads()?.iter().find(|(name, _, _)| name == word).map(|(name, _, _)| name.as_str())
}

/// Each prelude word with its number of parameters and of those with a default; None while the prelude is being read
fn prelude_heads() -> Option<&'static [(String, usize, usize)]> {
	static PARAMETERS: std::sync::OnceLock<Vec<(String, usize, usize)>> = std::sync::OnceLock::new();
	thread_local!(static READING_PRELUDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
	// parsing the prelude asks for its own words (the parser's operand words, library_words::is_operand_word): none
	// known yet; waiting for them would wait for itself
	if PARAMETERS.get().is_none() && READING_PRELUDE.get() {
		return None;
	}
	let parameters = PARAMETERS.get_or_init(|| {
		if let Some(kept) = kept_words() {
			return kept.heads.clone();
		}
		READING_PRELUDE.set(true);
		let source = std_module(PRELUDE_MODULE).unwrap_or_default();
		let has_default = |parameter: &Node| matches!(parameter.drop_meta(), Node::Key(_, Op::Assign, _));
		let parameters = statements(crate::normalize::without_hints(|| WarpParser::parse(source)))
			.iter()
			.filter_map(|statement| match statement.drop_meta() {
				Node::Key(head, Op::Define, _) => match head.drop_meta() {
					Node::List(items, _, _) => Some((
						leftmost_symbol(head)?,
						items.len() - 1,
						items[1..].iter().filter(|item| has_default(item)).count(),
					)),
					_ => None,
				},
				_ => None,
			})
			.collect();
		READING_PRELUDE.set(false);
		parameters
	});
	Some(parameters)
}

/// The name a prelude word's definition has in the program (`first` → `prelude·first`): a program's own `first`, or
/// a local of that name, never meets it; library_words writes the calls of the word so. None for no prelude word
pub fn prelude_name(word: &str) -> Option<String> {
	prelude_word_arity(word).map(|_| format!("{PRELUDE_MODULE}{QUALIFIER}{word}"))
}

/// The texts of a list a standard module assigns at its top: `html_elements = ["html", …]` of markup; empty if missing
pub fn std_module_list(module: &str, list: &str) -> Vec<String> {
	let source = std_module(module).unwrap_or_default();
	statements(crate::normalize::without_hints(|| WarpParser::parse(source)))
		.iter()
		.find_map(|statement| match statement.drop_meta() {
			Node::Key(name, Op::Assign, items) if name.drop_meta().name() == list => match items.drop_meta() {
				Node::List(items, _, _) => Some(items.iter().map(Node::text_or_name).collect()),
				_ => None,
			},
			_ => None,
		})
		.unwrap_or_default()
}

/// The file of the program's folder or search directories that `use module` finds before the standard module of
/// that name (a local file wins)
pub fn module_file_shadowing(module: &str) -> Option<PathBuf> {
	let folder = program_file().as_deref().map(folder_of);
	Loader::new(&SEARCH_DIRECTORIES, folder).find(module)
}

/// `HashSet(xs)` as `Set(xs)`, with a note
pub(crate) fn with_std_aliases(node: Node, aliases: &[(&str, &str)]) -> Node {
	// a note points at the innermost positioned node around the written word
	if node.get_lineinfo().is_some() {
		crate::normalize::set_position_of(&node);
	}
	match node {
		Node::Symbol(name) => match aliases.iter().find(|(alias, _)| *alias == name) {
			Some((alias, class)) => {
				crate::errors::diagnostic::note_alias(alias, class);
				Node::Symbol(class.to_string())
			}
			None => Node::Symbol(name),
		},
		other => other.map_children(|child| with_std_aliases(child, aliases)),
	}
}
