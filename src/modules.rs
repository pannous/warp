//! `use <name>` file modules: `name.wasp` or `name.warp` is loaded and its definitions become part of the program.
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::wasp_parser::WaspParser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Where modules are looked for, in this order: the current directory, the samples, the library
pub const SEARCH_DIRECTORIES: [&str; 3] = [".", "samples", "lib"];
pub const MODULE_EXTENSIONS: [&str; 2] = ["wasp", "warp"];
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
	let mut loader = Loader { directories, loaded: HashSet::new() };
	loader.resolve(program).unwrap_or_else(|failure| failure)
}

struct Loader<'a> {
	directories: &'a [&'a str],
	loaded: HashSet<PathBuf>,
}

impl Loader<'_> {
	fn resolve(&mut self, node: Node) -> Result<Node, Node> {
		if let Some(name) = used_module(&node) {
			return Ok(match self.import(&name, &node)? {
				imported if imported.is_empty() => Node::Empty,
				imported => Node::List(imported, Bracket::None, Separator::Semicolon),
			});
		}
		match node {
			Node::List(items, bracket, separator) => {
				let mut resolved = Vec::with_capacity(items.len());
				for item in items {
					match used_module(&item) {
						Some(name) => resolved.extend(self.import(&name, &item)?),
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
	fn import(&mut self, name: &str, statement: &Node) -> Result<Vec<Node>, Node> {
		if is_builtin_library(name) {
			return Ok(vec![statement.clone()]);
		}
		let Some(path) = self.find(name) else {
			return match crate::ffi::is_ffi_library(name) {
				true => Ok(vec![statement.clone()]),
				false => Err(error(&format!("module not found: {name}"))),
			};
		};
		if !self.loaded.insert(path.canonicalize().unwrap_or_else(|_| path.clone())) {
			return Ok(vec![]);
		}
		let source = std::fs::read_to_string(&path).map_err(|failure| error(&format!("module not readable: {name}: {failure}")))?;
		let module = WaspParser::parse(&source);
		if let Some(failure) = module.first_error() {
			return Err(failure.clone());
		}
		let module = self.resolve(module)?;
		Ok(statements(module).into_iter().filter(is_declaration).collect())
	}

	fn find(&self, name: &str) -> Option<PathBuf> {
		if Path::new(name).components().count() != 1 {
			return None;
		}
		self.directories.iter().flat_map(|directory| {
			MODULE_EXTENSIONS.iter().map(move |extension| Path::new(directory).join(format!("{name}.{extension}")))
		}).find(|path| path.is_file())
	}
}

fn is_builtin_library(name: &str) -> bool {
	matches!(crate::ffi::resolve_library_alias(name), "m" | "c")
}

/// `use name`
fn used_module(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 => match (items[0].drop_meta(), items[1].drop_meta()) {
			(Node::Symbol(keyword), Node::Symbol(name)) if keyword == USE_KEYWORD => Some(name.clone()),
			_ => None,
		},
		_ => None,
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
