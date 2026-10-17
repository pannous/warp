//! Calls: arity of math functions and one-value operators, braceless calls whose reading is ambiguous
use super::*;

/// `sin(0, 5)`, `pow(2)`, `√(16, 2)`: a math function or a one-value operator given another number of values, which
/// used to drop or invent values silently
pub(crate) fn check_call_arity(program: &Node) -> Option<Diagnostic> {
	let context = function_definitions(program);
	call_arity_error(program, &context)
}

pub(crate) fn call_arity_error(node: &Node, context: &Context) -> Option<Diagnostic> {
	let values = |count: usize| {
		if count == 1 {
			"1 value".to_string()
		} else {
			format!("{count} values")
		}
	};
	match node {
		Node::Meta { node, .. } => call_arity_error(node, context),
		// `import (sin, floor) from 'm'` names functions, it calls none
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::modules::is_import_keyword(word)) => {
			None
		}
		Node::List(items, bracket, _) => {
			if let (Bracket::Round, Some(Node::Symbol(name))) = (bracket, items.first().map(Node::drop_meta)) {
				let libm = crate::ffi::libm_f64_functions()
					.iter()
					.filter(|(function, _)| function == name)
					.map(|(_, arity)| *arity);
				let arities: Vec<usize> =
					libm.chain(crate::wasm_emitter::text::text_builtins::text_builtin_arities(name)).collect();
				let given = items.len() - 1;
				if !arities.is_empty() && !context.user_functions.contains_key(name) && !arities.contains(&given) {
					return Some(Diagnostic::at(node, format!("{name} takes {}, got {given}", values(arities[0]))));
				}
			}
			items.iter().find_map(|item| call_arity_error(item, context))
		}
		Node::Key(empty, op @ (Op::Sqrt | Op::Cbrt | Op::Abs), operand) if empty.is_nothing() => {
			match operand.drop_meta() {
				Node::List(items, Bracket::Round, Separator::Comma) if items.len() > 1 => {
					Some(Diagnostic::at(node, format!("{op} takes 1 value, got {}", operand.serialize())))
				}
				_ => call_arity_error(operand, context),
			}
		}
		Node::Key(left, _, right) => call_arity_error(left, context).or_else(|| call_arity_error(right, context)),
		_ => None,
	}
}

/// A braceless argument takes arithmetic (`f 3-1` is `f(3-1)`, also in `1 + f 3-1`), so a second braceless call inside it
/// is ambiguous (wiki/precedence.md): `square 3 + square 3` reads as `square(3 + square 3)` or `square(3) + square(3)`.
/// Bad.md's recursive `fib it-1 + fib it-2` would silently mean `fib(it-1 + fib(it-2))`, so it is rejected with both readings.
pub(crate) fn check_ambiguous_calls(node: &Node) -> Option<Diagnostic> {
	/// `return x`, `let x`, `def f`, `use m`: a statement word, no function applied braceless
	fn is_statement_word(word: &str) -> bool {
		is_declaration_word(word)
			|| crate::operators::is_function_keyword(word)
			|| crate::modules::is_import_keyword(word)
			|| RETURNING_KEYWORDS.contains(&word)
	}
	fn braceless_call(node: &Node) -> Option<(&String, &Node)> {
		match node.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 => match items[0].drop_meta() {
				Node::Symbol(head) if !is_statement_word(head) => Some((head, &items[1])),
				_ => None,
			},
			_ => None,
		}
	}
	fn holds_call(node: &Node) -> bool {
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => holds_call(left) || holds_call(right),
			other => braceless_call(other).is_some(),
		}
	}
	fn explicit(node: &Node) -> String {
		if let Some((head, argument)) = braceless_call(node) {
			return format!("{head}({})", explicit(argument));
		}
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => {
				format!("{} {} {}", explicit(left), op.as_str(), explicit(right))
			}
			other => other.serialize(),
		}
	}
	if let Some((head, argument)) = braceless_call(node) {
		if let Node::Key(left, op, right) = argument.drop_meta() {
			if op.is_arithmetic() && holds_call(argument) {
				let whole = format!("{head}({})", explicit(argument));
				let first = format!("{head}({}) {} {}", explicit(left), op.as_str(), explicit(right));
				let written = format!("{head} {}", argument.drop_meta().serialize());
				return Some(
					Diagnostic::at(node, format!("ambiguous braceless call: {written}"))
						.fix(format!("{first} or {whole}"))
						.offer(format!("{head} of {} only", left.serialize()), &written, &first)
						.offer(format!("{head} of the whole {}", argument.drop_meta().serialize()), &written, &whole),
				);
			}
		}
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => check_ambiguous_calls(left).or_else(|| check_ambiguous_calls(right)),
		Node::List(items, _, _) => items.iter().find_map(check_ambiguous_calls),
		_ => None,
	}
}
