//! `f -x` and `print #xs` as calls of the word before them

use crate::analyzer::{extract_user_functions_inner, is_type_call_word};
use crate::compiler::context::{Context, UserFunctionDef};
use crate::lowering::nodes::children_rewritten;
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

/// The printing words, which `print #xs` and `print -1` call like a function of the program (card print-people)
const PRINT_WORDS: [&str; 2] = ["print", "puts"];

/// `f -x` with a callable `f` is the call `f(-x)`, and `print #xs` is `print(#xs)` (card print-people): a user function,
/// a real function, print, a rounding word (`floor -2.5`), a type (`int -3`, `hex -1`) or a library word of one value, as the math words
/// take `abs -2`. A minus written apart, `f - 2`, subtracts, as `abs - 2` does (warp_parser::SpacedMinus). A built-in name
/// that the program also binds as a variable or parameter (`exp-1`) stays a subtraction.
pub fn lower_negated_calls(node: Node) -> Node {
	let mut ctx = Context::new();
	extract_user_functions_inner(&mut ctx, &node);
	let mut bound = ctx.parameter_names();
	crate::library_words::collect_assigned_names(&node, &mut bound);
	bound.extend(crate::soft_keywords::names_given(&node)); // fields and lambda parameters too: `size -1` of a field size
	negate_calls(node, &ctx.user_functions, &bound, &HashSet::new())
}

/// `locals`: the parameters and variables of the function around, which no function of the same name calls there
/// (`words = [p]; words#1` beside a function words, card local-variable-named)
fn negate_calls(
	node: Node,
	functions: &std::collections::BTreeMap<String, UserFunctionDef>,
	bound: &HashSet<String>,
	locals: &HashSet<String>,
) -> Node {
	if let Some((head, body, rebuild)) = crate::type_tests::function_definition(&node) {
		let mut inner = locals.clone();
		inner.extend(crate::lowering::memoization::head_parameters(&head));
		crate::library_words::collect_assigned_names(&body, &mut inner);
		return rebuild(head, negate_calls(body, functions, bound, &inner));
	}
	// `int[100]` is the subscript `int#101` of a typed array: a type or library word takes no count
	let is_function = |operand: &Node, op: Op| match operand.drop_meta() {
		Node::Symbol(name) if locals.contains(name) => false,
		Node::Symbol(name) if functions.contains_key(name) => {
			functions.get(name).is_some_and(|function| !function.params.is_empty())
		}
		Node::Symbol(name) if op == Op::Hash => {
			(crate::numbers::real::FUNCTIONS.contains(&name.as_str()) || PRINT_WORDS.contains(&name.as_str()))
				&& !bound.contains(name)
		}
		Node::Symbol(name) => is_builtin_callable(name) && !bound.contains(name),
		_ => false,
	};
	match node {
		Node::Key(function, op @ (Op::Sub | Op::Hash), argument)
			if is_function(&function, op) && !(op == Op::Sub && is_spaced_minus(&function)) =>
		{
			let prefix = if op == Op::Sub { Op::Neg } else { Op::Hash };
			let argument =
				Node::Key(Box::new(Node::Empty), prefix, Box::new(negate_calls(*argument, functions, bound, locals)));
			parens(vec![*function, argument])
		}
		// `-sin 0` negates the call `sin 0`, not the word sin (card braceless-calls)
		Node::List(items, bracket @ Bracket::None, separator @ Separator::Space)
			if items.len() > 1 && negated_word(&items[0]).is_some_and(|function| is_function(function, Op::Sub)) =>
		{
			let mut items = items;
			items[0] = negated_word(&items[0]).expect("guarded").clone();
			let call = negate_calls(Node::List(items, bracket, separator), functions, bound, locals);
			Node::Key(Box::new(Node::Empty), Op::Neg, Box::new(call))
		}
		other => children_rewritten(other, |child| negate_calls(child, functions, bound, locals)),
	}
}

/// The word `w` of `-w`
fn negated_word(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Key(nothing, Op::Neg, word) if nothing.is_empty_node() && matches!(word.drop_meta(), Node::Symbol(_)) => {
			Some(word)
		}
		_ => None,
	}
}

/// A built-in word that applies to the value after it: print, a rounding word, a type, a library word of one value
/// (`sin`, `floor`, `reverse`)
fn is_builtin_callable(name: &str) -> bool {
	crate::lowering::library_signatures::typed_first_arity(name) == Some(1)
		|| PRINT_WORDS.contains(&name)
		|| crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name)
		|| is_type_call_word(name)
		|| crate::library_words::is_operand_word(name)
}

/// The name before a minus written apart (warp_parser::SpacedMinus)
fn is_spaced_minus(operand: &Node) -> bool {
	match operand {
		Node::Meta { node, data } => {
			matches!(data.as_ref(), Node::Data(dada) if dada.downcast_ref::<crate::warp_parser::SpacedMinus>().is_some())
				|| is_spaced_minus(node)
		}
		_ => false,
	}
}
