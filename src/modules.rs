//! `use <name>` file modules: `name.wasp` or `name.warp` is loaded and its definitions become part of the program.
//! A name found nowhere locally but in the registry packages.warp is a package: its git repository, fetched into packages/<name>.
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::versions::{declared_version, is_version_keyword, version_of, Requirement, Version, MINIMUM_KEYWORD};
use crate::warp_parser::WarpParser;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

mod loader;
use loader::*;
mod std_modules;
pub use std_modules::*;
mod packages;
pub use packages::*;

/// Where modules are looked for, in this order: the current directory, the samples, the library (lib/extra: modules
/// that are not standard, like netbase, P194)
pub const SEARCH_DIRECTORIES: [&str; 7] = [".", "include", "lib", "lib/extra", "src", "source", "samples"];
pub const MODULE_EXTENSIONS: [&str; 2] = ["wasp", "warp"];
/// `use x`, and its aliases `require x` and `import x`: the declarations of the file x
pub(crate) const USE_KEYWORDS: [&str; 3] = ["use", "require", "import"];
/// `include x`: the whole file, spliced in place
const INCLUDE_KEYWORD: &str = "include";

/// `use x`, `require x`, `import x`, `include x`: a statement that brings in a module, the one list every pass asks
pub(crate) fn is_import_keyword(word: &str) -> bool {
	USE_KEYWORDS.contains(&word) || word == INCLUDE_KEYWORD
}
const USE_KEYWORD: &str = "use";
/// `from list import zip, unique`: only those words of the module
const IMPORT_KEYWORD: &str = "import";
/// name: "git url" for every package, compiled in so warp finds it from any directory
const PACKAGE_REGISTRY: &str = include_str!("../packages.warp");
/// the directory of the file it is written in, as text: `read(module_directory + "/data/x")` finds a module's own files
const MODULE_DIRECTORY: &str = "module_directory";
/// A module word a program variable shadows is renamed apart: `words` of lib/text is `lib·words` in its definitions
const SHADOWED_WORD_PREFIX: &str = "lib·";
/// where packages are fetched to, below the working directory
pub const PACKAGES_DIRECTORY: &str = "packages";
/// where a tagged version of a package is fetched once per machine and linked from packages/: a tag never changes
const PACKAGE_CACHE: &str = ".cache/warp/packages";
/// where an unpinned clone in packages/ goes when a pin takes its place
const REPLACED_DIRECTORY: &str = ".replaced";
/// one fetch at a time within a process; parallel processes clone to their own staging directory and rename
static FETCHING: Mutex<()> = Mutex::new(());
/// Statements a module contributes; everything else in a module (expressions, calls) is its own business
/// `use folder`, `use package`, `use project`: every definition of that scope is visible, looked up by name (D15)
const SCOPE_WORDS: [(&str, Scope); 3] = [
	("folder", Scope::Folder),
	("package", Scope::Package),
	("project", Scope::Project),
];
/// Folders a package or project scope never looks into, besides hidden ones and nested repositories
const UNSCOPED_DIRECTORIES: [&str; 3] = [PACKAGES_DIRECTORY, "target", "node_modules"];
const PROJECT_MARKER: &str = ".git";
/// `stored x = v` too: a used module's persisted signal is the program's (card web-stores)
const GLOBAL_KEYWORD: &str = "global";
const DECLARATION_KEYWORDS: [&str; 4] = ["let", "var", GLOBAL_KEYWORD, crate::stored_values::STORED_WORD];

fn is_declaration_keyword(keyword: &str) -> bool {
	DECLARATION_KEYWORDS.contains(&keyword)
		|| is_import_keyword(keyword)
		|| crate::words::CONSTANT_KEYWORDS.contains(&keyword)
}

/// Replace every `use <module>` of the program by the definitions of that module, each module once.
/// `use math` and other native libraries stay in place for the FFI; an unknown name is an error value.
pub fn resolve(program: Node) -> Node {
	resolve_in(program, &SEARCH_DIRECTORIES)
}

pub fn resolve_in(program: Node, directories: &[&str]) -> Node {
	crate::compile_context::change(|context| context.module_definitions.clear());
	let own_names: Vec<String> = statements_of(&program).iter().filter_map(declared_name).collect();
	let file = program_file();
	let mut loader = Loader::new(directories, file.as_deref().map(folder_of));
	loader.program_variables = program_variables(&program);
	let folder = loader.including_directory.clone();
	let program = with_module_directory(program, folder.as_deref().unwrap_or(Path::new(".")));
	let resolved = loader
		.resolve(program)
		.and_then(|program| loader.implicit_std_modules(&program).map(|_| program))
		.and_then(|program| match loader.scope {
			Some(scope) => loader.with_scope(program, scope, file.as_deref()),
			None => Ok(program),
		})
		.map(|program| crate::wasm_modules::rewrite_uses(program, &loader.wasm_modules))
		.map(|program| with_needed_definitions(program, loader.hidden_apart()));
	let files = loader
		.loaded
		.iter()
		.chain(&loader.included)
		.filter(|path| path.is_file())
		.cloned()
		.collect();
	crate::compile_context::change(|context| {
		context.module_files = files;
		context.early_class_modules.clear(); // for this program only
		own_names.iter().for_each(|own| {
			context.module_definitions.remove(own); // the program's own word wins
		});
	});
	resolved.unwrap_or_else(|failure| failure)
}

/// The classes of the modules the program uses, in front of it: those of a file (`use shapes`) all, those of a standard
/// module the program names (`use collections; s = Stack()`). class_methods, which turns a class's methods into
/// functions and its method calls into theirs, runs before `resolve` loads a module's other definitions; the loader
/// then leaves these modules' classes out (early_class_modules). A class the program declares itself wins.
pub fn insert_module_classes(program: Node) -> Node {
	let mut program = with_type_module_uses(program);
	let file = program_file();
	let loader = Loader::new(&SEARCH_DIRECTORIES, file.as_deref().map(folder_of));
	let own_names: HashSet<String> = statements_of(&program).iter().filter_map(declared_name).collect();
	let (mut std_definitions, mut file_classes, mut early, mut class_modules) =
		(vec![], vec![], HashSet::new(), vec![]);
	let mut defined: Vec<String> = vec![]; // every used module's words, a file's copy of a standard module's too
	for used in statements(program.clone())
		.iter()
		.filter_map(used_module)
		.filter(|used| used.import == Import::Use)
	{
		if let Some(module) = loader
			.find(&used.name)
			.is_none()
			.then(|| loader.find_with(&used.name, &crate::wasm_modules::MODULE_EXTENSIONS))
			.flatten()
		{
			let module = module.to_string_lossy();
			let classes = crate::wasm_modules::classes(&module);
			let names: Vec<String> = classes.iter().filter_map(declared_name).collect();
			program =
				crate::wasm_modules::with_bare_classes(program, &crate::wasm_modules::module_alias(&module), &names);
			file_classes.extend(classes);
			continue;
		}
		let (path, source, is_std) = match loader.find(&used.name) {
			Some(path) => match crate::web::read_text(&path.to_string_lossy()) {
				Some(source) => (path, source, false),
				None => continue,
			},
			None => match std_module(&used.name) {
				Some(source) => (std_path(std_module_name(&used.name)), source.to_string(), true),
				None => continue,
			},
		};
		// a look for classes only: the loader's own read of a file module hints it, a standard module never
		let module = crate::normalize::without_hints(|| WarpParser::parse(&source));
		if module.first_error().is_some() {
			continue; // the loader reports it
		}
		let definitions = statements(module);
		defined.extend(definitions.iter().filter_map(declared_name));
		match is_std {
			true => {
				if definitions.iter().any(is_class) {
					class_modules.push(used.name.clone());
				}
				std_definitions.extend(definitions)
			}
			false => file_classes.extend(definitions.into_iter().filter(is_class)),
		}
		early.insert(path.canonicalize().unwrap_or(path));
	}
	crate::compile_context::change(|context| context.early_class_modules = early);
	let is_foreign = |class: &Node| declared_name(class).is_none_or(|name| !own_names.contains(&name));
	let aliases: Vec<(&str, &str)> = STD_ALIASES
		.into_iter()
		.filter(|(alias, word)| defined.iter().any(|name| name == word) && !own_names.contains(*alias))
		.collect();
	// `collections.Counter(xs)`: the bare class before class_methods reads its construction; other qualified calls
	// (`math.factorial` of `use python math`, `json.loads`) wait for foreign_modules and welcome_forms
	let class_modules: Vec<&str> = class_modules.iter().map(String::as_str).collect();
	let program = with_std_aliases(crate::welcome_forms::module_calls(program, &class_modules), &aliases);
	let std_classes = std_definitions
		.into_iter()
		.filter(|definition| is_class(definition) && is_foreign(definition))
		.collect();
	let file_classes: Vec<Node> = file_classes.into_iter().filter(is_foreign).collect();
	let program = with_implicit_class_modules(with_needed_definitions(program, std_classes));
	match file_classes.is_empty() {
		true => program,
		false => match program {
			Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
				Node::List([file_classes, statements].concat(), Bracket::None, separator)
			}
			single => Node::List(
				[file_classes, vec![single]].concat(),
				Bracket::None,
				Separator::Semicolon,
			),
		},
	}
}

/// `HashSet(xs)` as `Set(xs)`, with a note
fn with_std_aliases(node: Node, aliases: &[(&str, &str)]) -> Node {
	// a note points at the innermost positioned node around the written word
	if node.get_lineinfo().is_some() {
		crate::normalize::set_position_of(&node);
	}
	match node {
		Node::Symbol(name) => match aliases.iter().find(|(alias, _)| *alias == name) {
			Some((alias, class)) => {
				crate::diagnostic::note_alias(alias, class);
				Node::Symbol(class.to_string())
			}
			None => Node::Symbol(name),
		},
		other => other.map_children(|child| with_std_aliases(child, aliases)),
	}
}

fn is_class(statement: &Node) -> bool {
	matches!(statement.drop_meta(), Node::Type { .. })
}

/// The program with the standard modules' definitions it calls, and those they call, in front: a word nobody calls
/// is never compiled (its parameters would have no kinds)
/// An implicit module with classes (units: `quantity("5 km")` gives a Quantity) in front of the program, the definitions
/// it needs only, before class_methods: the instances its functions give then take their class's operators and methods.
/// The program defining them now, implicit_std_modules loads it no more, nor its own `use units`
fn with_implicit_class_modules(mut program: Node) -> Node {
	for (module, needed) in IMPLICIT_MODULES {
		if !needed(&program) {
			continue;
		}
		let source = std_module(module).expect("an implicit module is embedded");
		let definitions = statements(crate::normalize::without_hints(|| WarpParser::parse(source)));
		if definitions.iter().any(is_class) {
			program = with_needed_definitions(without_use_of(program, module), definitions);
		}
	}
	program
}

/// The program without its `use <module>`
fn without_use_of(program: Node, module: &str) -> Node {
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
		.filter(|module| crate::type_tests::is_type_word(module))
		.filter_map(|module| Some((module, std_module_definitions(module)?)))
		.collect()
}

fn with_type_module_uses(program: Node) -> Node {
	let defined = own_functions(&program);
	// `from list import zip` brings only zip: the module's other words stay out (test_from_import)
	let imported =
		|statement: &Node| used_module(statement).or_else(|| imported_words(statement).map(|(used, _, _)| used));
	let used: HashSet<String> = statements_of(&program)
		.iter()
		.filter_map(imported)
		.map(|used| used.name)
		.collect();
	let type_modules = type_modules();
	let is_module_name = |receiver: &Node| {
		receiver
			.symbol_name()
			.is_some_and(|name| std_module(name).is_some() && !defined.contains(name))
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
		modules
			.into_iter()
			.map(|module| WarpParser::parse(&format!("use {module}")))
			.collect(),
	)
}

/// The names the program declares, but not its methods: `take()` of a class Box, `take(b:Box)` add to list's take
/// (notes/dispatch.md "Methods"), a plain `take(xs, n)` is the program's own take
fn own_functions(program: &Node) -> HashSet<String> {
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
fn collect_own_functions(node: &Node, classes: &HashSet<String>, own: &mut HashSet<String>) {
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
			own.extend(declared_name(node));
			collect_own_functions(left, classes, own);
			collect_own_functions(right, classes, own);
		}
		Node::List(items, _, _) => {
			own.extend(declared_name(node));
			items.iter().for_each(|item| collect_own_functions(item, classes, own));
		}
		other => own.extend(declared_name(other)),
	}
}

/// The words called with arguments: `unique` of `unique(xs)`, `unique xs`, `xs.unique()` and `take` of `xs.take(2)`; `book.title`
/// may be a field, `list.zip(a, b)` of a module name is a qualified word
fn called_words(program: &Node, is_module_name: &dyn Fn(&Node) -> bool) -> HashSet<String> {
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
		let mut locals: HashSet<String> = crate::lowering::memoization::head_parameters(&head)
			.into_iter()
			.collect();
		crate::library_words::collect_assigned_names(&body, &mut locals);
		names.extend(called_inside.into_iter().filter(|name| !locals.contains(name)));
		return;
	}
	let called = |call: &[Node]| call.first().and_then(Node::symbol_name).map(String::from);
	match node.drop_meta() {
		Node::Key(receiver, Op::Dot | Op::SafeDot, member) => {
			collect_called_words(receiver, is_module_name, names);
			if let Node::List(call, _, _) = member.drop_meta() {
				if !is_module_name(receiver) {
					names.extend(called(call));
				}
				call.iter()
					.skip(1)
					.for_each(|argument| collect_called_words(argument, is_module_name, names));
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
			items
				.iter()
				.for_each(|item| collect_called_words(item, is_module_name, names));
		}
		Node::Type { body, .. } => collect_called_words(body, is_module_name, names),
		_ => {}
	}
}

fn with_statements_first(program: Node, first: Vec<Node>) -> Node {
	if first.is_empty() {
		return program;
	}
	match program {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List(first.into_iter().chain(statements).collect(), Bracket::None, separator)
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_statements_first(node, first)),
		single => Node::List(
			first.into_iter().chain(std::iter::once(single)).collect(),
			Bracket::None,
			Separator::Newline,
		),
	}
}

fn with_needed_definitions(program: Node, definitions: Vec<Node>) -> Node {
	if definitions.is_empty() {
		return program;
	}
	let mut needed: Vec<Node> = vec![];
	let mut mentioned = called_names(std::slice::from_ref(&program));
	loop {
		let now_needed: Vec<Node> = definitions
			.iter()
			.filter(|definition| declared_name(definition).is_some_and(|name| is_needed(&name, &mentioned)))
			.cloned()
			.collect();
		if now_needed.len() == needed.len() {
			break;
		}
		mentioned.extend(called_names(&now_needed));
		needed = now_needed;
	}
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
fn is_needed(name: &str, mentioned: &HashSet<String>) -> bool {
	mentioned.contains(name)
		|| crate::lowering::overloads::companion_of(name).is_some_and(|function| mentioned.contains(function))
}

/// The names the statements mention or call, as methods too (`"hé".to_utf8()`), in interpolations too, with the library
/// words they spell
fn called_names(statements: &[Node]) -> HashSet<String> {
	with_library_words(
		mentioned_names(statements)
			.into_iter()
			.chain(method_names(statements))
			.collect(),
	)
}

/// The names with the library words they spell (`isdigit` → is_digit, `round` → round_to) and the prelude names of
/// those (`prelude·is_digit`): library_words, which writes them so, runs after the modules are resolved
fn with_library_words(names: HashSet<String>) -> HashSet<String> {
	let spelled: Vec<&str> = names
		.iter()
		.flat_map(|name| crate::library_words::words_spelled_by(name))
		.collect();
	let qualified: Vec<String> = names
		.iter()
		.map(String::as_str)
		.chain(spelled.iter().copied())
		.filter_map(prelude_name)
		.collect();
	let spelled: Vec<String> = spelled.into_iter().map(str::to_string).collect();
	names.into_iter().chain(spelled).chain(qualified).collect()
}

/// The variables a program assigns at its top: `words = […]`, also declared global: `global hue = 0`, `global hue`
fn program_variables(program: &Node) -> HashSet<String> {
	statements_of(program)
		.iter()
		.filter_map(|statement| match statement.drop_meta() {
			Node::Key(variable, Op::Assign, _) => variable.drop_meta().symbol_name().map(String::from),
			Node::Key(keyword, Op::Colon, declared) if keyword.is_symbol(GLOBAL_KEYWORD) => leftmost_symbol(declared),
			_ => None,
		})
		.collect()
}

/// Module words a program variable shadows, renamed in the module's definitions (`words` → `lib·words`): the module's
/// own code calls its own words (lexical scope), the program's `words` is its variable. Likewise a local of a module
/// function named like a program variable: a program's `global hue` is not draw's `hue` in hsv (card program-global).
/// A module's `global` is a setting, which the program's `tempo = 90` sets (lib/sound.warp's tempo, note_seconds)
fn shadowed_apart(definitions: Vec<Node>, variables: &HashSet<String>) -> Vec<Node> {
	let declared: Vec<String> = definitions.iter().filter_map(declared_name).collect();
	let settings: Vec<String> = definitions
		.iter()
		.filter(|definition| is_global_declaration(definition))
		.filter_map(declared_name)
		.collect();
	let shadowed: Vec<String> = declared
		.iter()
		.filter(|name| variables.contains(*name) && !settings.contains(name))
		.cloned()
		.collect();
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
fn assigned_names(definition: &Node) -> HashSet<String> {
	if matches!(definition.drop_meta(), Node::Key(_, Op::Assign, _)) || is_global_declaration(definition) {
		return HashSet::new();
	}
	definition.assigned_names(|op| *op == Op::Assign || op.is_compound_assign())
}

/// `global x = …`
fn is_global_declaration(statement: &Node) -> bool {
	matches!(statement.drop_meta(), Node::Key(keyword, Op::Colon, _) if keyword.is_symbol(GLOBAL_KEYWORD))
}

/// `names` renamed `lib·name` in the definitions: the definitions still call them, the program does not see them
fn renamed_apart(definitions: Vec<Node>, names: &[String]) -> Vec<Node> {
	definitions
		.into_iter()
		.map(|definition| {
			names.iter().fold(definition, |node, name| {
				renamed_word(node, name, &format!("{SHADOWED_WORD_PREFIX}{name}"))
			})
		})
		.collect()
}

/// `from` as a word renamed `to`; a field `x.from` keeps its name
fn renamed_word(node: Node, from: &str, to: &str) -> Node {
	match node {
		Node::Symbol(name) if name == from => Node::Symbol(to.to_string()),
		Node::Key(receiver, Op::Dot, field) => Node::Key(Box::new(renamed_word(*receiver, from, to)), Op::Dot, field),
		other => other.map_children(|child| renamed_word(child, from, to)),
	}
}

/// `path` as written in the program: a relative path is next to the program's file, or in the working directory for
/// inline code
pub fn beside_program(path: &str) -> String {
	let folder = program_file().as_deref().map(folder_of);
	match folder {
		Some(folder) if Path::new(path).is_relative() => folder.join(path).to_string_lossy().into_owned(),
		_ => path.to_string(),
	}
}

/// The WebAssembly file `use name` finds when no warp module and no standard module has that name: `use rust_demo` →
/// ./rust_demo.wasm
pub fn wasm_module_file(name: &str) -> Option<PathBuf> {
	if names_a_module(name) {
		return None;
	}
	program_loader().find_with(name, &crate::wasm_modules::MODULE_EXTENSIONS)
}

/// Does `use name` name anything: a module (warp, standard, WebAssembly), a scope, a package or a C library
pub fn resolves(name: &str) -> bool {
	names_a_module(name)
		|| wasm_module_file(name).is_some()
		|| scope_word(name).is_some()
		|| package_repository(name).is_some()
		|| is_builtin_library(name)
		|| crate::ffi::is_ffi_library(name)
}

/// Is `name` a warp module beside the program or in the search directories, or a standard module
pub fn names_a_module(name: &str) -> bool {
	program_loader().find(name).is_some() || std_module(name).is_some()
}

fn program_loader() -> Loader<'static> {
	Loader::new(&SEARCH_DIRECTORIES, program_file().as_deref().map(folder_of))
}

/// The file being compiled, None for inline code
pub fn program_file() -> Option<PathBuf> {
	crate::compile_context::read(|context| context.program_file.clone())
}

/// Compile `body` as the program of `file`: `use folder`, `use package` and `use project` start from its folder
pub fn with_program_file<R>(file: &Path, body: impl FnOnce() -> R) -> R {
	crate::compile_context::scoped(|context| &mut context.program_file, Some(file.to_path_buf()), body)
}

fn folder_of(file: &Path) -> PathBuf {
	match file.parent() {
		Some(folder) if !folder.as_os_str().is_empty() => folder.to_path_buf(),
		_ => PathBuf::from("."),
	}
}

/// The standard library's modules written in warp (notes/stdlib.md), embedded so `use list` needs no files
const STD_MODULES: [(&str, &str); 24] = [
	(PRELUDE_MODULE, include_str!("../lib/prelude.warp")),
	("memory", include_str!("../lib/memory.warp")),
	("net", include_str!("../lib/net.warp")),
	("collections", include_str!("../lib/collections.warp")),
	("hash", include_str!("../lib/hash.warp")),
	("regex", include_str!("../lib/regex.warp")),
	("file", include_str!("../lib/file.warp")),
	("json", include_str!("../lib/json.warp")),
	("os", include_str!("../lib/os.warp")),
	("list", include_str!("../lib/list.warp")),
	("math", include_str!("../lib/math.warp")),
	("text", include_str!("../lib/text.warp")),
	("random", include_str!("../lib/random.warp")),
	("map", include_str!("../lib/map.warp")),
	("time", include_str!("../lib/time.warp")),
	("matrix", include_str!("../lib/matrix.warp")),
	("draw", include_str!("../lib/draw.warp")),
	("markup", include_str!("../lib/markup.warp")),
	("router", include_str!("../lib/router.warp")),
	("wagi", include_str!("../lib/wagi.warp")),
	("i18n", include_str!("../lib/i18n.warp")),
	(UNITS_MODULE, include_str!("../lib/units.warp")),
	(AGENT_MODULE, include_str!("../lib/agent.warp")),
	(SOUND_MODULE, include_str!("../lib/sound.warp")),
];
/// Other names of standard modules (word choices are aliases): `use graphics` is `use draw`
const STD_MODULE_ALIASES: [(&str, &str); 1] = [("graphics", "draw")];
/// The standard modules' folder (P194: std/ merged into lib/), embedded in the binary
const STD_FOLDER: &str = "lib";
/// lib/prelude.warp: the warp-written words of every program, loaded when mentioned (Loader::use_prelude)
const PRELUDE_MODULE: &str = "prelude";
/// Between a module's name and its word in a qualified definition name, as between a nested function's outer and own
const QUALIFIER: char = '·';
/// P171: module words a program calls without their `use` (the prelude); only these definitions come along
const PRELUDE_WORDS: [(&str, &[&str]); 2] = [
	("file", &["write", "exists", "write_bytes", "read_bytes"]),
	("os", &["exec"]),
];
const FILE_URL_PREFIX: &str = "file://";
/// lib/units.warp: quantities whose unit is known only at run time, `quantity("5 km")` (notes/units_runtime.md)
const UNITS_MODULE: &str = "units";
const QUANTITY: &str = "quantity";
/// A served route's path parameters (`post "/todos/:id:int/toggle"`, lowering/serve.rs) are read by the router's words
const ROUTE_PARTS: [&str; 3] = ["route_segment", "route_parameter", "route_matches"];
/// lib/agent.warp: `agent "prompt"` asks Claude (card g_X_F0)
const AGENT_MODULE: &str = "agent";
/// lib/sound.warp: `play 440Hz for 0.5s`, `beep` (card basic-sound), loaded when a program calls one of its words
const SOUND_MODULE: &str = "sound";
const SOUND_WORDS: [&str; 17] = [
	"pattern",
	"live_loop",
	"play",
	"melody",
	"tone",
	"beep",
	"play_file",
	"stop_sound",
	"sound_queued",
	"wait_sound",
	"render_sound",
	"midi_send",
	"midi_outputs",
	"midi_receive",
	"midi_on",
	"write_midi",
	"read_midi",
];
/// The sound words every program plays without `use sound` (card basic-sound); the module's rarer words need it (card
/// bizarre-global, P171), also when these words bring the module along
const SOUND_SETUP_FREE_WORDS: [&str; 4] = ["play", "beep", "stop_sound", "wait_sound"];
/// Whether a program needs a module
type NeededBy = fn(&Node) -> bool;
/// The standard modules a program needs without `use`: P183 a file URL → file, a page → markup (lowering/page_html.rs),
/// routes or a server route's path parameters → router, a route's regular expression → regex (lowering/routes.rs), a call of quantity → units (run-time units),
/// a call of agent → agent, a call of play, beep, stop_sound or wait_sound → sound, a WAGI build (`warp build --wagi`) → wagi
const IMPLICIT_MODULES: [(&str, NeededBy); 8] = [
	("file", mentions_file_url),
	("markup", |_| crate::pipeline::compile_mode().renders_itself()),
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
	(SOUND_MODULE, |program| {
		first_called_undefined(program, SOUND_SETUP_FREE_WORDS).is_some()
	}),
	("wagi", |_| crate::pipeline::compile_mode().for_wagi),
];

/// Whether the program plays sound: it calls a sound word or says `use sound` (lowering/music_words.rs reads its note names)
pub fn uses_sound(program: &Node) -> bool {
	first_called_undefined(program, SOUND_WORDS).is_some()
		|| statements_of(program)
			.iter()
			.filter_map(used_module)
			.any(|used| used.name == SOUND_MODULE)
}

/// A rare sound word the program calls: without `use sound` its error names the `use` (card bizarre-global)
fn rare_sound_word(program: &Node) -> Option<&'static str> {
	first_called_undefined(
		program,
		SOUND_WORDS
			.into_iter()
			.filter(|word| !SOUND_SETUP_FREE_WORDS.contains(word)),
	)
}

/// Whether the program calls `word` without defining it
fn calls_undefined(program: &Node, word: &str) -> bool {
	first_called_undefined(program, [word]).is_some()
}

/// The first of `words` the program calls without defining it
fn first_called_undefined<'a>(program: &Node, words: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
	let called = called_names(statements_of(program));
	words
		.into_iter()
		.find(|word| called.contains(*word) && defined(program, word).is_none())
}
/// Other languages' names of the standard modules' classes and words (Java, Python, Rust, C#), each read as warp's with
/// a note, when a used module defines that word
const STD_ALIASES: [(&str, &str); 27] = [
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

fn is_builtin_library(name: &str) -> bool {
	matches!(crate::ffi::resolve_library_alias(name), "m" | "c")
}

#[derive(Clone, Copy, PartialEq)]
enum Import {
	/// the declarations of the file
	Use,
	/// the whole file
	Include,
}

struct Used {
	import: Import,
	name: String,
	requirement: Option<Requirement>,
}

/// The words of a statement, commas flattened: the parser groups `use list` before the comma of `use list, text`
fn statement_words(node: &Node) -> Option<Vec<&Node>> {
	match node.drop_meta() {
		Node::List(items, _, Separator::Colon) => match items.split_first()?.0.drop_meta() {
			Node::List(head, _, _) => Some(head.iter().chain(&items[1..]).collect()),
			_ => None,
		},
		Node::List(items, _, _) => Some(items.iter().collect()),
		_ => None,
	}
}

/// The modules a `use` statement names, each with the statement that uses it alone: `use list, text` and
/// `use list text` (card std-use) use each; `use js Math` is a foreign module (lowering/foreign_modules.rs), not two
fn used_modules(node: &Node) -> Option<Vec<(Used, Node)>> {
	if let Some(used) = used_module(node) {
		return Some(vec![(used, node.clone())]);
	}
	let words = statement_words(node)?;
	let [keyword, names @ ..] = words.as_slice() else {
		return None;
	};
	let plain_name = |name: &&Node| matches!(name.drop_meta(), Node::Symbol(word) if !crate::foreign_modules::FOREIGN_RUNTIMES.contains(&word.as_str()) && !crate::foreign_modules::CRATE_RUNTIMES.contains(&word.as_str()) && ![MINIMUM_KEYWORD, "as", "from"].contains(&word.as_str()) && !is_version_keyword(name));
	if names.len() < 2 || !names.iter().all(plain_name) {
		return None;
	}
	names
		.iter()
		.map(|name| {
			let statement = Node::List(
				vec![(*keyword).clone(), (*name).clone()],
				Bracket::None,
				Separator::Space,
			);
			used_module(&statement).map(|used| (used, statement))
		})
		.collect()
}

/// `from list import zip, unique`: the module, the statement that uses it, and the only words it brings (card std-import)
fn imported_words(node: &Node) -> Option<(Used, Node, Vec<String>)> {
	let words = statement_words(node)?;
	match words.as_slice() {
		[from, module, import, names @ ..] if from.is_symbol(MINIMUM_KEYWORD) && import.is_symbol(IMPORT_KEYWORD) => {
			words_from(module, names)
		}
		[import, names @ .., from, module] if import.is_symbol(IMPORT_KEYWORD) && from.is_symbol(MINIMUM_KEYWORD) => {
			std_words_from(module, names)
		}
		_ => None,
	}
}

/// `import square from math`, `import (zip, unique) from list`: the from-import of a standard module defining every
/// named word; any other `import f from lib` stays the foreign import (analyzer/imports.rs: `import sqrt from math` is libm's)
fn std_words_from(module: &Node, names: &[&Node]) -> Option<(Used, Node, Vec<String>)> {
	let Node::Symbol(module_name) = module.drop_meta() else {
		return None;
	};
	let defined = std_module_definitions(std_module_name(module_name))?;
	let imported = words_from(module, names)?;
	imported.2.iter().all(|name| defined.contains(name)).then_some(imported)
}

/// The module `use`d and the words named, alone or in one group: `zip, unique`, `(zip, unique)`
fn words_from(module: &Node, names: &[&Node]) -> Option<(Used, Node, Vec<String>)> {
	let names = match names {
		[group] if matches!(group.drop_meta(), Node::List(_, Bracket::Round, _)) => group.drop_meta().iter().collect(),
		_ => names.iter().map(|name| (*name).clone()).collect::<Vec<Node>>(),
	};
	let names = names
		.iter()
		.map(|name| match name.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		})
		.collect::<Option<Vec<String>>>()
		.filter(|names| !names.is_empty())?;
	let statement = Node::List(
		vec![Node::Symbol(USE_KEYWORD.into()), (*module).clone()],
		Bracket::None,
		Separator::Space,
	);
	used_module(&statement).map(|used| (used, statement, names))
}

/// `use name`, `require name`, `import name` and `include name`: the name is a symbol, a text or a path `lib/name`, `name.warp`.
/// A version may follow: `use name version 1.2.3` exactly that one, `use name from 1.2.3` / `use name >= 1.2.3` that or later
fn used_module(node: &Node) -> Option<Used> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let [keyword, argument, rest @ ..] = items.as_slice() else {
		return None;
	};
	let Node::Symbol(keyword) = keyword.drop_meta() else {
		return None;
	};
	let import = if USE_KEYWORDS.contains(&keyword.as_str()) {
		Import::Use
	} else if keyword == INCLUDE_KEYWORD {
		Import::Include
	} else {
		return None;
	};
	let (name, requirement) = match (argument.drop_meta(), rest) {
		(Node::Key(name, Op::Ge, minimum), []) => (path_of(name)?, Some(Requirement::Minimum(version_of(minimum)?))),
		(name, []) => (path_of(name)?, None),
		(name, [version]) if version_of(version).is_some() && is_version_list(version) => {
			(path_of(name)?, Some(Requirement::Exact(version_of(version)?)))
		}
		(name, [word, version]) if word.is_symbol(MINIMUM_KEYWORD) => {
			(path_of(name)?, Some(Requirement::Minimum(version_of(version)?)))
		}
		(name, [word, version]) if is_version_keyword(word) => {
			(path_of(name)?, Some(Requirement::Exact(version_of(version)?)))
		}
		_ => return None,
	};
	Some(Used {
		import,
		name,
		requirement,
	})
}

/// `version 1.2.3`, as the parser binds it
fn is_version_list(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(is_version_keyword))
}

pub(crate) fn path_of(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		Node::Key(directory, Op::Div, name) => Some(format!("{}/{}", path_of(directory)?, path_of(name)?)),
		Node::Key(name, Op::Dot, extension) => Some(format!("{}.{}", path_of(name)?, path_of(extension)?)),
		_ => None,
	}
}

/// A path naming its file itself rather than a name to search for: absolute, or starting at `./` or `../`
fn is_explicit_path(name: &str) -> bool {
	Path::new(name).is_absolute() || name.starts_with("./") || name.starts_with("../")
}

/// A relative path that stays below its directory
fn is_plain_relative_path(name: &str) -> bool {
	Path::new(name)
		.components()
		.all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn mentions_file_url(program: &Node) -> bool {
	let mut file_url = false;
	program.visit(&mut |node| {
		if let Node::Text(text) = node {
			file_url |= text.starts_with(FILE_URL_PREFIX)
		}
	});
	file_url
}

/// The program's main-level definition of `name`
fn defined(program: &Node, name: &str) -> Option<Node> {
	statements_of(program)
		.iter()
		.find(|statement| declared_name(statement).as_deref() == Some(name))
		.cloned()
}

/// The statements of a module, borrowed (statements takes them)
fn statements_of(module: &Node) -> &[Node] {
	match module.drop_meta() {
		Node::Empty => &[],
		plain => plain.sequence_statements(),
	}
}

fn statements(module: Node) -> Vec<Node> {
	match module {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items,
		Node::Meta { node, .. } => statements(*node),
		Node::Empty => vec![],
		single => vec![single],
	}
}

pub(crate) fn is_declaration(statement: &Node) -> bool {
	match statement.drop_meta() {
		Node::Type { .. } => true,
		Node::Key(_, Op::Assign | Op::Define, _) => true,
		Node::Key(scope, Op::Colon, _) => {
			matches!(scope.drop_meta(), Node::Symbol(keyword) if is_declaration_keyword(keyword))
		}
		Node::List(items, _, _) if items.len() >= 2 => {
			matches!(items[0].drop_meta(), Node::Symbol(keyword) if is_function_keyword(keyword) || is_declaration_keyword(keyword))
		}
		_ => false,
	}
}

/// The name a declaration defines: `f(x) := …`, `x = …`, `x:int = 1`, `fun f(x) {…}`, `global x = …`, `class T {…}`
fn declared_name(statement: &Node) -> Option<String> {
	if !is_declaration(statement) {
		return None; // a list `[a, b]` declares nothing
	}
	match statement.drop_meta() {
		Node::Type { name, .. } => leftmost_symbol(name),
		Node::Key(target, Op::Assign | Op::Define, _) => leftmost_symbol(target),
		// `global x = 0`, as the parser reads it: `global: (x = 0)`
		Node::Key(keyword, Op::Colon, declared) if matches!(keyword.drop_meta(), Node::Symbol(keyword) if is_declaration_keyword(keyword)) => {
			declared_name(declared)
		}
		Node::List(items, _, _) if items.len() >= 2 => leftmost_symbol(&items[1]),
		_ => None,
	}
}

fn leftmost_symbol(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::List(items, _, _) => leftmost_symbol(items.first()?),
		Node::Key(left, Op::Colon | Op::Assign | Op::Define, _) => leftmost_symbol(left),
		_ => None,
	}
}

/// Words that bind the names after them: `fun f(x)`, `for i in`, `import f from`, `global x`
fn binds_names(keyword: &str) -> bool {
	is_function_keyword(keyword) || is_declaration_keyword(keyword) || keyword == crate::words::FOR_WORD
}

/// The signature part of a definition: `f(x)` of `f(x): body` and of `fun f(x) {body}`
fn header(definition: &Node) -> &Node {
	match definition.drop_meta() {
		Node::Key(header, Op::Colon | Op::Assign | Op::Define, _) => header,
		Node::List(items, _, _)
			if items
				.first()
				.is_some_and(|first| matches!(first.drop_meta(), Node::List(..))) =>
		{
			&items[0]
		}
		_ => definition,
	}
}

/// Every name the program binds anywhere: variables, functions, parameters, loop variables, imports.
/// Generous on purpose: a bound name never comes from a sibling
fn bound_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(target, op, value)
			if matches!(op, Op::Assign | Op::Define | Op::FatArrow) || op.is_compound_assign() =>
		{
			names.extend(mentioned_names(std::slice::from_ref(target.as_ref())));
			bound_names(value, names);
		}
		Node::Key(left, _, right) => {
			bound_names(left, names);
			bound_names(right, names);
		}
		Node::List(items, _, _) => {
			if let [keyword, bound, ..] = items.as_slice() {
				if matches!(keyword.drop_meta(), Node::Symbol(word) if binds_names(word)) {
					names.extend(mentioned_names(std::slice::from_ref(header(bound))));
				}
			}
			items.iter().for_each(|item| bound_names(item, names));
		}
		Node::Type { name, body } => {
			bound_names(name, names);
			names.extend(leftmost_symbol(name));
			bound_names(body, names);
		}
		_ => {}
	}
}

fn same_definitions(first: &[Node], second: &[Node]) -> bool {
	first.len() == second.len() && first.iter().zip(second).all(|(a, b)| a.serialize() == b.serialize())
}

/// Every word the statements mention
/// The methods called in statements: `f` of `x.f(…)` and of `x.f`
fn method_names(statements: &[Node]) -> Vec<String> {
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
fn template_holes(statements: &[Node]) -> Vec<Node> {
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

fn mentioned_names(statements: &[Node]) -> Vec<String> {
	// a function's parameters are its own: `f(x, melody) := x + melody` mentions no module word melody
	fn collect(node: &Node, names: &mut Vec<String>) {
		let parameters: Vec<String> = crate::lowering::nodes::function_head(node)
			.map(|function| function.parameters())
			.unwrap_or_default()
			.into_iter()
			.filter_map(crate::lowering::nodes::parameter_name)
			.collect();
		if parameters.is_empty() {
			return collect_parts(node, names);
		}
		let mut mentioned = vec![];
		collect_parts(node, &mut mentioned);
		names.extend(mentioned.into_iter().filter(|name| !parameters.contains(name)));
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

/// `require x` and `import x` of a native library stay in the program as `use x`
fn as_use(statement: &Node) -> Node {
	match statement {
		Node::List(items, bracket, separator) if items.len() == 2 => Node::List(
			vec![Node::Symbol(USE_KEYWORD.to_string()), items[1].clone()],
			bracket.clone(),
			separator.clone(),
		),
		other => other.clone(),
	}
}

/// A module's path as failures name it: without `./` steps (`./././greeting.warp` is `greeting.warp`)
fn shown_path(path: &Path) -> PathBuf {
	path.components()
		.filter(|component| !matches!(component, std::path::Component::CurDir))
		.collect()
}
