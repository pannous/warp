//! Doc strings (card doc-strings, decision reflection): a text literal as the first of several statements of a function
//! body is the function's doc and is not evaluated, as in Python; the comment right above a definition is its doc too.
//! Both become the `doc` meta entry of the body, which user_function reads into UserFunctionDef::doc for `f.doc` and
//! `help(f)` (reflection.rs). In reflection mode (`use reflection`) the body carries where the definition was written
//! too, the `written_at` entry: a function value's .line, .column and .file

use crate::node::{symbol, Bracket, Node};
use crate::operators::Op;

pub const DOC_KEY: &str = "doc";
const COMMENT_KEY: &str = "comment";
const WRITTEN_AT_KEY: &str = "written_at";

pub fn lower(node: Node) -> Node {
	match node {
		Node::List(statements, bracket, separator) if separator.separates_statements() => {
			Node::List(statements.into_iter().map(documented).collect(), bracket, separator)
		}
		statement => documented(statement),
	}
}

/// A statement whose functions carry their docs on their bodies; any other statement with its blocks looked into
fn documented(statement: Node) -> Node {
	let mut definitions = crate::compiler::context::Context::new();
	crate::analyzer::extract_user_functions_inner(&mut definitions, &statement);
	let functions: Vec<_> = definitions
		.user_functions
		.into_values()
		.filter(|function| !function.name.contains(crate::analyzer::NESTED_DEF_SEPARATOR))
		.collect();
	if functions.is_empty() {
		return statement.map_children(inner_statements_documented);
	}
	let comment = leading_comment(&statement).map(str::to_string);
	let position = crate::compiler::compile_context::read(|context| context.reflection_mode)
		.then(|| first_position(&statement))
		.flatten();
	functions.into_iter().fold(statement, |statement, function| {
		let (text, body) = leading_text(&function.body);
		let doc: Vec<String> = comment.iter().cloned().chain(text).collect();
		if doc.is_empty() && position.is_none() {
			return statement;
		}
		let body = match doc.is_empty() {
			true => body,
			false => with_doc(body, &doc.join("\n")),
		};
		let body = match &position {
			Some(position) => Node::meta(body, Node::key(WRITTEN_AT_KEY, Node::data(position.clone()))),
			None => body,
		};
		replaced(statement, &function.body, &body)
	})
}

/// The statement lists inside a statement that defines no function: a class body, a block
fn inner_statements_documented(node: Node) -> Node {
	match node.drop_meta() {
		Node::List(_, _, separator) if separator.separates_statements() => node.under_meta(lower),
		_ => node.map_children(inner_statements_documented),
	}
}

/// `{"doubles"; x*2}` → ("doubles", `{x*2}`); a body of one statement keeps its text as its value
fn leading_text(body: &Node) -> (Option<String>, Node) {
	if let Node::List(statements, bracket @ (Bracket::Curly | Bracket::Round | Bracket::None), separator) =
		body.drop_meta()
	{
		if let [first, rest @ ..] = statements.as_slice() {
			if let (Node::Text(doc), true) = (first.drop_meta(), separator.separates_statements() && !rest.is_empty()) {
				let rest = Node::List(rest.to_vec(), bracket.clone(), separator.clone());
				return (Some(doc.clone()), rest.with_meta_of(body));
			}
		}
	}
	(None, body.clone())
}

/// The body with its doc inside its position's Meta layer, which stays outermost: std_aliases.rs tells a module
/// file's calls by it (get_lineinfo)
fn with_doc(body: Node, doc: &str) -> Node {
	match body {
		Node::Meta { node, data } if body.get_lineinfo().is_some() => Node::meta(with_doc(*node, doc), *data),
		body => {
			Node::meta(body, Node::Key(Box::new(symbol(DOC_KEY)), Op::Colon, Box::new(Node::Text(doc.to_string()))))
		}
	}
}

/// The doc a function body carries (doc_strings::lower)
pub fn doc_of(body: &Node) -> Option<String> {
	match body {
		Node::Meta { node, data } => entry_text(data, DOC_KEY).map(str::to_string).or_else(|| doc_of(node)),
		_ => None,
	}
}

/// The first position the parser recorded in a statement, through its Meta layers
fn first_position(statement: &Node) -> Option<crate::compiler::meta::LineInfo> {
	statement.get_lineinfo().or_else(|| match statement {
		Node::Meta { node, .. } => first_position(node),
		Node::Key(left, _, right) => first_position(left).or_else(|| first_position(right)),
		Node::List(items, _, _) => items.iter().find_map(first_position),
		_ => None,
	})
}

/// Where a function body's definition was written, in reflection mode (doc_strings::lower)
pub fn written_at(body: &Node) -> Option<crate::compiler::meta::LineInfo> {
	match body {
		Node::Meta { node, data } => match data.as_ref() {
			Node::Key(name, _, value) if name.symbol_name() == Some(WRITTEN_AT_KEY) => match value.as_ref() {
				Node::Data(dada) => dada.downcast_ref::<crate::compiler::meta::LineInfo>().cloned(),
				_ => None,
			},
			_ => written_at(node),
		},
		_ => None,
	}
}

/// The comment the parser put on the first word of a statement
pub(crate) fn leading_comment(node: &Node) -> Option<&str> {
	match node {
		Node::Meta { node, data } => entry_text(data, COMMENT_KEY).or_else(|| leading_comment(node)),
		Node::Key(left, _, _) => leading_comment(left),
		Node::List(items, _, _) => items.first().and_then(leading_comment),
		_ => None,
	}
}

/// The text of a Meta layer's entry `key: "…"`
fn entry_text<'a>(data: &'a Node, key: &str) -> Option<&'a str> {
	match data {
		Node::Key(name, _, value) if name.symbol_name() == Some(key) => match value.drop_meta() {
			Node::Text(text) => Some(text),
			_ => None,
		},
		_ => None,
	}
}

/// `node` with each part equal to `old` replaced by `new`
fn replaced(node: Node, old: &Node, new: &Node) -> Node {
	match node == *old {
		true => new.clone(),
		false => node.map_children(|child| replaced(child, old, new)),
	}
}
