//! Compile-time lambdas. Functions are not first-class values yet, so a lambda is lowered where it stands:
//! - `f = x=>x*x`, `f = (x y)->x+y` define the function `f` at that point (captured variables by value, like every
//!   definition); `f 3` calls it; the parser itself reads `f = {it*2}` as the definition `f := {it*2}`
//! - `{x*x}(x=5)` defines an anonymous function and calls it at once
//! - `map [1 2 3] {it*it}`, `map(xs, x=>x+1)`, `xs.map(f)` over a literal block, a lambda or a defined function is a loop
//! - a lambda anywhere else is a closure (closures.rs); `map` over a value that is no function is the error `map needs a function, got …`

use super::nodes::{call, children_rewritten, is_kinded_type_word, key, named_assignment, Counter};
use crate::analyzer::call_name;
use crate::compiler::context::{Context, UserFunctionDef};
use crate::errors::diagnostic::Diagnostic;
use crate::library_words::substitute;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;
use crate::words::{EACH_WORD, IMPLICIT_PARAMETER, ON_WORD, SORT_WORD};
use std::collections::HashSet;

const PARTIAL_LIST: &str = "partial_list";
const LIST_PLACEHOLDER: &str = "loop_list";
const START_PLACEHOLDER: &str = "loop_start";
const CALL_PLACEHOLDER: &str = "loop_call";
const LOOP_ITEM_PREFIX: &str = "loop_item_";

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
const REDUCE_WORD: &str = "reduce";
/// `any(xs)`, `all(xs)` without a function test the items themselves
const TRUTH_WORDS: [&str; 2] = ["any", "all"];
const TRUTH_ITEM: &str = "truth_item";
const FOLD_WORD: &str = "fold";
/// Iteration words as other languages spell them: sorting with a function (Python sorted, Ruby sort_by, Kotlin
/// sortedBy), each (JS and Kotlin forEach, Rust for_each, PHP foreach)
const ITERATION_SPELLINGS: [(&str, &[&str]); 2] = [
	(SORT_WORD, &["sorted", "sort_by", "sortBy", "sortedBy", "sorted_by"]),
	(EACH_WORD, &["forEach", "for_each", "foreach"]),
];
/// The label of the sorting function: Swift `sorted(by: >)`, Python `sorted(xs, key=…)`
const SORT_LABELS: [&str; 2] = ["by", "key"];
/// The label of the list an iteration word goes over: `map f on: xs` (card labels-calls)
const LIST_LABELS: [&str; 1] = [ON_WORD];
const COMPARISONS: [Op; 4] = [Op::Lt, Op::Gt, Op::Le, Op::Ge];

const ITERATIONS: [Iteration; 9] = [
	Iteration { word: "map", extra_arguments: 0, function_arguments: 1, template: "(out=[]; for item in loop_list { out = out + [loop_call] }; out)" },
	Iteration { word: "filter", extra_arguments: 0, function_arguments: 1, template: "(out=[]; for item in loop_list { if loop_call { out = out + [item] } }; out)" },
	Iteration { word: EACH_WORD, extra_arguments: 0, function_arguments: 1, template: "(value=ø; for item in loop_list { value = loop_call }; value)" },
	Iteration { word: "fold", extra_arguments: 1, function_arguments: 2, template: "(acc=loop_start; for item in loop_list { acc = loop_call }; acc)" },
	Iteration { word: "find", extra_arguments: 0, function_arguments: 1, template: "(found=ø; searching=1; for item in loop_list { if searching and loop_call { found = item; searching = 0 } }; found)" },
	Iteration { word: "any", extra_arguments: 0, function_arguments: 1, template: "(hit=false; for item in loop_list { if loop_call { hit = true } }; hit)" },
	Iteration { word: "all", extra_arguments: 0, function_arguments: 1, template: "(held=true; for item in loop_list { if not loop_call { held = false } }; held)" },
	Iteration {
		word: "reduce",
		extra_arguments: 0,
		function_arguments: 2,
		template: "(list=loop_list; if count(list) == 0 then empty_extremum(reduce) else (acc=list#1; index=2; while index <= count(list) { item=list#index; acc = loop_call; index=index+1 }; acc))",
	},
	// a stable bottom-up merge sort of a copy in n log n (card comparator-sort): runs of width items merged in pairs,
	// the right run's item first only when loop_call, whether acc sorts before item (sort_order); sorting by a key of
	// one item takes key_sort_template
	Iteration {
		word: SORT_WORD,
		extra_arguments: 0,
		function_arguments: 2,
		template: "(out=[]; for value in loop_list { out = out + [value] }; size=count(out); width=1; while width < size { merged=[]; low=1; while low <= size { middle=low+width; if middle > size+1 { middle=size+1 }; high=middle+width; if high > size+1 { high=size+1 }; left=low; right=middle; while left < middle or right < high { if left >= middle { moving=1 } else { if right >= high { moving=0 } else { acc=out#right; item=out#left; if loop_call { moving=1 } else { moving=0 } } }; if moving { merged = merged + [out#right]; right=right+1 } else { merged = merged + [out#left]; left=left+1 } }; low=high }; out=merged; width=width*2 }; out)",
	},
];

/// Sorting by a key of one item (`xs.sort_by(p => p.k)`): the keys computed once, then a merge sort by them in n log n
fn key_sort_template() -> String {
	let sorted_by = crate::wasm_emitter::reflection::library_ops::LIST_SORTED_BY;
	format!("(list=loop_list; out=[]; for item in list {{ out = out + [loop_call] }}; {sorted_by}(list, out))")
}

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
/// strict run, which calls it as a closure
pub fn lower(node: Node) -> Node {
	lowering(node, false)
}

/// The final run: an iteration word over a function known only at run time calls it as a closure; over a value that is no
/// function it is the error `map needs a function, got …`
pub fn lower_strict(node: Node) -> Node {
	lowering(node, true)
}

fn lowering(node: Node, strict: bool) -> Node {
	let context = crate::analyzer::function_context(&node);
	let classes = crate::modules::needed_definitions::class_names(&node);
	let first_fresh = first_fresh_number(&node);
	let path_variables = crate::warp_parser::path_variables(&node);
	Lowering { context, classes, path_variables, counter: Counter::starting_at(first_fresh), strict }.expand(node)
}

/// `loop_item_1`, the variable of a loop inlined from a function: an `it` in its body is no loop item but the `it` of
/// the code around the function (`scale := xs.map(x => x * it)`)
pub(crate) fn is_inlined_loop_variable(name: &str) -> bool {
	name.strip_prefix(LOOP_ITEM_PREFIX).is_some_and(|number| number.parse::<usize>().is_ok())
}

/// The number after every `loop_out_3`, `loop_item_4·items` an earlier run left: an inner loop lowered by a later run
/// must not reuse the outer loop's names
fn first_fresh_number(node: &Node) -> usize {
	let mut next = 0;
	node.visit(&mut |part| {
		if let Node::Symbol(name) = part {
			let fresh = name
				.strip_prefix("loop_")
				.and_then(|rest| rest.split('·').next())
				.and_then(|rest| rest.rsplit_once('_'))
				.and_then(|(_, number)| number.parse::<usize>().ok());
			if let Some(number) = fresh {
				next = next.max(number + 1);
			}
		}
	});
	next
}

/// Binary operators that can stand alone as a value: `fold xs 0 +`
const OPERATOR_VALUES: [(Op, &str); 13] = [
	(Op::Add, "+"),
	(Op::Sub, "-"),
	(Op::Mul, "*"),
	(Op::Div, "/"),
	(Op::Mod, "%"),
	(Op::Pow, "^"),
	(Op::Eq, "=="),
	(Op::Ne, "!="),
	(Op::Lt, "<"),
	(Op::Gt, ">"),
	(Op::Le, "<="),
	(Op::Ge, ">="),
	(Op::And, "and"),
];
const OPERATOR_PARAMETERS: [&str; 2] = ["operator_left", "operator_right"];
/// The value of an operator applied to nothing: the start of a fold that leaves the list empty
const OPERATOR_IDENTITIES: [(&str, i64); 2] = [("+", 0), ("*", 1)];

fn operator_symbol(op: Op) -> Option<Node> {
	OPERATOR_VALUES.iter().find(|(known, _)| *known == op).map(|(_, text)| symbol(text))
}

/// `+` as the function `(a b)->a+b`; also the operator with nothing on either side, as `by: >` parses
fn operator_lambda(node: &Node) -> Option<Lambda> {
	let op = match node.drop_meta() {
		Node::Symbol(text) => OPERATOR_VALUES.iter().find(|(_, known)| known == text)?.0,
		Node::Key(left, op, right) if matches!((left.drop_meta(), right.drop_meta()), (Node::Empty, Node::Empty)) => {
			operator_symbol(*op).map(|_| *op)?
		}
		_ => return None,
	};
	let [left, right] = OPERATOR_PARAMETERS.map(symbol);
	Some(Lambda::new(OPERATOR_PARAMETERS.map(String::from).to_vec(), key(left, op, right)))
}

/// A function written as a value: a lambda, a block or an operator
fn literal_function(node: &Node) -> Option<Lambda> {
	arrow_lambda(node)
		.or_else(|| block_lambda(node))
		.or_else(|| operator_lambda(node))
		.or_else(|| arrow_lambda(&it_function(node)?))
}

/// `map`, `filter`, `sorted` … as a word, whether or not the program defines a function of that name
/// `find`, `map`, `filter`, … under any spelling: a class's method of that name keeps them for other values
pub(crate) fn is_iteration_word(name: &str) -> bool {
	iteration_word(&Node::Symbol(name.to_string())).is_some()
}

fn iteration_word(word: &Node) -> Option<&'static Iteration> {
	let Node::Symbol(name) = word.drop_meta() else {
		return None;
	};
	let canonical = ITERATION_SPELLINGS
		.iter()
		.find(|(_, spellings)| spellings.contains(&name.as_str()))
		.map_or(name.as_str(), |(word, _)| word);
	ITERATIONS.iter().find(|iteration| iteration.word == canonical)
}

/// `map square xs`, `filter even xs`: an iteration word whose first argument may be its function; sort, any and all take
/// the list first, their function being optional
pub(crate) fn takes_a_function_first(word: &Node) -> bool {
	iteration_word(word).is_some_and(|iteration| {
		iteration.extra_arguments == 0 && iteration.word != SORT_WORD && !TRUTH_WORDS.contains(&iteration.word)
	})
}

/// `xs.map(it * 2)` in a function of one parameter: the `it` of the argument is the argument function's own
pub(crate) fn has_own_it(head: &Node, argument: &Node) -> bool {
	iteration_word(head).is_some() && it_function(argument).is_some()
}

/// `xs.map(it * 2)`, `apply(it + 1, 3)`: an expression of `it` where a function is expected is the function `it => …`
/// (clear intent, notes/welcoming.md); a bare `it` stays the value it names
pub(crate) fn it_function(node: &Node) -> Option<Node> {
	let is_bare = matches!(node.drop_meta(), Node::Symbol(_));
	let it = || Box::new(symbol(IMPLICIT_PARAMETER));
	(!is_bare && mentions_outside_blocks(node, IMPLICIT_PARAMETER))
		.then(|| Node::Key(it(), Op::Arrow, Box::new(node.clone())))
}

/// `a > b`, also as the one statement of a block, and any other bool: `a < b or a == b and c`, `if … then a < b else …`
/// (card sort-comparator)
fn is_boolean_body(body: &Node) -> bool {
	match body.drop_meta() {
		Node::Key(_, op, _) if COMPARISONS.contains(op) => true,
		Node::List(items, _, _) if items.len() == 1 => is_boolean_body(&items[0]),
		_ => crate::analyzer::is_boolean(body, &crate::analyzer::Scope::new()),
	}
}

/// `map f on: xs`: the list its label names comes first, the other arguments keep their order after it
fn labeled_list_first(mut arguments: Vec<Node>) -> Vec<Node> {
	let labeled_list = |node: &Node| match node.drop_meta() {
		Node::Key(label, Op::Colon, list) if label.symbol_name().is_some_and(|name| LIST_LABELS.contains(&name)) => {
			Some(list.as_ref().clone())
		}
		_ => None,
	};
	match arguments.iter().position(|argument| labeled_list(argument).is_some()) {
		Some(at) => {
			let list = labeled_list(&arguments.remove(at)).expect("found");
			[vec![list], arguments].concat()
		}
		None => arguments,
	}
}

/// The sorting function without its label: `by: >`, `key: s => s.length`, `key=f`
fn unlabeled(function: Node) -> Node {
	let is_label = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if SORT_LABELS.contains(&word.as_str()));
	match function.drop_meta() {
		Node::Key(label, Op::Colon | Op::Assign, inner) if is_label(label) => inner.as_ref().clone(),
		Node::Key(head, arrow @ Op::Arrow, body) => match head.drop_meta() {
			Node::Key(label, Op::Colon, parameters) if is_label(label) => {
				Node::Key(parameters.clone(), *arrow, body.clone())
			}
			_ => function,
		},
		_ => function,
	}
}

/// `x +` at the end of an operand list is the item `x` and the operator `+`: an operator as a value has nothing after it
fn dangling_operator(node: &Node) -> Option<(Node, Node)> {
	let Node::Key(operand, op, nothing) = node.drop_meta() else {
		return None;
	};
	if !nothing.is_empty_node() || operand.is_empty_node() {
		return None;
	}
	Some((operand.as_ref().clone(), operator_symbol(*op)?))
}

/// `a-b` in the body of `(a b)->a-b` reads as one hyphenated name: it is the difference when its parts are parameters
fn subtract_kebab_parameters(node: Node, params: &[String]) -> Node {
	match node {
		Node::Symbol(name) => {
			let parts: Vec<&str> = name.split('-').collect();
			if parts.len() > 1 && parts.iter().all(|part| params.iter().any(|param| param == part)) {
				let mut terms = parts.into_iter().map(symbol);
				let first = terms.next().expect("split gives a part");
				terms.fold(first, |difference, term| key(difference, Op::Sub, term))
			} else {
				Node::Symbol(name)
			}
		}
		other => children_rewritten(other, |child| subtract_kebab_parameters(child, params)),
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
				Node::Key(name, Op::Colon | Op::Assign, _) if matches!(name.drop_meta(), Node::Symbol(_)) => {
					Some(name.name())
				}
				_ => None,
			})
			.collect(),
		_ => None,
	}
}

/// `x=>body`, `(x y)->body` and the same in a group `(x=>body)` or a block
pub(crate) fn arrow_lambda(node: &Node) -> Option<Lambda> {
	match node.drop_meta() {
		Node::Key(head, Op::Arrow, body) => arrow_parts(head, body),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => arrow_lambda(&items[0]),
		// Kotlin `{ x -> x*2 }`, `{ a: Int, b: Int -> a + b }`, Ruby `{ |x| x*2 }`, Swift `{ x in x*2 }` and `{ $0 * 2 }`
		Node::List(items, Bracket::Curly, _) => match items.as_slice() {
			[single] => arrow_lambda(single),
			[leading @ .., last] => match last.drop_meta() {
				Node::Key(head, Op::Arrow, body) => {
					let parameters = Node::List(
						leading.iter().chain([head.as_ref()]).cloned().collect(),
						Bracket::Round,
						Separator::Colon,
					);
					arrow_parts(&parameters, body)
				}
				_ => None,
			},
			_ => None,
		}
		.or_else(|| swift_closure(items))
		.or_else(|| shorthand_block_lambda(node, items)),
		_ => None,
	}
}

/// The lambda of `head => body`: TypeScript's result type `(a: number): number => …`, Swift's `(x: Int) -> Int in body`
fn arrow_parts(head: &Node, body: &Node) -> Option<Lambda> {
	let head = &c_style_parameters(head);
	let (left, result_type) = match head.drop_meta() {
		Node::Key(parameters, Op::Colon, result)
			if matches!(parameters.drop_meta(), Node::List(_, Bracket::Round, _)) =>
		{
			(parameters.as_ref(), Some(result.as_ref().clone()))
		}
		_ => (head, None),
	};
	let (body, result_type) = match body.drop_meta() {
		Node::List(parts, Bracket::None, separator @ Separator::Space)
			if matches!(left.drop_meta(), Node::List(_, Bracket::Round, _)) =>
		{
			match parts.as_slice() {
				[result, word, rest @ ..] if !rest.is_empty() && word.is_symbol(SWIFT_IN) => {
					let body = match rest {
						[single] => single.clone(),
						many => Node::List(many.to_vec(), Bracket::None, separator.clone()),
					};
					(body, Some(result.clone()))
				}
				_ => (body.clone(), result_type),
			}
		}
		_ => (body.clone(), result_type),
	};
	let params = parameter_names(left)?;
	let body = subtract_kebab_parameters(body, &params);
	let written: Vec<Node> = match left.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		other => vec![other.clone()],
	};
	let declares =
		written.iter().any(|parameter| matches!(parameter.drop_meta(), Node::Key(_, Op::Colon | Op::Assign, _)));
	Some(Lambda { params, body, written: if declares { written } else { vec![] }, result_type })
}

/// PHP's `fn($x) => …` without its keyword, C#'s `(int x, int y) => …` as `(x: int, y: int)`
fn c_style_parameters(head: &Node) -> Node {
	// C#'s `int x`, Go's `x int`
	let typed = |parameter: &Node| match parameter.drop_meta() {
		Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [kind, name] if is_kinded_type_word(kind) && matches!(name.drop_meta(), Node::Symbol(_))) => {
			key(words[1].clone(), Op::Colon, words[0].clone())
		}
		Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [name, kind] if is_kinded_type_word(kind) && matches!(name.drop_meta(), Node::Symbol(_)) && !is_kinded_type_word(name)) => {
			key(words[0].clone(), Op::Colon, words[1].clone())
		}
		_ => parameter.clone(),
	};
	match head.drop_meta() {
		Node::List(words, _, _) if matches!(words.as_slice(), [keyword, _] if matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word))) => {
			c_style_parameters(&words[1])
		}
		// OCaml/F# `fun a b -> …`, parsed as `(fun a) b`
		Node::List(_, Bracket::None, Separator::Space)
			if keyword_and_names(head).is_some_and(|names| names.len() > 1) =>
		{
			Node::List(keyword_and_names(head).expect("guarded"), Bracket::Round, Separator::Colon)
		}
		Node::List(words, Bracket::Round | Bracket::None, Separator::Space)
			if words.len() == 2 && words.iter().any(is_kinded_type_word) && typed(head) != *head =>
		{
			Node::List(vec![typed(head)], Bracket::Round, Separator::Colon)
		}
		Node::List(words, Bracket::Round, separator) if words.iter().any(|word| typed(word) != *word) => {
			Node::List(words.iter().map(typed).collect(), Bracket::Round, separator.clone())
		}
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

pub(crate) fn flatten_phrase(node: &Node, words: &mut Vec<Node>) {
	match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) => items.iter().for_each(|item| flatten_phrase(item, words)),
		other => words.push(other.clone()),
	}
}

/// `{it*it}`: a block that is no object; its parameter is `it` when the body uses it
/// Swift's `{ $0 * 2 }`; a block without a `$` name is no lambda, and is not copied to find that out (closures::tail
/// asks for every block)
fn shorthand_block_lambda(node: &Node, items: &[Node]) -> Option<Lambda> {
	let is_shorthand = |symbol: &str| symbol.starts_with(SHORTHAND_MARK);
	items.iter().find_map(|item| symbol_outside_blocks(item, &is_shorthand))?;
	block_lambda(node).filter(|lambda| lambda.params.first().is_some_and(|param| param.starts_with(SHORTHAND_MARK)))
}

fn block_lambda(node: &Node) -> Option<Lambda> {
	let Node::List(items, Bracket::Curly, separator) = node.drop_meta() else {
		return None;
	};
	if items.is_empty() || items.iter().any(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _))) {
		return None;
	}
	let body = match items.as_slice() {
		[single] => single.clone(),
		// `{ it.map { it*10 } }`: words side by side stay a juxtaposition, a call with its trailing block
		many if *separator == Separator::Space => Node::List(many.to_vec(), Bracket::None, Separator::Space),
		many => Node::List(many.to_vec(), Bracket::Round, separator.clone()),
	};
	let params = if body.mentions(IMPLICIT_PARAMETER) {
		vec![IMPLICIT_PARAMETER.to_string()]
	} else {
		shorthand_parameters(&body)
	};
	Some(Lambda::new(params, body))
}

/// Swift's shorthand arguments `$0`, `$1` … up to the highest one the body uses
fn shorthand_parameters(body: &Node) -> Vec<String> {
	let names = |count: usize| (0..count).map(|index| format!("{SHORTHAND_MARK}{index}"));
	let count = (0..MAX_SHORTHAND_ARGUMENTS)
		.rev()
		.find(|index| mentions_outside_blocks(body, &format!("{SHORTHAND_MARK}{index}")))
		.map_or(0, |highest| highest + 1);
	names(count).collect()
}

/// A mention not inside a nested block or lambda: `{ return { $0 + k } }` has no `$0` of its own, the inner closure has
fn mentions_outside_blocks(node: &Node, name: &str) -> bool {
	symbol_outside_blocks(node, &|symbol| symbol == name).is_some()
}

fn symbol_outside_blocks<'a>(node: &'a Node, wanted: &dyn Fn(&str) -> bool) -> Option<&'a Node> {
	match node.drop_meta() {
		Node::Symbol(symbol) if wanted(symbol) => Some(node),
		Node::Key(_, Op::Arrow, _) | Node::List(_, Bracket::Curly, _) => None,
		Node::Key(left, _, right) => {
			symbol_outside_blocks(left, wanted).or_else(|| symbol_outside_blocks(right, wanted))
		}
		Node::List(items, _, _) => items.iter().find_map(|item| symbol_outside_blocks(item, wanted)),
		_ => None,
	}
}

/// The parameter `name` replaced by `argument` in a lambda body, except inside an inner lambda that binds the same name
/// (Kotlin): in `it.map{ it*10 }` the inner `it` is the inner lambda's
fn bind_parameter(node: Node, name: &str, argument: &Node) -> Node {
	let binds_name =
		|block: &Node| block_lambda(block).is_some_and(|lambda| lambda.params.iter().any(|param| param == name));
	match node {
		Node::Symbol(symbol) if symbol == name => argument.clone(),
		Node::Key(receiver, Op::Dot, method)
			if crate::declarations::glued_block(&method).is_some_and(|(_, block)| binds_name(&block)) =>
		{
			Node::Key(Box::new(bind_parameter(*receiver, name, argument)), Op::Dot, method)
		}
		Node::List(items, bracket, separator) if items.len() > 1 && crate::declarations::is_callee(&items[0]) => {
			Node::List(
				items
					.into_iter()
					.map(|item| if binds_name(&item) { item } else { bind_parameter(item, name, argument) })
					.collect(),
				bracket,
				separator,
			)
		}
		other => children_rewritten(other, |child| bind_parameter(child, name, argument)),
	}
}

/// Swift `{ x in x*2 }`, `{ a, b in a+b }`: names, `in`, a body that uses them (`{ x in xs }` stays membership)
fn swift_closure(items: &[Node]) -> Option<Lambda> {
	let (last, leading) = items.split_last()?;
	let Node::List(parts, Bracket::None, Separator::Space) = last.drop_meta() else {
		return None;
	};
	let [name, word, body @ ..] = parts.as_slice() else {
		return None;
	};
	if !word.is_symbol(SWIFT_IN) || body.is_empty() {
		return None;
	}
	let params: Vec<String> =
		leading.iter().chain([name]).map(|param| param.symbol_name().map(String::from)).collect::<Option<_>>()?;
	let body = match body {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), Bracket::None, Separator::Space),
	};
	params.iter().any(|param| body.mentions(param)).then(|| Lambda::new(params, body))
}

/// Swift's trailing closure `xs.reduce(0) { $0 + $1 }`: the call's word and arguments, then the closure
fn trailing_closure(items: &[Node]) -> Option<Vec<Node>> {
	let [call, closure] = items else { return None };
	let Node::List(call_items, _, _) = call.drop_meta() else {
		return None;
	};
	let (word, arguments) = call_items.split_first()?;
	if !matches!(word.drop_meta(), Node::Symbol(_)) || !matches!(closure.drop_meta(), Node::List(_, Bracket::Curly, _))
	{
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
		REDUCE_WORD if extras == 1 => {
			ITERATIONS.iter().find(|fold| fold.word == FOLD_WORD).expect("fold is an iteration")
		}
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
	Some(key(parameters, Op::Arrow, lambda.body))
}

/// The definition `name(params) := body` of a lambda, a block with `it` or an operator given as a value
pub fn lambda_definition(name: &str, function: &Node) -> Option<Node> {
	let lambda = arrow_lambda(function)
		.or_else(|| block_lambda(function).filter(|lambda| !lambda.params.is_empty()))
		.or_else(|| operator_lambda(function))?;
	Some(definition(name, lambda))
}

fn definition(name: &str, lambda: Lambda) -> Node {
	let parameters =
		if lambda.written.is_empty() { lambda.params.into_iter().map(Node::Symbol).collect() } else { lambda.written };
	let head = call(name, parameters);
	// a result type converts the body, as `f(a:number):number := …` is lowered before this pass runs
	let body = match lambda.result_type {
		Some(result_type) => {
			let grouped = parens(vec![lambda.body]);
			key(grouped, Op::As, result_type)
		}
		None => lambda.body,
	};
	key(head, Op::Define, body)
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
			Node::Key(name, Op::Assign, value)
				if matches!(name.drop_meta(), Node::Symbol(_)) && arrow_lambda(value).is_some() =>
			{
				Node::Key(name.clone(), Op::Colon, value.clone())
			}
			_ => item,
		})
		.collect()
}

/// `Path` of `find(folder:Path, pattern:text)`
fn first_parameter_type(defined: &UserFunctionDef) -> Option<String> {
	defined.params.first()?.annotation.as_ref().map(|annotation| annotation.drop_meta().name())
}

struct Lowering {
	context: Context,
	/// the program's classes: a function whose first parameter is one is a method, the iteration word stays for other values
	classes: HashSet<String>,
	path_variables: HashSet<String>,
	counter: Counter,
	strict: bool,
}

impl Lowering {
	fn fresh(&self, prefix: &str) -> String {
		let number = self.counter.next_number();
		format!("{prefix}_{number}")
	}

	fn expand(&self, node: Node) -> Node {
		match node {
			// the parser already reads `f = {it*2}` as the definition `f := {it*2}`
			Node::Key(target, Op::Assign | Op::Define, value)
				if matches!(target.drop_meta(), Node::Symbol(_)) && arrow_lambda(&value).is_some() =>
			{
				let Node::Symbol(name) = target.drop_meta() else { unreachable!("guarded") };
				let lambda = arrow_lambda(&value).expect("guarded");
				definition(name, Lambda { body: self.expand(lambda.body), ..lambda })
			}
			// `sum := fold +`: the iteration word with only its function is a function of the list
			Node::Key(target, Op::Assign | Op::Define, value)
				if matches!(target.drop_meta(), Node::Symbol(_)) && self.partial_application(&value).is_some() =>
			{
				let Node::Symbol(name) = target.drop_meta() else { unreachable!("guarded") };
				let body = self.partial_application(&value).expect("guarded");
				definition(name, Lambda::new(vec![PARTIAL_LIST.to_string()], self.expand(body)))
			}
			// a function head `find(folder:Path, pattern:text) := …` is no call: a class's method may share an iteration's word
			Node::Key(head, Op::Define, body) if !matches!(head.drop_meta(), Node::Symbol(_)) => {
				key(*head, Op::Define, self.expand(*body))
			}
			Node::Key(receiver, Op::Dot, method) if self.iteration_method(&method).is_some() => {
				let (iteration, mut arguments) = self.iteration_method(&method).expect("guarded");
				let function = arguments.pop().expect("a function");
				let extras: Vec<Node> = arguments.into_iter().map(|argument| self.expand(argument)).collect();
				let (receiver, function) = (self.expand(*receiver), self.expand(function));
				match self.iterate(iteration, receiver.clone(), extras.clone(), function.clone()) {
					Some(loop_node) => loop_node,
					None => key(receiver, Op::Dot, call(iteration.word, [extras, vec![function]].concat())),
				}
			}
			Node::List(items, bracket, separator) => {
				let items = named_function_arguments(items, &bracket);
				let items = flatten_prefix_application(items, &bracket, &separator);
				// a one-item group `(x != ø)` is no operand list: its explicit ø stays the operand
				let items: Vec<Node> = if separator == Separator::Space && items.len() > 1 {
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
				// nor in a comma list `(y -> y + 3, 7)`, the arguments of a call (card errors-needs)
				let is_sequence = bracket == Bracket::Square || separator == Separator::Colon;
				let immediate = if is_sequence { None } else { self.immediate_call(&items) };
				immediate
					.or_else(|| self.iteration_call(&items, &bracket, &separator))
					.unwrap_or(Node::List(items, bracket, separator))
			}
			other => children_rewritten(other, |child| self.expand(child)),
		}
	}

	/// The iteration a word names, unless the program defines a function of that name other than a class's method:
	/// `find(file:Path)` leaves `find(xs, f)` the iteration (card cross-module-overload)
	fn iteration_of(&self, word: &Node) -> Option<&'static Iteration> {
		iteration_word(word).filter(|_| match self.context.user_functions.get(&word.drop_meta().name()) {
			Some(defined) => self.is_method(defined),
			None => true,
		})
	}

	/// `find(./notes, "*.txt")`: a path written without quotes where a class's method of the word takes a Path calls
	/// that method, as generics.rs as_path makes it one; other first arguments iterate
	fn calls_method(&self, word: &Node, first_argument: &Node) -> bool {
		let takes_path = |defined| first_parameter_type(defined).is_some_and(|class| class == crate::words::PATH_CLASS);
		self.context.user_functions.get(&word.drop_meta().name()).is_some_and(takes_path)
			&& crate::warp_parser::is_written_path(first_argument, &self.path_variables)
	}

	fn is_method(&self, defined: &UserFunctionDef) -> bool {
		first_parameter_type(defined).is_some_and(|class| self.classes.contains(&class))
	}

	/// `word(extras, function)` after a dot: the iteration and its arguments, the function last
	fn iteration_method(&self, method: &Node) -> Option<(&'static Iteration, Vec<Node>)> {
		let items = match crate::declarations::glued_block(method) {
			// `xs.map{ it*2 }`: `map{…}` after a dot is the call with its lambda, not the data `map:{…}`
			Some((word, block)) if block_lambda(&block).is_some_and(|lambda| !lambda.params.is_empty()) => {
				vec![word, block]
			}
			_ => {
				let Node::List(items, _, _) = method.drop_meta() else {
					return None;
				};
				trailing_closure(items).unwrap_or_else(|| items.clone())
			}
		};
		let (word, arguments) = items.split_first()?;
		let iteration = with_start(self.iteration_of(word)?, arguments.len().checked_sub(1)?); // `xs.sort()` has no function
		let is_function = |node: &Node| arrow_lambda(node).is_some();
		let arguments = match arguments {
			// JS's `xs.reduce((a, b) => a + b, 0)`: the start value after the function
			[function, start] if iteration.word == FOLD_WORD && is_function(function) && !is_function(start) => {
				vec![start.clone(), function.clone()]
			}
			_ => arguments.to_vec(),
		};
		(arguments.len() == iteration.extra_arguments + 1).then_some((iteration, arguments))
	}

	/// `{x*x}(x=5)`: an anonymous function called with its bindings; `{it*2} 3` and `{it^2}[1 2 3]`: with its argument,
	/// which a list broadcasts over like for a named function (broadcasting.rs)
	fn immediate_call(&self, items: &[Node]) -> Option<Node> {
		let [block, arguments] = items else { return None };
		let lambda = block_lambda(block).or_else(|| arrow_lambda(block))?;
		let is_function = |node: &Node| {
			block_lambda(node).or_else(|| arrow_lambda(node)).is_some_and(|lambda| !lambda.params.is_empty())
		};
		let juxtaposed;
		let entries = match arguments.drop_meta() {
			Node::List(entries, Bracket::Round, _) => entries,
			argument if !lambda.params.is_empty() && !is_function(argument) => {
				juxtaposed = vec![argument.clone()];
				&juxtaposed
			}
			_ => return None,
		};
		let bindings: Vec<(String, Node)> = entries.iter().filter_map(named_assignment).collect();
		let (params, values) = if bindings.len() == entries.len() && !bindings.is_empty() {
			(
				bindings.iter().map(|(name, _)| name.clone()).collect(),
				bindings.into_iter().map(|(_, value)| value).collect(),
			)
		} else {
			(lambda.params.clone(), entries.clone())
		};
		let name = self.fresh("lambda");
		let defined = definition(&name, Lambda::new(params, lambda.body));
		Some(crate::broadcasting::lower(Node::List(
			vec![defined, call(&name, values)],
			Bracket::Round,
			Separator::Semicolon,
		)))
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
			let rest: Vec<Node> = rest.iter().filter(|item| !item.is_symbol(ON_WORD)).cloned().collect();
			let rest = labeled_list_first(rest);
			let rest = match rest.as_slice() {
				[first, second]
					if iteration.extra_arguments == 0
						&& self.is_function_value(first)
						&& !self.is_function_value(second) =>
				{
					vec![second.clone(), first.clone()]
				}
				_ => rest,
			};
			// Python's `any(xs)`, `all(xs)`: the items themselves are the conditions
			if let ([list], true) = (rest.as_slice(), is_call && TRUTH_WORDS.contains(&iteration.word)) {
				let item = symbol(TRUTH_ITEM);
				return self.iterate(iteration, list.clone(), vec![], key(item.clone(), Op::Arrow, item));
			}
			let [list, extras @ .., function] = rest.as_slice() else {
				return None;
			};
			let iteration = with_start(iteration, extras.len());
			if extras.len() != iteration.extra_arguments || self.calls_method(head, list) {
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
		let list = symbol(PARTIAL_LIST);
		let identity = match function.drop_meta() {
			Node::Symbol(text) => {
				OPERATOR_IDENTITIES.iter().find(|(known, _)| known == text).map(|(_, identity)| Node::int(*identity))
			}
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
	/// `Err(None)` when the function is not known yet (a parameter, called as a closure by the strict run)
	fn applied(
		&self,
		iteration: &Iteration,
		list: &Node,
		function: &Node,
		arguments: &[Node],
	) -> Result<Node, Option<Node>> {
		match literal_function(function) {
			Some(lambda) if lambda.params.len() == arguments.len() => Ok(lambda
				.params
				.iter()
				.zip(arguments)
				.fold(lambda.body, |body, (param, argument)| bind_parameter(body, param, argument))),
			// a block that does not use `it` takes no argument and is run for every item
			Some(lambda) if lambda.params.is_empty() && arguments.len() == 1 => Ok(lambda.body),
			Some(_) => {
				let count = if iteration.function_arguments == 1 { "one argument" } else { "two arguments" };
				Err(Some(
					Diagnostic::at(function, format!("{} takes a function of {count}", iteration.word)).into_error(),
				))
			}
			None => match function.drop_meta() {
				Node::Symbol(name) if self.context.user_functions.contains_key(name) => {
					Ok(call(name, arguments.to_vec()))
				}
				Node::Symbol(_) if !self.strict => Err(None),
				// a function value known only at run time: a variable or parameter holding a closure, or a call returning one
				_ if crate::closures::may_be_function_value(function) => {
					Ok(crate::closures::closure_call(function.clone(), arguments.to_vec()))
				}
				_ => {
					let params = if iteration.function_arguments == 1 { "x" } else { "(a b)" };
					let fix = format!("{} {} ({params} => …)", iteration.word, list.serialize());
					Err(Some(crate::closures::needs_a_function(iteration.word, None, function, &fix)))
				}
			},
		}
	}

	/// Whether the first of `pair` sorts before the second: a comparison holds (Swift `by: >`), any other comparator is
	/// negative (JS `(a,b) => a-b`); a key of one item (Python `key=len`) sorts by key_sort_template instead
	fn sort_order(
		&self,
		iteration: &Iteration,
		list: &Node,
		function: &Node,
		pair: [Node; 2],
	) -> Result<Node, Option<Node>> {
		let compared = self.applied(iteration, list, function, &pair)?;
		Ok(if self.compares(function) { compared } else { key(compared, Op::Lt, Node::int(0)) })
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
			Some(lambda) => is_boolean_body(&lambda.body),
			None => match function.drop_meta() {
				Node::Symbol(name) => {
					self.context.user_functions.get(name).is_some_and(|defined| is_boolean_body(&defined.body))
				}
				_ => false,
			},
		}
	}

	/// The loop of an iteration word with the body of a literal function inlined
	fn iterate(&self, iteration: &Iteration, list: Node, extras: Vec<Node>, function: Node) -> Option<Node> {
		let names: Vec<(&str, String)> = [
			"out", "item", "acc", "list", "index", "value", "moving", "size", "width", "merged", "low", "middle",
			"high", "left", "right",
		]
		.into_iter()
		.map(|name| (name, self.fresh(&format!("loop_{name}"))))
		.collect();
		let symbol =
			|name: &str| Node::Symbol(names.iter().find(|(prefix, _)| *prefix == name).expect("a loop name").1.clone());
		let sorting = iteration.word == SORT_WORD;
		let function = if sorting { unlabeled(function) } else { function };
		let by_key = sorting && self.parameter_count(&function) == Some(1);
		let arguments: Vec<Node> = if iteration.function_arguments == 2 && !by_key {
			vec![symbol("acc"), symbol("item")]
		} else {
			vec![symbol("item")]
		};
		let applied = if sorting && !by_key {
			self.sort_order(iteration, &list, &function, [symbol("acc"), symbol("item")])
		} else {
			self.applied(iteration, &list, &function, &arguments)
		};
		let applied = match applied {
			Ok(applied) => applied,
			Err(error) => return error,
		};
		let mut template = if by_key { key_sort_template() } else { iteration.template.to_string() };
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
	items
		.into_iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(inner, Bracket::None, Separator::Space) => spliced_applications(inner.clone()),
			_ => vec![item],
		})
		.collect()
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
