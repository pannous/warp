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
pub mod binaryen;
#[cfg(feature = "native")]
pub mod cargo_crates;
#[cfg(feature = "native")]
pub mod component_builder;
#[cfg(feature = "native")]
pub mod components;
#[cfg(feature = "native")]
pub mod foreign;
#[cfg(feature = "native")]
pub mod package_tools;
mod packages;
#[cfg(feature = "native")]
pub mod std_adapters;
pub mod std_docs;
pub mod wasm_modules;
pub use packages::*;
mod declarations;
pub(crate) use declarations::*;
mod implicit_modules;
pub use implicit_modules::*;
mod needed_definitions;
use needed_definitions::*;
mod program_file;
pub use program_file::*;
mod use_statements;
pub(crate) use use_statements::*;

/// Where modules are looked for, in this order: the current directory, the samples, the library (lib/extra: modules
/// that are not standard, like netbase, P194)
pub const SEARCH_DIRECTORIES: [&str; 7] = [".", "include", "lib", "lib/extra", "src", "source", "samples"];
pub const MODULE_EXTENSIONS: [&str; 2] = ["wasp", "warp"];

/// Replace every `use <module>` of the program by the definitions of that module, each module once.
/// `use math` and other native libraries stay in place for the FFI; an unknown name is an error value.
pub fn resolve(program: Node) -> Node {
	resolve_in(program, &SEARCH_DIRECTORIES)
}

pub fn resolve_in(program: Node, directories: &[&str]) -> Node {
	crate::compiler::compile_context::change(|context| context.module_definitions.clear());
	let own_names: Vec<String> = statements_of(&program).iter().filter_map(name_declared_by).collect();
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
		.map(|program| crate::modules::wasm_modules::rewrite_uses(program, &loader.wasm_modules))
		.map(|program| with_needed_definitions(program, loader.hidden_apart()));
	let files = loader.loaded.iter().chain(&loader.included).filter(|path| path.is_file()).cloned().collect();
	crate::compiler::compile_context::change(|context| {
		context.module_files = files;
		context.early_class_modules.clear(); // for this program only
		own_names.iter().for_each(|own| {
			context.module_definitions.remove(own); // the program's own word wins
		});
	});
	resolved.unwrap_or_else(|failure| failure)
}

/// The modules a program `use`s and the implicit ones it needs (`play` → sound): their classes go in front alike, the
/// module itself stays a used module (its note names, its settings); units moves whole (with_implicit_class_modules)
fn used_and_implicit_modules(program: &Node) -> Vec<String> {
	let mut names: Vec<String> = statements_of(program)
		.iter()
		.filter_map(used_module)
		.filter(|used| used.import == Import::Use)
		.map(|used| used.name)
		.collect();
	for (module, needed) in IMPLICIT_MODULES {
		if !names.iter().any(|name| name == module) && !MODULES_COMPILED_AS_PROGRAM.contains(&module) && needed(program)
		{
			names.push(module.to_string())
		}
	}
	names
}

/// The classes of the modules the program uses, in front of it: those of a file (`use shapes`) all, those of a standard
/// module the program names (`use collections; s = Stack()`). class_methods, which turns a class's methods into
/// functions and its method calls into theirs, runs before `resolve` loads a module's other definitions; the loader
/// then leaves these modules' classes out (early_class_modules). A class the program declares itself wins.
pub fn insert_module_classes(program: Node) -> Node {
	let mut program = with_type_module_uses(program);
	let file = program_file();
	let loader = Loader::new(&SEARCH_DIRECTORIES, file.as_deref().map(folder_of));
	let own_names: HashSet<String> = statements_of(&program).iter().filter_map(name_declared_by).collect();
	let (mut std_definitions, mut file_classes, mut early, mut class_modules) =
		(vec![], vec![], HashSet::new(), vec![]);
	let mut defined: Vec<String> = vec![]; // every used module's words, a file's copy of a standard module's too
	for name in used_and_implicit_modules(&program) {
		if let Some(module) = loader
			.find(&name)
			.is_none()
			.then(|| loader.find_with(&name, &crate::modules::wasm_modules::MODULE_EXTENSIONS))
			.flatten()
		{
			let module = module.to_string_lossy();
			let classes = crate::modules::wasm_modules::classes(&module);
			let names: Vec<String> = classes.iter().filter_map(name_declared_by).collect();
			program = crate::modules::wasm_modules::with_bare_classes(
				program,
				&crate::modules::wasm_modules::module_alias(&module),
				&names,
			);
			file_classes.extend(classes);
			continue;
		}
		let (path, source, is_std) = match loader.find(&name) {
			Some(path) => match crate::web::read_text(&path.to_string_lossy()) {
				Some(source) => (path, source, false),
				None => continue,
			},
			None => match std_module(&name) {
				Some(source) => (std_path(std_module_name(&name)), source.to_string(), true),
				None => continue,
			},
		};
		// a look for classes only: the loader's own read of a file module hints it, a standard module never
		let module = crate::normalize::without_hints(|| WarpParser::parse(&source));
		if module.first_error().is_some() {
			continue; // the loader reports it
		}
		let definitions = statements(module);
		defined.extend(definitions.iter().filter_map(name_declared_by));
		match is_std {
			true => {
				if definitions.iter().any(is_class) {
					class_modules.push(name.clone());
				}
				std_definitions.extend(definitions)
			}
			false => file_classes.extend(definitions.into_iter().filter(is_class)),
		}
		early.insert(path.canonicalize().unwrap_or(path));
	}
	crate::compiler::compile_context::change(|context| context.early_class_modules = early);
	let is_foreign = |class: &Node| name_declared_by(class).is_none_or(|name| !own_names.contains(&name));
	let aliases: Vec<(&str, &str)> = STD_ALIASES
		.into_iter()
		.filter(|(alias, word)| defined.iter().any(|name| name == word) && !own_names.contains(*alias))
		.collect();
	// `collections.Counter(xs)`: the bare class before class_methods reads its construction; other qualified calls
	// (`math.factorial` of `use python math`, `json.loads`) wait for foreign_modules and welcome_forms
	let class_modules: Vec<&str> = class_modules.iter().map(String::as_str).collect();
	let program = with_std_aliases(crate::welcome_forms::module_calls(program, &class_modules), &aliases);
	// the classes the program's calls reach, also through the module's functions (sound's sample() makes a Sample)
	let std_classes: Vec<Node> = needed_definitions(&program, &std_definitions)
		.into_iter()
		.filter(|definition| is_class(definition) && is_foreign(definition))
		.collect();
	let class_names: Vec<String> = std_classes.iter().filter_map(name_declared_by).collect();
	let module_code = Node::List(std_definitions, Bracket::None, Separator::Newline);
	let returned = crate::lowering::class_methods::returned_classes(&module_code, &class_names);
	crate::compiler::compile_context::change(|context| context.module_returned_classes = returned);
	let file_classes: Vec<Node> = file_classes.into_iter().filter(is_foreign).collect();
	let program = with_implicit_class_modules(with_statements_in_front(program, std_classes));
	match file_classes.is_empty() {
		true => program,
		false => match program {
			Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
				Node::List([file_classes, statements].concat(), Bracket::None, separator)
			}
			single => Node::List([file_classes, vec![single]].concat(), Bracket::None, Separator::Semicolon),
		},
	}
}
