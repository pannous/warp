//! Compile-time lambdas. Functions are not first-class values yet, so a lambda is lowered where it stands:
//! - `f = x=>x*x`, `f = (x y)->x+y` define the function `f` at that point (captured variables by value, like every
//!   definition); `f 3` calls it; the parser itself reads `f = {it*2}` as the definition `f := {it*2}`
//! - `{x*x}(x=5)` defines an anonymous function and calls it at once
//! - `map [1 2 3] {it*it}`, `map(xs, x=>x+1)`, `xs.map(f)` over a literal block, a lambda or a defined function is a loop
//! - a lambda anywhere else is a closure (closures.rs); `map` over a value that is no function is the error `map needs a function, got …`

use crate::analyzer::{call_name, extract_user_functions};
use crate::context::Context;
use crate::diagnostic::Diagnostic;
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;
use std::cell::Cell;

pub const IMPLICIT_PARAMETER: &str = "it";
pub(crate) const ON_WORD: &str = "on";
const PARTIAL_LIST: &str = "partial_list";
const LIST_PLACEHOLDER: &str = "loop_list";
const START_PLACEHOLDER: &str = "loop_start";
const CALL_PLACEHOLDER: &str = "loop_call";

/// An iteration word over a list: the arguments between the list and the function, the arguments of the function
/// (`reduce` and `fold` take the accumulator and the item), and the loop it lowers to. `out`, `item`, `acc`, `list`, `index`
/// are replaced by fresh names. A loop appends with `out = out + [x]`, never `out.add(x)`, which a user function `add`
/// would take.
struct Iteration {
	word: &'static str,
	extra_arguments: usize,
	function_arguments: usize,
	template: &'static str,
}

/// Swift's `{ x in x*2 }` and its shorthand arguments `{ $0 + $1 }`
const SWIFT_IN: &str = "in";
const SHORTHAND_MARK: char = '$';
const MAX_SHORTHAND_ARGUMENTS: usize = 10;
const SORT_WORD: &str = "sort";
const REDUCE_WORD: &str = "reduce";
/// `any(xs)`, `all(xs)` without a function test the items themselves
const TRUTH_WORDS: [&str; 2] = ["any", "all"];
const TRUTH_ITEM: &str = "truth_item";
const FOLD_WORD: &str = "fold";
const EACH_WORD: &str = "each";
/// Iteration words as other languages spell them: sorting with a function (Python sorted, Ruby sort_by, Kotlin
/// sortedBy), each (JS and Kotlin forEach, Rust for_each, PHP foreach)
const ITERATION_SPELLINGS: [(&str, &[&str]); 2] = [
	(SORT_WORD, &["sorted", "sort_by", "sortBy", "sortedBy", "sorted_by"]),
	(EACH_WORD, &["forEach", "for_each", "foreach"]),
];
/// The label of the sorting function: Swift `sorted(by: >)`, Python `sorted(xs, key=…)`
const SORT_LABELS: [&str; 2] = ["by", "key"];
const COMPARISONS: [Op; 4] = [Op::Lt, Op::Gt, Op::Le, Op::Ge];

const ITERATIONS: [Iteration; 9] = [
	Iteration { word: "map", extra_arguments: 0, function_arguments: 1, template: "(out=[]; for item in loop_list { out = out + [loop_call] }; out)" },
	Iteration { word: "filter", extra_arguments: 0, function_arguments: 1, template: "(out=[]; for item in loop_list { if loop_call { out = out + [item] } }; out)" },
	Iteration { word: EACH_WORD, extra_arguments: 0, function_arguments: 1, template: "(value=ø; for item in loop_list { value = loop_call }; value)" },
	Iteration { word: "fold", extra_arguments: 1, function_arguments: 2, template: "(acc=loop_start; for item in loop_list { acc = loop_call }; acc)" },
	Iteration { word: "find", extra_arguments: 0, function_arguments: 1, template: "(found=ø; searching=1; for item in loop_list { if searching and loop_call { found = item; searching = 0 } }; found)" },
	Iteration { word: "any", extra_arguments: 0, function_arguments: 1, template: "(hit=0; for item in loop_list { if loop_call { hit = 1 } }; hit)" },
	Iteration { word: "all", extra_arguments: 0, function_arguments: 1, template: "(held=1; for item in loop_list { if not loop_call { held = 0 } }; held)" },
	Iteration {
		word: "reduce",
		extra_arguments: 0,
		function_arguments: 2,
		template: "(list=loop_list; if count(list) == 0 then empty_extremum(reduce) else (acc=list#1; index=2; while index <= count(list) { item=list#index; acc = loop_call; index=index+1 }; acc))",
	},
	// a stable insertion sort of a copy; loop_call is whether acc sorts before item (sort_order)
	Iteration {
		word: SORT_WORD,
		extra_arguments: 0,
		function_arguments: 2,
		template: "(out=[]; for value in loop_list { out = out + [value]; index=count(out); moving=1; while moving and index > 1 { acc=out#index; item=out#(index-1); if loop_call { out#index=item; out#(index-1)=acc; index=index-1 } else { moving=0 } } }; out)",
	},
];

pub(crate) struct Lambda {
	pub(crate) params: Vec<String>,
	pub(crate) body: Node,
	/// The parameters as written when one of them declares a type or a default (`(x:float, n = 2)=>…`): the definition
	/// keeps them
	pub(crate) written: Vec<Node>,
	/// TypeScript's result type `(a: number): number => …`
	pub(crate) result_type: Option<Node>,
}

impl Lambda {
	fn new(params: Vec<String>, body: Node) -> Self {
		Lambda { params, body, written: vec![], result_type: None }
	}
}

/// Lower the lambdas and iteration words whose function is known; an iteration word over a parameter is left for the
/// specialisation of its function (function_values.rs)
pub fn lower(node: Node) -> Node {
	lowering(node, false)
}

/// The final run: an iteration word over a function known only at run time calls it as a closure; over a value that is no
/// function it is the error `map needs a function, got …`
pub fn lower_strict(node: Node) -> Node {
	lowering(node, true)
}

fn lowering(node: Node, strict: bool) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let first_fresh = first_fresh_number(&node);
	Lowering { context, counter: Cell::new(first_fresh), strict }.expand(node)
}

/// The number after every `loop_out_3`, `loop_item_4·items` an earlier run left: an inner loop lowered by a later run
/// must not reuse the outer loop's names
fn first_fresh_number(node: &Node) -> usize {
	let mut next = 0;
	node.visit(&mut |part| {
		if let Node::Symbol(name) = part {
			let fresh = name.strip_prefix("loop_").and_then(|rest| rest.split('·').next()).and_then(|rest| rest.rsplit_once('_')).and_then(|(_, number)| number.parse::<usize>().ok());
			if let Some(number) = fresh {
				next = next.max(number + 1);
			}
		}
	});
	next
}

/// Binary operators that can stand alone as a value: `fold xs 0 +`
const OPERATOR_VALUES: [(Op, &str); 13] = [
	(Op::Add, "+"), (Op::Sub, "-"), (Op::Mul, "*"), (Op::Div, "/"), (Op::Mod, "%"), (Op::Pow, "^"),
	(Op::Eq, "=="), (Op::Ne, "!="), (Op::Lt, "<"), (Op::Gt, ">"), (Op::Le, "<="), (Op::Ge, ">="), (Op::And, "and"),
];
const OPERATOR_PARAMETERS: [&str; 2] = ["operator_left", "operator_right"];
/// The value of an operator applied to nothing: the start of a fold that leaves the list empty
const OPERATOR_IDENTITIES: [(&str, i64); 2] = [("+", 0), ("*", 1)];

fn operator_symbol(op: Op) -> Option<Node> {
	OPERATOR_VALUES.iter().find(|(known, _)| *known == op).map(|(_, text)| Node::Symbol(text.to_string()))
}

/// `+` as the function `(a b)->a+b`; also the operator with nothing on either side, as `by: >` parses
fn operator_lambda(node: &Node) -> Option<Lambda> {
	let op = match node.drop_meta() {
		Node::Symbol(text) => OPERATOR_VALUES.iter().find(|(_, known)| known == text)?.0,
		Node::Key(left, op, right) if matches!((left.drop_meta(), right.drop_meta()), (Node::Empty, Node::Empty)) => operator_symbol(*op).map(|_| *op)?,
		_ => return None,
	};
	let [left, right] = OPERATOR_PARAMETERS.map(|name| Node::Symbol(name.to_string()));
	Some(Lambda::new(OPERATOR_PARAMETERS.map(String::from).to_vec(), Node::Key(Box::new(left), op, Box::new(right))))
}

/// A function written as a value: a lambda, a block or an operator
fn literal_function(node: &Node) -> Option<Lambda> {
	arrow_lambda(node).or_else(|| block_lambda(node)).or_else(|| operator_lambda(node))
}

/// `a > b`, also as the one statement of a block
fn is_comparison(body: &Node) -> bool {
	match body.drop_meta() {
		Node::Key(_, op, _) => COMPARISONS.contains(op),
		Node::List(items, _, _) if items.len() == 1 => is_comparison(&items[0]),
		_ => false,
	}
}

/// The sorting function without its label: `by: >`, `key: s => s.length`, `key=f`
fn unlabeled(function: Node) -> Node {
	let is_label = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if SORT_LABELS.contains(&word.as_str()));
	match function.drop_meta() {
		Node::Key(label, Op::Colon | Op::Assign, inner) if is_label(label) => inner.as_ref().clone(),
		Node::Key(head, arrow @ (Op::Arrow | Op::FatArrow), body) => match head.drop_meta() {
			Node::Key(label, Op::Colon, parameters) if is_label(label) => Node::Key(parameters.clone(), *arrow, body.clone()),
			_ => function,
		},
		_ => function,
	}
}

/// `x +` at the end of an operand list is the item `x` and the operator `+`: an operator as a value has nothing after it
fn dangling_operator(node: &Node) -> Option<(Node, Node)> {
	let Node::Key(operand, op, nothing) = node.drop_meta() else { return None };
	if !matches!(nothing.drop_meta(), Node::Empty) || matches!(operand.drop_meta(), Node::Empty) {
		return None;
	}
	Some((operand.as_ref().clone(), operator_symbol(*op)?))
}

pub(crate) fn mentions(node: &Node, name: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol == name,
		Node::Key(left, _, right) => mentions(left, name) || mentions(right, name),
		Node::List(items, _, _) => items.iter().any(|item| mentions(item, name)),
		_ => false,
	}
}

/// `a-b` in the body of `(a b)->a-b` reads as one hyphenated name: it is the difference when its parts are parameters
fn subtract_kebab_parameters(node: Node, params: &[String]) -> Node {
	match node {
		Node::Symbol(name) => {
			let parts: Vec<&str> = name.split('-').collect();
			if parts.len() > 1 && parts.iter().all(|part| params.iter().any(|param| param == part)) {
				let mut terms = parts.into_iter().map(|part| Node::Symbol(part.to_string()));
				let first = terms.next().expect("split gives a part");
				terms.fold(first, |difference, term| Node::Key(Box::new(difference), Op::Sub, Box::new(term)))
			} else {
				Node::Symbol(name)
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(subtract_kebab_parameters(*left, params)), op, Box::new(subtract_kebab_parameters(*right, params))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| subtract_kebab_parameters(item, params)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(subtract_kebab_parameters(*node, params)), data },
		other => other,
	}
}

fn parameter_names(left: &Node) -> Option<Vec<String>> {
	match left.drop_meta() {
		Node::Symbol(name) => Some(vec![name.clone()]),
		Node::Empty => Some(vec![]), // `() => body`
		// Kotlin's `{ x: Int -> … }`
		Node::Key(name, Op::Colon, _) if matches!(name.drop_meta(), Node::Symbol(_)) => Some(vec![name.name()]),
		Node::List(items, _, _) => items
			.iter()
			.map(|item| match item.drop_meta() {
				Node::Symbol(name) => Some(name.clone()),
				// `x:float`, a default `n = 2`
				Node::Key(name, Op::Colon | Op::Assign, _) if matches!(name.drop_meta(), Node::Symbol(_)) => Some(name.name()),
				_ => None,
			})
			.collect(),
		_ => None,
	}
}

/// `x=>body`, `(x y)->body` and the same in a group `(x=>body)` or a block
pub(crate) fn arrow_lambda(node: &Node) -> Option<Lambda> {
	match node.drop_meta() {
		Node::Key(head, Op::Arrow | Op::FatArrow, body) => arrow_parts(head, body),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => arrow_lambda(&items[0]),
		// Kotlin `{ x -> x*2 }`, `{ a: Int, b: Int -> a + b }`, Ruby `{ |x| x*2 }`, Swift `{ x in x*2 }` and `{ $0 * 2 }`
		Node::List(items, Bracket::Curly, _) => match items.as_slice() {
			[single] => arrow_lambda(single),
			[leading @ .., last] => match last.drop_meta() {
				Node::Key(head, Op::Arrow, body) => {
					let parameters = Node::List(leading.iter().chain([head.as_ref()]).cloned().collect(), Bracket::Round, Separator::Colon);
					arrow_parts(&parameters, body)
				}
				_ => None,
			},
			_ => None,
		}
		.or_else(|| swift_closure(items))
		.or_else(|| block_lambda(node).filter(|lambda| lambda.params.first().is_some_and(|param| param.starts_with(SHORTHAND_MARK)))),
		_ => None,
	}
}

/// The lambda of `head => body`: TypeScript's result type `(a: number): number => …`, Swift's `(x: Int) -> Int in body`
fn arrow_parts(head: &Node, body: &Node) -> Option<Lambda> {
	let head = &c_style_parameters(head);
	let (left, result_type) = match head.drop_meta() {
		Node::Key(parameters, Op::Colon, result) if matches!(parameters.drop_meta(), Node::List(_, Bracket::Round, _)) => (parameters.as_ref(), Some(result.as_ref().clone())),
		_ => (head, None),
	};
	let (body, result_type) = match body.drop_meta() {
		Node::List(parts, Bracket::None, separator @ Separator::Space) if matches!(left.drop_meta(), Node::List(_, Bracket::Round, _)) => match parts.as_slice() {
			[result, word, rest @ ..] if !rest.is_empty() && matches!(word.drop_meta(), Node::Symbol(word) if word == SWIFT_IN) => {
				let body = match rest { [single] => single.clone(), many => Node::List(many.to_vec(), Bracket::None, separator.clone()) };
				(body, Some(result.clone()))
			}
			_ => (body.clone(), result_type),
		},
		_ => (body.clone(), result_type),
	};
	let params = parameter_names(left)?;
	let body = subtract_kebab_parameters(body, &params);
	let written: Vec<Node> = match left.drop_meta() { Node::List(items, _, _) => items.clone(), other => vec![other.clone()] };
	let declares = written.iter().any(|parameter| matches!(parameter.drop_meta(), Node::Key(_, Op::Colon | Op::Assign, _)));
	Some(Lambda { params, body, written: if declares { written } else { vec![] }, result_type })
}

/// PHP's `fn($x) => …` without its keyword, C#'s `(int x, int y) => …` as `(x: int, y: int)`
fn c_style_parameters(head: &Node) -> Node {
	// C#'s `int x`, Go's `x int`
	let typed = |parameter: &Node| match parameter.drop_meta() {
		Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [kind, name] if is_type_word(kind) && matches!(name.drop_meta(), Node::Symbol(_))) => {
			Node::Key(Box::new(words[1].clone()), Op::Colon, Box::new(words[0].clone()))
		}
		Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [name, kind] if is_type_word(kind) && matches!(name.drop_meta(), Node::Symbol(_)) && !is_type_word(name)) => {
			Node::Key(Box::new(words[0].clone()), Op::Colon, Box::new(words[1].clone()))
		}
		_ => parameter.clone(),
	};
	match head.drop_meta() {
		Node::List(words, _, _) if matches!(words.as_slice(), [keyword, _] if matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word))) => c_style_parameters(&words[1]),
		// OCaml/F# `fun a b -> …`, parsed as `(fun a) b`
		Node::List(_, Bracket::None, Separator::Space) if keyword_and_names(head).is_some_and(|names| names.len() > 1) => {
			Node::List(keyword_and_names(head).expect("guarded"), Bracket::Round, Separator::Colon)
		}
		Node::List(words, Bracket::Round | Bracket::None, Separator::Space) if words.len() == 2 && words.iter().any(is_type_word) && typed(head) != *head => Node::List(vec![typed(head)], Bracket::Round, Separator::Colon),
		Node::List(words, Bracket::Round, separator) if words.iter().any(|word| typed(word) != *word) => Node::List(words.iter().map(typed).collect(), Bracket::Round, separator.clone()),
		_ => head.clone(),
	}
}

/// `fun a b` (as `(fun a) b`): the names after the function keyword
fn keyword_and_names(head: &Node) -> Option<Vec<Node>> {
	let mut words = vec![];
	flatten_phrase(head, &mut words);
	let (keyword, names) = words.split_first()?;
	let is_keyword = matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word));
	(is_keyword && names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_)))).then(|| names.to_vec())
}

fn flatten_phrase(node: &Node, words: &mut Vec<Node>) {
	match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) => items.iter().for_each(|item| flatten_phrase(item, words)),
		other => words.push(other.clone()),
	}
}

fn is_type_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if crate::analyzer::type_word_kind(word).is_some())
}

/// `{it*it}`: a block that is no object; its parameter is `it` when the body uses it
fn block_lambda(node: &Node) -> Option<Lambda> {
	let Node::List(items, Bracket::Curly, separator) = node.drop_meta() else { return None };
	if items.is_empty() || items.iter().any(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _))) {
		return None;
	}
	let body = match items.as_slice() {
		[single] => single.clone(),
		// `{ it.map { it*10 } }`: words side by side stay a juxtaposition, a call with its trailing block
		many if *separator == Separator::Space => Node::List(many.to_vec(), Bracket::None, Separator::Space),
		many => Node::List(many.to_vec(), Bracket::Round, separator.clone()),
	};
	let params = if mentions(&body, IMPLICIT_PARAMETER) { vec![IMPLICIT_PARAMETER.to_string()] } else { shorthand_parameters(&body) };
	Some(Lambda::new(params, body))
}

/// Swift's shorthand arguments `$0`, `$1` … up to the highest one the body uses
fn shorthand_parameters(body: &Node) -> Vec<String> {
	let names = |count: usize| (0..count).map(|index| format!("{SHORTHAND_MARK}{index}"));
	let count = (0..MAX_SHORTHAND_ARGUMENTS).rev().find(|index| mentions_outside_blocks(body, &format!("{SHORTHAND_MARK}{index}"))).map_or(0, |highest| highest + 1);
	names(count).collect()
}

/// A mention not inside a nested block or lambda: `{ return { $0 + k } }` has no `$0` of its own, the inner closure has
fn mentions_outside_blocks(node: &Node, name: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol == name,
		Node::Key(_, Op::FatArrow, _) => false,
		Node::Key(left, _, right) => mentions_outside_blocks(left, name) || mentions_outside_blocks(right, name),
		Node::List(_, Bracket::Curly, _) => false,
		Node::List(items, _, _) => items.iter().any(|item| mentions_outside_blocks(item, name)),
		_ => false,
	}
}

/// The parameter `name` replaced by `argument` in a lambda body, except inside an inner lambda that binds the same name
/// (Kotlin): in `it.map{ it*10 }` the inner `it` is the inner lambda's
fn bind_parameter(node: Node, name: &str, argument: &Node) -> Node {
	let binds_name = |block: &Node| block_lambda(block).is_some_and(|lambda| lambda.params.iter().any(|param| param == name));
	match node {
		Node::Symbol(symbol) if symbol == name => argument.clone(),
		Node::Key(receiver, Op::Dot, method) if crate::declarations::glued_block(&method).is_some_and(|(_, block)| binds_name(&block)) => {
			Node::Key(Box::new(bind_parameter(*receiver, name, argument)), Op::Dot, method)
		}
		Node::List(items, bracket, separator) if items.len() > 1 && crate::declarations::is_callee(&items[0]) => {
			Node::List(items.into_iter().map(|item| if binds_name(&item) { item } else { bind_parameter(item, name, argument) }).collect(), bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(bind_parameter(*left, name, argument)), op, Box::new(bind_parameter(*right, name, argument))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| bind_parameter(item, name, argument)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(bind_parameter(*node, name, argument)), data },
		other => other,
	}
}

/// Swift `{ x in x*2 }`, `{ a, b in a+b }`: names, `in`, a body that uses them (`{ x in xs }` stays membership)
fn swift_closure(items: &[Node]) -> Option<Lambda> {
	let (last, leading) = items.split_last()?;
	let Node::List(parts, Bracket::None, Separator::Space) = last.drop_meta() else { return None };
	let [name, word, body @ ..] = parts.as_slice() else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(word) if word == SWIFT_IN) || body.is_empty() {
		return None;
	}
	let params: Vec<String> = leading.iter().chain([name]).map(|param| match param.drop_meta() {
		Node::Symbol(param) => Some(param.clone()),
		_ => None,
	}).collect::<Option<_>>()?;
	let body = match body {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), Bracket::None, Separator::Space),
	};
	params.iter().any(|param| mentions(&body, param)).then(|| Lambda::new(params, body))
}

/// Swift's trailing closure `xs.reduce(0) { $0 + $1 }`: the call's word and arguments, then the closure
fn trailing_closure(items: &[Node]) -> Option<Vec<Node>> {
	let [call, closure] = items else { return None };
	let Node::List(call_items, _, _) = call.drop_meta() else { return None };
	let (word, arguments) = call_items.split_first()?;
	if !matches!(word.drop_meta(), Node::Symbol(_)) || !matches!(closure.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let arguments = match arguments {
		[Node::List(group, Bracket::Round, _)] => group.clone(),
		other => other.to_vec(),
	};
	Some([vec![word.clone()], arguments, vec![closure.clone()]].concat())
}

/// `reduce` given a start value is `fold`: Swift's `xs.reduce(0, +)`, JS's `xs.reduce(f, 0)` aside
fn with_start(iteration: &'static Iteration, extras: usize) -> &'static Iteration {
	match iteration.word {
		REDUCE_WORD if extras == 1 => ITERATIONS.iter().find(|fold| fold.word == FOLD_WORD).expect("fold is an iteration"),
		_ => iteration,
	}
}

/// `{it*2}` as the arrow lambda `it => it*2`, for passes that know only arrows (closures.rs)
pub(crate) fn block_as_arrow(node: &Node) -> Option<Node> {
	let lambda = block_lambda(node).filter(|lambda| !lambda.params.is_empty())?;
	let parameters = match lambda.params.as_slice() {
		[single] => Node::Symbol(single.clone()),
		several => Node::List(several.iter().cloned().map(Node::Symbol).collect(), Bracket::Round, Separator::Colon),
	};
	Some(Node::Key(Box::new(parameters), Op::FatArrow, Box::new(lambda.body)))
}

/// The definition `name(params) := body` of a lambda, a block with `it` or an operator given as a value
pub fn lambda_definition(name: &str, function: &Node) -> Option<Node> {
	let lambda = arrow_lambda(function).or_else(|| block_lambda(function).filter(|lambda| !lambda.params.is_empty())).or_else(|| operator_lambda(function))?;
	Some(definition(name, lambda))
}

fn definition(name: &str, lambda: Lambda) -> Node {
	let parameters = if lambda.written.is_empty() { lambda.params.into_iter().map(Node::Symbol).collect() } else { lambda.written };
	let head = Node::List([vec![Node::Symbol(name.to_string())], parameters].concat(), Bracket::Round, Separator::None);
	// a result type converts the body, as `f(a:number):number := …` is lowered before this pass runs
	let body = match lambda.result_type {
		Some(result_type) => {
			let grouped = Node::List(vec![lambda.body], Bracket::Round, Separator::None);
			Node::Key(Box::new(grouped), Op::As, Box::new(result_type))
		}
		None => lambda.body,
	};
	Node::Key(Box::new(head), Op::Define, Box::new(body))
}

/// Python's `sorted(xs, key=x => -x)`: in a call a function given as `name = …` is the named argument `key: x => -x`
/// (P37), never the definition of a function `key`
fn named_function_arguments(items: Vec<Node>, bracket: &Bracket) -> Vec<Node> {
	let is_call = *bracket == Bracket::Round && items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(_));
	if !is_call {
		return items;
	}
	items
		.into_iter()
		.map(|item| match item.drop_meta() {
			Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Node::Symbol(_)) && arrow_lambda(value).is_some() => {
				Node::Key(name.clone(), Op::Colon, value.clone())
			}
			_ => item,
		})
		.collect()
}

fn call(name: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(name.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

struct Lowering {
	context: Context,
	counter: Cell<usize>,
	strict: bool,
}

impl Lowering {
	fn fresh(&self, prefix: &str) -> String {
		let number = self.counter.get();
		self.counter.set(number + 1);
		format!("{prefix}_{number}")
	}

	fn expand(&self, node: Node) -> Node {
		match node {
			// the parser already reads `f = {it*2}` as the definition `f := {it*2}`
			Node::Key(target, Op::Assign | Op::Define, value) if matches!(target.drop_meta(), Node::Symbol(_)) && arrow_lambda(&value).is_some() => {
				let Node::Symbol(name) = target.drop_meta() else { unreachable!("guarded") };
				let lambda = arrow_lambda(&value).expect("guarded");
				definition(name, Lambda { body: self.expand(lambda.body), ..lambda })
			}
			// `sum := fold +`: the iteration word with only its function is a function of the list
			Node::Key(target, Op::Assign | Op::Define, value) if matches!(target.drop_meta(), Node::Symbol(_)) && self.partial_application(&value).is_some() => {
				let Node::Symbol(name) = target.drop_meta() else { unreachable!("guarded") };
				let body = self.partial_application(&value).expect("guarded");
				definition(name, Lambda::new(vec![PARTIAL_LIST.to_string()], self.expand(body)))
			}
			Node::Key(receiver, Op::Dot, method) if self.iteration_method(&method).is_some() => {
				let (iteration, mut arguments) = self.iteration_method(&method).expect("guarded");
				let function = arguments.pop().expect("a function");
				let extras: Vec<Node> = arguments.into_iter().map(|argument| self.expand(argument)).collect();
				let (receiver, function) = (self.expand(*receiver), self.expand(function));
				match self.iterate(iteration, receiver.clone(), extras.clone(), function.clone()) {
					Some(loop_node) => loop_node,
					None => Node::Key(Box::new(receiver), Op::Dot, Box::new(call(iteration.word, [extras, vec![function]].concat()))),
				}
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::List(items, bracket, separator) => {
				let items = named_function_arguments(items, &bracket);
				let items = flatten_prefix_application(items, &bracket, &separator);
				let items: Vec<Node> = if separator == Separator::Space {
					items
						.into_iter()
						.flat_map(|item| match dangling_operator(&item) {
							Some((operand, operator)) => vec![operand, operator],
							None => vec![item],
						})
						.collect()
				} else {
					items
				};
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				// in a list literal `[(x => x + 1) (x => x + 2)]` the functions are elements, never one applied to the next
				let immediate = if bracket == Bracket::Square { None } else { self.immediate_call(&items) };
				immediate.or_else(|| self.iteration_call(&items, &bracket, &separator)).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	/// The iteration a word names, unless the program defines a function of that name
	fn iteration_of(&self, word: &Node) -> Option<&'static Iteration> {
		let Node::Symbol(name) = word.drop_meta() else { return None };
		let canonical = ITERATION_SPELLINGS.iter().find(|(_, spellings)| spellings.contains(&name.as_str())).map_or(name.as_str(), |(word, _)| word);
		ITERATIONS.iter().find(|iteration| iteration.word == canonical && !self.context.user_functions.contains_key(name))
	}

	/// `word(extras, function)` after a dot: the iteration and its arguments, the function last
	fn iteration_method(&self, method: &Node) -> Option<(&'static Iteration, Vec<Node>)> {
		let items = match crate::declarations::glued_block(method) {
			// `xs.map{ it*2 }`: `map{…}` after a dot is the call with its lambda, not the data `map:{…}`
			Some((word, block)) if block_lambda(&block).is_some_and(|lambda| !lambda.params.is_empty()) => vec![word, block],
			_ => {
				let Node::List(items, _, _) = method.drop_meta() else { return None };
				trailing_closure(items).unwrap_or_else(|| items.clone())
			}
		};
		let (word, arguments) = items.split_first()?;
		let iteration = with_start(self.iteration_of(word)?, arguments.len().checked_sub(1)?); // `xs.sort()` has no function
		let is_function = |node: &Node| arrow_lambda(node).is_some();
		let arguments = match arguments {
			// JS's `xs.reduce((a, b) => a + b, 0)`: the start value after the function
			[function, start] if iteration.word == FOLD_WORD && is_function(function) && !is_function(start) => vec![start.clone(), function.clone()],
			_ => arguments.to_vec(),
		};
		(arguments.len() == iteration.extra_arguments + 1).then_some((iteration, arguments))
	}

	/// `{x*x}(x=5)`: an anonymous function called with its bindings; `{it*2} 3` and `{it^2}[1 2 3]`: with its argument,
	/// which a list broadcasts over like for a named function (broadcasting.rs)
	fn immediate_call(&self, items: &[Node]) -> Option<Node> {
		let [block, arguments] = items else { return None };
		let lambda = block_lambda(block).or_else(|| arrow_lambda(block))?;
		let is_function = |node: &Node| block_lambda(node).or_else(|| arrow_lambda(node)).is_some_and(|lambda| !lambda.params.is_empty());
		let juxtaposed;
		let entries = match arguments.drop_meta() {
			Node::List(entries, Bracket::Round, _) => entries,
			argument if !lambda.params.is_empty() && !is_function(argument) => {
				juxtaposed = vec![argument.clone()];
				&juxtaposed
			}
			_ => return None,
		};
		let bindings: Vec<(String, Node)> = entries
			.iter()
			.filter_map(|entry| match entry.drop_meta() {
				Node::Key(name, Op::Assign, value) => match name.drop_meta() {
					Node::Symbol(name) => Some((name.clone(), value.as_ref().clone())),
					_ => None,
				},
				_ => None,
			})
			.collect();
		let (params, values) = if bindings.len() == entries.len() && !bindings.is_empty() {
			(bindings.iter().map(|(name, _)| name.clone()).collect(), bindings.into_iter().map(|(_, value)| value).collect())
		} else {
			(lambda.params.clone(), entries.clone())
		};
		let name = self.fresh("lambda");
		let defined = definition(&name, Lambda::new(params, lambda.body));
		Some(crate::broadcasting::lower(Node::List(vec![defined, call(&name, values)], Bracket::Round, Separator::Semicolon)))
	}

	/// `map xs f`, `map(xs, f)`, `fold xs 0 f`, and `xs.map {it*it}` (the method word, then the arguments as the next items)
	fn iteration_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let is_call = call_name(items, bracket, separator).is_some();
		let is_prefix = *bracket == Bracket::None && *separator == Separator::Space;
		if !is_call && !is_prefix {
			return None;
		}
		let (head, rest) = items.split_first()?;
		if let Some(iteration) = self.iteration_of(head) {
			// `map square on xs` of a known `square`: the parser applied it, `map (square (on xs))`
			let rest = spliced_applications(rest.to_vec());
			// `map square on xs`, `map &square xs`: the function may come first
			let rest: Vec<Node> = rest.iter().filter(|item| !matches!(item.drop_meta(), Node::Symbol(word) if word == ON_WORD)).cloned().collect();
			let rest = match rest.as_slice() {
				[first, second] if iteration.extra_arguments == 0 && self.is_function_value(first) && !self.is_function_value(second) => vec![second.clone(), first.clone()],
				_ => rest,
			};
			// Python's `any(xs)`, `all(xs)`: the items themselves are the conditions
			if let ([list], true) = (rest.as_slice(), is_call && TRUTH_WORDS.contains(&iteration.word)) {
				let item = Node::Symbol(TRUTH_ITEM.to_string());
				return self.iterate(iteration, list.clone(), vec![], Node::Key(Box::new(item.clone()), Op::FatArrow, Box::new(item)));
			}
			let [list, extras @ .., function] = rest.as_slice() else { return None };
			let iteration = with_start(iteration, extras.len());
			if extras.len() != iteration.extra_arguments {
				return None;
			}
			self.iterate(iteration, list.clone(), extras.to_vec(), function.clone())
		} else if let Node::Key(receiver, Op::Dot, word) = head.drop_meta() {
			let [extras @ .., function] = rest else { return None };
			let iteration = with_start(self.iteration_of(word)?, extras.len());
			if !is_prefix || extras.len() != iteration.extra_arguments {
				return None;
			}
			self.iterate(iteration, receiver.as_ref().clone(), extras.to_vec(), function.clone())
		} else {
			None
		}
	}

	/// `fold +`, `map square`, `reduce (a b)->a+b`: the call of the iteration word over the parameter `partial_list`
	fn partial_application(&self, value: &Node) -> Option<Node> {
		let (word, function) = match dangling_operator(value) {
			Some((word, operator)) => (word, operator),
			None => match value.drop_meta() {
				Node::List(items, _, Separator::Space) => match items.as_slice() {
					[word, function] => (word.clone(), function.clone()),
					_ => return None,
				},
				_ => return None,
			},
		};
		let iteration = self.iteration_of(&word)?;
		if !self.is_function_value(&function) {
			return None;
		}
		let list = Node::Symbol(PARTIAL_LIST.to_string());
		let identity = match function.drop_meta() {
			Node::Symbol(text) => OPERATOR_IDENTITIES.iter().find(|(known, _)| known == text).map(|(_, identity)| Node::int(*identity)),
			_ => None,
		};
		Some(match (iteration.extra_arguments, identity) {
			(0, _) => call(iteration.word, vec![list, function]),
			// a fold without a start: the identity of its operator, else the first item starts (reduce)
			(_, Some(identity)) => call(iteration.word, vec![list, identity, function]),
			(_, None) => call("reduce", vec![list, function]),
		})
	}

	/// A function given as a value: a lambda or block, an operator, or the name of a defined function
	fn is_function_value(&self, node: &Node) -> bool {
		arrow_lambda(node).is_some()
			|| block_lambda(node).is_some()
			|| operator_lambda(node).is_some()
			|| matches!(node.drop_meta(), Node::Symbol(name) if self.context.user_functions.contains_key(name))
	}

	/// The function applied to the symbols `arguments`: the body of a literal function with its parameters replaced, or the call of a
	/// defined function; anything else cannot be inlined
	/// `Err(None)` when the function is not known yet (a parameter of a function that is specialised later)
	fn applied(&self, iteration: &Iteration, list: &Node, function: &Node, arguments: &[Node]) -> Result<Node, Option<Node>> {
		match literal_function(function) {
			Some(lambda) if lambda.params.len() == arguments.len() => {
				Ok(lambda.params.iter().zip(arguments).fold(lambda.body, |body, (param, argument)| bind_parameter(body, param, argument)))
			}
			// a block that does not use `it` takes no argument and is run for every item
			Some(lambda) if lambda.params.is_empty() && arguments.len() == 1 => Ok(lambda.body),
			Some(_) => {
				let count = if iteration.function_arguments == 1 { "one argument" } else { "two arguments" };
				Err(Some(Diagnostic::at(function, format!("{} takes a function of {count}", iteration.word)).into_error()))
			}
			None => match function.drop_meta() {
				Node::Symbol(name) if self.context.user_functions.contains_key(name) => Ok(call(name, arguments.to_vec())),
				Node::Symbol(_) if !self.strict => Err(None),
				// a function value known only at run time: a variable or parameter holding a closure, or a call returning one
				_ if crate::closures::may_be_function_value(function) => Ok(crate::closures::closure_call(function.clone(), arguments.to_vec())),
				_ => {
					let params = if iteration.function_arguments == 1 { "x" } else { "(a b)" };
					let fix = format!("{} {} ({params} => …)", iteration.word, list.serialize());
					Err(Some(crate::closures::needs_a_function(iteration.word, None, function, &fix)))
				}
			},
		}
	}

	/// Whether the first of `pair` sorts before the second: a key of one item is smaller (Python `key=len`), a comparison
	/// holds (Swift `by: >`), any other comparator is negative (JS `(a,b) => a-b`)
	fn sort_order(&self, iteration: &Iteration, list: &Node, function: &Node, pair: [Node; 2]) -> Result<Node, Option<Node>> {
		let less = |left: Node, right: Node| Node::Key(Box::new(left), Op::Lt, Box::new(right));
		if self.parameter_count(function) == Some(1) {
			let [first, second] = pair;
			return Ok(less(self.applied(iteration, list, function, &[first])?, self.applied(iteration, list, function, &[second])?));
		}
		let compared = self.applied(iteration, list, function, &pair)?;
		Ok(if self.compares(function) { compared } else { less(compared, Node::int(0)) })
	}

	fn parameter_count(&self, function: &Node) -> Option<usize> {
		match literal_function(function) {
			Some(lambda) => Some(lambda.params.len()),
			None => match function.drop_meta() {
				Node::Symbol(name) => self.context.user_functions.get(name).map(|defined| defined.params.len()),
				_ => None,
			},
		}
	}

	/// A function whose result is a comparison: `(a b) => a > b`, `>`, `def before(a,b){ a < b }`
	fn compares(&self, function: &Node) -> bool {
		match literal_function(function) {
			Some(lambda) => is_comparison(&lambda.body),
			None => match function.drop_meta() {
				Node::Symbol(name) => self.context.user_functions.get(name).is_some_and(|defined| is_comparison(&defined.body)),
				_ => false,
			},
		}
	}

	/// The loop of an iteration word with the body of a literal function inlined
	fn iterate(&self, iteration: &Iteration, list: Node, extras: Vec<Node>, function: Node) -> Option<Node> {
		let names: Vec<(&str, String)> =
			["out", "item", "acc", "list", "index", "value", "moving"].into_iter().map(|name| (name, self.fresh(&format!("loop_{name}")))).collect();
		let symbol = |name: &str| Node::Symbol(names.iter().find(|(prefix, _)| *prefix == name).expect("a loop name").1.clone());
		let arguments: Vec<Node> = if iteration.function_arguments == 2 { vec![symbol("acc"), symbol("item")] } else { vec![symbol("item")] };
		let applied = if iteration.word == SORT_WORD {
			self.sort_order(iteration, &list, &unlabeled(function), [symbol("acc"), symbol("item")])
		} else {
			self.applied(iteration, &list, &function, &arguments)
		};
		let applied = match applied {
			Ok(applied) => applied,
			Err(error) => return error,
		};
		let mut template = iteration.template.to_string();
		for (name, fresh) in &names {
			template = replace_word(&template, name, fresh);
		}
		let mut program = substitute(substitute(parse(&template), LIST_PLACEHOLDER, &list), CALL_PLACEHOLDER, &applied);
		if let Some(start) = extras.first() {
			program = substitute(program, START_PLACEHOLDER, start);
		}
		Some(program)
	}
}

/// `map xs f` with names only is read as `(map xs) f`: a braceless application whose head is a braceless application continues it
/// `[square, [on, xs]]`, `[[square, on], xs]` → `[square, on, xs]`: juxtapositions nested in the items, spliced in
fn spliced_applications(items: Vec<Node>) -> Vec<Node> {
	items.into_iter().flat_map(|item| match item.drop_meta() {
		Node::List(inner, Bracket::None, Separator::Space) => spliced_applications(inner.clone()),
		_ => vec![item],
	}).collect()
}

fn flatten_prefix_application(items: Vec<Node>, bracket: &Bracket, separator: &Separator) -> Vec<Node> {
	if *bracket != Bracket::None || *separator != Separator::Space {
		return items;
	}
	match items.split_first() {
		Some((head, rest)) => match head.drop_meta() {
			Node::List(inner, Bracket::None, Separator::Space) if !rest.is_empty() => {
				let flattened = flatten_prefix_application(inner.clone(), &Bracket::None, &Separator::Space);
				[flattened, rest.to_vec()].concat()
			}
			_ => items,
		},
		None => items,
	}
}

/// `text` with every whole word `word` replaced (a word is a run of letters, digits and `_`)
fn replace_word(text: &str, word: &str, replacement: &str) -> String {
	let mut result = String::new();
	let mut current = String::new();
	let flush = |current: &mut String, result: &mut String| {
		result.push_str(if current == word { replacement } else { current });
		current.clear();
	};
	for character in text.chars() {
		if character.is_alphanumeric() || character == '_' {
			current.push(character);
		} else {
			flush(&mut current, &mut result);
			result.push(character);
		}
	}
	flush(&mut current, &mut result);
	result
}
