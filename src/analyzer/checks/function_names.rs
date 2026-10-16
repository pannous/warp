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
				// `norm x := …` without parentheses: the word applied to the definition of its parameter
				(Some(word), Some(Node::Key(_, Op::Define, _))) if items.len() == 2 => {
					word.drop_meta().symbol_name().and_then(operator_named)
				}
				_ => None,
			},
			_ => None,
		};
		// `sqrt = 5` names a variable
		let noun = match node {
			Node::Key(head, Op::Assign, _) if matches!(head.drop_meta(), Node::Key(_, _, operand) if operand.is_nothing()) => {
				VARIABLE_NOUN
			}
			_ => FUNCTION_NOUN,
		};
		let named = defined.map(|(word, op)| (word, op, noun)).or_else(|| bound_operator_word(node));
		clash = named.map(|(word, op, noun)| (node.clone(), word, op, noun));
	});
	let (definition, word, op, noun) = clash?;
	Some(Diagnostic::at(&definition, format!("{word} is an operator ({op}); rename your {noun}")))
}

const FUNCTION_NOUN: &str = "function";
const VARIABLE_NOUN: &str = "variable";
const PARAMETER_NOUN: &str = "parameter";

/// `for sqrt in xs`, `for each abs in xs`, `g(sqrt) := …`: a loop variable or parameter named by an operator word,
/// which its body would read as the operator, as `sqrt = 5` would (card library-word)
fn bound_operator_word(node: &Node) -> Option<(String, Op, &'static str)> {
	let bare_operator = |part: &Node| match part.drop_meta() {
		Node::Key(empty, op, operand) if empty.is_nothing() && operand.is_nothing() => {
			operator_word(part).filter(|(_, known)| known == op)
		}
		_ => None,
	};
	match node {
		Node::List(items, _, _) if items.first().is_some_and(|word| word.is_symbol("for")) => {
			let variable = if items.get(1).is_some_and(|word| word.is_symbol("each")) { 2 } else { 1 };
			let variable = items.get(variable)?;
			let named = variable.drop_meta().symbol_name().and_then(operator_named);
			named.or_else(|| bare_operator(variable)).map(|(word, op)| (word, op, VARIABLE_NOUN))
		}
		Node::Key(head, Op::Define | Op::Assign, _) => match head.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.first().is_some_and(|name| name.symbol_name().is_some()) => {
				items[1..].iter().find_map(bare_operator).map(|(word, op)| (word, op, PARAMETER_NOUN))
			}
			_ => None,
		},
		_ => None,
	}
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
			Some(Node::Symbol(name)) => operator_named(name),
			Some(call @ Node::List(..)) => operator_word(call),
			_ => None,
		},
		_ => None,
	}
}

fn operator_named(name: &str) -> Option<(String, Op)> {
	crate::warp_parser::PREFIX_OPERATOR_WORDS
		.iter()
		.chain(&OPERATOR_LIKE_WORDS)
		.find(|(word, _)| *word == name)
		.map(|(word, op)| (word.to_string(), *op))
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
