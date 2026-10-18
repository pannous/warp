//! Loading modules: the Loader finds, fetches and parses the modules a program uses, and the sibling files of its folder, package or project (D15 `use folder|package|project`)
use super::*;
use crate::node::sequence;

/// the directory of the file it is written in, as text: `read(module_directory + "/data/x")` finds a module's own files
pub(crate) const MODULE_DIRECTORY: &str = "module_directory";
/// `use folder`, `use package`, `use project`: every definition of that scope is visible, looked up by name (D15)
pub(crate) const SCOPE_WORDS: [(&str, Scope); 3] =
	[("folder", Scope::Folder), ("package", Scope::Package), ("project", Scope::Project)];
/// Folders a package or project scope never looks into, besides hidden ones and nested repositories
pub(crate) const UNSCOPED_DIRECTORIES: [&str; 3] = [PACKAGES_DIRECTORY, "target", "node_modules"];
pub(crate) const PROJECT_MARKER: &str = ".git";

/// Another module file of the program's folder, parsed only when its text mentions a name the program needs
pub(super) struct Sibling {
	pub(super) path: PathBuf,
	pub(super) source: String,
	pub(super) statements: Option<Vec<Node>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Scope {
	/// the files of the program's folder
	Folder,
	/// every file below the package folder: the nearest folder holding `<folder name>.warp`
	Package,
	/// every file below the project root: the nearest folder holding .git
	Project,
}

pub(super) fn scope_word(name: &str) -> Option<Scope> {
	SCOPE_WORDS.iter().find(|(word, _)| *word == name).map(|(_, scope)| *scope)
}

/// The other module files of the scope around the program (the working directory for inline code)
pub(super) fn scope_files(scope: Scope, file: Option<&Path>) -> Result<Vec<Sibling>, Node> {
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
	Ok(paths
		.into_iter()
		.filter_map(|path| Some(Sibling { source: std::fs::read_to_string(&path).ok()?, path, statements: None }))
		.collect())
}

pub(super) fn is_module_file(path: &Path) -> bool {
	path.is_file()
		&& path.extension().is_some_and(|extension| MODULE_EXTENSIONS.iter().any(|known| extension == *known))
}

pub(super) fn collect_module_files(directory: &Path, below: bool, paths: &mut Vec<PathBuf>) {
	let Ok(entries) = std::fs::read_dir(directory) else {
		return;
	};
	for path in entries.filter_map(|entry| entry.ok().map(|entry| entry.path())) {
		if is_module_file(&path) {
			paths.push(path);
		} else if below && path.is_dir() && is_scoped_directory(&path) {
			collect_module_files(&path, below, paths);
		}
	}
}

/// Hidden folders, fetched packages, build output and nested repositories belong to no scope
pub(super) fn is_scoped_directory(directory: &Path) -> bool {
	let name = directory_name(directory);
	!name.starts_with('.') && !UNSCOPED_DIRECTORIES.contains(&name.as_str()) && !directory.join(PROJECT_MARKER).exists()
}

fn directory_name(directory: &Path) -> String {
	directory.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
}

pub(super) fn package_root(folder: &Path) -> Result<PathBuf, Node> {
	let holds_its_module = |directory: &Path| {
		let name = directory_name(directory);
		MODULE_EXTENSIONS.iter().any(|extension| directory.join(format!("{name}.{extension}")).is_file())
	};
	enclosing_root(
		folder,
		holds_its_module,
		"use package: no package folder around {}: a package folder holds <folder name>.warp",
	)
}

pub(super) fn project_root(folder: &Path) -> Result<PathBuf, Node> {
	let holds_marker = |directory: &Path| directory.join(PROJECT_MARKER).exists();
	enclosing_root(
		folder,
		holds_marker,
		&format!("use project: no project root around {{}}: the project root holds {PROJECT_MARKER}"),
	)
}

/// The nearest folder around `folder` (itself included) that `is_root`; `missing` reports none, its `{}` the folder
fn enclosing_root(folder: &Path, is_root: impl Fn(&Path) -> bool, missing: &str) -> Result<PathBuf, Node> {
	let absolute = folder.canonicalize().unwrap_or_else(|_| folder.to_path_buf());
	let root = absolute.ancestors().find(|directory| is_root(directory));
	root.map(Path::to_path_buf).ok_or_else(|| error(&missing.replace("{}", &folder.display().to_string())))
}

impl Sibling {
	/// The statements of the file; a file that does not parse is reported and contributes nothing
	pub(super) fn statements(&mut self) -> Result<&[Node], Node> {
		if self.statements.is_none() {
			let module = WarpParser::parse(&self.source);
			let parsed = match module.first_error() {
				Some(failure) => {
					let warning = crate::errors::diagnostic::Diagnostic::at(
						&module,
						format!("folder scope skips {}: {}", self.path.display(), failure.serialize()),
					);
					crate::errors::diagnostic::report(&[warning])?;
					vec![]
				}
				None => statements(with_module_directory(module, &folder_of(&self.path))),
			};
			self.statements = Some(parsed);
		}
		Ok(self.statements.as_deref().unwrap_or_default())
	}

	pub(super) fn declarations_of(&mut self, name: &str) -> Result<Vec<Node>, Node> {
		if !self.source.contains(name) {
			return Ok(vec![]);
		}
		Ok(self
			.statements()?
			.iter()
			.filter(|statement| is_declaration(statement) && name_declared_by(statement).as_deref() == Some(name))
			.cloned()
			.collect())
	}
}

pub(super) fn with_module_directory(module: Node, directory: &Path) -> Node {
	crate::library_words::substitute(module, MODULE_DIRECTORY, &Node::Text(directory.display().to_string()))
}

#[derive(Default)]
pub(super) struct Loader<'a> {
	pub(super) directories: &'a [&'a str],
	/// files whose declarations a `use` brought in, and files an `include` spliced in: each is loaded once
	pub(super) loaded: HashSet<PathBuf>,
	pub(super) included: HashSet<PathBuf>,
	/// the packages used and the version loaded (None: its module declares none): a program uses one version of each
	pub(super) packages: HashMap<String, Option<Version>>,
	/// the directory of the file being loaded; None for the program itself, which searches from the working directory
	pub(super) including_directory: Option<PathBuf>,
	/// the widest `use folder|package|project` met
	pub(super) scope: Option<Scope>,
	/// the paths of the imported WebAssembly modules, whose exports the program reads (wasm_modules::rewrite_uses)
	pub(super) wasm_modules: Vec<String>,
	/// the definitions of the standard modules used: the program gets those it calls (with_needed_definitions)
	pub(super) std_definitions: Vec<Node>,
	/// the program's top-level variables and own functions: a module word of the same name is renamed apart in the module
	/// (shadowed_apart)
	pub(super) program_words: ProgramWords,
	/// how many standard modules' sources are being loaded, one inside the other: from the second on, their `use`s
	/// are their own, not the program's
	pub(super) std_module_depth: usize,
	/// whether the modules being loaded are those the program needs without naming them (implicit_std_modules)
	pub(super) loading_implicitly: bool,
	/// the standard modules the program (or a file module) uses, and those only standard modules use (hidden_apart)
	pub(super) program_std_modules: HashSet<String>,
	pub(super) std_modules_of_std_modules: HashSet<String>,
	/// whether a `fast f(x) := …` calls fast_math's words by their qualified names (fast_body)
	pub(super) fast_functions: bool,
	/// whether the module being loaded began with `fast`: each of its functions is fast
	pub(super) fast_module: bool,
	/// the words each file module (a file or package, not a standard module) defines itself, by its file
	pub(super) file_module_words: HashMap<PathBuf, HashSet<String>>,
	/// each `use` (None: all its words) and `from … import` (the words named) of a file module, by the program (no
	/// importer) or a module: what the program sees of the file modules (visible_words)
	pub(super) module_imports: Vec<ModuleImport>,
	/// the file modules being loaded, one inside the other: the last one's imports are its own
	pub(super) loading_modules: Vec<PathBuf>,
	/// the file module the last `use` loaded or found loaded already
	pub(super) used_file_module: Option<PathBuf>,
	/// the module file of each package loaded: a second `use` of the package names it again
	pub(super) package_modules: HashMap<String, PathBuf>,
}

pub(super) struct ModuleImport {
	pub(super) importer: Option<PathBuf>,
	pub(super) module: PathBuf,
	/// as the `use` names it
	pub(super) name: String,
	pub(super) words: Option<Vec<String>>,
}

/// marks the statements a `use` of the program brought in, until finish_imports renames their hidden words
const IMPORTED_STATEMENTS: &str = "·imported statements";

impl<'a> Loader<'a> {
	pub(super) fn new(directories: &'a [&'a str], including_directory: Option<PathBuf>) -> Self {
		Loader { directories, including_directory, ..Default::default() }
	}

	/// The standard definitions with the words of a module only other standard modules use renamed apart
	/// (`take` → `lib·take` when lib/text.warp's `use list` brought it): the program sees a module's words only with
	/// its own `use`, a bare `take` is the error naming the module. A word a used module defines too stays one word:
	/// shell's `unique(file:Path)` and list's `unique(xs)` are one generic function (card cross-module-overload)
	pub(super) fn hidden_apart(&self) -> Vec<Node> {
		let definitions_of = |modules: &HashSet<String>| -> HashSet<&String> {
			modules.iter().flat_map(|module| std_module_definitions(module).unwrap_or_default()).collect()
		};
		let visible = definitions_of(&self.program_std_modules);
		let only_used_by_modules: HashSet<String> =
			self.std_modules_of_std_modules.difference(&self.program_std_modules).cloned().collect();
		let hidden: Vec<&String> =
			definitions_of(&only_used_by_modules).into_iter().filter(|word| !visible.contains(word)).collect();
		crate::compiler::compile_context::change(|context| {
			let names = &mut context.module_definitions;
			hidden.iter().for_each(|word| {
				names.remove(*word);
				names.insert(format!("{SHADOWED_WORD_PREFIX}{word}"));
			});
		});
		let renames = shadowed_renames(hidden.iter().map(|word| word.as_str()));
		crate::lowering::word_operators::lower_call_forms_in(self.std_definitions.clone())
			.into_iter()
			.map(|definition| renamed_words(definition, &renames))
			.collect()
	}

	pub(super) fn resolve(&mut self, node: Node) -> Result<Node, Node> {
		if let Some(imported) = self.imported_by(&node)? {
			return Ok(match imported {
				imported if imported.is_empty() => Node::Empty,
				imported => sequence(imported),
			});
		}
		match node {
			Node::List(items, bracket, separator) => {
				let mut resolved = Vec::with_capacity(items.len());
				for item in items {
					let item = self.fast_body(item);
					match self.imported_by(&item)? {
						Some(imported) => resolved.extend(imported),
						None => resolved.push(self.resolve(item)?),
					}
				}
				Ok(Node::List(resolved, bracket, separator))
			}
			Node::Meta { node, data } => Ok(Node::Meta { node: Box::new(self.resolve(*node)?), data }),
			other => Ok(other),
		}
	}

	/// `fast f(x) := …` (card fast-scope): its body calls fast_math's words (sin cos exp pow atan2) by their qualified
	/// names `fast_math·sin`, which use_fast_functions loads, and divides Ints as floats; the emitter wraps its Ints
	/// (compile_mode::is_fast). The rest of the program stays exact
	fn fast_body(&mut self, statement: Node) -> Node {
		let Some((name, body)) = fast_definition(&statement, self.fast_module) else {
			return statement;
		};
		let body = with_fast_names(float_division(body), std_module_definitions(FAST_MODULE).unwrap_or_default());
		self.fast_functions = true;
		if let Some(function) = leftmost_symbol(&name) {
			crate::compiler::compile_context::change(|context| context.fast_functions.insert(function));
		}
		Node::Key(Box::new(name), Op::Define, Box::new(body))
	}

	/// fast_math's definitions named `fast_math·sin` …, for the `fast` functions (fast_body); loaded apart from a `use
	/// fast_math`, which binds the plain words
	pub(super) fn use_fast_functions(&mut self) -> Result<(), Node> {
		if !self.fast_functions {
			return Ok(());
		}
		let source = std_module(FAST_MODULE).expect("lib/fast_math.warp is embedded");
		let path = std_path(&fast_name(FAST_MODULE));
		let definitions = self.load_unhinted(path, source)?;
		let words = std_module_definitions(FAST_MODULE).unwrap_or_default();
		self.std_definitions.extend(definitions.into_iter().map(|definition| with_fast_names(definition, words)));
		crate::compiler::compile_context::change(|context| {
			context.module_definitions.extend(words.iter().map(|word| fast_name(word)));
		});
		Ok(())
	}

	/// The statements of each module a `use` names
	pub(super) fn import_all(&mut self, uses: Vec<(Used, Node)>) -> Result<Vec<Node>, Node> {
		let mut imported = vec![];
		for (used, statement) in uses {
			imported.extend(self.import_noted(&used, &statement, None)?.0);
		}
		Ok(imported)
	}

	/// The statements a `use` or `from … import` statement brings in, None for any other statement. Those the program's
	/// own statement brings are marked, for finish_imports
	fn imported_by(&mut self, statement: &Node) -> Result<Option<Vec<Node>>, Node> {
		let imported = match (imported_words(statement), used_modules(statement)) {
			(Some((used, statement, words)), _) => self.import_words(&used, &statement, &words)?,
			(None, Some(uses)) => self.import_all(uses)?,
			(None, None) => return Ok(None),
		};
		let by_the_program = self.loading_modules.is_empty() && self.std_module_depth == 0;
		Ok(Some(match by_the_program && !imported.is_empty() {
			true => vec![Node::Meta {
				node: Box::new(sequence(imported)),
				data: Box::new(Node::Symbol(IMPORTED_STATEMENTS.into())),
			}],
			false => imported,
		}))
	}

	/// A module's statements (import), and whether it is a file module: then the import is noted for finish_imports
	fn import_noted(
		&mut self,
		used: &Used,
		statement: &Node,
		words: Option<&[String]>,
	) -> Result<(Vec<Node>, bool), Node> {
		self.used_file_module = None;
		let imported = self.import(used, statement)?;
		let Some(module) = self.used_file_module.take() else {
			return Ok((imported, false));
		};
		let importer = self.loading_modules.last().cloned();
		let words = words.map(<[String]>::to_vec);
		self.module_imports.push(ModuleImport { importer, module, name: used.name.clone(), words });
		Ok((imported, true))
	}

	/// The program with the statements its `use`s brought in, the words of file modules it does not see renamed apart
	/// in them: a `from … import` passes on only the words it names, a `use` all a module sees (decision
	/// package-reexport), whichever file imports a module first (card import-hides)
	pub(super) fn finish_imports(&self, program: Node) -> Result<Node, Node> {
		let mut seen = HashMap::new();
		for import in &self.module_imports {
			let visible = self.visible_words(Some(&import.module), &mut seen);
			if let Some(missing) = import.words.iter().flatten().find(|word| !visible.contains(*word)) {
				return Err(error(&format!("{} has no {missing}", import.name)));
			}
		}
		let visible = self.visible_words(None, &mut seen);
		let hidden: Vec<String> =
			self.file_module_words.values().flatten().filter(|word| !visible.contains(*word)).cloned().collect();
		Ok(with_imports_renamed(program, &hidden))
	}

	/// The words of file modules a module (None: the program) sees: its own and those its imports pass on
	fn visible_words(
		&self,
		module: Option<&PathBuf>,
		seen: &mut HashMap<Option<PathBuf>, HashSet<String>>,
	) -> HashSet<String> {
		let key = module.cloned();
		if let Some(visible) = seen.get(&key) {
			return visible.clone();
		}
		seen.insert(key.clone(), HashSet::new()); // an import cycle sees what is known so far
		let mut visible = module.and_then(|module| self.file_module_words.get(module)).cloned().unwrap_or_default();
		for import in self.module_imports.iter().filter(|import| import.importer.as_ref() == module) {
			let passed = self.visible_words(Some(&import.module), seen);
			match &import.words {
				Some(words) => visible.extend(words.iter().filter(|word| passed.contains(*word)).cloned()),
				None => visible.extend(passed),
			}
		}
		seen.insert(key, visible.clone());
		visible
	}

	/// A module's statements with only `words` visible to the program: its other words are renamed apart, those of a
	/// file module by finish_imports. A module loaded already brings nothing new
	pub(super) fn import_words(&mut self, used: &Used, statement: &Node, words: &[String]) -> Result<Vec<Node>, Node> {
		let first_std_definition = self.std_definitions.len();
		let (imported, file_module) = self.import_noted(used, statement, Some(words))?;
		if file_module {
			return Ok(imported);
		}
		let std_definitions = self.std_definitions.split_off(first_std_definition);
		let module_words: HashSet<String> =
			imported.iter().chain(&std_definitions).filter_map(name_declared_by).collect();
		if module_words.is_empty() {
			return Ok(imported);
		}
		if let Some(missing) = words.iter().find(|word| !module_words.contains(*word)) {
			return Err(error(&format!("{} has no {missing}", used.name)));
		}
		let hidden: Vec<String> = module_words.into_iter().filter(|word| !words.contains(word)).collect();
		self.std_definitions.extend(renamed_apart(std_definitions, &hidden));
		Ok(renamed_apart(imported, &hidden))
	}

	/// The statements a `use` stands for: the module's definitions, nothing for a module already loaded,
	/// the `use` itself for a native library
	pub(super) fn import(&mut self, used: &Used, statement: &Node) -> Result<Vec<Node>, Node> {
		let (import, name) = (used.import, used.name.as_str());
		if let Some(scope) = scope_word(name).filter(|_| import == Import::Use && used.requirement.is_none()) {
			if scope != Scope::Folder {
				crate::modules::reads_beyond_folders("use package or use project");
			}
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
			if let Some(source) = std_module(name) {
				return match import {
					Import::Use => self.use_std_module(name, source),
					// `include math`: the whole module spliced in place, as an included file is
					Import::Include => crate::normalize::without_hints(|| {
						self.load_source(import, std_path(std_module_name(name)), source)
					}),
				};
			}
			if let Some(module) = self.find_with(name, &crate::modules::wasm_modules::MODULE_EXTENSIONS) {
				return Ok(self.use_wasm_module(&module, statement, import));
			}
			if import == Import::Use && package_repository(name).is_some() {
				crate::modules::reads_beyond_folders("a package");
				return self.use_package(name, used.requirement.as_ref());
			}
			return match import == Import::Use && crate::ffi::is_ffi_library(name) {
				true => {
					crate::modules::reads_beyond_folders("a C library");
					Ok(vec![as_use(statement)])
				}
				false => Err(self.not_found(name, import)),
			};
		};
		use crate::errors::diagnostic::{report, Diagnostic};
		let shadows = |shadowed: String| Diagnostic::at(statement, format!("{} shadows {shadowed}", path.display()));
		// only what warp knows without fetching: a local module is no reason to ask the network
		let is_known_package = import == Import::Use && known_package(name);
		if let Some(repository) = is_known_package.then(|| package_repository(name)).flatten() {
			report(&[shadows(format!("the package {name} ({repository}): rename the file to use the package"))])?;
		}
		if let Some(requirement) = &used.requirement {
			check_version(name, &path, requirement)?;
		}
		if std_module(name).is_some() && !edited_since_build(&path) {
			report(
				&[shadows(format!("the standard module {name}: its text differs from the one warp embeds")).weak()],
			)?;
		}
		self.load(import, name, path)
	}

	/// A registered package: fetched once, then its module `<name>.warp` if it has one; a package without is data.
	/// A required version the default branch does not declare comes from the matching git tag. Its source, and what
	/// it uses, gets no style hints: the user does not write it (card hints-fire). A program uses one version of a
	/// package: a second `use` the loaded one satisfies brings nothing new, one asking for another version is the error
	/// naming both (card use-version-twice).
	pub(super) fn use_package(&mut self, name: &str, requirement: Option<&Requirement>) -> Result<Vec<Node>, Node> {
		if let Some(loaded) = self.packages.get(name) {
			return match (requirement, loaded) {
				(Some(requirement), Some(version)) if !requirement.allows(version) => Err(error(&format!(
					"use {name} {requirement}: {name} {version} is used already; a program uses one version of a package"
				))),
				_ => {
					self.used_file_module = self.package_modules.get(name).cloned();
					Ok(vec![])
				}
			};
		}
		let package = registered_package(name).map_err(|failure| error(&failure))?;
		let mut directory = package_directory(name).map_err(|failure| error(&failure))?;
		let mut library = package.library;
		if let Some(requirement) = requirement {
			let declared = package_module(&directory, name).and_then(|path| module_version(&path));
			if !declared.is_some_and(|version| requirement.allows(&version)) {
				directory = fetch_package_version(name, requirement).map_err(|failure| error(&failure))?;
				library = None; // the library is the pinned version's release asset
			}
		}
		let version = package_module(&directory, name).and_then(|path| module_version(&path));
		self.packages.insert(name.to_string(), version);
		if let Some(library) = library {
			let binding = package.proxy.unwrap_or(directory);
			return crate::normalize::without_hints(|| self.use_library(name, &library, &binding));
		}
		let Some(path) = package_module(&directory, name) else {
			return Ok(vec![]);
		};
		let loaded = crate::normalize::without_hints(|| self.load(Import::Use, name, path));
		if let Some(module) = &self.used_file_module {
			self.package_modules.insert(name.to_string(), module.clone());
		}
		loaded
	}

	/// A package's prebuilt library through the binding `<name>.warp` of the folder declaring it (a proxy's folder in
	/// the default registry, else the package's own), loaded as if it lay beside the library: its `import <library>`
	/// finds `<library>.wasm` there (`library` of the manifest or entry)
	fn use_library(&mut self, name: &str, library: &str, folder: &Path) -> Result<Vec<Node>, Node> {
		let binding_path = folder.join(format!("{name}.warp"));
		let binding = crate::web::read_text(&binding_path.to_string_lossy()).ok_or_else(|| {
			error(&format!("package {name}: no binding {} for its library {library}", binding_path.display()))
		})?;
		#[cfg(feature = "native")]
		let directory = crate::modules::package_tools::library_directory(name, library);
		// browsers cannot fetch release assets (no CORS): the package commits its library, read from the tag's raw files
		#[cfg(not(feature = "native"))]
		let directory = package_directory(name).map(|directory| {
			let module = directory.join(library);
			module.parent().map_or(directory.clone(), Path::to_path_buf)
		});
		let directory = directory.map_err(|failure| error(&failure))?;
		// the binding's `import <library>` asks for only the library's module and header there
		let file = Path::new(library).file_name().map_or(library.into(), |file| file.to_string_lossy());
		let files = ["wasm", "h"].map(|extension| format!("{file}.{extension}"));
		crate::web::list_directory(&directory.to_string_lossy(), &files);
		self.load_source(Import::Use, directory.join(format!("{name}.warp")), &binding)
	}

	/// The standard modules a program uses without `use`: those IMPLICIT_MODULES picks for it, the prelude words it
	/// mentions (P171); a module the program used is loaded already
	pub(super) fn implicit_std_modules(&mut self, program: &Node) -> Result<(), Node> {
		if let Some(word) = rare_sound_word(program).filter(|_| !self.program_std_modules.contains(SOUND_MODULE)) {
			return Err(error(&crate::ffi::undefined_function_message(word)));
		}
		for (module, needed) in IMPLICIT_MODULES {
			if needed(program) {
				self.use_std_module(module, std_module(module).expect("an implicit module is embedded"))?;
			}
		}
		// `style{ … }`: a standard function taking its argument as data, called or as a tag
		for (module, line) in std_data_functions() {
			let name = defined_function_name(line);
			if tags_or_calls(program, name) && defined(program, name).is_none() {
				self.use_std_module(module, std_module(module).expect("a standard module is embedded"))?;
			}
		}
		let mut mentioned = HashSet::new();
		program.visit(&mut |node| {
			if let Node::Symbol(name) = node {
				mentioned.insert(name.clone());
			}
		});
		// a program's own `write(x, y)` wins
		let own: HashSet<String> = statements_of(program).iter().filter_map(name_declared_by).collect();
		for (module, words) in PRELUDE_WORDS {
			let words: Vec<&str> =
				words.iter().copied().filter(|word| mentioned.contains(*word) && !own.contains(*word)).collect();
			if !words.is_empty() {
				let source = std_module(module).expect("a prelude module is embedded");
				let definitions = self.load_unhinted(std_path(module), source)?;
				self.std_definitions.extend(definitions.into_iter().filter(|definition| {
					name_declared_by(definition).is_some_and(|name| words.contains(&name.as_str()))
				}));
			}
		}
		self.use_fast_functions()?;
		self.use_prelude(program)
	}

	/// lib/prelude.warp, its words named `prelude·first` (prelude_name), when the program or a used standard module
	/// mentions one of them in any spelling (`isdigit`); with_needed_definitions keeps those called
	pub(super) fn use_prelude(&mut self, program: &Node) -> Result<(), Node> {
		// the used standard modules' definitions the program reaches: a style page calls ~20 of lib/markup.warp's, and
		// with_needed_definitions keeps no others, so the prelude words of the rest are never needed
		let callers = [vec![program.clone()], needed_definitions(program, &self.std_definitions)].concat();
		let mentioned = called_names(&callers);
		let prelude = std_module_definitions(PRELUDE_MODULE).unwrap_or_default();
		if !prelude.iter().any(|word| mentioned.contains(word)) {
			return Ok(());
		}
		// a program's own `first(xs) := …` takes its calls from the prelude's: a hint, as a used module's is a warning
		let replaced: Vec<&str> =
			prelude.iter().filter(|word| self.program_words.contains(word)).map(String::as_str).collect();
		let replaced = crate::compiler::phase_times::timed("prelude: replaced declarations", || {
			prelude_declarations_of(&replaced)
		});
		warn_replaced(&replaced, &self.program_words, PRELUDE_MODULE, crate::errors::diagnostic::Level::Hint);
		let reached = reached_prelude_definitions(&callers);
		let module = sequence(reached);
		let definitions = crate::compiler::phase_times::timed("prelude: load reached", || {
			crate::normalize::without_hints(|| self.load_module(Import::Use, std_path(PRELUDE_MODULE), module))
		})?;
		// an operator (`infix operator ↑(x: number, n: number)`) keeps its name: a program's own `infix operator ↑(v: V,
		// n: int)` overloads it by parameter type (card easy-add)
		let qualified_words: Vec<&String> =
			prelude.iter().filter(|word| !word.starts_with(crate::operators::OPERATOR_FUNCTION_PREFIX)).collect();
		// a word the program defines too came back `lib·round_to` (shadowed_apart): the prelude's is qualified all the
		// same, so the library's spellings (`round(x, 2)`) keep calling it while the program's calls reach its own
		let renames: HashMap<String, String> = qualified_words
			.iter()
			.flat_map(|word| {
				let prelude_word = prelude_name(word).unwrap_or_default();
				[(word.to_string(), prelude_word.clone()), (format!("{SHADOWED_WORD_PREFIX}{word}"), prelude_word)]
			})
			.collect();
		let qualified = |definition: Node| renamed_words(definition, &renames);
		let qualified: Vec<Node> = crate::compiler::phase_times::timed("prelude: qualified names", || {
			definitions.into_iter().map(qualified).collect()
		});
		self.std_definitions.extend(qualified);
		crate::compiler::compile_context::change(|context| {
			let names = &mut context.module_definitions;
			qualified_words.iter().for_each(|word| {
				names.remove(*word);
				names.extend(prelude_name(word));
			});
		});
		Ok(())
	}

	/// A standard module: its definitions wait aside until the program is resolved; its source shows the program
	/// no style hints
	pub(super) fn use_std_module(&mut self, name: &str, source: &str) -> Result<Vec<Node>, Node> {
		let name = std_module_name(name);
		match self.std_module_depth {
			0 => self.program_std_modules.insert(name.to_string()),
			_ => self.std_modules_of_std_modules.insert(name.to_string()),
		};
		self.std_module_depth += 1;
		let definitions = self.load_unhinted(std_path(name), source);
		self.std_module_depth -= 1;
		self.std_definitions.extend(definitions?);
		Ok(vec![])
	}

	/// A module the program's writer does not write (a standard module, a package's binding): its source shows no style hints
	fn load_unhinted(&mut self, path: PathBuf, source: &str) -> Result<Vec<Node>, Node> {
		crate::normalize::without_hints(|| self.load_source(Import::Use, path, source))
	}

	pub(super) fn load(&mut self, import: Import, name: &str, path: PathBuf) -> Result<Vec<Node>, Node> {
		let source = crate::web::read_text(&path.to_string_lossy())
			.ok_or_else(|| error(&format!("module not readable: {name}")))?;
		self.load_source(import, path, &source)
	}

	/// A module from its source: `path` names it (each is loaded once) and is where its own `use`s look
	pub(super) fn load_source(&mut self, import: Import, path: PathBuf, source: &str) -> Result<Vec<Node>, Node> {
		let loaded = match import {
			Import::Use => &mut self.loaded,
			Import::Include => &mut self.included,
		};
		let file = path.canonicalize().unwrap_or_else(|_| path.clone());
		if !loaded.insert(file.clone()) {
			if self.file_module_words.contains_key(&file) {
				self.used_file_module = Some(file);
			}
			return Ok(vec![]);
		}
		let shown = shown_path(&path);
		let classes_early = classes_early(import, &file);
		let lower = || {
			let module = crate::compiler::meta::parsing_file(&shown.to_string_lossy(), || WarpParser::parse(source));
			match module.first_error() {
				Some(failure) => Err(crate::errors::diagnostic::located_in_file(failure, &shown)),
				None => lowered_source(&path, module, classes_early),
			}
		};
		let module = match is_std_source(&shown.to_string_lossy()) {
			true => lowered_std_module(source, classes_early, lower)?,
			false => lower()?,
		};
		self.join_lowered(import, path, file, module)
	}

	/// lib/prelude.warp's definitions the program reaches, already parsed (use_prelude)
	fn load_module(&mut self, import: Import, path: PathBuf, module: Node) -> Result<Vec<Node>, Node> {
		let file = path.canonicalize().unwrap_or_else(|_| path.clone());
		match self.loaded.insert(file.clone()) {
			true => self.load_parsed(import, path, file, module),
			false => Ok(vec![]),
		}
	}

	fn load_parsed(&mut self, import: Import, path: PathBuf, file: PathBuf, module: Node) -> Result<Vec<Node>, Node> {
		let classes_early = classes_early(import, &file);
		let module = match is_std_source(&shown_path(&path).to_string_lossy()) {
			true => lowered_std_definitions(&module, classes_early, || {
				lowered_source(&path, module.clone(), classes_early)
			})?,
			false => lowered_source(&path, module, classes_early)?,
		};
		self.join_lowered(import, path, file, module)
	}

	/// A module's lowered source joins the program: its own `use`s resolved, its declarations the program's words
	fn join_lowered(&mut self, import: Import, path: PathBuf, file: PathBuf, module: Node) -> Result<Vec<Node>, Node> {
		let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
		let module = with_module_directory(module, &directory);
		let outer_directory = self.including_directory.replace(directory);
		let (module, fast) = without_fast_pragma(module);
		let outer_fast = std::mem::replace(&mut self.fast_module, fast);
		let file_module = import == Import::Use && self.std_module_depth == 0;
		if file_module {
			let declares = |statement: &&Node| is_declaration(statement) && used_modules(statement).is_none();
			let own = statements(module.clone()).iter().filter(declares).filter_map(name_declared_by).collect();
			self.file_module_words.insert(file.clone(), own);
			self.loading_modules.push(file.clone());
		}
		let module = self.resolve(module);
		if file_module {
			self.loading_modules.pop();
			self.used_file_module = Some(file);
		}
		self.fast_module = outer_fast;
		self.including_directory = outer_directory;
		let statements = statements(module?);
		Ok(match import {
			Import::Use => {
				let declarations: Vec<Node> = statements.into_iter().filter(is_declaration).collect();
				// a module the program named: its words were the program's before its own definition took them
				if self.std_module_depth <= 1 && !self.loading_implicitly {
					let module = path.file_stem().unwrap_or_default().to_string_lossy();
					warn_replaced(
						&declarations,
						&self.program_words,
						&module,
						crate::errors::diagnostic::Level::Normal,
					);
				}
				let declarations = shadowed_apart(declarations, &self.program_words);
				crate::compiler::compile_context::change(|context| {
					context.module_definitions.extend(declarations.iter().filter_map(name_declared_by))
				});
				declarations
			}
			Import::Include => statements,
		})
	}

	/// The file of a module: every search directory below the directory of the including file, then below the working
	/// directory; a name that already ends in an extension is tried as written
	pub(super) fn find(&self, name: &str) -> Option<PathBuf> {
		self.find_with(name, &MODULE_EXTENSIONS)
	}

	/// never the program's own file: hash.warp saying `use hash` means the standard module; nor warp's own lib/hash.warp,
	/// which is the embedded standard module itself (found in lib when a program runs in warp's repository).
	/// In the browser a program without a file (the playground's) has no folder of its own: a standard module's name is
	/// the standard module, no request looks next to the page for a file shadowing it (card playground-module)
	pub(super) fn find_with(&self, name: &str, extensions: &[&str]) -> Option<PathBuf> {
		if !cfg!(feature = "native") && program_file().is_none() && std_module(name).is_some() {
			return None;
		}
		let program = program_file().and_then(|file| file.canonicalize().ok());
		let is_program = |path: &PathBuf| program.is_some() && path.canonicalize().ok() == program;
		self.candidates_with(name, extensions).into_iter().find(|path| {
			module_exists(path)
				&& !is_program(path)
				&& !is_embedded_std_file(path)
				&& !crate::modules::shadows_with_a_sample(path)
		})
	}

	pub(super) fn candidates(&self, name: &str) -> Vec<PathBuf> {
		self.candidates_with(name, &MODULE_EXTENSIONS)
	}

	pub(super) fn candidates_with(&self, name: &str, extensions: &[&str]) -> Vec<PathBuf> {
		let has_extension = extensions.iter().any(|extension| name.ends_with(&format!(".{extension}")));
		let file_names: Vec<String> = match has_extension {
			true => vec![name.to_string()],
			false => extensions.iter().map(|extension| format!("{name}.{extension}")).collect(),
		};
		// `use "./helper.warp"`, `use "/srv/helper.warp"`: that file, `./` from the including file's folder
		if is_explicit_path(name) {
			crate::modules::reads_beyond_folders("a module by its path");
			let base = self.including_directory.clone().unwrap_or_default();
			return file_names.iter().map(|file_name| base.join(file_name)).collect();
		}
		if !is_plain_relative_path(name) {
			return vec![];
		}
		if name.contains('/') {
			crate::modules::reads_beyond_folders("a module of a subfolder");
		}
		let bases: Vec<PathBuf> =
			self.including_directory.iter().cloned().chain(std::iter::once(PathBuf::new())).collect();
		let mut candidates = Vec::new();
		for base in &bases {
			for directory in self.directories {
				crate::modules::looks_in(&base.join(directory));
				candidates.extend(file_names.iter().map(|file_name| base.join(directory).join(file_name)));
			}
		}
		candidates
	}

	/// D15 `use folder|package|project`: a name the program uses but does not define comes from the one file of the scope
	/// defining it, together with what that definition uses in turn and that file's own `use`s; two files defining it
	/// differently is an error
	pub(super) fn with_scope(&mut self, program: Node, scope: Scope, file: Option<&Path>) -> Result<Node, Node> {
		let mut siblings = scope_files(scope, file)?;
		let mut own = statements(program.clone());
		let mut known = HashSet::new();
		program_bound_names(&program, &mut known);
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
					let files: Vec<String> =
						providers.iter().map(|(index, _)| siblings[*index].path.display().to_string()).collect();
					return Err(error(&format!(
						"{name} is defined in more than one file of the scope: {}; rename one, or `use` the one you mean",
						files.join(" and ")
					)));
				}
			};
			if using_siblings.insert(index) {
				imports.extend(self.uses_of(&mut siblings[index])?);
			}
			let mut parameters = HashSet::new();
			declarations.iter().for_each(|declaration| program_bound_names(declaration, &mut parameters));
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
	pub(super) fn uses_of(&mut self, sibling: &mut Sibling) -> Result<Vec<Node>, Node> {
		let uses: Vec<Node> =
			sibling.statements()?.iter().filter(|statement| used_modules(statement).is_some()).cloned().collect();
		let outer_directory = self.including_directory.replace(folder_of(&sibling.path));
		// after finish_imports: the groups are spliced in as they are
		let resolved = uses
			.into_iter()
			.map(|statement| self.resolve(statement).map(|node| with_imports_renamed(node, &[])))
			.collect::<Result<Vec<Node>, Node>>();
		self.including_directory = outer_directory;
		Ok(resolved?.into_iter().flat_map(statements).collect())
	}

	/// `import fourty_two` of `fourty_two.wasm` / `.wat`: `use "<its absolute path>"`, which the FFI imports from
	/// (wasm_modules.rs); `include` also runs its entry point, whose value it is (P139)
	pub(super) fn use_wasm_module(&mut self, module: &Path, statement: &Node, import: Import) -> Vec<Node> {
		// a page file or URL stays as written: a browser's file system shim made page:x into /page:x (card takes-three)
		let address =
			module.to_string_lossy().starts_with(crate::web::PAGE_PREFIX) || module.to_string_lossy().contains("://");
		let canonical = if address { None } else { module.canonicalize().ok() };
		let path = canonical.unwrap_or_else(|| module.to_path_buf()).display().to_string();
		self.wasm_modules.push(path.clone());
		let used = match as_use(statement) {
			Node::List(_, bracket, separator) => {
				Node::List(vec![Node::Symbol(USE_KEYWORD.to_string()), Node::Text(path.clone())], bracket, separator)
			}
			other => other,
		};
		let entry = (import == Import::Include).then(|| crate::modules::wasm_modules::entry_call(&path)).flatten();
		std::iter::once(used).chain(entry).collect()
	}

	/// A file that cannot be found: an error listing where it was looked for
	pub(super) fn not_found(&self, name: &str, import: Import) -> Node {
		if import == Import::Use {
			return error(&format!("module not found: {name}"));
		}
		let searched: Vec<String> = self
			.candidates(name)
			.iter()
			.map(|path| path.display().to_string())
			.map(|path| if path.starts_with(['.', '/']) { path } else { format!("./{path}") })
			.collect();
		error(&format!("include not found: {name}; searched {}", searched.join(", ")))
	}
}

/// A module's path as failures name it: without `./` steps (`./././greeting.warp` is `greeting.warp`)
/// Whether insert_module_classes put the module's classes in front of the program already
fn classes_early(import: Import, file: &PathBuf) -> bool {
	import == Import::Use
		&& crate::compiler::compile_context::read(|context| context.early_class_modules.contains(file))
}

/// A parsed module after the passes it runs before joining the program (pipeline::lower_module_source)
fn lowered_source(path: &Path, module: Node, classes_early: bool) -> Result<Node, Node> {
	let shown = shown_path(path);
	// classes insert_module_classes put in front of the program already: left out before the module's passes lower them
	// alias lines are the loader's (modules::resolve), no code of the module
	let kept = |statement: &Node| !(classes_early && is_class(statement)) && word_alias(statement).is_none();
	let module = sequence(statements(module).into_iter().filter(kept).collect());
	let module = crate::compiler::phase_times::timed_as(
		|| format!("module source passes of {}", shown.display()),
		|| crate::pipeline::lower_module_source(module),
	);
	// a pass that refuses the module (a keyword defined) gives its error in place of the module's statements
	match module.drop_meta() {
		Node::Error(_) => Err(crate::errors::diagnostic::located_in_file(&module, &shown)),
		_ => Ok(module),
	}
}

pub(crate) fn shown_path(path: &Path) -> PathBuf {
	path.components().filter(|component| !matches!(component, std::path::Component::CurDir)).collect()
}

/// The program with each group of imported statements spliced in place, `hidden` renamed apart in them
fn with_imports_renamed(node: Node, hidden: &[String]) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items = items.into_iter().flat_map(|item| match imported_statements(&item) {
				Some(statements) => renamed_apart(statements, hidden),
				None => vec![with_imports_renamed(item, hidden)],
			});
			Node::List(items.collect(), bracket, separator)
		}
		node => match imported_statements(&node) {
			Some(statements) => sequence(renamed_apart(statements, hidden)),
			None => node.map_children(|child| with_imports_renamed(child, hidden)),
		},
	}
}

/// The statements a marked group holds (imported_by)
fn imported_statements(node: &Node) -> Option<Vec<Node>> {
	match node {
		Node::Meta { node, data } if data.is_symbol(IMPORTED_STATEMENTS) => Some(statements(*node.clone())),
		_ => None,
	}
}

/// The prelude's definitions the program's statements reach, and those their bodies reach once lowered (a pass may
/// write a call of another prelude word): lowering all of lib/prelude.warp for each program cost more than compiling
/// a small one (card cache-lowered)
fn reached_prelude_definitions(program: &[Node]) -> Vec<Node> {
	crate::compiler::phase_times::timed("prelude reach look-ahead", || reached_prelude_definitions_untimed(program))
}

fn reached_prelude_definitions_untimed(program: &[Node]) -> Vec<Node> {
	let reachable = |callers: &Node| {
		needed_definitions(callers, &prelude_declarations_reached(&called_names(statements_of(callers))))
	};
	let mut reached = reachable(&sequence(program.to_vec()));
	// a look ahead: the program's own fresh names and hints are not this lowering's
	// each definition lowered alone, so a kept one serves every program reaching it; what the lowerings write in the
	// compile context is put back after
	let lowered_aside = |definitions: Vec<Node>| -> Vec<Node> {
		let fresh_number = crate::compiler::compile_context::read(|context| context.fresh_number);
		let effects = crate::compiler::compile_context::LoweredEffects::now();
		let lowered: Vec<Node> = crate::compiler::compile_context::scoped(
			|context| &mut context.fresh_number,
			fresh_number,
			|| crate::normalize::without_hints(|| definitions.iter().flat_map(lowered_prelude_definition).collect()),
		);
		effects.apply();
		lowered
	};
	loop {
		let lowered = lowered_aside(reached.clone());
		let callers = sequence([program, &lowered].concat());
		let more = reachable(&callers);
		if more.len() == reached.len() {
			return reached;
		}
		reached = more;
	}
}
