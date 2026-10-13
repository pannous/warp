//! The standard modules: embedded lib/<module>.warp sources, their names and words, and the prelude words with their parameters
use super::*;

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
	let standard_folder = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join(STD_FOLDER)
		.canonicalize()
		.ok();
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
	STD_MODULES
		.iter()
		.find(|(module, _)| *module == name)
		.map(|(_, source)| *source)
}

/// A standard module's own name for one of its aliases (`graphics` → draw), so each is loaded once
pub(super) fn std_module_name(name: &str) -> &str {
	STD_MODULE_ALIASES
		.iter()
		.find(|(alias, _)| *alias == name)
		.map_or(name, |(_, module)| *module)
}

/// The names the standard module at `index` of STD_MODULES defines, in their order; each module is parsed for them
/// once, when first asked (a compile asks for a few of them)
pub(super) fn std_module_words(index: usize) -> &'static [String] {
	pub(super) static DEFINED: [std::sync::OnceLock<Vec<String>>; STD_MODULES.len()] =
		[const { std::sync::OnceLock::new() }; STD_MODULES.len()];
	DEFINED[index].get_or_init(|| {
		let source = STD_MODULES[index].1;
		statements(crate::normalize::without_hints(|| WarpParser::parse(source)))
			.iter()
			.filter(|statement| used_module(statement).is_none())
			.filter_map(declared_name)
			.collect()
	})
}

/// The standard module that defines `word` (`zip` → list), for the error of a word used without its `use`
pub fn std_module_defining(word: &str) -> Option<&'static str> {
	(0..STD_MODULES.len())
		.find(|index| std_module_words(*index).iter().any(|name| name == word))
		.map(|index| STD_MODULES[index].0)
}

/// The names a standard module defines, in their order (`dir(time)`); None for no standard module
pub fn std_module_definitions(module: &str) -> Option<&'static [String]> {
	STD_MODULES
		.iter()
		.position(|(name, _)| *name == module)
		.map(std_module_words)
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
	let source = std_module(PRELUDE_MODULE).unwrap_or_default();
	let parameters = statements(crate::normalize::without_hints(|| WarpParser::parse(source)))
		.into_iter()
		.find_map(|statement| match statement.drop_meta() {
			Node::Key(head, Op::Define, _) if leftmost_symbol(head).as_deref() == Some(word) => {
				match head.drop_meta() {
					Node::List(items, _, _) => Some(
						items[1..]
							.iter()
							.map(|item| match item.drop_meta() {
								Node::Key(name, Op::Assign, default) => {
									(name.drop_meta().name(), Some(default.as_ref().clone()))
								}
								other => (other.name(), None),
							})
							.collect::<Vec<_>>(),
					),
					_ => None,
				}
			}
			_ => None,
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
		let (slot, value) = named_slot(argument).unwrap_or_else(|| {
			(
				slots.iter().position(Option::is_none).unwrap_or(slots.len()),
				argument.clone(),
			)
		});
		match slots.get_mut(slot) {
			Some(empty @ None) => *empty = Some(value),
			_ => return arguments, // too many, or one twice: the arity check names it
		}
	}
	let filled: Option<Vec<Node>> = slots
		.into_iter()
		.zip(&parameters)
		.map(|(slot, (_, default))| slot.or_else(|| default.clone()))
		.collect();
	filled.unwrap_or(arguments)
}

pub(super) fn prelude_word_parameters(word: &str) -> Option<(usize, usize)> {
	pub(super) static PARAMETERS: std::sync::OnceLock<Vec<(String, usize, usize)>> = std::sync::OnceLock::new();
	thread_local!(static READING_PRELUDE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
	// parsing the prelude asks for its own words (the parser's operand words, library_words::is_operand_word): none
	// known yet; waiting for them would wait for itself
	if PARAMETERS.get().is_none() && READING_PRELUDE.get() {
		return None;
	}
	let parameters = PARAMETERS.get_or_init(|| {
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
	parameters
		.iter()
		.find(|(name, _, _)| name == word)
		.map(|(_, arity, defaults)| (*arity, *defaults))
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
				// "p" parses as a character
				Node::List(items, _, _) => Some(
					items
						.iter()
						.map(|item| match item.drop_meta() {
							Node::Char(character) => character.to_string(),
							other => other.name(),
						})
						.collect(),
				),
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
