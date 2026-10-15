//! Function names: an operator word or a type word cannot name a function
use super::*;

/// `norm` is the prelude's word (the Euclidean length of a list, decision S3), named and refused as a function like abs
const OPERATOR_LIKE_WORDS: [(&str, Op); 1] = [("norm", Op::Abs)];

/// The error for a user function named like a type word; `{name}` is the word
pub(crate) const TYPE_WORD_FUNCTION_CLASH: &str = "{name} is a type; rename your function";

/// `sqrt(x) := …`, `def abs(x): …`: a prefix operator word reads as the operator, so the definition could never be
/// called (P141); it arrives as `(√ ø x) := …`. Checked on the source, before a pass reads the operator
pub fn check_operator_word_functions(program: &Node) -> Option<Diagnostic> {
	// a method may be named so: `p.norm()` reads as the method (card class-method-named)
	let mut methods = std::collections::HashSet::new();
	program.visit(&mut |node| {
		if let Node::Type { body, .. } = node {
			body.visit(&mut |part| {
				methods.insert(part as *const Node);
			});
		}
	});
	let mut clash = None;
	program.visit(&mut |node| {
		if clash.is_some() || methods.contains(&(node as *const Node)) {
			return;
		}
		let defined = match node {
			Node::Key(head, Op::Define | Op::Assign, _) => operator_word(head),
			Node::List(items, _, _) => match (items.first(), items.get(1).map(Node::drop_meta)) {
				(Some(def), Some(Node::Key(head, Op::Colon, _)))
					if crate::operators::is_function_keyword(&crate::declarations::word(def)) =>
				{
					operator_word(head)
				}
				(Some(def), Some(head)) if crate::operators::is_function_keyword(&crate::declarations::word(def)) => {
					operator_word(head)
				}
				_ => None,
			},
			_ => None,
		};
		clash = defined.map(|(word, op)| (node.clone(), word, op));
	});
	let (definition, word, op) = clash?;
	Some(Diagnostic::at(&definition, format!("{word} is an operator ({op}); rename your function")))
}

/// The operator word a definition's head names: `norm(x)` keeps its name (the parser's defines_named), also as the
/// first part of `fun norm(x) { … }`; `not(x)` arrives as the operator
pub(crate) fn operator_word(head: &Node) -> Option<(String, Op)> {
	match head.drop_meta() {
		Node::Key(empty, op, _) if empty.is_nothing() => crate::warp_parser::PREFIX_OPERATOR_WORDS
			.iter()
			.find(|(_, known)| known == op)
			.map(|(word, op)| (word.to_string(), *op)),
		Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => crate::warp_parser::PREFIX_OPERATOR_WORDS
				.iter()
				.chain(&OPERATOR_LIKE_WORDS)
				.find(|(word, _)| word == name)
				.map(|(word, op)| (word.to_string(), *op)),
			Some(call @ Node::List(..)) => operator_word(call),
			_ => None,
		},
		_ => None,
	}
}

/// `double := it*2` or `double(x) := …`: a type word names a type, never a function (user decision P20)
pub(crate) fn check_type_word_functions(program: &Node) -> Option<Diagnostic> {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, program);
	let mut clashes: Vec<&String> = ctx.user_functions.keys().filter(|name| type_word_kind(name).is_some()).collect();
	clashes.sort();
	let name = clashes.first()?;
	let mut definition = None;
	program.visit(&mut |node| {
		if let (None, Node::Key(left, Op::Define | Op::Assign, _)) = (definition, node) {
			let head = match left.drop_meta() {
				Node::List(items, _, _) => items.first().map(Node::drop_meta),
				other => Some(other),
			};
			if matches!(head, Some(Node::Symbol(word)) if word == *name) {
				definition = Some(node);
			}
		}
	});
	Some(Diagnostic::at(definition.unwrap_or(program), TYPE_WORD_FUNCTION_CLASH.replace("{name}", name)))
}
