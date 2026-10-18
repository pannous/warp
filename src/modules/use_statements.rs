//! The statements naming modules: `use x`, `include x`, `import x`, `from x import y` and the paths and versions they give
use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Import {
	/// the declarations of the file
	Use,
	/// the whole file
	Include,
}

pub(super) struct Used {
	pub(super) import: Import,
	pub(super) name: String,
	pub(super) requirement: Option<Requirement>,
}

/// `use x`, and its aliases `require x` and `import x`: the declarations of the file x
pub(crate) const USE_KEYWORDS: [&str; 3] = ["use", "require", "import"];
/// `include x`: the whole file, spliced in place
pub(crate) const INCLUDE_KEYWORD: &str = "include";
pub(crate) const USE_KEYWORD: &str = "use";
/// `from list import zip, unique`: only those words of the module
pub(crate) const IMPORT_KEYWORD: &str = "import";

/// `fast` alone as the first statement of a program or module (user 2026-10-16, card fast-scope): the program's sin cos
/// exp pow atan2 are lib/fast_math.warp's polynomials, its Ints machine i64s that wrap, its `/` of Ints a float
pub(crate) fn is_fast_pragma(statement: &Node) -> bool {
	statement.drop_meta().is_symbol(crate::words::FAST_WORD)
}

/// The statements after a first `fast`, and whether there was one
pub(super) fn without_fast_pragma(code: Node) -> (Node, bool) {
	match code {
		Node::List(mut items, bracket, separator) if items.first().is_some_and(is_fast_pragma) => {
			items.remove(0);
			(Node::List(items, bracket, separator), true)
		}
		other => (other, false),
	}
}

/// A fast program's pragma as the module it means: `use fast_math`
pub(super) fn fast_math_use() -> Node {
	use_of(Node::Symbol(FAST_MODULE.into()))
}

/// `use module`
fn use_of(module: Node) -> Node {
	Node::List(vec![Node::Symbol(USE_KEYWORD.into()), module], Bracket::None, Separator::Space)
}

/// `fast f(x) := body`, or any `f(x) := body` of a fast module: the head and the body
pub(super) fn fast_definition(statement: &Node, fast_module: bool) -> Option<(Node, Node)> {
	let definition = match statement.drop_meta() {
		Node::List(items, Bracket::None, _) => match items.as_slice() {
			[modifier, definition] if is_fast_pragma(modifier) => definition,
			_ => return None,
		},
		definition if fast_module => definition,
		_ => return None,
	};
	match definition.drop_meta() {
		Node::Key(name, Op::Define, body) => Some((name.as_ref().clone(), body.as_ref().clone())),
		_ => None,
	}
}

/// fast_math's `sin` as a `fast` function calls it: `fast_math·sin`
pub(super) fn fast_name(word: &str) -> String {
	format!("{FAST_MODULE}{QUALIFIER}{word}")
}

/// The node with fast_math's `words` called by their qualified names (fast_name)
pub(super) fn with_fast_names(node: Node, words: &[String]) -> Node {
	renamed_words(node, &words.iter().map(|word| (word.clone(), fast_name(word))).collect())
}

/// `a / b` of fast code as `(a as float) / b`: a float, never an exact ratio
pub(crate) fn float_division(node: Node) -> Node {
	match node {
		Node::Key(left, Op::Div, right) => {
			let left = Node::Key(
				Box::new(float_division(*left)),
				Op::As,
				Box::new(Node::Symbol(crate::words::FLOAT_WORD.into())),
			);
			Node::Key(Box::new(left), Op::Div, Box::new(float_division(*right)))
		}
		other => other.map_children(float_division),
	}
}

/// `use x`, `require x`, `import x`, `include x`: a statement that brings in a module, the one list every pass asks
pub(crate) fn is_import_keyword(word: &str) -> bool {
	USE_KEYWORDS.contains(&word) || word == INCLUDE_KEYWORD
}

pub(crate) fn is_builtin_library(name: &str) -> bool {
	matches!(crate::ffi::resolve_library_alias(name), "m" | "c")
}

/// The words of a statement, commas flattened: the parser groups `use list` before the comma of `use list, text`
pub(crate) fn statement_words(node: &Node) -> Option<Vec<&Node>> {
	match node.drop_meta() {
		Node::List(items, _, Separator::Comma) => match items.split_first()?.0.drop_meta() {
			Node::List(head, _, _) => Some(head.iter().chain(&items[1..]).collect()),
			_ => None,
		},
		Node::List(items, _, _) => Some(items.iter().collect()),
		_ => None,
	}
}

/// The modules a `use` statement names, each with the statement that uses it alone: `use list, text` and
/// `use list text` (card std-use) use each; `use js Math` is a foreign module (lowering/foreign/foreign_modules.rs), not two
pub(super) fn used_modules(node: &Node) -> Option<Vec<(Used, Node)>> {
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
			let statement = Node::List(vec![(*keyword).clone(), (*name).clone()], Bracket::None, Separator::Space);
			used_module(&statement).map(|used| (used, statement))
		})
		.collect()
}

/// `from list import zip, unique`: the module, the statement that uses it, and the only words it brings (card std-import)
pub(super) fn imported_words(node: &Node) -> Option<(Used, Node, Vec<String>)> {
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
pub(super) fn std_words_from(module: &Node, names: &[&Node]) -> Option<(Used, Node, Vec<String>)> {
	let defined = std_module_definitions(std_module_name(module.symbol_name()?))?;
	let imported = words_from(module, names)?;
	imported.2.iter().all(|name| defined.contains(name)).then_some(imported)
}

/// The module `use`d and the words named, alone or in one group: `zip, unique`, `(zip, unique)`
pub(super) fn words_from(module: &Node, names: &[&Node]) -> Option<(Used, Node, Vec<String>)> {
	let names = match names {
		[group] if matches!(group.drop_meta(), Node::List(_, Bracket::Round, _)) => group.drop_meta().iter().collect(),
		_ => names.iter().map(|name| (*name).clone()).collect::<Vec<Node>>(),
	};
	let names = names
		.iter()
		.map(|name| name.symbol_name().map(String::from))
		.collect::<Option<Vec<String>>>()
		.filter(|names| !names.is_empty())?;
	let statement = use_of(module.clone());
	used_module(&statement).map(|used| (used, statement, names))
}

/// `use name`, `require name`, `import name` and `include name`: the name is a symbol, a text or a path `lib/name`, `name.warp`.
/// A version may follow: `use name version 1.2.3` exactly that one, `use name from 1.2.3` / `use name >= 1.2.3` that or later
pub(super) fn used_module(node: &Node) -> Option<Used> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let [keyword, argument, rest @ ..] = items.as_slice() else {
		return None;
	};
	let keyword = keyword.symbol_name()?;
	let import = if USE_KEYWORDS.contains(&keyword) {
		Import::Use
	} else if keyword == INCLUDE_KEYWORD {
		Import::Include
	} else {
		return None;
	};
	let (name, requirement) = match (argument.drop_meta(), rest) {
		(Node::Key(name, Op::Ge, minimum), []) => (path_of(name)?, Some(Requirement(Op::Ge, version_of(minimum)?))),
		(name, []) => (path_of(name)?, None),
		(name, [version]) if version_of(version).is_some() && is_version_list(version) => {
			(path_of(name)?, Some(Requirement(Op::Eq, version_of(version)?)))
		}
		(name, [word, version]) if word.is_symbol(MINIMUM_KEYWORD) => {
			(path_of(name)?, Some(Requirement(Op::Ge, version_of(version)?)))
		}
		(name, [word, version]) if is_version_keyword(word) => {
			(path_of(name)?, Some(Requirement(Op::Eq, version_of(version)?)))
		}
		_ => return None,
	};
	Some(Used { import, name, requirement })
}

/// `version 1.2.3`, as the parser binds it
pub(crate) fn is_version_list(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(is_version_keyword))
}

/// A path or a repository as written: a text, a word, or `folder/name`, `owner/repo.js` without quotes
pub(crate) fn path_of(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		Node::Key(directory, Op::Div, name) => Some(format!("{}/{}", path_of(directory)?, path_of(name)?)),
		Node::Key(name, Op::Dot, extension) => Some(format!("{}.{}", path_of(name)?, path_of(extension)?)),
		_ => None,
	}
}

/// A path naming its file itself rather than a name to search for: absolute, or starting at `./` or `../`
pub(crate) fn is_explicit_path(name: &str) -> bool {
	Path::new(name).is_absolute() || name.starts_with("./") || name.starts_with("../")
}

/// A relative path that stays below its directory
pub(crate) fn is_plain_relative_path(name: &str) -> bool {
	Path::new(name).components().all(|component| matches!(component, std::path::Component::Normal(_)))
}

/// `require x` and `import x` of a native library stay in the program as `use x`
pub(crate) fn as_use(statement: &Node) -> Node {
	match statement {
		Node::List(items, bracket, separator) if items.len() == 2 => Node::List(
			vec![Node::Symbol(USE_KEYWORD.to_string()), items[1].clone()],
			bracket.clone(),
			separator.clone(),
		),
		other => other.clone(),
	}
}
