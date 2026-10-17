//! Compile-time lambdas. Functions are not first-class values yet, so a lambda is lowered where it stands:
//! - `f = x=>x*x`, `f = (x y)->x+y` define the function `f` at that point (captured variables by value, like every
//!   definition); `f 3` calls it; the parser itself reads `f = {it*2}` as the definition `f := {it*2}`
//! - `{x*x}(x=5)` defines an anonymous function and calls it at once
//! - `map [1 2 3] {it*it}`, `map(xs, x=>x+1)`, `xs.map(f)` over a literal block, a lambda or a defined function is a loop
//! - a lambda anywhere else is a closure (closures.rs); `map` over a value that is no function is the error `map needs a function, got …`

use super::nodes::{call, children_rewritten, is_kinded_type_word, key, named_assignment, Counter};
use crate::analyzer::{call_name, function_definitions};
use crate::compiler::context::{Context, UserFunctionDef};
use crate::errors::diagnostic::Diagnostic;
use crate::library_words::substitute;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse_template;
use crate::words::{EACH_WORD, IMPLICIT_PARAMETER, ON_WORD, SORT_WORD};
use std::collections::{HashMap, HashSet, VecDeque};

/// The most arguments an iteration gives its function: fold and reduce two
const MOST_ITERATION_ARGUMENTS: usize = 2;
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
	let bound = crate::lowering::module_properties::bound_names(&node);
	Lowering { context, classes, path_variables, bound, counter: Counter::starting_at(first_fresh), strict }
		.expand(node)
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

/// `xs map f`, `ps map .a`, `xs filter {it > 2}` (cards infix-map, map-dot): an iteration word between a list and its
/// function is the method call `xs.map(f)`, left to right, `xs map f filter g` is `xs.map(f).filter(g)`; a prefix
/// `map f xs` stays. The list is the rightmost operand before the word: `f() := xs map g` maps xs, `return xs map g`
/// returns the mapped list, `ps sort by .a` labels the function. A function of the program calls what follows,
/// `f sorted xs`
pub fn lower_infix_iterations(program: Node) -> Node {
	let functions: HashSet<String> = function_definitions(&program).user_functions.into_keys().collect();
	let (parameters, labels) = definition_parameters(&program);
	let variables: HashSet<String> = crate::music_words::defined_names(&program)
		.into_iter()
		.filter(|name| !functions.contains(name))
		.chain(parameters)
		.chain([IMPLICIT_PARAMETER.to_string()])
		.collect();
	infix_iterations(program, &ProgramWords { variables, labels })
}

/// `variables`: the words the program assigns or takes as parameters; any other word before an iteration word is
/// a function or a module's word, `square all 1 to 5`, and the iteration stays prefix. `labels`: each function's
/// parameter labels, `f(xs, each y)`, which stay labels in a call of that function, `f [1 2 3] each 2`
struct ProgramWords {
	variables: HashSet<String>,
	labels: HashMap<String, HashSet<String>>,
}

/// `f(ys:list, n, each d) := …`: the parameter names ys, n and d, typed or not, and each function's labels (f: each)
fn definition_parameters(program: &Node) -> (Vec<String>, HashMap<String, HashSet<String>>) {
	let (mut names, mut labels) = (Vec::new(), HashMap::<String, HashSet<String>>::new());
	program.visit(&mut |node| {
		let Node::Key(head, Op::Define | Op::Assign, _) = node else { return };
		let Some(Node::List(items, _, _)) = definition_call(head) else { return };
		let function = crate::node::leading_name(&items[0]);
		for parameter in &items[1..] {
			match parameter.drop_meta() {
				Node::List(words, Bracket::None, _) if words.len() > 1 => {
					labels.entry(function.clone()).or_default().insert(crate::node::leading_name(&words[0]));
					names.push(crate::node::leading_name(&words[words.len() - 1]));
				}
				_ => names.push(crate::node::leading_name(parameter)),
			}
		}
	});
	(names, labels)
}

fn infix_iterations(node: Node, words: &ProgramWords) -> Node {
	let is_list_operand = |node: &Node| {
		is_list_operand(node) && node.drop_meta().symbol_name().is_none_or(|name| words.variables.contains(name))
	};
	match node {
		// `ys = sort xs by >`: the parser hung the phrase on the operator, `(sort xs by) > ø`
		Node::Key(..) if dangling_sort_operator(&node).is_some_and(|(words, label)| sorts_before(&words[..label])) => {
			let (words_of_sort, _) = dangling_sort_operator(&node).expect("checked");
			infix_iterations(Node::List(words_of_sort, Bracket::None, Separator::Space), words)
		}
		// `melody(notes, each duration) := …`: a definition's head names its parameters, `each` a label
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if definition_call(&head).is_some() => {
			Node::Key(head, op, Box::new(infix_iterations(*body, words)))
		}
		Node::List(items, bracket, separator) if is_phrase(&bracket, &separator) => {
			// a label of the function this phrase calls
			let called_labels = items.first().and_then(|head| words.labels.get(&crate::node::leading_name(head)));
			let is_label =
				|node: &Node| called_labels.is_some_and(|labels| labels.contains(&crate::node::leading_name(node)));
			let items = items.into_iter().map(|item| infix_iterations(item, words)).collect();
			let mut pending: VecDeque<Node> = prefix_sort_grouped(labeled_sort_functions(items), &bracket).into();
			let mut combined: Vec<Node> = Vec::with_capacity(pending.len());
			let mut changed = false;
			while let Some(item) = pending.pop_front() {
				// `ys map` grouped as a phrase of its own before the function: its words are this phrase's
				if let Some(phrase) = phrase_ending_in_iteration(&item, is_list_operand)
					.filter(|phrase| !pending.is_empty() && !phrase.last().is_some_and(is_label))
				{
					phrase.into_iter().rev().for_each(|word| pending.push_front(word));
					continue;
				}
				let after_list =
					combined.last().is_some_and(|left| is_list_operand(rightmost_operand(left))) && !is_label(&item);
				// `xs map f`, or `xs (map f)` where the phrase grouped the prefix call first (`ys = xs map f`)
				let iteration = after_list
					.then(|| grouped_iteration(&item).or_else(|| bare_iteration(&item, &mut pending)))
					.flatten();
				match iteration {
					Some((word, function)) => {
						let left = combined.pop().expect("a list before");
						combined.push(attach_iteration(left, &word, function));
						changed = true;
					}
					None => combined.push(item),
				}
			}
			match (changed, &bracket) {
				(true, Bracket::None | Bracket::Round) if combined.len() == 1 => combined.pop().expect("one"),
				_ => Node::List(combined, bracket, separator),
			}
		}
		other => other.map_children(|child| infix_iterations(child, words)),
	}
}

/// the `f(x)` of a definition head `f(x)` or `f(x): int` before `:=`: a function's name and parameters
fn definition_call(head: &Node) -> Option<&Node> {
	match head.drop_meta() {
		Node::Key(head, Op::Colon | Op::As, _) => definition_call(head),
		call @ Node::List(items, Bracket::Round, _)
			if items.first().is_some_and(|name| matches!(name.drop_meta(), Node::Symbol(_))) =>
		{
			Some(call)
		}
		_ => None,
	}
}

/// Words side by side in one statement: `xs map f`, `(xs map f)`, a block's one phrase `{ return xs map f }`
fn is_phrase(bracket: &Bracket, separator: &Separator) -> bool {
	matches!(bracket, Bracket::None | Bracket::Round | Bracket::Curly)
		&& !separator.separates_statements()
		&& *separator != Separator::Comma
}

/// `map f` with the iteration word as its own item: the function after it
fn bare_iteration(item: &Node, pending: &mut VecDeque<Node>) -> Option<(String, Node)> {
	iteration_word(item)?;
	Some((item.name(), pending.pop_front()?))
}

/// A sort label after an iteration word and the function after it are the labeled function, `by: f`, the way a call's
/// parentheses write it: `sort xs by >`, `xs sort by >`, `sort ps by .a`, `sort ps by {it.a}` (card sort-xs)
fn labeled_sort_functions(items: Vec<Node>) -> Vec<Node> {
	let mut labeled = Vec::with_capacity(items.len());
	let mut after_iteration = false;
	let mut rest = items.into_iter();
	while let Some(item) = rest.next() {
		// `by >` and `by > map f`: the parser hung the rest of the phrase on the operator
		if let Some((words, _)) = dangling_sort_operator(&item).filter(|_| after_iteration) {
			labeled.extend(words);
			continue;
		}
		let function = match item.drop_meta() {
			_ if !after_iteration => None,
			Node::Key(label, Op::Colon, function) if is_sort_label(label) => {
				Some((label.as_ref().clone(), function.as_ref().clone()))
			}
			_ if is_sort_label(&item) => rest.next().map(|function| (item.clone(), function)),
			_ => None,
		};
		after_iteration |= iteration_word(&item).is_some();
		labeled.push(match function {
			Some((label, function)) if member_of(&function).is_some() => {
				key(label, Op::Colon, member_lambda(&function))
			}
			Some((label, function)) => key(label, Op::Colon, function),
			None => item,
		});
	}
	labeled
}

/// `sort xs by: > map f`: the prefix sort ends at its labeled function, the call `sort(xs, by: >)` the list of what
/// follows; `(sort xs by: >)` is that call too
fn prefix_sort_grouped(mut items: Vec<Node>, bracket: &Bracket) -> Vec<Node> {
	let labeled = |item: &Node| matches!(item.drop_meta(), Node::Key(label, Op::Colon, _) if is_sort_label(label));
	let Some(word) = items.first().filter(|first| iteration_word(first).is_some()).map(Node::name) else {
		return items;
	};
	let last = items.len() - 1;
	// the list comes first: `sort xs by: >`; `.sorted(by: >)` is a method's arguments
	let call_ends_at = (2..=last).find(|&at| {
		labeled(&items[at])
			&& if at == last { *bracket == Bracket::Round } else { iteration_word(&items[at + 1]).is_some() }
	});
	match call_ends_at {
		Some(at) => {
			let rest = items.split_off(at + 1);
			[vec![call(&word, items.split_off(1))], rest].concat()
		}
		None => items,
	}
}

/// An iteration word among the words, the one a sort label after them belongs to
fn sorts_before(words: &[Node]) -> bool {
	words.iter().any(|word| iteration_word(word).is_some())
}

/// The words of `… by > rest` with the operator as the labeled function, `… by: > rest`, and where that label stands,
/// when the parser hung them on the operator after a sort label: `(sort xs by) > ø`, `by > (map f)`
fn dangling_sort_operator(node: &Node) -> Option<(Vec<Node>, usize)> {
	// `by: > map f`: the colon's value is the operator with the rest hung on it, as `by > map f`
	if let Node::Key(label, Op::Colon, value) = node.drop_meta() {
		return match value.drop_meta() {
			Node::Key(nothing, op, rest) if nothing.is_empty_node() && !rest.is_empty_node() => {
				dangling_sort_operator(&key(label.as_ref().clone(), *op, rest.as_ref().clone()))
			}
			_ => None,
		};
	}
	let Node::Key(left, op, right) = node.drop_meta() else { return None };
	if !op.is_operation() || operator_symbol(*op).is_none() {
		return None;
	}
	let mut words = phrase_words(left);
	let label = words.pop().filter(is_sort_label)?;
	let label_at = words.len();
	words.push(key(label, Op::Colon, key(Node::Empty, *op, Node::Empty)));
	words.extend(phrase_words(right).into_iter().filter(|word| !word.is_empty_node()));
	Some((words, label_at))
}

/// The words of a phrase, nested phrases spliced in (`(sort xs) by`), or the one node
fn phrase_words(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, separator) if !separator.separates_statements() => {
			items.iter().flat_map(phrase_words).collect()
		}
		_ => vec![node.clone()],
	}
}

/// `ys map` of `f(ys) := ys map sq`, which the parser grouped before the function: its words
fn phrase_ending_in_iteration(node: &Node, is_list_operand: impl Fn(&Node) -> bool) -> Option<Vec<Node>> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, separator) if !separator.separates_statements() => match items.as_slice() {
			[.., left, word] if iteration_word(word).is_some() && is_list_operand(rightmost_operand(left)) => {
				Some(items.clone())
			}
			_ => None,
		},
		_ => None,
	}
}

/// An operator that takes a whole phrase on its right: `f() := xs map g`, `n = xs map g`, `a + xs map g`; a lookup
/// `p.xs map g` keeps its receiver
fn takes_phrase_on_the_right(op: &Op) -> bool {
	matches!(op, Op::Assign | Op::Define | Op::Colon)
		|| op.is_compound_assign()
		|| op.is_operation() && !op.is_prefix() && !op.is_suffix()
}

/// The operand right before the next word: `xs` of `f() := xs`, of `return xs`
fn rightmost_operand(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::Key(_, op, right) if takes_phrase_on_the_right(op) => rightmost_operand(right),
		Node::List(items, Bracket::None, separator) if !separator.separates_statements() && !items.is_empty() => {
			rightmost_operand(items.last().expect("an item"))
		}
		other => other,
	}
}

/// `xs.word(function)` in place of the rightmost operand
fn attach_iteration(left: Node, word: &str, function: Node) -> Node {
	match left {
		Node::Meta { node, data } => Node::Meta { node: Box::new(attach_iteration(*node, word, function)), data },
		Node::Key(target, op, right) if takes_phrase_on_the_right(&op) => {
			Node::Key(target, op, Box::new(attach_iteration(*right, word, function)))
		}
		Node::List(mut items, Bracket::None, separator) if !separator.separates_statements() && !items.is_empty() => {
			let last = items.pop().expect("an item");
			items.push(attach_iteration(last, word, function));
			Node::List(items, Bracket::None, separator)
		}
		list => key(list, Op::Dot, call(word, vec![function])),
	}
}

/// `map f` of a phrase grouped as a call without brackets: the iteration word and its one function
fn grouped_iteration(node: &Node) -> Option<(String, Node)> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, separator) if !separator.separates_statements() => match items.as_slice() {
			[word, function] if iteration_word(word).is_some() => Some((word.name(), function.clone())),
			_ => None,
		},
		_ => None,
	}
}

/// What may stand left of an infix iteration word: not a word that calls what follows, `print map f xs`,
/// `count filter f xs`, or a statement word, `return find x`, `await all jobs`
fn is_list_operand(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => {
			!(crate::library_words::is_calling_word(name)
				|| is_iteration_word(name)
				|| crate::soft_keywords::is_statement_word(name)
				|| crate::operators::is_operator_word(name))
		}
		_ => true,
	}
}

/// `xs.map(.floor)`, `people.sort(by: .age)`, `f = .name`: a leading-dot word in a code position, an argument in a call's
/// parentheses or the value of `=` / `:=`, is the lambda `it => it.age` on its member, as Swift's `.member` (card
/// dot-lambda), written out before any pass reads it so it goes the way of a written lambda. Element attributes and data
/// entries are no code: `div{ class: .card }` keeps its symbol. `&floor` stays the reference to the function itself
pub fn lower_member_lambdas(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::Round, separator) => {
			Node::List(items.into_iter().map(argument_lambda).collect(), Bracket::Round, separator)
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) if member_of(&value).is_some() => {
			Node::Key(target, op, Box::new(member_lambda(&value)))
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_member_lambdas(*node)), data },
		other => other.map_children(lower_member_lambdas),
	}
}

/// An argument `.age` or `by: .age` as its lambda
fn argument_lambda(argument: Node) -> Node {
	match argument.drop_meta() {
		_ if member_of(&argument).is_some() => member_lambda(&argument),
		Node::Key(label, Op::Colon, value) if member_of(value).is_some() => {
			Node::Key(label.clone(), Op::Colon, Box::new(member_lambda(value)))
		}
		_ => lower_member_lambdas(argument),
	}
}

/// `it => it.age` of `.age`
fn member_lambda(dot_word: &Node) -> Node {
	let member = member_of(dot_word).expect("a dot word");
	let it = || symbol(IMPLICIT_PARAMETER);
	key(it(), Op::Arrow, key(it(), Op::Dot, symbol(member)))
}

/// `age` of `.age`
fn member_of(node: &Node) -> Option<&str> {
	let name = node.symbol_name()?;
	name.strip_prefix('.').filter(|member| member.starts_with(|letter: char| letter.is_alphabetic()))
}

/// `map`, `filter`, `sorted` … as a word, whether or not the program defines a function of that name
/// `find`, `map`, `filter`, … under any spelling: a class's method of that name keeps them for other values
/// `by` of `sort by: f`, `key` of `sort(key=f)`
pub(crate) fn is_sort_label(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if SORT_LABELS.contains(&word.as_str()))
}

pub(crate) fn is_iteration_word(name: &str) -> bool {
	iteration_word(&Node::Symbol(name.to_string())).is_some()
}

fn iteration_word(word: &Node) -> Option<&'static Iteration> {
	let name = word.symbol_name()?;
	let canonical =
		ITERATION_SPELLINGS.iter().find(|(_, spellings)| spellings.contains(&name)).map_or(name, |(word, _)| word);
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

/// `map f on: xs`: the list its label names comes first, the other arguments keep their order after it; a function the
/// program defines was applied to the label, `(square on): xs`, its words stay arguments
fn labeled_list_first(mut arguments: Vec<Node>) -> Vec<Node> {
	match arguments.iter().position(|argument| labeled_list(argument).is_some()) {
		Some(at) => {
			let (list, words) = labeled_list(&arguments.remove(at)).expect("found");
			arguments.splice(at..at, words);
			[vec![list], arguments].concat()
		}
		None => arguments,
	}
}

/// The list of the labeled argument `on: xs`, with the words a program's function took before the label,
/// `(square on): xs` → xs, [square]
pub(crate) fn labeled_list(argument: &Node) -> Option<(Node, Vec<Node>)> {
	let is_label = |word: &Node| word.symbol_name().is_some_and(|name| LIST_LABELS.contains(&name));
	let Node::Key(label, Op::Colon, list) = argument.drop_meta() else {
		return None;
	};
	match label.drop_meta() {
		Node::List(words, _, _) if words.last().is_some_and(is_label) => {
			Some((list.as_ref().clone(), words[..words.len() - 1].to_vec()))
		}
		_ if is_label(label) => Some((list.as_ref().clone(), vec![])),
		_ => None,
	}
}

/// The sorting function without its label: `by: >`, `key: s => s.length`, `key=f`
fn unlabeled(function: Node) -> Node {
	match function.drop_meta() {
		Node::Key(label, Op::Colon | Op::Assign, inner) if is_sort_label(label) => inner.as_ref().clone(),
		Node::Key(head, arrow @ Op::Arrow, body) => match head.drop_meta() {
			Node::Key(label, Op::Colon, parameters) if is_sort_label(label) => {
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
						Separator::Comma,
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
			Node::List(keyword_and_names(head).expect("guarded"), Bracket::Round, Separator::Comma)
		}
		Node::List(words, Bracket::Round | Bracket::None, Separator::Space)
			if words.len() == 2 && words.iter().any(is_kinded_type_word) && typed(head) != *head =>
		{
			Node::List(vec![typed(head)], Bracket::Round, Separator::Comma)
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
		several => Node::List(several.iter().cloned().map(Node::Symbol).collect(), Bracket::Round, Separator::Comma),
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
	/// the names the program gives (module_properties::bound_names): a function value may be any of them
	bound: HashSet<String>,
	counter: Counter,
	strict: bool,
}

impl Lowering {
	/// A word no variable, parameter, function or library word of the program is
	fn names_nothing(&self, name: &str) -> bool {
		!self.bound.contains(name)
			&& !self.path_variables.contains(name)
			&& !self.context.user_functions.contains_key(name)
			&& crate::lowering::library_signatures::library_signature(name).is_none()
	}

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
				let is_sequence = bracket == Bracket::Square || separator == Separator::Comma;
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

	/// A function given as a value: a lambda or block, an operator, the name of a defined function or of a library word
	/// of one value (`map floor xs`)
	fn is_function_value(&self, node: &Node) -> bool {
		arrow_lambda(node).is_some()
			|| block_lambda(node).is_some()
			|| operator_lambda(node).is_some()
			|| matches!(node.drop_meta(), Node::Symbol(name)
				if self.context.user_functions.contains_key(name) || crate::library_words::is_one_value_function(name)
					|| (1..=MOST_ITERATION_ARGUMENTS).any(|arguments| self.arity_variant(name, arguments).is_some()))
	}

	fn arity_variant(&self, name: &str, arguments: usize) -> Option<String> {
		crate::overloads::variant_called_with(name, arguments, |name| self.context.user_functions.contains_key(name))
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
				// `map(f)` of f overloaded by arity: the variant taking as many arguments as the iteration gives
				Node::Symbol(name) if self.arity_variant(name, arguments.len()).is_some() => {
					Ok(call(&self.arity_variant(name, arguments.len()).expect("guarded"), arguments.to_vec()))
				}
				// `map floor xs`: a library word of one value applies to each item as a defined function does
				Node::Symbol(name) if arguments.len() == 1 && crate::library_words::is_one_value_function(name) => {
					Ok(call(crate::library_words::canonical_spelling(name), arguments.to_vec()))
				}
				Node::Symbol(_) if !self.strict => Err(None),
				// `xs.map(odd)` of an odd nothing defines: named now, not a run-time "not a function" (card map-odd)
				Node::Symbol(name) if self.names_nothing(name) => {
					Err(Some(crate::ffi::undefined_function_diagnostic(function, name).into_error()))
				}
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
				// `sort(xs, by: count)`: a library word of one value is a key
				Node::Symbol(name) => self
					.context
					.user_functions
					.get(name)
					.map(|defined| defined.params.len())
					.or_else(|| crate::library_words::is_one_value_function(name).then_some(1)),
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
		let mut program = substitute(
			substitute(parse_template(&template, &list), LIST_PLACEHOLDER, &list),
			CALL_PLACEHOLDER,
			&applied,
		);
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
