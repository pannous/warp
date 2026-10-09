//! wiki/type.md: C# 10's file-scoped declarations. `class Foo;` (no body) takes in the definitions after it, up to the
//! first other statement, as its body; `namespace test;` names the file's words: `test.bar()` is `bar()` and
//! `test.Foo` is `Foo`. Both must stand at the top level of the program.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const NAMESPACE_WORD: &str = "namespace";
/// The parser's attribute on the name of a class declared `class Foo;`
pub const FILE_CLASS_MARK: &str = "file_class";

/// The class name without the parser's mark, which is its outermost attribute
fn unmarked(name: Node) -> Node {
	match name {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name().ends_with(FILE_CLASS_MARK)) => *node,
		other => other,
	}
}

pub fn lower(program: Node) -> Node {
	let Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) = program else { return program };
	let mut namespaces: Vec<String> = vec![];
	let mut lowered = vec![];
	let mut statements = statements.into_iter().peekable();
	while let Some(statement) = statements.next() {
		if let Some(name) = namespace_name(&statement) {
			namespaces.push(name);
			continue;
		}
		match statement.drop_meta() {
			Node::Type { name, body } if name.attribute(FILE_CLASS_MARK).is_some() => {
				let members: Vec<Node> = std::iter::from_fn(|| statements.next_if(is_member)).collect();
				let body = match members.is_empty() {
					true => body.clone(),
					false => Box::new(crate::warp_parser::WarpParser::class_body(Node::List(members, Bracket::Curly, Separator::Semicolon))),
				};
				lowered.push(Node::Type { name: Box::new(unmarked(*name.clone())), body });
			}
			_ => lowered.push(statement),
		}
	}
	let program = Node::List(lowered, Bracket::None, separator);
	namespaces.iter().fold(program, |program, namespace| unqualified(program, namespace))
}

/// `namespace test`
fn namespace_name(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[word, name] if matches!(word.drop_meta(), Node::Symbol(word) if word == NAMESPACE_WORD) => match name.drop_meta() {
				Node::Symbol(name) => Some(name.clone()),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

/// A definition a file class takes in: a method `bar() := …`, `int bar(){…}`, `def bar…`, a field `x: int`
fn is_member(statement: &Node) -> bool {
	let is_declaring_word = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word)
		if crate::operators::is_function_keyword(word) || crate::analyzer::builtin_type_kind(word).is_some());
	match statement.drop_meta() {
		Node::Key(_, Op::Define, _) => true,
		Node::Key(target, Op::Assign, _) => matches!(target.drop_meta(), Node::List(_, Bracket::Round, _)),
		Node::Key(field, Op::Colon, _) => matches!(field.drop_meta(), Node::Symbol(_)),
		Node::List(items, _, _) => items.len() >= 2 && is_declaring_word(&items[0]),
		_ => false,
	}
}

/// `test.bar()` as `bar()`, `test.Foo` as `Foo`
fn unqualified(node: Node, namespace: &str) -> Node {
	match node {
		Node::Key(qualifier, Op::Dot, member) if matches!(qualifier.drop_meta(), Node::Symbol(name) if name == namespace) => unqualified(*member, namespace),
		other => other.map_children(|child| unqualified(child, namespace)),
	}
}
