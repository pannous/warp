//! `use <name>` file modules: `name.wasp` or `name.warp` is loaded and its definitions become part of the program.
//! A name found nowhere locally but in the registry packages.wasp is a package: its git repository, fetched into packages/<name>.
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::versions::{declared_version, is_version_keyword, version_of, Requirement, Version, MINIMUM_KEYWORD};
use crate::wasp_parser::WaspParser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// Where modules are looked for, in this order: the current directory, the samples, the library (lib/extra: modules
/// that are not standard, like netbase, P194)
pub const SEARCH_DIRECTORIES: [&str; 7] = [".", "include", "lib", "lib/extra", "src", "source", "samples"];
pub const MODULE_EXTENSIONS: [&str; 2] = ["wasp", "warp"];
/// `use x`, and its aliases `require x` and `import x`: the declarations of the file x
pub(crate) const USE_KEYWORDS: [&str; 3] = ["use", "require", "import"];
/// `include x`: the whole file, spliced in place
const INCLUDE_KEYWORD: &str = "include";
const USE_KEYWORD: &str = "use";
/// name: "git url" for every package, compiled in so warp finds it from any directory
const PACKAGE_REGISTRY: &str = include_str!("../packages.wasp");
/// the directory of the file it is written in, as text: `read(module_directory + "/data/x")` finds a module's own files
const MODULE_DIRECTORY: &str = "module_directory";
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
const SCOPE_WORDS: [(&str, Scope); 3] = [("folder", Scope::Folder), ("package", Scope::Package), ("project", Scope::Project)];
/// Folders a package or project scope never looks into, besides hidden ones and nested repositories
const UNSCOPED_DIRECTORIES: [&str; 3] = [PACKAGES_DIRECTORY, "target", "node_modules"];
const PROJECT_MARKER: &str = ".git";
/// `stored x = v` too: a used module's persisted signal is the program's (card web-stores)
const DECLARATION_KEYWORDS: [&str; 6] = ["use", "import", "let", "var", "global", crate::stored_values::STORED_WORD];

fn is_declaration_keyword(keyword: &str) -> bool {
	DECLARATION_KEYWORDS.contains(&keyword) || crate::analyzer::CONSTANT_KEYWORDS.contains(&keyword)
}

/// Replace every `use <module>` of the program by the definitions of that module, each module once.
/// `use math` and other native libraries stay in place for the FFI; an unknown name is an error value.
pub fn resolve(program: Node) -> Node {
	resolve_in(program, &SEARCH_DIRECTORIES)
}

pub fn resolve_in(program: Node, directories: &[&str]) -> Node {
	MODULE_DEFINITIONS.with(|names| names.borrow_mut().clear());
	let own_names: Vec<String> = statements(program.clone()).iter().filter_map(declared_name).collect();
	let file = PROGRAM_FILE.with(|current| current.borrow().clone());
	let mut loader = Loader::new(directories, file.as_deref().map(folder_of));
	let folder = loader.including_directory.clone();
	let program = with_module_directory(program, folder.as_deref().unwrap_or(Path::new(".")));
	let resolved = loader.resolve(program).and_then(|program| loader.implicit_std_modules(&program).map(|_| program)).and_then(|program| match loader.scope {
		Some(scope) => loader.with_scope(program, scope, file.as_deref()),
		None => Ok(program),
	}).map(|program| crate::wasm_modules::rewrite_uses(program, &loader.wasm_modules))
		.map(|program| with_needed_definitions(program, loader.std_definitions));
	EARLY_CLASS_MODULES.with(|modules| modules.borrow_mut().clear()); // for this program only
	MODULE_DEFINITIONS.with(|names| own_names.iter().for_each(|own| { names.borrow_mut().remove(own); })); // the program's own word wins
	resolved.unwrap_or_else(|failure| failure)
}

/// The classes of the modules the program uses, in front of it: those of a file (`use shapes`) all, those of a standard
/// module the program names (`use collections; s = Stack()`). class_methods, which turns a class's methods into
/// functions and its method calls into theirs, runs before `resolve` loads a module's other definitions; the loader
/// then leaves these modules' classes out (EARLY_CLASS_MODULES). A class the program declares itself wins.
pub fn insert_module_classes(program: Node) -> Node {
	let file = PROGRAM_FILE.with(|current| current.borrow().clone());
	let loader = Loader::new(&SEARCH_DIRECTORIES, file.as_deref().map(folder_of));
	let own_names: HashSet<String> = statements(program.clone()).iter().filter_map(declared_name).collect();
	let (mut std_definitions, mut file_classes, mut early, mut class_modules) = (vec![], vec![], HashSet::new(), vec![]);
	let mut defined: Vec<String> = vec![]; // every used module's words, a file's copy of a standard module's too
	for used in statements(program.clone()).iter().filter_map(used_module).filter(|used| used.import == Import::Use) {
		let (path, source, is_std) = match loader.find(&used.name) {
			Some(path) => match crate::web::read_text(&path.to_string_lossy()) {
				Some(source) => (path, source, false),
				None => continue,
			},
			None => match std_module(&used.name) {
				Some(source) => (std_path(&used.name), source.to_string(), true),
				None => continue,
			},
		};
		// a look for classes only: the loader's own read of a file module hints it, a standard module never
		let module = crate::normalize::without_hints(|| WaspParser::parse(&source));
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
	EARLY_CLASS_MODULES.with(|modules| *modules.borrow_mut() = early);
	let is_foreign = |class: &Node| declared_name(class).is_none_or(|name| !own_names.contains(&name));
	let aliases: Vec<(&str, &str)> = STD_ALIASES.into_iter().filter(|(alias, word)| defined.iter().any(|name| name == word) && !own_names.contains(*alias)).collect();
	// `collections.Counter(xs)`: the bare class before class_methods reads its construction; other qualified calls
	// (`math.factorial` of `use python math`, `json.loads`) wait for foreign_modules and welcome_forms
	let class_modules: Vec<&str> = class_modules.iter().map(String::as_str).collect();
	let program = with_std_aliases(crate::welcome_forms::module_calls(program, &class_modules), &aliases);
	let std_classes = std_definitions.into_iter().filter(|definition| is_class(definition) && is_foreign(definition)).collect();
	let file_classes: Vec<Node> = file_classes.into_iter().filter(is_foreign).collect();
	let program = with_needed_definitions(program, std_classes);
	match file_classes.is_empty() {
		true => program,
		false => match program {
			Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => Node::List([file_classes, statements].concat(), Bracket::None, separator),
			single => Node::List([file_classes, vec![single]].concat(), Bracket::None, Separator::Semicolon),
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
fn with_needed_definitions(program: Node, definitions: Vec<Node>) -> Node {
	if definitions.is_empty() {
		return program;
	}
	let mut needed: Vec<Node> = vec![];
	// a standard word called as a method (`"hé".to_utf8()`) is needed too
	let mentioned_or_called = |statements: &[Node]| mentioned_names(statements).into_iter().chain(method_names(statements));
	let mut mentioned: HashSet<String> = mentioned_or_called(std::slice::from_ref(&program)).collect();
	// the program's own variable `words = […]` wins over the module's words(t)
	let own: HashSet<String> = statements(program.clone()).iter().filter_map(|statement| match statement.drop_meta() {
		Node::Key(variable, Op::Assign, _) => match variable.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}).collect();
	loop {
		let now_needed: Vec<Node> = definitions.iter().filter(|definition| declared_name(definition).is_some_and(|name| mentioned.contains(&name) && !own.contains(&name))).cloned().collect();
		if now_needed.len() == needed.len() {
			break;
		}
		mentioned.extend(mentioned_or_called(&now_needed));
		needed = now_needed;
	}
	if needed.is_empty() {
		return program;
	}
	let (statements, separator) = match program {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => (statements, separator),
		single => (vec![single], Separator::Semicolon),
	};
	Node::List([needed, statements].concat(), Bracket::None, separator)
}


thread_local! {
	/// The file being compiled; its folder is in scope (D15). None for inline code
	static PROGRAM_FILE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
	/// The modules whose classes insert_module_classes put in front of the program: the loader leaves them out
	static EARLY_CLASS_MODULES: std::cell::RefCell<HashSet<PathBuf>> = std::cell::RefCell::new(HashSet::new());
	/// The names a used module defines for the program being compiled: their bodies never see the program's variables
	static MODULE_DEFINITIONS: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
}

/// Whether `name` is defined by a module the program uses (`use markup`'s html_element), not by the program
pub fn is_module_definition(name: &str) -> bool {
	MODULE_DEFINITIONS.with(|names| names.borrow().contains(name))
}

/// `path` as written in the program: a relative path is next to the program's file, or in the working directory for
/// inline code
pub fn beside_program(path: &str) -> String {
	let folder = PROGRAM_FILE.with(|current| current.borrow().as_deref().map(folder_of));
	match folder {
		Some(folder) if Path::new(path).is_relative() => folder.join(path).to_string_lossy().into_owned(),
		_ => path.to_string(),
	}
}

/// The file being compiled, None for inline code
pub fn program_file() -> Option<PathBuf> {
	PROGRAM_FILE.with(|current| current.borrow().clone())
}

/// Compile `body` as the program of `file`: `use folder`, `use package` and `use project` start from its folder
pub fn with_program_file<R>(file: &Path, body: impl FnOnce() -> R) -> R {
	let previous = PROGRAM_FILE.with(|current| current.replace(Some(file.to_path_buf())));
	let result = body();
	PROGRAM_FILE.with(|current| *current.borrow_mut() = previous);
	result
}

fn folder_of(file: &Path) -> PathBuf {
	match file.parent() {
		Some(folder) if !folder.as_os_str().is_empty() => folder.to_path_buf(),
		_ => PathBuf::from("."),
	}
}

/// Another module file of the program's folder, parsed only when its text mentions a name the program needs
struct Sibling {
	path: PathBuf,
	source: String,
	statements: Option<Vec<Node>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Scope {
	/// the files of the program's folder
	Folder,
	/// every file below the package folder: the nearest folder holding `<folder name>.wasp`
	Package,
	/// every file below the project root: the nearest folder holding .git
	Project,
}

fn scope_word(name: &str) -> Option<Scope> {
	SCOPE_WORDS.iter().find(|(word, _)| *word == name).map(|(_, scope)| *scope)
}

/// The other module files of the scope around the program (the working directory for inline code)
fn scope_files(scope: Scope, file: Option<&Path>) -> Result<Vec<Sibling>, Node> {
	let folder = file.map(folder_of).unwrap_or_else(|| PathBuf::from("."));
	let mut paths = Vec::new();
	match scope {
		Scope::Folder => collect_module_files(&folder, false, &mut paths),
		Scope::Package => collect_module_files(&package_root(&folder)?, true, &mut paths),
		Scope::Project => collect_module_files(&project_root(&folder)?, true, &mut paths),
	}
	let itself = file.and_then(|file| file.canonicalize().ok());
	paths.retain(|path| path.canonicalize().ok() != itself);
	paths.sort();
	Ok(paths.into_iter().filter_map(|path| Some(Sibling { source: std::fs::read_to_string(&path).ok()?, path, statements: None })).collect())
}

fn is_module_file(path: &Path) -> bool {
	path.is_file() && path.extension().is_some_and(|extension| MODULE_EXTENSIONS.iter().any(|known| extension == *known))
}

fn collect_module_files(directory: &Path, below: bool, paths: &mut Vec<PathBuf>) {
	let Ok(entries) = std::fs::read_dir(directory) else { return };
	for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
		if is_module_file(&path) {
			paths.push(path);
		} else if below && path.is_dir() && is_scoped_directory(&path) {
			collect_module_files(&path, below, paths);
		}
	}
}

/// Hidden folders, fetched packages, build output and nested repositories belong to no scope
fn is_scoped_directory(directory: &Path) -> bool {
	let name = directory.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
	!name.starts_with('.') && !UNSCOPED_DIRECTORIES.contains(&name.as_str()) && !directory.join(PROJECT_MARKER).exists()
}

fn package_root(folder: &Path) -> Result<PathBuf, Node> {
	let absolute = folder.canonicalize().unwrap_or_else(|_| folder.to_path_buf());
	let holds_its_module = |directory: &&Path| {
		let name = directory.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
		MODULE_EXTENSIONS.iter().any(|extension| directory.join(format!("{name}.{extension}")).is_file())
	};
	absolute.ancestors().find(holds_its_module).map(Path::to_path_buf)
		.ok_or_else(|| error(&format!("use package: no package folder around {}: a package folder holds <folder name>.wasp", folder.display())))
}

fn project_root(folder: &Path) -> Result<PathBuf, Node> {
	let absolute = folder.canonicalize().unwrap_or_else(|_| folder.to_path_buf());
	absolute.ancestors().find(|directory| directory.join(PROJECT_MARKER).exists()).map(Path::to_path_buf)
		.ok_or_else(|| error(&format!("use project: no project root around {}: the project root holds {PROJECT_MARKER}", folder.display())))
}

impl Sibling {
	/// The statements of the file; a file that does not parse is reported and contributes nothing
	fn statements(&mut self) -> Result<&[Node], Node> {
		if self.statements.is_none() {
			let module = WaspParser::parse(&self.source);
			let parsed = match module.first_error() {
				Some(failure) => {
					let warning = crate::diagnostic::Diagnostic::at(&module, format!("folder scope skips {}: {}", self.path.display(), failure.serialize()));
					crate::diagnostic::report(&[warning])?;
					vec![]
				}
				None => statements(with_module_directory(module, &folder_of(&self.path))),
			};
			self.statements = Some(parsed);
		}
		Ok(self.statements.as_deref().unwrap_or_default())
	}

	fn declarations_of(&mut self, name: &str) -> Result<Vec<Node>, Node> {
		if !self.source.contains(name) {
			return Ok(vec![]);
		}
		Ok(self.statements()?.iter().filter(|statement| is_declaration(statement) && declared_name(statement).as_deref() == Some(name)).cloned().collect())
	}
}

fn with_module_directory(module: Node, directory: &Path) -> Node {
	crate::library_words::substitute(module, MODULE_DIRECTORY, &Node::Text(directory.display().to_string()))
}

struct Loader<'a> {
	directories: &'a [&'a str],
	/// files whose declarations a `use` brought in, and files an `include` spliced in: each is loaded once
	loaded: HashSet<PathBuf>,
	included: HashSet<PathBuf>,
	/// the directory of the file being loaded; None for the program itself, which searches from the working directory
	including_directory: Option<PathBuf>,
	/// the widest `use folder|package|project` met
	scope: Option<Scope>,
	/// the paths of the imported WebAssembly modules, whose exports the program reads (wasm_modules::rewrite_uses)
	wasm_modules: Vec<String>,
	/// the definitions of the standard modules used: the program gets those it calls (with_needed_definitions)
	std_definitions: Vec<Node>,
}

impl<'a> Loader<'a> {
	fn new(directories: &'a [&'a str], including_directory: Option<PathBuf>) -> Self {
		Loader { directories, loaded: HashSet::new(), included: HashSet::new(), including_directory, scope: None, wasm_modules: vec![], std_definitions: vec![] }
	}

	fn resolve(&mut self, node: Node) -> Result<Node, Node> {
		if let Some(used) = used_module(&node) {
			return Ok(match self.import(&used, &node)? {
				imported if imported.is_empty() => Node::Empty,
				imported => Node::List(imported, Bracket::None, Separator::Semicolon),
			});
		}
		match node {
			Node::List(items, bracket, separator) => {
				let mut resolved = Vec::with_capacity(items.len());
				for item in items {
					match used_module(&item) {
						Some(used) => resolved.extend(self.import(&used, &item)?),
						None => resolved.push(self.resolve(item)?),
					}
				}
				Ok(Node::List(resolved, bracket, separator))
			}
			Node::Meta { node, data } => Ok(Node::Meta { node: Box::new(self.resolve(*node)?), data }),
			other => Ok(other),
		}
	}

	/// The statements a `use` stands for: the module's definitions, nothing for a module already loaded,
	/// the `use` itself for a native library
	fn import(&mut self, used: &Used, statement: &Node) -> Result<Vec<Node>, Node> {
		let (import, name) = (used.import, used.name.as_str());
		if let Some(scope) = scope_word(name).filter(|_| import == Import::Use && used.requirement.is_none()) {
			self.scope = self.scope.max(Some(scope));
			return Ok(vec![]);
		}
		// `use math`: the C library libm and the standard module math
		if import == Import::Use && is_builtin_library(name) {
			if let Some(source) = std_module(name) {
				self.use_std_module(name, source)?;
			}
			return Ok(vec![as_use(statement)]);
		}
		let Some(path) = self.find(name) else {
			if let Some(source) = std_module(name).filter(|_| import == Import::Use) {
				return self.use_std_module(name, source);
			}
			if let Some(module) = self.find_with(name, &crate::wasm_modules::MODULE_EXTENSIONS) {
				return Ok(self.use_wasm_module(&module, statement, import));
			}
			if import == Import::Use && package_repository(name).is_some() {
				return self.use_package(name, used.requirement.as_ref());
			}
			return match import == Import::Use && crate::ffi::is_ffi_library(name) {
				true => Ok(vec![as_use(statement)]),
				false => Err(self.not_found(name, import)),
			};
		};
		if let Some(requirement) = &used.requirement {
			check_version(name, &path, requirement)?;
		}
		self.load(import, name, path)
	}

	/// A registered package: fetched once, then its module `<name>.wasp` if it has one; a package without is data.
	/// A required version the default branch does not declare comes from the matching git tag.
	fn use_package(&mut self, name: &str, requirement: Option<&Requirement>) -> Result<Vec<Node>, Node> {
		let mut directory = package_directory(name).map_err(|failure| error(&failure))?;
		if let Some(requirement) = requirement {
			let declared = package_module(&directory, name).and_then(|path| module_version(&path));
			if !declared.is_some_and(|version| requirement.allows(&version)) {
				directory = fetch_package_version(name, requirement).map_err(|failure| error(&failure))?;
			}
		}
		match package_module(&directory, name) {
			Some(path) => self.load(Import::Use, name, path),
			None => Ok(vec![]),
		}
	}

	/// The standard modules a program uses without `use`: the file module for a file URL (P183), the prelude words it
	/// mentions (P171); a module the program used is loaded already
	fn implicit_std_modules(&mut self, program: &Node) -> Result<(), Node> {
		let mut mentioned = HashSet::new();
		let mut file_url = false;
		program.visit(&mut |node| match node {
			Node::Symbol(name) => { mentioned.insert(name.clone()); }
			Node::Text(text) => file_url |= text.starts_with(FILE_URL_PREFIX),
			_ => {}
		});
		if file_url {
			self.use_std_module("file", std_module("file").expect("lib/file.wasp is embedded"))?;
		}
		// a program's own `write(x, y)` wins
		let own: HashSet<String> = statements(program.clone()).iter().filter_map(declared_name).collect();
		for (module, words) in PRELUDE_WORDS {
			let words: Vec<&str> = words.iter().copied().filter(|word| mentioned.contains(*word) && !own.contains(*word)).collect();
			if !words.is_empty() {
				let source = std_module(module).expect("a prelude module is embedded");
				let definitions = crate::normalize::without_hints(|| self.load_source(Import::Use, std_path(module), source))?;
				self.std_definitions.extend(definitions.into_iter().filter(|definition| declared_name(definition).is_some_and(|name| words.contains(&name.as_str()))));
			}
		}
		Ok(())
	}

	/// A standard module: its definitions wait aside until the program is resolved; its source shows the program
	/// no style hints
	fn use_std_module(&mut self, name: &str, source: &str) -> Result<Vec<Node>, Node> {
		let definitions = crate::normalize::without_hints(|| self.load_source(Import::Use, std_path(name), source))?;
		self.std_definitions.extend(definitions);
		Ok(vec![])
	}

	fn load(&mut self, import: Import, name: &str, path: PathBuf) -> Result<Vec<Node>, Node> {
		let source = crate::web::read_text(&path.to_string_lossy()).ok_or_else(|| error(&format!("module not readable: {name}")))?;
		self.load_source(import, path, &source)
	}

	/// A module from its source: `path` names it (each is loaded once) and is where its own `use`s look
	fn load_source(&mut self, import: Import, path: PathBuf, source: &str) -> Result<Vec<Node>, Node> {
		let loaded = match import {
			Import::Use => &mut self.loaded,
			Import::Include => &mut self.included,
		};
		if !loaded.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
			return Ok(vec![]);
		}
		let module = WaspParser::parse(source);
		if let Some(failure) = module.first_error() {
			return Err(failure.clone());
		}
		// classes insert_module_classes put in front of the program already: left out before the module's passes lower them
		let classes_early = import == Import::Use && EARLY_CLASS_MODULES.with(|modules| modules.borrow().contains(&path.canonicalize().unwrap_or_else(|_| path.clone())));
		let module = match classes_early {
			true => Node::List(statements(module).into_iter().filter(|statement| !is_class(statement)).collect(), Bracket::None, Separator::Semicolon),
			false => module,
		};
		let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
		let module = with_module_directory(crate::pipeline::lower_module_source(module), &directory);
		let outer_directory = self.including_directory.replace(directory);
		let module = self.resolve(module);
		self.including_directory = outer_directory;
		let statements = statements(module?);
		Ok(match import {
			Import::Use => {
				let declarations: Vec<Node> = statements.into_iter().filter(is_declaration).collect();
				MODULE_DEFINITIONS.with(|names| names.borrow_mut().extend(declarations.iter().filter_map(declared_name)));
				declarations
			}
			Import::Include => statements,
		})
	}

	/// The file of a module: every search directory below the directory of the including file, then below the working
	/// directory; a name that already ends in an extension is tried as written
	fn find(&self, name: &str) -> Option<PathBuf> {
		self.find_with(name, &MODULE_EXTENSIONS)
	}

	/// never the program's own file: hash.wasp saying `use hash` means the standard module; nor warp's own lib/hash.wasp,
	/// which is the embedded standard module itself (found in lib when a program runs in warp's repository)
	fn find_with(&self, name: &str, extensions: &[&str]) -> Option<PathBuf> {
		let program = program_file().and_then(|file| file.canonicalize().ok());
		let is_program = |path: &PathBuf| program.is_some() && path.canonicalize().ok() == program;
		self.candidates_with(name, extensions).into_iter().find(|path| module_exists(path) && !is_program(path) && !is_embedded_std_file(path))
	}

	fn candidates(&self, name: &str) -> Vec<PathBuf> {
		self.candidates_with(name, &MODULE_EXTENSIONS)
	}

	fn candidates_with(&self, name: &str, extensions: &[&str]) -> Vec<PathBuf> {
		if !is_plain_relative_path(name) {
			return vec![];
		}
		let has_extension = extensions.iter().any(|extension| name.ends_with(&format!(".{extension}")));
		let file_names: Vec<String> = match has_extension {
			true => vec![name.to_string()],
			false => extensions.iter().map(|extension| format!("{name}.{extension}")).collect(),
		};
		let bases: Vec<PathBuf> = self.including_directory.iter().cloned().chain(std::iter::once(PathBuf::new())).collect();
		let mut candidates = Vec::new();
		for base in &bases {
			for directory in self.directories {
				candidates.extend(file_names.iter().map(|file_name| base.join(directory).join(file_name)));
			}
		}
		candidates
	}

	/// D15 `use folder|package|project`: a name the program uses but does not define comes from the one file of the scope
	/// defining it, together with what that definition uses in turn and that file's own `use`s; two files defining it
	/// differently is an error
	fn with_scope(&mut self, program: Node, scope: Scope, file: Option<&Path>) -> Result<Node, Node> {
		let mut siblings = scope_files(scope, file)?;
		let mut own = statements(program.clone());
		let mut known = HashSet::new();
		bound_names(&program, &mut known);
		let mut needed: Vec<String> = mentioned_names(&own).into_iter().filter(|name| !known.contains(name)).collect();
		let mut imports = Vec::new();
		let mut using_siblings = HashSet::new();
		while let Some(name) = needed.pop() {
			if !known.insert(name.clone()) {
				continue;
			}
			let mut providers: Vec<(usize, Vec<Node>)> = Vec::new();
			for (index, sibling) in siblings.iter_mut().enumerate() {
				let declarations = sibling.declarations_of(&name)?;
				let same_as_before = providers.iter().any(|(_, earlier)| same_definitions(earlier, &declarations));
				if !declarations.is_empty() && !same_as_before {
					providers.push((index, declarations)); // the same definition in two files is no conflict
				}
			}
			let (index, declarations) = match providers.len() {
				0 => continue,
				1 => providers.remove(0),
				_ => {
					let files: Vec<String> = providers.iter().map(|(index, _)| siblings[*index].path.display().to_string()).collect();
					return Err(error(&format!("{name} is defined in more than one file of the scope: {}; rename one, or `use` the one you mean", files.join(" and "))));
				}
			};
			if using_siblings.insert(index) {
				imports.extend(self.uses_of(&mut siblings[index])?);
			}
			let mut parameters = HashSet::new();
			declarations.iter().for_each(|declaration| bound_names(declaration, &mut parameters));
			parameters.remove(&name);
			needed.extend(mentioned_names(&declarations).into_iter().filter(|used| !parameters.contains(used)));
			imports.extend(declarations);
		}
		if imports.is_empty() {
			return Ok(program);
		}
		imports.append(&mut own);
		Ok(Node::List(imports, Bracket::None, Separator::Newline))
	}

	/// The modules a sibling file uses, resolved from its folder
	fn uses_of(&mut self, sibling: &mut Sibling) -> Result<Vec<Node>, Node> {
		let uses: Vec<Node> = sibling.statements()?.iter().filter(|statement| used_module(statement).is_some()).cloned().collect();
		let outer_directory = self.including_directory.replace(folder_of(&sibling.path));
		let resolved = uses.into_iter().map(|statement| self.resolve(statement)).collect::<Result<Vec<Node>, Node>>();
		self.including_directory = outer_directory;
		Ok(resolved?.into_iter().flat_map(statements).collect())
	}

	/// `import fourty_two` of `fourty_two.wasm` / `.wat`: `use "<its absolute path>"`, which the FFI imports from
	/// (wasm_modules.rs); `include` also runs its entry point, whose value it is (P139)
	fn use_wasm_module(&mut self, module: &Path, statement: &Node, import: Import) -> Vec<Node> {
		let path = module.canonicalize().unwrap_or_else(|_| module.to_path_buf()).display().to_string();
		self.wasm_modules.push(path.clone());
		let used = match as_use(statement) {
			Node::List(_, bracket, separator) => Node::List(vec![Node::Symbol(USE_KEYWORD.to_string()), Node::Text(path.clone())], bracket, separator),
			other => other,
		};
		let entry = (import == Import::Include).then(|| crate::wasm_modules::entry_call(&path)).flatten();
		std::iter::once(used).chain(entry).collect()
	}

	/// A file that cannot be found: an error listing where it was looked for
	fn not_found(&self, name: &str, import: Import) -> Node {
		if import == Import::Use {
			return error(&format!("module not found: {name}"));
		}
		let searched: Vec<String> = self.candidates(name).iter().map(|path| path.display().to_string())
			.map(|path| if path.starts_with(['.', '/']) { path } else { format!("./{path}") }).collect();
		error(&format!("include not found: {name}; searched {}", searched.join(", ")))
	}
}

/// The standard library's modules written in wasp (notes/stdlib.md), embedded so `use list` needs no files
const STD_MODULES: [(&str, &str); 19] = [
	("memory", include_str!("../lib/memory.wasp")),
	("net", include_str!("../lib/net.wasp")),
	("collections", include_str!("../lib/collections.wasp")),
	("hash", include_str!("../lib/hash.wasp")),
	("regex", include_str!("../lib/regex.wasp")),
	("file", include_str!("../lib/file.wasp")),
	("json", include_str!("../lib/json.wasp")),
	("os", include_str!("../lib/os.wasp")),
	("list", include_str!("../lib/list.wasp")),
	("math", include_str!("../lib/math.wasp")),
	("text", include_str!("../lib/text.wasp")),
	("random", include_str!("../lib/random.wasp")),
	("map", include_str!("../lib/map.wasp")),
	("time", include_str!("../lib/time.wasp")),
	("matrix", include_str!("../lib/matrix.wasp")),
	("draw", include_str!("../lib/draw.wasp")),
	("markup", include_str!("../lib/markup.wasp")),
	("router", include_str!("../lib/router.wasp")),
	("i18n", include_str!("../lib/i18n.wasp")),
];
/// The standard modules' folder (P194: std/ merged into lib/), embedded in the binary
const STD_FOLDER: &str = "lib";
/// P171: module words a program calls without their `use` (the prelude); only these definitions come along
const PRELUDE_WORDS: [(&str, &[&str]); 1] = [("file", &["write", "exists"])];
/// P183: a file URL in the program loads the file module, no `use file` needed
const FILE_URL_PREFIX: &str = "file://";
/// Other languages' names of the standard modules' classes and words (Java, Python, Rust, C#), each read as wasp's with
/// a note, when a used module defines that word
const STD_ALIASES: [(&str, &str); 27] = [
	("HashSet", "Set"), ("TreeSet", "Set"), ("LinkedHashSet", "Set"), ("frozenset", "Set"),
	("ArrayDeque", "Deque"), ("VecDeque", "Deque"), ("deque", "Deque"),
	("OrderedDict", "OrderedMap"), ("LinkedHashMap", "OrderedMap"),
	// P169: the C library's historic names of `use math`'s words (atan2 takes y first, angle(x, y) does not: no alias)
	// sqrt and cbrt are the operators √ and ∛ already
	("pow", "power"), ("exp", "exponential"), ("log2", "binary_log"),
	("log10", "decimal_log"), ("sin", "sine"), ("cos", "cosine"), ("tan", "tangent"), ("asin", "arc_sine"),
	("acos", "arc_cosine"), ("atan", "arc_tangent"), ("sinh", "hyperbolic_sine"), ("cosh", "hyperbolic_cosine"),
	("tanh", "hyperbolic_tangent"), ("hypot", "hypotenuse"), ("ceil", "ceiling"), ("trunc", "whole_part"),
	("fmod", "remainder"), ("ln", "natural_log"),
];

/// warp's own lib/<module>.wasp of a standard module: the source the binary embeds (natively the lib folder of the
/// repository it was built from; in the browser lib/ of the served repository, the page's file root)
fn is_embedded_std_file(path: &Path) -> bool {
	let stem = path.file_stem().and_then(|stem| stem.to_str());
	if stem.is_none_or(|stem| std_module(stem).is_none()) {
		return false;
	}
	let folder = path.parent().unwrap_or(Path::new(""));
	if !cfg!(feature = "native") {
		return folder == Path::new(STD_FOLDER);
	}
	let standard_folder = Path::new(env!("CARGO_MANIFEST_DIR")).join(STD_FOLDER).canonicalize().ok();
	standard_folder.is_some() && folder.canonicalize().ok() == standard_folder
}

/// The name an embedded module is loaded under, once per program
fn std_path(name: &str) -> PathBuf {
	PathBuf::from(format!("{STD_FOLDER}/{name}.wasp"))
}

/// The standard modules' names: `list`, `math`, …
pub fn std_module_names() -> impl Iterator<Item = &'static str> {
	STD_MODULES.iter().map(|(module, _)| *module)
}

fn std_module(name: &str) -> Option<&'static str> {
	STD_MODULES.iter().find(|(module, _)| *module == name).map(|(_, source)| *source)
}

/// The standard module that defines `word` (`zip` → list), for the error of a word used without its `use`
pub fn std_module_defining(word: &str) -> Option<&'static str> {
	static DEFINED: std::sync::OnceLock<Vec<(&'static str, Vec<String>)>> = std::sync::OnceLock::new();
	let defined = DEFINED.get_or_init(|| STD_MODULES.iter().map(|(module, source)| (*module, statements(crate::normalize::without_hints(|| WaspParser::parse(source))).iter().filter_map(declared_name).collect())).collect());
	defined.iter().find(|(_, names)| names.iter().any(|name| name == word)).map(|(module, _)| *module)
}

/// The texts of a list a standard module assigns at its top: `html_elements = ["html", …]` of markup; empty if missing
pub fn std_module_list(module: &str, list: &str) -> Vec<String> {
	let source = std_module(module).unwrap_or_default();
	statements(crate::normalize::without_hints(|| WaspParser::parse(source))).iter().find_map(|statement| match statement.drop_meta() {
		Node::Key(name, Op::Assign, items) if name.drop_meta().name() == list => match items.drop_meta() {
			// "p" parses as a character
			Node::List(items, _, _) => Some(items.iter().map(|item| match item.drop_meta() {
				Node::Char(character) => character.to_string(),
				other => other.name(),
			}).collect()),
			_ => None,
		},
		_ => None,
	}).unwrap_or_default()
}

/// The file of the program's folder or search directories that `use module` finds before the standard module of
/// that name (a local file wins)
pub fn module_file_shadowing(module: &str) -> Option<PathBuf> {
	let folder = program_file().as_deref().map(folder_of);
	Loader::new(&SEARCH_DIRECTORIES, folder).find(module)
}

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

/// A package of the registry: `name: "git url"` follows the default branch,
/// `name: {repository: "git url", version: 1.2.3}` is pinned to the git tag of that version
struct Registered {
	url: String,
	pinned: Option<Version>,
}

fn registered(name: &str) -> Option<Registered> {
	statements(WaspParser::parse(PACKAGE_REGISTRY)).iter().find_map(|entry| {
		let Node::Key(key, _, value) = entry.drop_meta() else { return None };
		if !matches!(key.drop_meta(), Node::Symbol(key) if key == name) {
			return None;
		}
		match value.drop_meta() {
			Node::Text(url) => Some(Registered { url: url.clone(), pinned: None }),
			pinned => match pinned["repository"].drop_meta() {
				Node::Text(url) => Some(Registered { url: url.clone(), pinned: Some(version_of(&pinned["version"])?) }),
				_ => None,
			},
		}
	})
}

/// The git url of a package in the registry
pub fn package_repository(name: &str) -> Option<String> {
	registered(name).map(|package| package.url)
}

/// The version a package is pinned to in the registry, None for one that follows its default branch
pub fn pinned_version(name: &str) -> Option<Version> {
	registered(name).and_then(|package| package.pinned)
}

/// Where the pinned version of a package is cloned once per machine: ~/.cache/warp/packages/<name>@<version>
pub fn cached_package(name: &str, version: &Version) -> PathBuf {
	package_cache().join(format!("{name}@{version}"))
}

/// The directory of a package, packages/<name>: the pinned version, or a clone of the default branch;
/// whatever is there already stays, so a local checkout linked there stands in for the package
pub fn fetch_package(name: &str) -> Result<PathBuf, String> {
	fetch_package_into(Path::new(PACKAGES_DIRECTORY), name)
}

/// fetch_package with another packages directory than packages/
pub fn fetch_package_into(packages: &Path, name: &str) -> Result<PathBuf, String> {
	let package = registered(name).ok_or(format!("unknown package: {name}"))?;
	match &package.pinned {
		Some(version) => fetch_tagged(packages, name, &package.url, version, name),
		None => clone(name, &package.url, &packages.join(name), None),
	}
}

/// The directory of the version of a package a requirement picks among its git tags (`v1.2.3` or `1.2.3`):
/// that version, or the latest from the minimum on; cloned into packages/<name>@<version>
pub fn fetch_package_version(name: &str, requirement: &Requirement) -> Result<PathBuf, String> {
	let url = package_repository(name).ok_or(format!("unknown package: {name}"))?;
	let tags = version_tags(&url).map_err(|failure| format!("package {name}: no versions from {url}: {failure}"))?;
	let Some((_, version)) = tags.iter().filter(|(_, version)| requirement.allows(version)).max_by(|a, b| a.1.cmp(&b.1)) else {
		let versions: Vec<String> = tags.iter().map(|(_, version)| version.to_string()).collect();
		return Err(format!("package {name} has no {requirement} (tagged: {})", versions.join(", ")));
	};
	fetch_tagged(Path::new(PACKAGES_DIRECTORY), name, &url, version, &format!("{name}@{version}"))
}

/// packages/<link_name>, a link to the version's shallow clone in the machine's package cache: fetched once per machine,
/// never again per checkout, export or test run. A link into the cache to another version (the pin moved) or to a
/// removed clone is replaced. A clean clone of the repository there (an unpinned fetch) moves to packages/.replaced;
/// anything else there (a local checkout, a clone with changes) stands in for the pin, with a warning.
fn fetch_tagged(packages: &Path, name: &str, url: &str, version: &Version, link_name: &str) -> Result<PathBuf, String> {
	static LINKING: Mutex<()> = Mutex::new(()); // clone() takes FETCHING itself
	let _linking = LINKING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let cached = cached_package(name, version);
	let link = packages.join(link_name);
	let stale = |target: PathBuf| target.starts_with(package_cache()) && (target != cached || !cached.exists());
	if std::fs::read_link(&link).is_ok_and(stale) {
		std::fs::remove_file(&link).map_err(|failure| format!("package {name}: stale link {}: {failure}", link.display()))?;
	}
	if is_clean_clone(&link, url) {
		set_aside(packages, name, version, &link)?;
	}
	if link.exists() {
		if std::fs::read_link(&link).map_or(true, |target| target != cached) {
			warn_overridden(name, version, &link);
		}
		return Ok(link);
	}
	if !cached.exists() {
		let tags = version_tags(url).map_err(|failure| format!("package {name}: no versions from {url}: {failure}"))?;
		let tag = tags.iter().find(|(_, tagged)| tagged == version).map(|(tag, _)| tag.as_str())
			.ok_or(format!("package {name} has no tag of version {version} at {url}"))?;
		clone(name, url, &cached, Some(tag))?;
	}
	std::fs::create_dir_all(packages).map_err(|failure| format!("package {name}: {failure}"))?;
	#[cfg(not(unix))]
	return Err(format!("package {name}: not linked to {}: no symbolic links on this platform", cached.display()));
	#[cfg(unix)]
	match std::os::unix::fs::symlink(&cached, &link) {
		Err(failure) if !link.exists() => Err(format!("package {name}: not linked to {}: {failure}", cached.display())),
		_ => Ok(link),
	}
}

/// A directory (not a link) holding a git clone of url without any change: what an unpinned fetch left there
fn is_clean_clone(directory: &Path, url: &str) -> bool {
	let git = |arguments: &[&str]| Command::new("git").arg("-C").arg(directory).args(arguments).output().ok()
		.filter(|output| output.status.success()).map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
	is_real_directory(directory)
		&& directory.join(".git").exists()
		&& git(&["remote", "get-url", "origin"]).as_deref() == Some(url)
		&& git(&["status", "--porcelain"]).is_some_and(|changes| changes.is_empty())
}

fn is_real_directory(path: &Path) -> bool {
	std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// Moves an unpinned clone out of the pin's way into packages/.replaced, never deleting it
fn set_aside(packages: &Path, name: &str, version: &Version, clone: &Path) -> Result<(), String> {
	let replaced = packages.join(REPLACED_DIRECTORY);
	std::fs::create_dir_all(&replaced).map_err(|failure| format!("package {name}: {failure}"))?;
	let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
	let destination = replaced.join(format!("{}.{seconds}.{}", clone.file_name().unwrap_or_default().to_string_lossy(), std::process::id()));
	match std::fs::rename(clone, &destination) {
		Ok(()) => eprintln!("package {name}: {} was an unpinned clone, moved to {}; now the pinned {version}", clone.display(), destination.display()),
		Err(_) if !is_real_directory(clone) => {} // another process moved it first
		Err(failure) => return Err(format!("package {name}: {} not moved out of the pin's way: {failure}", clone.display())),
	}
	Ok(())
}

/// A local checkout or a changed clone in packages/ wins over the pin, loudly, once per package and process
fn warn_overridden(name: &str, version: &Version, directory: &Path) {
	static WARNED: Mutex<Vec<String>> = Mutex::new(Vec::new());
	let mut warned = WARNED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let key = directory.display().to_string();
	if !warned.contains(&key) {
		eprintln!("warning: package {name}: {key} (a local checkout or a clone with changes) overrides the pinned version {version}");
		warned.push(key);
	}
}

fn package_cache() -> PathBuf {
	PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(PACKAGE_CACHE)
}

/// The tags of a repository that name versions, with their versions
fn version_tags(url: &str) -> Result<Vec<(String, Version)>, String> {
	let listing = Command::new("git").args(["ls-remote", "--tags", "--refs", url]).output().map_err(|failure| failure.to_string())?;
	if !listing.status.success() {
		return Err(String::from_utf8_lossy(&listing.stderr).trim().to_string());
	}
	Ok(String::from_utf8_lossy(&listing.stdout).lines()
		.filter_map(|line| line.split("refs/tags/").nth(1))
		.filter_map(|tag| Some((tag.to_string(), Version::parse(tag)?)))
		.collect())
}

/// A shallow clone of a repository, of a tag or the default branch, into directory unless it is there
fn clone(name: &str, url: &str, directory: &Path, tag: Option<&str>) -> Result<PathBuf, String> {
	let _fetching = FETCHING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	if directory.exists() {
		return Ok(directory.to_path_buf());
	}
	let parent = directory.parent().unwrap_or(Path::new("."));
	std::fs::create_dir_all(parent).map_err(|failure| format!("package {name}: {failure}"))?;
	let directory_name = directory.file_name().unwrap_or_default().to_string_lossy();
	let staging = parent.join(format!(".{directory_name}.fetching.{}", std::process::id()));
	let mut command = Command::new("git");
	command.args(["clone", "--quiet", "--depth", "1"]);
	if let Some(tag) = tag {
		command.args(["--branch", tag]);
	}
	let clone = command.arg(url).arg(&staging).output().map_err(|failure| format!("package {name} not fetched: git: {failure}"))?;
	if !clone.status.success() {
		return Err(format!("package {name} not fetched from {url}: {}", String::from_utf8_lossy(&clone.stderr).trim()));
	}
	if std::fs::rename(&staging, directory).is_err() && directory.exists() {
		std::fs::remove_dir_all(&staging).ok(); // another process fetched it first
	}
	Ok(directory.to_path_buf())
}

fn package_module(directory: &Path, name: &str) -> Option<PathBuf> {
	MODULE_EXTENSIONS.iter().map(|extension| directory.join(format!("{name}.{extension}"))).find(|path| module_exists(path))
}

fn module_exists(path: &Path) -> bool {
	crate::web::file_exists(&path.to_string_lossy())
}

/// Where a registered package's files are: its fetched clone, or in the browser the raw files of its repository at the
/// pinned tag (GitHub serves them to any page) or default branch
fn package_directory(name: &str) -> Result<PathBuf, String> {
	#[cfg(feature = "native")]
	return fetch_package(name);
	#[cfg(not(feature = "native"))]
	{
		let package = registered(name).ok_or(format!("unknown package: {name}"))?;
		let repository = package.url.trim_end_matches(".git").replace("https://github.com/", "https://raw.githubusercontent.com/");
		let reference = package.pinned.map_or("HEAD".to_string(), |version| format!("v{version}"));
		Ok(PathBuf::from(format!("{repository}/{reference}")))
	}
}

/// The version a module file declares with a top level `version 1.2.3`
fn module_version(path: &Path) -> Option<Version> {
	let source = crate::web::read_text(&path.to_string_lossy())?;
	declared_version(&statements(WaspParser::parse(&source)))
}

/// A local module a `use … version` names must declare a version the requirement allows
fn check_version(name: &str, path: &Path, requirement: &Requirement) -> Result<(), Node> {
	match module_version(path) {
		Some(version) if requirement.allows(&version) => Ok(()),
		Some(version) => Err(error(&format!("module {name} is version {version}, {requirement} required"))),
		None => Err(error(&format!("module {name} declares no version, {requirement} required"))),
	}
}

struct Used {
	import: Import,
	name: String,
	requirement: Option<Requirement>,
}

/// `use name`, `require name`, `import name` and `include name`: the name is a symbol, a text or a path `lib/name`, `name.wasp`.
/// A version may follow: `use name version 1.2.3` exactly that one, `use name from 1.2.3` / `use name >= 1.2.3` that or later
fn used_module(node: &Node) -> Option<Used> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [keyword, argument, rest @ ..] = items.as_slice() else { return None };
	let Node::Symbol(keyword) = keyword.drop_meta() else { return None };
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
		(name, [version]) if version_of(version).is_some() && is_version_list(version) => (path_of(name)?, Some(Requirement::Exact(version_of(version)?))),
		(name, [word, version]) if is_word(word, MINIMUM_KEYWORD) => (path_of(name)?, Some(Requirement::Minimum(version_of(version)?))),
		(name, [word, version]) if is_version_keyword(word) => (path_of(name)?, Some(Requirement::Exact(version_of(version)?))),
		_ => return None,
	};
	Some(Used { import, name, requirement })
}

/// `version 1.2.3`, as the parser binds it
fn is_version_list(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(is_version_keyword))
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word)
}

pub(crate) fn path_of(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		Node::Key(directory, Op::Div, name) => Some(format!("{}/{}", path_of(directory)?, path_of(name)?)),
		Node::Key(name, Op::Dot, extension) => Some(format!("{}.{}", path_of(name)?, path_of(extension)?)),
		_ => None,
	}
}

/// A relative path that stays below its directory
fn is_plain_relative_path(name: &str) -> bool {
	Path::new(name).components().all(|component| matches!(component, std::path::Component::Normal(_)))
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
		Node::Key(scope, Op::Colon, _) => matches!(scope.drop_meta(), Node::Symbol(keyword) if is_declaration_keyword(keyword)),
		Node::List(items, _, _) if items.len() >= 2 => {
			matches!(items[0].drop_meta(), Node::Symbol(keyword) if is_function_keyword(keyword) || is_declaration_keyword(keyword))
		}
		_ => false,
	}
}

/// The name a declaration defines: `f(x) := …`, `x = …`, `x:int = 1`, `fun f(x) {…}`, `global x = …`, `class T {…}`
fn declared_name(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::Type { name, .. } => leftmost_symbol(name),
		Node::Key(target, Op::Assign | Op::Define, _) => leftmost_symbol(target),
		// `global x = 0`, as the parser reads it: `global: (x = 0)`
		Node::Key(keyword, Op::Colon, declared) if matches!(keyword.drop_meta(), Node::Symbol(keyword) if is_declaration_keyword(keyword)) => declared_name(declared),
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
	is_function_keyword(keyword) || is_declaration_keyword(keyword) || matches!(keyword, "for" | "require" | "include")
}

/// The signature part of a definition: `f(x)` of `f(x): body` and of `fun f(x) {body}`
fn header(definition: &Node) -> &Node {
	match definition.drop_meta() {
		Node::Key(header, Op::Colon | Op::Assign | Op::Define, _) => header,
		Node::List(items, _, _) if items.first().is_some_and(|first| matches!(first.drop_meta(), Node::List(..))) => &items[0],
		_ => definition,
	}
}

/// Every name the program binds anywhere: variables, functions, parameters, loop variables, imports.
/// Generous on purpose: a bound name never comes from a sibling
fn bound_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define | Op::FatArrow) || op.is_compound_assign() => {
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
	names
}

fn mentioned_names(statements: &[Node]) -> Vec<String> {
	fn collect(node: &Node, names: &mut Vec<String>) {
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
			Node::Meta { node, .. } => collect(node, names),
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
		Node::List(items, bracket, separator) if items.len() == 2 => {
			Node::List(vec![Node::Symbol(USE_KEYWORD.to_string()), items[1].clone()], bracket.clone(), separator.clone())
		}
		other => other.clone(),
	}
}
