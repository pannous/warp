//! Method syntax for builtin functions: `x.round()` is `round(x)`; `listOf(1, 2)` and `list()` are lists
use super::*;
use crate::node::parens;

/// Method syntax for builtin functions: the called method `x.f(args)` is `f(x, args)` when f is a rounding, extremum,
/// text or libm function the program does not define. Runs first, so the passes that know these calls see their plain form; user
/// functions are called so later, in `method_call`
pub fn lower_function_methods(node: Node) -> Node {
	let context = crate::analyzer::function_definitions(&node);
	let arities: HashMap<String, usize> =
		context.user_functions.iter().map(|(name, function)| (name.clone(), function.params.len())).collect();
	let mut defined: HashSet<String> = arities.keys().cloned().collect();
	crate::type_tests::collect_outer_assigned_names(&node, &mut defined);
	// a libc function is no method: `m.remove(k)` is the map's, not stdio's remove(path)
	let is_libm = |name: &str| crate::ffi::get_ffi_signature(name).is_some_and(|signature| signature.library != "c");
	let is_builtin = |name: &str, arguments: usize| {
		let is_text_builtin =
			crate::wasm_emitter::text::text_builtins::text_builtin_kind(name, arguments + 1).is_some();
		crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name)
			|| crate::min_max::is_extremum(name)
			|| is_text_builtin
			|| is_libm(name)
	};
	// `xs.map(square)` of a user function map(list, fn): the receiver is its first argument, before function values
	// are made (function_values.rs), which would otherwise see map(square)
	let takes_receiver = |name: &str, arguments: usize| arities.get(name) == Some(&(arguments + 1));
	let node = list_constructor_calls(node, &defined);
	function_methods_as_calls(node, &defined, &|name, arguments, defined| {
		(is_builtin(name, arguments) && !defined.contains(name)) || takes_receiver(name, arguments)
	})
}

/// A function definition rewritten in its own scope: the names its body assigns join the defined ones there only
fn in_function_scope(
	node: &Node,
	defined: &HashSet<String>,
	rewrite: impl Fn(Node, &HashSet<String>) -> Node,
) -> Option<Node> {
	let (head, body, rebuild) = crate::type_tests::function_definition(node)?;
	let mut locals = defined.clone();
	collect_assigned_names(&body, &mut locals);
	Some(rebuild(head, rewrite(body, &locals)))
}

/// Python's `list()`: the empty list, as `[]` parses; Kotlin's `listOf(1, 2)`, `arrayOf`, `mutableListOf`: the list
/// `[1, 2]` (`listOf()` is `[]` too); a word the program defines keeps its call
fn list_constructor_calls(node: Node, defined: &HashSet<String>) -> Node {
	if let Some(scoped) = in_function_scope(&node, defined, list_constructor_calls) {
		return scoped;
	}
	let is_constructor = |word: &Node| matches!(word.drop_meta(), Node::Symbol(name) if (name == LIST_WORD || LIST_CONSTRUCTORS.contains(&name.as_str())) && !defined.contains(name));
	match node {
		Node::List(items, Bracket::Round, Separator::None) if items.first().is_some_and(is_constructor) => {
			let elements: Vec<Node> = match &items[1..] {
				[Node::List(arguments, Bracket::Round, _)] => arguments.clone(),
				arguments => arguments.to_vec(),
			};
			let is_list_word = items[0].is_symbol(LIST_WORD);
			match elements.is_empty() {
				true => Node::Empty,
				false if is_list_word => {
					parens(items.into_iter().map(|item| list_constructor_calls(item, defined)).collect())
				} // `list(x)` is `x as list`
				false => Node::List(
					elements.into_iter().map(|element| list_constructor_calls(element, defined)).collect(),
					Bracket::Square,
					Separator::Colon,
				),
			}
		}
		other => other.map_children(|child| list_constructor_calls(child, defined)),
	}
}

fn function_methods_as_calls(
	node: Node,
	defined: &HashSet<String>,
	is_function: &dyn Fn(&str, usize, &HashSet<String>) -> bool,
) -> Node {
	if let Some(scoped) =
		in_function_scope(&node, defined, |body, locals| function_methods_as_calls(body, locals, is_function))
	{
		return scoped;
	}
	let lowered = |node: Node| function_methods_as_calls(node, defined, is_function);
	match node {
		Node::Key(receiver, Op::Dot, method) => {
			let (receiver, method) = (lowered(*receiver), lowered(*method));
			match method.drop_meta() {
				Node::List(items, Bracket::Round, _)
					if let [word] = items.as_slice()
						&& let Some(name) = word.symbol_name().filter(|name| !defined.contains(*name))
						&& let Some(operation) = prefix_operator_method(&receiver, name) =>
				{
					operation
				}
				Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if is_function(name, items.len() - 1, defined)) => {
					parens([vec![items[0].clone(), receiver], items[1..].to_vec()].concat())
				}
				_ => key(receiver, Op::Dot, method),
			}
		}
		other => children_rewritten(other, lowered),
	}
}
