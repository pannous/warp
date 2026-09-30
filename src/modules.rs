//! `use <name>` file modules: `name.wasp` or `name.warp` is loaded and its definitions become part of the program.
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::wasp_parser::WaspParser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Where modules are looked for, in this order: the current directory, the samples, the library
pub const SEARCH_DIRECTORIES: [&str; 6] = [".", "include", "lib", "src", "source", "samples"];
pub const MODULE_EXTENSIONS: [&str; 2] = ["wasp", "warp"];
/// `use x`, and its aliases `require x` and `import x`: the declarations of the file x
const USE_KEYWORDS: [&str; 3] = ["use", "require", "import"];
/// `include x`: the whole file, spliced in place
const INCLUDE_KEYWORD: &str = "include";
const USE_KEYWORD: &str = "use";
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
	loader.resolve(program).unwrap_or_else(|failure| failure)
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
		if let Some((import, name)) = used_module(&node) {
			return Ok(match self.import(import, &name, &node)? {
				imported if imported.is_empty() => Node::Empty,
				imported => Node::List(imported, Bracket::None, Separator::Semicolon),
			});
		}
		match node {
			Node::List(items, bracket, separator) => {
				let mut resolved = Vec::with_capacity(items.len());
				for item in items {
					match used_module(&item) {
						Some((import, name)) => resolved.extend(self.import(import, &name, &item)?),
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
	fn import(&mut self, import: Import, name: &str, statement: &Node) -> Result<Vec<Node>, Node> {
		if import == Import::Use && is_builtin_library(name) {
			return Ok(vec![as_use(statement)]);
		}
		let Some(path) = self.find(name) else {
			return match import == Import::Use && crate::ffi::is_ffi_library(name) {
				true => Ok(vec![as_use(statement)]),
				false => Err(self.not_found(name, import)),
			};
		};
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
		let outer_directory = std::mem::replace(&mut self.including_directory, path.parent().map(Path::to_path_buf));
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

/// `use name`, `require name`, `import name` and `include name`: the name is a symbol, a text or a path `lib/name`, `name.wasp`
fn used_module(node: &Node) -> Option<(Import, String)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [keyword, argument] = items.as_slice() else { return None };
	let Node::Symbol(keyword) = keyword.drop_meta() else { return None };
	let import = if USE_KEYWORDS.contains(&keyword.as_str()) {
		Import::Use
	} else if keyword == INCLUDE_KEYWORD {
		Import::Include
	} else {
		return None;
	};
	Some((import, path_of(argument)?))
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
