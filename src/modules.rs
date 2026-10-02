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

/// Where modules are looked for, in this order: the current directory, the samples, the library
pub const SEARCH_DIRECTORIES: [&str; 6] = [".", "include", "lib", "src", "source", "samples"];
pub const MODULE_EXTENSIONS: [&str; 2] = ["wasp", "warp"];
/// `use x`, and its aliases `require x` and `import x`: the declarations of the file x
const USE_KEYWORDS: [&str; 3] = ["use", "require", "import"];
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
/// one fetch at a time within a process; parallel processes clone to their own staging directory and rename
static FETCHING: Mutex<()> = Mutex::new(());
/// Statements a module contributes; everything else in a module (expressions, calls) is its own business
const DECLARATION_KEYWORDS: [&str; 5] = ["use", "import", "let", "var", "global"];

fn is_declaration_keyword(keyword: &str) -> bool {
	DECLARATION_KEYWORDS.contains(&keyword) || crate::analyzer::CONSTANT_KEYWORDS.contains(&keyword)
}

/// Replace every `use <module>` of the program by the definitions of that module, each module once.
/// `use math` and other native libraries stay in place for the FFI; an unknown name is an error value.
pub fn resolve(program: Node) -> Node {
	resolve_in(program, &SEARCH_DIRECTORIES)
}

pub fn resolve_in(program: Node, directories: &[&str]) -> Node {
	let mut loader = Loader { directories, loaded: HashSet::new(), included: HashSet::new(), including_directory: None };
	loader.resolve(with_module_directory(program, Path::new("."))).unwrap_or_else(|failure| failure)
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
}

impl Loader<'_> {
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
		if import == Import::Use && is_builtin_library(name) {
			return Ok(vec![as_use(statement)]);
		}
		let Some(path) = self.find(name) else {
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
		let mut directory = fetch_package(name).map_err(|failure| error(&failure))?;
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

	fn load(&mut self, import: Import, name: &str, path: PathBuf) -> Result<Vec<Node>, Node> {
		let loaded = match import {
			Import::Use => &mut self.loaded,
			Import::Include => &mut self.included,
		};
		if !loaded.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
			return Ok(vec![]);
		}
		let source = std::fs::read_to_string(&path).map_err(|failure| error(&format!("module not readable: {name}: {failure}")))?;
		let module = WaspParser::parse(&source);
		if let Some(failure) = module.first_error() {
			return Err(failure.clone());
		}
		let directory = path.parent().map(Path::to_path_buf).unwrap_or_default();
		let module = with_module_directory(module, &directory);
		let outer_directory = std::mem::replace(&mut self.including_directory, Some(directory));
		let module = self.resolve(module);
		self.including_directory = outer_directory;
		let statements = statements(module?);
		Ok(match import {
			Import::Use => statements.into_iter().filter(is_declaration).collect(),
			Import::Include => statements,
		})
	}

	/// The file of a module: every search directory below the directory of the including file, then below the working
	/// directory; a name that already ends in an extension is tried as written
	fn find(&self, name: &str) -> Option<PathBuf> {
		self.candidates(name).into_iter().find(|path| path.is_file())
	}

	fn candidates(&self, name: &str) -> Vec<PathBuf> {
		if !is_plain_relative_path(name) {
			return vec![];
		}
		let has_extension = MODULE_EXTENSIONS.iter().any(|extension| name.ends_with(&format!(".{extension}")));
		let file_names: Vec<String> = match has_extension {
			true => vec![name.to_string()],
			false => MODULE_EXTENSIONS.iter().map(|extension| format!("{name}.{extension}")).collect(),
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

/// The directory of a package, packages/<name>: the pinned version, or a clone of the default branch;
/// whatever is there already stays, so a local checkout linked there stands in for the package
pub fn fetch_package(name: &str) -> Result<PathBuf, String> {
	let package = registered(name).ok_or(format!("unknown package: {name}"))?;
	match &package.pinned {
		Some(version) => fetch_tagged(name, &package.url, version, name),
		None => clone(name, &package.url, &Path::new(PACKAGES_DIRECTORY).join(name), None),
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
	fetch_tagged(name, &url, version, &format!("{name}@{version}"))
}

/// packages/<link_name>, a link to the version's shallow clone in the machine's package cache: fetched once per machine,
/// never again per checkout, export or test run. A link into the cache to another version (the pin moved) or to a
/// removed clone is replaced.
fn fetch_tagged(name: &str, url: &str, version: &Version, link_name: &str) -> Result<PathBuf, String> {
	let cached = package_cache().join(format!("{name}@{version}"));
	let link = Path::new(PACKAGES_DIRECTORY).join(link_name);
	let stale = |target: PathBuf| target.starts_with(package_cache()) && (target != cached || !cached.exists());
	if std::fs::read_link(&link).is_ok_and(stale) {
		std::fs::remove_file(&link).map_err(|failure| format!("package {name}: stale link {}: {failure}", link.display()))?;
	}
	if link.exists() {
		return Ok(link);
	}
	if !cached.exists() {
		let tags = version_tags(url).map_err(|failure| format!("package {name}: no versions from {url}: {failure}"))?;
		let tag = tags.iter().find(|(_, tagged)| tagged == version).map(|(tag, _)| tag.as_str())
			.ok_or(format!("package {name} has no tag of version {version} at {url}"))?;
		clone(name, url, &cached, Some(tag))?;
	}
	std::fs::create_dir_all(PACKAGES_DIRECTORY).map_err(|failure| format!("package {name}: {failure}"))?;
	match std::os::unix::fs::symlink(&cached, &link) {
		Err(failure) if !link.exists() => Err(format!("package {name}: not linked to {}: {failure}", cached.display())),
		_ => Ok(link),
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
	MODULE_EXTENSIONS.iter().map(|extension| directory.join(format!("{name}.{extension}"))).find(|path| path.is_file())
}

/// The version a module file declares with a top level `version 1.2.3`
fn module_version(path: &Path) -> Option<Version> {
	let source = std::fs::read_to_string(path).ok()?;
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

fn path_of(node: &Node) -> Option<String> {
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

fn is_declaration(statement: &Node) -> bool {
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

/// `require x` and `import x` of a native library stay in the program as `use x`
fn as_use(statement: &Node) -> Node {
	match statement {
		Node::List(items, bracket, separator) if items.len() == 2 => {
			Node::List(vec![Node::Symbol(USE_KEYWORD.to_string()), items[1].clone()], bracket.clone(), separator.clone())
		}
		other => other.clone(),
	}
}
