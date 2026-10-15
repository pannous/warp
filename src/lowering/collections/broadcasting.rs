//! Functions broadcast over lists and pairs (wiki/broadcasting.md, notes/broadcasting.md): `square [1 2 3]` is
//! `[square(1) square(2) square(3)]`, `square [a:1 b:2]` is `[a:square(1) b:square(2)]`, and a variable only ever assigned
//! a list literal is mapped (`square xs` → `map xs square`). A function of one parameter broadcasts when the parameter is
//! declared with a scalar type (`x:int`, `square number`, P50) or, undeclared, is an arithmetic operand of the body; a
//! function of a list (`count(xs)`, `xs#2`, `xs:list`) takes the list whole. Operators never broadcast (`[1 2 3]*2`).
//! A field read on a list of instances reads it of each: `people's name`, `people.name`, `name of people` are
//! `people.map(p => p.name)` (card people-map).

use super::nodes::{call, called_name, children_rewritten, is_colon_pair, key, parameter_name, Counter};
use crate::analyzer::{annotated_kind, list_element_type};
use crate::function_values::{definitions, Definition};
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::traits::InstanceTypes;
use crate::type_kinds::Kind;
use crate::words::EXTREMUM_WORDS;
use crate::words::{ALL_WORD, COUNT_WORD, DOT_WORD, EACH_WORD, IMPLICIT_PARAMETER, LOG_WORD, MAP_WORD, SUM_WORD};
use std::collections::{HashMap, HashSet};

const ARITHMETIC: [Op; 6] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Pow];
/// Parameter types a comparison's result fits
const TRUTH_TYPES: [&str; 3] = ["bool", "boolean", "any"];
const BROADCAST_ITEM: &str = "broadcast_item";
/// Library words of one text or number that broadcast over a list like a user function of a scalar, as do libm's words
/// of one number (`sin xs`, card cos-instead)
const SCALAR_LIBRARY_WORDS: [&str; 6] = ["upper", "lower", "trim", "floor", "ceil", "round"];
/// Library words of several arguments, which of them takes one value: `log(xs, 10)` maps over xs
const LIBRARY_SCALAR_PARAMETERS: [(&str, &[bool]); 1] = [(LOG_WORD, &[true, false])];
/// Prefix operators of one number: `abs [-1 2]`, `sqrt xs`
const SCALAR_OPERATORS: [Op; 3] = [Op::Abs, Op::Sqrt, Op::Cbrt];
/// The prelude word deciding an element-wise operator at run time, and its lambda's parameters (no program writes them)
pub const ELEMENT_WISE_ANY: &str = "element_wise_any";
const ELEMENT_WISE_SIDES: [&str; 2] = ["element_left", "element_right"];
/// The item `square all xs` applies square to
const ALL_ITEM: &str = "all_item";
/// Declared parameter kinds that take one element of a list (P50)
const SCALAR_KINDS: [Kind; 4] = [Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint];
/// `xs .op ys` of two lists (card gpu-vectors, notes/gpu.md): the lists held once, their items paired by index
pub(crate) const PAIRED_TEMPLATE: &str =
	"(LEFT = paired_left; RIGHT = paired_right; (1 to COUNT).map(paired_index => paired_item))";
const PAIRED_ITEM: &str = "LEFT#paired_index + RIGHT#paired_index";
pub(crate) const PAIRED_COUNT: &str =
	"(if #LEFT == #RIGHT then #LEFT else raise \"element-wise operator: the lists differ in length\")";
/// `sum(xs .* ys)` and `sum(xs .* 3)` fused into one loop, no list of the products built (5–20× faster, notes/gpu.md)
pub(crate) const PAIRED_SUM_TEMPLATE: &str = "(LEFT = paired_left; RIGHT = paired_right; SUM = 0; for paired_index in 1 to COUNT { SUM = SUM + paired_item }; SUM)";
const FUSED_SUM_TEMPLATE: &str = "(ITEMS = fused_list; SUM = 0; for ITEM in ITEMS { SUM = SUM + fused_item }; SUM)";

/// P84 (user: "This should have already been done with broadcasting"): several juxtaposed arguments of a function of one
/// parameter are one list, `sum 1 2 3` is `sum [1 2 3]`, and a scalar function broadcasts over it (`square 1 2 3`).
/// After the lambda passes, which make `sum := fold +` a function of one parameter; `f(1, 2, 3)` stays an arity error.
pub fn lower_several_arguments(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
	implicit_definitions(&program, &mut found);
	let single: HashSet<String> = found
		.iter()
		.filter(|definition| definition.params.len() == 1)
		.map(|definition| definition.name.clone())
		.collect();
	if single.is_empty() {
		return program;
	}
	let mut gathered = false;
	let program = gather_arguments(program, &single, &mut gathered);
	if gathered {
		lower(program)
	} else {
		program
	}
}

/// Functions are prefix operators (wiki/function.md): when a call has more juxtaposed arguments than its function takes,
/// the last function name among them takes the arguments after it, `square square 2` → `square (square 2)`. Before the
/// function value passes read that name as a value.
/// A call short of arguments takes the items after its comma, as in Ruby: `add 1, 2` → `add(1, 2)`, `x = add 1, 2`
pub fn lower_prefix_calls(program: Node) -> Node {
	let program = all_calls(program);
	let mut found = Vec::new();
	definitions(&program, &mut found);
	implicit_definitions(&program, &mut found); // `square := it*it` and `f = x => …` take arguments too (P143)
	let library_calls = LibraryCalls::of(&program, &found);
	let program = nest_library_calls(program, &library_calls);
	let mut arities: HashMap<String, Arity> = HashMap::new();
	for definition in &found {
		let count = definition.params.len();
		// P149 (user): an untyped parameter the body uses in arithmetic (`x + 1`, `it*it`) is no truth value either
		let typed = definition.params.first().is_some_and(|first| {
			rules_out_a_truth_value(first) || is_arithmetic_operand(&definition.body, &first.name())
		});
		let arity = arities.entry(definition.name.clone()).or_insert(Arity {
			fewest: count,
			most: count,
			typed_first: typed,
			converts: false,
		});
		*arity = Arity {
			fewest: arity.fewest.min(count),
			most: arity.most.max(count),
			typed_first: arity.typed_first && typed,
			converts: false,
		};
	}
	// `text hex 255`, `text(byte 65)`: a built-in type converts one value, nesting like a function of one parameter.
	// Next to a comparison it follows the user functions' rule: its value is typed as the type itself, `int 3 == 3` is
	// `(int 3) == 3` like `f(x: int)`; a truth type (`bool 3 == 3`) takes anything and is ambiguous like `same(x) := x`
	for name in type_calls_used(&program) {
		let typed = !TRUTH_TYPES.contains(&name.to_lowercase().as_str());
		arities.entry(name).or_insert(Arity { fewest: 1, most: 1, typed_first: typed, converts: true });
	}
	// `round(floor x)`, `upper(lower t)`: a library word of a typed value nests like a function of its parameters
	for (name, count) in library_calls_used(&program) {
		arities.entry(name).or_insert(Arity { fewest: count, most: count, typed_first: true, converts: false });
	}
	if arities.is_empty() {
		return program;
	}
	// `puts add 1 2`: the output words take one value
	for word in crate::wasm_emitter::OUTPUT_WORDS {
		arities.entry(word.to_string()).or_insert(Arity { fewest: 1, most: 1, typed_first: false, converts: false });
	}
	nest_prefix_calls(program, &arities)
}

/// `square all xs` (wiki/all.md): `all` splices the list into its items, the function applies to each
fn all_calls(node: Node) -> Node {
	match node {
		// also `square(all xs)`, whose call items are `square all xs`
		Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None)
			if all_call_parts(&items).is_some() =>
		{
			let (function, list) = all_call_parts(&items).expect("guarded");
			// a list literal spliced at compile time: `square all [[1, 2], [3]]` is `[square [1, 2], square [3]]`,
			// each call broadcast again
			if let Node::List(elements, Bracket::Square, element_separator) = list.drop_meta() {
				let calls = elements
					.iter()
					.map(|element| {
						Node::List(vec![function.clone(), all_calls(element.clone())], Bracket::None, Separator::Space)
					})
					.collect();
				return Node::List(calls, Bracket::Square, element_separator.clone());
			}
			let item = symbol(ALL_ITEM);
			let applied = parens(vec![function.clone(), item.clone()]);
			let each = key(item, Op::Arrow, applied);
			call(MAP_WORD, vec![all_calls(list.clone()), each])
		}
		other => other.map_children(all_calls),
	}
}

/// `all xs` as an argument: xs
fn all_marked(argument: &Node) -> Option<&Node> {
	match argument.drop_meta() {
		Node::List(words, Bracket::None, Separator::Space) if words.len() == 2 && words[0].is_symbol(ALL_WORD) => {
			Some(&words[1])
		}
		_ => None,
	}
}

/// The function and the list of `f all xs`, also as the parser nests a known function's argument, `f (all xs)`
pub(crate) fn all_call_parts(items: &[Node]) -> Option<(&Node, &Node)> {
	let is_all = |node: &Node| node.is_symbol(ALL_WORD);
	match items {
		[function, all, list] if is_all(all) => Some((function, list)),
		// `square (all xs)` as the parser nests a known function's argument, `(square all) xs` as a value
		[first, second] => match (first.drop_meta(), second.drop_meta()) {
			(_, Node::List(words, Bracket::None, Separator::Space)) if words.len() == 2 && is_all(&words[0]) => {
				Some((first, &words[1]))
			}
			(Node::List(words, Bracket::None, Separator::Space), _) if words.len() == 2 && is_all(&words[1]) => {
				Some((&words[0], second))
			}
			_ => None,
		},
		_ => None,
	}
	.filter(|(function, _)| matches!(function.drop_meta(), Node::Symbol(_)))
}

/// The fewest and the most parameters of a function's definitions
#[derive(Clone, Copy)]
struct Arity {
	fewest: usize,
	most: usize,
	/// Every definition's first parameter declares a type that is no truth value (`x: int`, `square number`)
	typed_first: bool,
	/// A built-in type converting its value (`int 3.7`, `hex 255`): nests like a function, compares like one
	converts: bool,
}

/// The built-in type words the program calls and does not name itself: `int`, `text`, `hex`, `byte` … (analyzer::applied_word)
fn type_calls_used(program: &Node) -> HashSet<String> {
	let mut used = HashSet::new();
	program.visit(&mut |node| {
		if let Node::Symbol(name) = node {
			if crate::analyzer::is_type_call_word(name) {
				used.insert(name.clone());
			}
		}
	});
	let names = crate::soft_keywords::names_given(program);
	used.retain(|name| !names.contains(name));
	used
}

/// The library words the program calls and does not name itself whose first parameter is typed, with their arity
fn library_calls_used(program: &Node) -> HashMap<String, usize> {
	let mut used = HashMap::new();
	program.visit(&mut |node| {
		if let Node::Symbol(name) = node {
			if let Some(count) = crate::lowering::library_signatures::typed_first_arity(name) {
				used.insert(name.clone(), count);
			}
		}
	});
	let names = crate::soft_keywords::names_given(program);
	used.retain(|name, _| !names.contains(name));
	used
}

/// `x: int`, `number` (P50, the type-named parameter, as `number: number`): not `b: bool`, not untyped
fn rules_out_a_truth_value(parameter: &Node) -> bool {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, type_node) => {
			let type_name = type_node.drop_meta().name().to_lowercase();
			!TRUTH_TYPES.contains(&type_name.as_str()) && annotated_kind(type_node).is_some()
		}
		_ => false,
	}
}

/// `sum`, `max`, `upper`, `count` …: a library word of one value, not redefined by the program
fn is_one_value_word(node: &Node, arities: &HashMap<String, Arity>) -> bool {
	let is_word = |name: &str| {
		crate::library_words::is_library_word(name) || EXTREMUM_WORDS.contains(&name) || name == COUNT_WORD
	};
	matches!(node.drop_meta(), Node::Symbol(name) if !arities.contains_key(name) && is_word(name))
}

fn arity_of(node: &Node, arities: &HashMap<String, Arity>) -> Option<Arity> {
	match node.drop_meta() {
		Node::Symbol(name) => arities.get(name).copied(),
		_ => None,
	}
}

/// `say 3 == 3` of a function that takes anything reads both ways (P143): a loud error offering each reading
fn ambiguous_call(at: &Node, name: &str, argument: &str, op: &Op, right: &str) -> Node {
	let written = format!("{name} {argument} {op} {right}");
	let call_compared = format!("({name} {argument}) {op} {right}");
	let comparison_argument = format!("{name}({argument} {op} {right})");
	crate::errors::diagnostic::Diagnostic::at(
		at,
		format!("{written} is ambiguous; write {call_compared} or {comparison_argument}"),
	)
	.offer("the call compared", &written, &call_compared)
	.offer("the comparison as the argument", &written, &comparison_argument)
	.into_error()
}

fn nest_prefix_calls(node: Node, arities: &HashMap<String, Arity>) -> Node {
	if let Some((head, body, rebuild)) = crate::lowering::type_tests::function_definition(&node) {
		if let Some(inner) = without_parameters(&head, arities) {
			return rebuild(head, nest_prefix_calls(body, &inner));
		}
	}
	let arity = |node: &Node| arity_of(node, arities);
	match node {
		Node::List(
			items,
			bracket @ (Bracket::None | Bracket::Round),
			separator @ (Separator::Space | Separator::None),
		) if items.len() > 2 && arity(&items[0]).is_some_and(|takes| items.len() - 1 > takes.most) => {
			let mut items: Vec<Node> = items.into_iter().map(|item| nest_prefix_calls(item, arities)).collect();
			let Some(inner) = (1..items.len() - 1).rev().find(|index| arity(&items[*index]).is_some()) else {
				return Node::List(items, bracket, separator);
			};
			let nested = Node::List(items.split_off(inner), Bracket::None, Separator::Space);
			items.push(nest_prefix_calls(nested, arities));
			nest_prefix_calls(Node::List(items, bracket, separator), arities)
		}
		// `sum square [1 2 3]`, `sum(square xs)`: a library word of one value takes the call of a user function after it;
		// a type word there is the library word's argument, `x.nearest(rational, 1000)`
		Node::List(
			items,
			bracket @ (Bracket::None | Bracket::Round),
			separator @ (Separator::Space | Separator::None),
		) if items.len() > 2
			&& is_one_value_word(&items[0], arities)
			&& arity(&items[1]).is_some_and(|takes| !takes.converts && items.len() - 2 <= takes.most) =>
		{
			let mut items = items;
			let call = Node::List(items.split_off(1), Bracket::None, Separator::Space);
			items.push(nest_prefix_calls(call, arities));
			Node::List(items, bracket, separator)
		}
		// `square 3 == 9` parses as `square (3 == 9)`. P143 (user): a parameter typed as no truth value takes the call,
		// `(square 3) == 9` (wiki/all.md `square [1 2 3] == [1 4 9]`); untyped, both readings work: a loud error
		Node::List(items, Bracket::None, Separator::Space) if matches!(items.as_slice(), [head, argument] if arity(head).is_some() && matches!(argument.drop_meta(), Node::Key(_, op, _) if op.is_comparison())) =>
		{
			let [head, argument]: [Node; 2] = items.try_into().expect("guarded");
			let Node::Key(left, op, right) = argument.drop_meta().clone() else { unreachable!("guarded") };
			if !arity(&head).expect("guarded").typed_first {
				return ambiguous_call(&argument, &head.drop_meta().name(), &left.serialize(), &op, &right.serialize());
			}
			let call = Node::List(vec![head, *left], Bracket::None, Separator::Space);
			key(nest_prefix_calls(call, arities), op, nest_prefix_calls(*right, arities))
		}
		// the same phrase the parser grouped the other way, `(f 3) == 9`, when it did not know f takes a value: as
		// ambiguous when the body accepts anything (`f := print it`)
		Node::Key(left, op, right)
			if op.is_comparison()
				&& matches!(left.drop_meta(), Node::List(items, Bracket::None, Separator::Space) if matches!(items.as_slice(), [head, _] if arity(head).is_some_and(|takes| !takes.typed_first))) =>
		{
			let Node::List(items, _, _) = left.drop_meta() else { unreachable!("guarded") };
			ambiguous_call(&left, &items[0].drop_meta().name(), &items[1].serialize(), &op, &right.serialize())
		}
		Node::List(items, Bracket::None, Separator::Colon) if items.len() > 1 => {
			let mut items: Vec<Node> = items.into_iter().map(|item| nest_prefix_calls(item, arities)).collect();
			let rest = items.split_off(1);
			let first = items.pop().expect("split after the first");
			match with_comma_arguments(first, &rest, arities) {
				Ok(call) => call,
				Err(first) => Node::List([vec![first], rest].concat(), Bracket::None, Separator::Colon),
			}
		}
		other => other.map_children(|child| nest_prefix_calls(child, arities)),
	}
}

/// The words of `nest_library_calls`: the calls whose arguments a library word may head, and the names the program gives
struct LibraryCalls {
	functions: HashSet<String>,
	named: HashSet<String>,
	/// the indexes of the parameters each program function calls: `twice(g, x) := g(g(x))` → [0]
	function_parameters: HashMap<String, Vec<usize>>,
}

impl LibraryCalls {
	fn of(program: &Node, found: &[Definition]) -> LibraryCalls {
		let functions = found.iter().map(|definition| definition.name.clone()).collect();
		let function_parameters = crate::function_values::higher_order_parameters(found);
		LibraryCalls { functions, named: crate::soft_keywords::names_given(program), function_parameters }
	}

	/// The library word at item `at` of a call calls the value after it: the caller takes no function there
	/// (`map reverse xs`, `twice(upper, "hi")` pass the word on)
	fn is_value_call(&self, items: &[Node], at: usize) -> bool {
		let takes_function = match items[0].drop_meta() {
			Node::Symbol(name) => match self.function_parameters.get(name) {
				Some(indexes) => indexes.contains(&(at - 1)),
				None => at == 1 && crate::lowering::lambdas::takes_a_function_first(&items[0]),
			},
			_ => false,
		};
		self.is_library_call(&items[at]) && !takes_function
	}

	/// A call taking values: the program's function, or a standard function or output word it does not name itself
	fn is_caller(&self, node: &Node) -> bool {
		let Node::Symbol(name) = node.drop_meta() else { return false };
		self.functions.contains(name) || (crate::library_words::is_calling_word(name) && !self.named.contains(name))
	}

	/// A library word of one value the program does not name itself: `reverse`, `sorted`, `first`, `split`
	fn is_library_call(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::Symbol(name)
			if crate::library_words::needs_one_value(name) && !self.named.contains(name))
	}
}

/// A library word of one value standing where its call expects a value, before a value, is that value's call:
/// `count(reverse xs)` is `count(reverse(xs))`, `max sorted xs` `max(sorted(xs))`, `first sorted reverse xs`
/// `first(sorted(reverse(xs)))`. After a word that may take its function first (`map reverse xs`) the word is the
/// function (card count-sorted)
fn nest_library_calls(node: Node, words: &LibraryCalls) -> Node {
	match node {
		Node::List(
			items,
			bracket @ (Bracket::None | Bracket::Round),
			separator @ (Separator::Space | Separator::None),
		) if items.len() > 2 && words.is_caller(&items[0]) => {
			// `first sorted xs == 1 and yes` compares (first sorted xs): the chain takes the comparison's first operand
			if words.is_value_call(&items, items.len() - 2)
				&& crate::library_words::is_comparison(&items[items.len() - 1])
			{
				let mut items = items;
				let comparison = items.pop().expect("more than two items");
				return crate::library_words::with_first_comparand(comparison, &|operand| {
					let phrase =
						Node::List([items.clone(), vec![operand]].concat(), bracket.clone(), separator.clone());
					nest_library_calls(phrase, words)
				});
			}
			let mut items: Vec<Node> = items.into_iter().map(|item| nest_library_calls(item, words)).collect();
			while let Some(at) = (1..items.len() - 1).rev().find(|at| words.is_value_call(&items, *at)) {
				let value = items.remove(at + 1);
				let word = items.remove(at);
				// written `reverse(xs)`: a later pass splices a phrase `reverse xs` back into its caller's arguments
				items.insert(at, Node::List(vec![word, value], Bracket::Round, Separator::None));
			}
			Node::List(items, bracket, separator)
		}
		other => other.map_children(|child| nest_library_calls(child, words)),
	}
}

/// In a function's body its parameters are values, no functions of the same name: `f(name) := contains(name, "-")`
/// beside a class's name() method
fn without_parameters(head: &Node, arities: &HashMap<String, Arity>) -> Option<HashMap<String, Arity>> {
	let Node::List(items, _, _) = head.drop_meta() else { return None };
	let shadowed: Vec<String> =
		items.iter().skip(1).filter_map(parameter_name).filter(|name| arities.contains_key(name)).collect();
	let unshadowed = arities.iter().filter(|(name, _)| !shadowed.contains(name));
	(!shadowed.is_empty()).then(|| unshadowed.map(|(name, arity)| (name.clone(), *arity)).collect())
}

/// `add 1` (also as the value of `x = add 1`) given the items after its comma, when it is short of arguments and they fit
fn with_comma_arguments(node: Node, rest: &[Node], arities: &HashMap<String, Arity>) -> Result<Node, Node> {
	match node {
		Node::Meta { node, data } => {
			let wrapped = |inner| Node::Meta { node: Box::new(inner), data: data.clone() };
			with_comma_arguments(*node, rest, arities).map(wrapped).map_err(wrapped)
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => match with_comma_arguments(*value, rest, arities) {
			Ok(call) => Ok(Node::Key(target, op, Box::new(call))),
			Err(value) => Err(Node::Key(target, op, Box::new(value))),
		},
		Node::List(mut items, Bracket::None, Separator::Space) => {
			let given = items.len() - 1;
			let fits = arity_of(&items[0], arities)
				.is_some_and(|takes| given < takes.fewest && given + rest.len() <= takes.most);
			if fits {
				return Ok(Node::List([items, rest.to_vec()].concat(), Bracket::None, Separator::Space));
			}
			// `puts add 1, 2`: the short call is the last argument
			let last = items.pop().expect("a call has its function");
			match with_comma_arguments(last, rest, arities) {
				Ok(call) if !items.is_empty() => {
					Ok(Node::List([items, vec![call]].concat(), Bracket::None, Separator::Space))
				}
				Ok(call) => Ok(call),
				Err(last) => Err(Node::List([items, vec![last]].concat(), Bracket::None, Separator::Space)),
			}
		}
		other => Err(other),
	}
}

/// `f 1 2 3` → `f [1 2 3]` of the functions `single`
fn gather_arguments(node: Node, single: &HashSet<String>, gathered: &mut bool) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space)
			if items.len() > 2 && matches!(items[0].drop_meta(), Node::Symbol(name) if single.contains(name)) =>
		{
			*gathered = true;
			let mut items: Vec<Node> = items.into_iter().map(|item| gather_arguments(item, single, gathered)).collect();
			let arguments = items.split_off(1);
			items.push(Node::List(arguments, Bracket::Square, Separator::Space));
			Node::List(items, Bracket::None, Separator::Space)
		}
		other => other.map_children(|child| gather_arguments(child, single, gathered)),
	}
}

pub fn lower(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
	implicit_definitions(&program, &mut found);
	// `f(c: text)` beside `f(b: list)`: an overload takes the list itself (overloads.rs), no definition broadcasts
	let every_overload_needs_a_scalar = |definition: &&Definition| {
		found.iter().filter(|other| other.name == definition.name && other.params.len() == 1).all(needs_a_scalar)
	};
	let mut broadcasting: HashSet<String> = found
		.iter()
		.filter(every_overload_needs_a_scalar)
		.filter(|definition| needs_a_scalar(definition))
		.map(|definition| definition.name.clone())
		.collect();
	// the library words of one scalar: `upper ["ab" "cd"]`, unless the program defines the name itself
	let defined: HashSet<&String> = found.iter().map(|definition| &definition.name).collect();
	broadcasting.extend(scalar_library_words().map(String::from).filter(|word| !defined.contains(word)));
	let library_scalar_parameters = LIBRARY_SCALAR_PARAMETERS
		.iter()
		.filter(|(word, _)| !defined.contains(&word.to_string()))
		.map(|(word, scalars)| (word.to_string(), scalars.to_vec()));
	// a parameter takes a scalar when every overload of as many parameters takes one there
	let mut scalar_parameters: HashMap<String, Vec<bool>> = library_scalar_parameters.collect();
	for definition in &found {
		let scalars: Vec<bool> =
			definition.params.iter().map(|param| takes_a_scalar(param, &definition.body)).collect();
		scalar_parameters
			.entry(definition.name.clone())
			.and_modify(|known| match known.len() == scalars.len() {
				true => known.iter_mut().zip(&scalars).for_each(|(known, scalar)| *known &= scalar),
				false => *known = scalars.clone(),
			})
			.or_insert(scalars);
	}
	let mut assigned = HashMap::new();
	collect_list_variables(&program, &broadcasting, &scalar_parameters, &mut assigned);
	let names: HashSet<String> = assigned.keys().cloned().collect();
	let list_variables: HashSet<String> =
		assigned.into_iter().filter(|(_, only_lists)| *only_lists).map(|(name, _)| name).collect();
	let program = each_singular(program, &list_variables, &names);
	let other_variables = names.difference(&list_variables).cloned().collect();
	let [defines_dot, defines_sum] = [DOT_WORD, SUM_WORD].map(|word| defined.contains(&word.to_string()));
	let instances = element_shapes(&program);
	Broadcast {
		functions: broadcasting,
		list_variables,
		other_variables,
		scalar_parameters,
		defines_dot,
		defines_sum,
		instances,
		paired: Default::default(),
		shadowed: Default::default(),
	}
	.rewrite(program)
}

/// `print each friend`: `each` and a name the program never assigns walk the list of its plural, `print all friends`
/// (wiki/plural.md); `each friends` is `all friends`
fn each_singular(node: Node, lists: &HashSet<String>, names: &HashSet<String>) -> Node {
	let walked_list = |word: &Node| {
		let name = word.symbol_name()?;
		// the English plural and the programmer's: `x` walks `xs` though English writes boxes
		let plurals = [name.to_string(), crate::extensions::strings::english_plural(name), format!("{name}s")];
		let is_plural_of = |plural: &String| *plural == name || !names.contains(name);
		plurals.into_iter().find(|plural| lists.contains(plural) && is_plural_of(plural))
	};
	match node {
		Node::List(items, bracket, separator) => {
			// `each xs {it*10}`: a leading each with a body is the loop over xs (wiki/Iteration.md), no `all` argument
			let is_loop = items.len() > 2 && items[0].is_symbol(EACH_WORD);
			let mut walked = Vec::with_capacity(items.len());
			let mut items = items.into_iter().map(|item| each_singular(item, lists, names)).peekable();
			while let Some(item) = items.next() {
				let list = items.peek().filter(|_| item.is_symbol(EACH_WORD)).and_then(walked_list);
				if let (true, Some(plural)) = (is_loop, &list) {
					items.next();
					walked.extend([item, symbol(plural)]); // `each friend {…}` loops over friends
					continue;
				}
				match list {
					Some(plural) => {
						items.next();
						let all = vec![symbol(ALL_WORD), symbol(&plural)];
						// `each friend` alone is the whole list node, `sq each x` nests it as `sq all xs` parses
						if items.peek().is_none() && walked.is_empty() {
							return Node::List(all, bracket, separator);
						}
						walked.push(Node::List(all, Bracket::None, Separator::Space));
					}
					None => walked.push(item),
				}
			}
			Node::List(walked, bracket, separator)
		}
		other => other.map_children(|child| each_singular(child, lists, names)),
	}
}

/// The shapes of the program's variables as type_constructor (a later pass) will construct its instances, for a program
/// that reads a field by name (`x.name`, `x's name`); None for any other
fn element_shapes(program: &Node) -> Option<InstanceTypes> {
	let mut reads_a_field = false;
	program.visit(&mut |part| {
		if let Node::Key(_, Op::Dot, field) = part {
			reads_a_field |= field.symbol_name().is_some()
		}
	});
	reads_a_field.then(|| InstanceTypes::of(&crate::type_constructor::lower(program.clone())))
}

/// `map(list, broadcast_item => applied(broadcast_item))`
fn each_item(list: Node, applied: impl Fn(Node) -> Node) -> Node {
	let item = symbol(BROADCAST_ITEM);
	let each = key(item.clone(), Op::Arrow, applied(item));
	call(MAP_WORD, vec![list, each])
}

/// A list each_item built: `map(xs, broadcast_item => …)`
fn is_broadcast(node: &Node) -> bool {
	let Node::List(items, _, _) = node.drop_meta() else { return false };
	match items.as_slice() {
		[head, _, each] if head.is_symbol(MAP_WORD) => {
			matches!(each.drop_meta(), Node::Key(item, Op::Arrow, _) if item.is_symbol(BROADCAST_ITEM))
		}
		_ => false,
	}
}

/// `square := it*it`: a function of the implicit parameter `it`
fn implicit_definitions(node: &Node, found: &mut Vec<Definition>) {
	node.visit(&mut |part| {
		let Node::Key(target, op @ (Op::Define | Op::Assign), body) = part else {
			return;
		};
		let Node::Symbol(name) = target.drop_meta() else { return };
		// `f = x => x + 1`: lambdas.rs makes it the definition f(x) later
		if let Some(lambda) = crate::lambdas::arrow_lambda(body) {
			let params = lambda.params.iter().map(|param| Node::Symbol(param.clone())).collect();
			found.push(Definition { name: name.clone(), params, body: lambda.body });
		} else if *op == Op::Define {
			let params = vec![symbol(IMPLICIT_PARAMETER)];
			found.push(Definition { name: name.clone(), params, body: body.as_ref().clone() });
		}
	});
}

fn needs_a_scalar(definition: &Definition) -> bool {
	let [param] = definition.params.as_slice() else {
		return false;
	};
	takes_a_scalar(param, &definition.body)
}

/// A parameter declared with a scalar type, or undeclared and an arithmetic operand of the body or the argument of a
/// scalar library word (`cosine(x) := cos(x)`)
fn takes_a_scalar(param: &Node, body: &Node) -> bool {
	match param.drop_meta() {
		Node::Key(_, Op::Colon, type_node) => {
			annotated_kind(type_node).is_some_and(|kind| SCALAR_KINDS.contains(&kind))
		}
		Node::Symbol(name) => {
			(is_arithmetic_operand(body, name) || is_scalar_word_argument(body, name)) && !tests_kind_of(body, name)
		}
		_ => false,
	}
}

/// SCALAR_LIBRARY_WORDS and libm's words of one number
fn scalar_library_words() -> impl Iterator<Item = &'static str> {
	let libm_words = crate::ffi::LIBM_F64_FUNCTIONS.iter().filter(|(_, arity)| *arity == 1).map(|(word, _)| *word);
	SCALAR_LIBRARY_WORDS.into_iter().chain(libm_words)
}

/// `cos(x)`, `floor x`: name is the one argument of a scalar library word in body
fn is_scalar_word_argument(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node.drop_meta() {
			found |= matches!(items.as_slice(), [word, argument] if argument.is_symbol(name) && scalar_library_words().any(|scalar| word.is_symbol(scalar)));
		}
	});
	found
}

/// `x is number` (`is_type x "number"`): a function testing its parameter's kind takes every kind itself
fn tests_kind_of(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node.drop_meta() {
			found |= matches!(items.as_slice(), [test, tested, _] if test.is_symbol(crate::words::IS_TYPE) && tested.is_symbol(name));
		}
	});
	found
}

fn is_arithmetic_operand(body: &Node, name: &str) -> bool {
	let is_param = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name);
	let is_list_literal = |node: &Node| matches!(node.drop_meta(), Node::List(_, Bracket::Square, _));
	let mut found = false;
	without_element_wise(body.clone()).visit(&mut |node| {
		if let Node::Key(left, op, right) = node {
			// `ys + [x]` concatenates: ys is a list
			let concatenates = *op == Op::Add && (is_list_literal(left) || is_list_literal(right));
			found |= ARITHMETIC.contains(op) && !concatenates && (is_param(left) || is_param(right));
		}
	});
	found
}

/// `a .+ b` as the group (a b): an element-wise operator takes a list or a number on either side at run time
/// (element_wise_any), so its sides are no arithmetic operands making a function broadcast (card compile-shaders)
fn without_element_wise(node: Node) -> Node {
	match element_wise_parts(&node) {
		Some((receiver, _, operand)) => parens(vec![without_element_wise(receiver), without_element_wise(operand)]),
		None => node.map_children(without_element_wise),
	}
}

/// The function and argument of `f x` or `f(x)` when f broadcasts
fn broadcasting_call<'a>(
	items: &'a [Node],
	bracket: &Bracket,
	separator: &Separator,
	functions: &HashSet<String>,
) -> Option<(&'a str, &'a Node)> {
	let is_call = matches!(
		(bracket, separator),
		(Bracket::Round, Separator::None) | (Bracket::None | Bracket::Round, Separator::Space)
	); // also `(f [1 2])`
	let [head, argument] = items else { return None };
	let Node::Symbol(name) = head.drop_meta() else {
		return None;
	};
	(is_call && functions.contains(name)).then_some((name.as_str(), argument))
}

/// Variable → whether every value assigned to it is a list literal that is no object; a broadcast over the variable itself
/// (`ps = moved ps`) keeps what it is, so it needs a list assigned elsewhere (`up(x) := { x = upper x }` maps no text)
fn collect_list_variables(
	node: &Node,
	functions: &HashSet<String>,
	scalar_parameters: &HashMap<String, Vec<bool>>,
	assigned: &mut HashMap<String, bool>,
) {
	// `xs = []` (which parses as ø) is a list once the program appends to xs: `xs.add(i)`, `xs = xs + [i]`
	let mut appended: HashSet<String> = HashSet::new();
	// `float[n]` is the subscript `float#(n+1)` still: a zero-filled list unless float names a variable
	let mut variables: HashSet<String> = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, _) = part {
			variables.extend(assigned_variable(target).map(|(name, _)| name.clone()));
		}
	});
	node.visit(&mut |part| match part {
		Node::Key(list, Op::Dot, call) => {
			if let (Node::Symbol(name), Node::List(items, _, _)) = (list.drop_meta(), call.drop_meta()) {
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(method)) if crate::list_mutations::appends(method, items.len() - 1)) {
					appended.insert(name.clone());
				}
			}
		}
		Node::Key(target, Op::Assign, value) => {
			if let (Node::Symbol(name), Node::Key(left, Op::Add, right)) = (target.drop_meta(), value.drop_meta()) {
				if matches!(left.drop_meta(), Node::Symbol(same) if same == name) && matches!(right.drop_meta(), Node::List(_, Bracket::Square, _)) {
					appended.insert(name.clone());
				}
			}
		}
		_ => {}
	});
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let Some((name, declared_type)) = assigned_variable(target) {
				if is_broadcast_over(value, name, functions) {
					return;
				}
				let declared_list =
					declared_type.is_some_and(|type_node| list_element_type(&type_node.serialize()).is_some());
				let is_list = declared_list
					|| is_list_literal(value)
					|| match value.drop_meta() {
						Node::List(items, Bracket::Square, _) => !items.iter().any(is_colon_pair),
						_ if is_range(value) => true,
						_ if element_wise_parts(value).is_some() => true,
						_ if crate::filled_lists::typed_array_value(value, &variables).is_some() => true,
						Node::Empty => appended.contains(name),
						// `x = floor [1.5 2.5]`, `ys = log(xs, 3)`: a broadcast over a list literal or a list variable
						Node::List(items, bracket, separator) => {
							let is_list_argument = |argument: &Node| {
								is_list_literal(argument)
									|| matches!(argument.drop_meta(), Node::Symbol(list) if assigned.get(list) == Some(&true))
							};
							broadcasts_a_list(items, bracket, separator, functions, scalar_parameters, is_list_argument)
						}
						// `xs = xs + [i]`
						Node::Key(left, Op::Add, right) => {
							matches!(left.drop_meta(), Node::Symbol(same) if same == name)
								&& matches!(right.drop_meta(), Node::List(_, Bracket::Square, _))
						}
						_ => false,
					};
				*assigned.entry(name.clone()).or_insert(true) &= is_list;
			}
		}
	});
}

/// The variable `x = …` or `x: type = …` assigns, and its declared type
fn assigned_variable(target: &Node) -> Option<(&String, Option<&Node>)> {
	match target.drop_meta() {
		Node::Symbol(name) => Some((name, None)),
		Node::Key(name, Op::Colon, declared_type) => match name.drop_meta() {
			Node::Symbol(name) => Some((name, Some(declared_type))),
			_ => None,
		},
		_ => None,
	}
}

/// `[1 2]`, and `(1 2)` or `{3 4}` of several literal values: no call and no block (wiki/list.md)
fn is_list_literal(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, Bracket::Square, _) => !items.is_empty() && !items.iter().any(is_colon_pair),
		Node::List(items, Bracket::Round | Bracket::Curly, _) => {
			items.len() > 1
				&& items.iter().all(|item| matches!(item.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_)))
		}
		_ => false,
	}
}

/// `1 to 4`, `(1 to 4)`, `1..4`
fn is_range(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Range | Op::To, _) => true,
		Node::List(items, Bracket::Round, _) => matches!(items.as_slice(), [only] if is_range(only)),
		_ => false,
	}
}

/// `f(list)` of a broadcasting f, or `g(list, n)` of a list for g's parameter of one value
fn broadcasts_a_list(
	items: &[Node],
	bracket: &Bracket,
	separator: &Separator,
	functions: &HashSet<String>,
	scalar_parameters: &HashMap<String, Vec<bool>>,
	is_list_argument: impl Fn(&Node) -> bool,
) -> bool {
	if let Some((_, argument)) = broadcasting_call(items, bracket, separator, functions) {
		return is_list_argument(argument);
	}
	let Some(scalars) = called_name(items, bracket, separator).and_then(|name| scalar_parameters.get(name)) else {
		return false;
	};
	let arguments = &items[1..];
	let lists = arguments.iter().zip(scalars).filter(|(argument, scalar)| **scalar && is_list_argument(argument));
	arguments.len() == scalars.len() && lists.count() == 1
}

/// `moved ps`, `moved(ps)` of a broadcasting function
fn is_broadcast_over(value: &Node, variable: &str, functions: &HashSet<String>) -> bool {
	let Node::List(items, bracket, separator) = value.drop_meta() else {
		return false;
	};
	broadcasting_call(items, bracket, separator, functions)
		.is_some_and(|(_, argument)| matches!(argument.drop_meta(), Node::Symbol(same) if same == variable))
}

struct Broadcast {
	functions: HashSet<String>,
	list_variables: HashSet<String>,
	/// The variables assigned anything but a list: `xs .* k` maps over xs at once
	other_variables: Vec<String>,
	/// Per function of several parameters: which parameters take one value (`add(a, b) := a + b`: both)
	scalar_parameters: HashMap<String, Vec<bool>>,
	defines_dot: bool,
	defines_sum: bool,
	/// The shapes of the program's variables: which field reads map over a list of instances
	instances: Option<InstanceTypes>,
	/// element-wise operators between two lists so far: each holds its lists under names of its own
	paired: Counter,
	/// The untyped parameters of the functions being rewritten: their own values, not the program's list variables of
	/// the same names (card param-named-like-global)
	shadowed: std::cell::RefCell<Vec<String>>,
}

impl Broadcast {
	/// `run` with a function's `parameters` among the shadowed names: its own values inside its body
	fn with_parameters<T>(&self, parameters: Vec<String>, run: impl FnOnce() -> T) -> T {
		let outer = self.shadowed.borrow().len();
		self.shadowed.borrow_mut().extend(parameters);
		let result = run();
		self.shadowed.borrow_mut().truncate(outer);
		result
	}

	fn rewrite(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				if let Some(reduced) = self.dot(&items, &bracket).or_else(|| self.fused_sum(&items, &bracket)) {
					return reduced;
				}
				// `map square [1 2 3]`: the iteration word applies square itself (lambdas.rs), no broadcast inside it; a
				// parenthesized argument is a value of its own, `(square xs) |> filter(…)` is `filter (square xs) …`
				let iterates = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::function_values::ITERATION_WORDS.contains(&word.as_str()));
				let items: Vec<Node> = items
					.into_iter()
					.map(|item| match item.drop_meta() {
						Node::List(inner, inner_bracket, inner_separator)
							if iterates && *inner_bracket != Bracket::Round =>
						{
							Node::List(
								inner.iter().cloned().map(|part| self.rewrite(part)).collect(),
								inner_bracket.clone(),
								inner_separator.clone(),
							)
						}
						_ => self.rewrite(item),
					})
					.collect();
				self.broadcast_call(&items, &bracket, &separator)
					.or_else(|| self.broadcast_one_argument(&items, &bracket, &separator))
					.unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Key(left, op, right) if left.is_empty_node() && SCALAR_OPERATORS.contains(&op) => {
				let right = self.rewrite(*right);
				self.broadcast_operator(op, &right).unwrap_or(Node::Key(left, op, Box::new(right)))
			}
			// a definition head names parameters, it calls nothing
			Node::Key(head, op @ (Op::Define | Op::Assign | Op::Arrow), body) if is_function_head(&head, op) => {
				let body = self.with_parameters(untyped_parameters(&head, op), || self.rewrite(*body));
				Node::Key(head, op, Box::new(body))
			}
			Node::Key(left, op, right) => {
				let (left, right) = (self.rewrite(*left), self.rewrite(*right));
				let key = key(left, op, right);
				if let Some(paired) = self.paired_lists(&key) {
					return paired;
				}
				if let Some(dispatched) = self.element_wise_any(&key) {
					return dispatched;
				}
				let Node::Key(left, op, right) = key else { unreachable!() };
				if op == Op::Mul && self.is_list(&left) && self.is_list(&right) {
					return self.inner_product(*left, *right);
				}
				let method_call = (op == Op::Dot)
					.then(|| self.broadcast_method(&left, &right).or_else(|| self.element_fields(&left, &right)))
					.flatten();
				method_call.unwrap_or(Node::Key(left, op, right))
			}
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}

	/// `f [1 2]`, `f([1 2])`, `f xs` of a broadcasting function `f`
	fn broadcast_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let (name, argument) = broadcasting_call(items, bracket, separator, &self.functions)?;
		match argument.drop_meta() {
			Node::List(elements, Bracket::Square, element_separator) if !elements.is_empty() => {
				let applied = elements.iter().map(|element| self.apply(name, element.clone())).collect();
				Some(Node::List(applied, Bracket::Square, element_separator.clone()))
			}
			_ if self.is_list_value(argument) => Some(each_item(argument.clone(), |item| call(name, vec![item]))),
			_ => None,
		}
	}

	/// `names.upper`, `names.upper()`: the method form of a broadcasting function maps over the list as `upper names`
	/// does (card method-broadcast, default pending the user's decision)
	fn broadcast_method(&self, receiver: &Node, method: &Node) -> Option<Node> {
		let name = match method.drop_meta() {
			Node::Symbol(name) => name,
			Node::List(items, Bracket::Round, _) => match items.as_slice() {
				[only] => match only.drop_meta() {
					Node::Symbol(name) => name,
					_ => return None,
				},
				_ => return None,
			},
			_ => return None,
		};
		if !self.is_list(receiver) {
			return None;
		}
		self.broadcast_call(&[Node::Symbol(name.clone()), receiver.clone()], &Bracket::Round, &Separator::None)
	}

	/// `people.name` of a list of instances whose type declares the field name: `map(people, item => item.name)`; a list's
	/// own words (`people.count`) and a parameter of the same name as the list stay as they are
	fn element_fields(&self, list: &Node, field: &Node) -> Option<Node> {
		let name = field.symbol_name()?;
		let shadowed = list
			.symbol_name()
			.is_some_and(|variable| self.shadowed.borrow().iter().any(|parameter| parameter == variable));
		if shadowed || crate::library_words::is_library_word(name) {
			return None;
		}
		let field = symbol(&self.instances.as_ref()?.element_field(list, name)?);
		Some(each_item(list.clone(), |item| key(item, Op::Dot, field.clone())))
	}

	/// A list variable, a range or a broadcast: `sqrt xs`, `sqrt (1 to 4)`, `round(floor(xs))` map over its items
	fn is_list_value(&self, node: &Node) -> bool {
		is_range(node)
			|| is_broadcast(node)
			|| matches!(node.drop_meta(), Node::Symbol(variable) if self.list_variables.contains(variable) && !self.shadowed.borrow().contains(variable))
	}

	/// `add [1 2] 10`, `add(10, xs)`, `add(all xs, 10)`, `square(all xs)`: a call whose one list argument goes to a
	/// parameter of one value (or is marked `all`, wiki/all.md) maps over that list, the other arguments as they are
	fn broadcast_one_argument(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let is_call = matches!(
			(bracket, separator),
			(Bracket::Round, Separator::None) | (Bracket::None | Bracket::Round, Separator::Space)
		);
		let (head, arguments) = items.split_first()?;
		let Node::Symbol(name) = head.drop_meta() else {
			return None;
		};
		if !is_call || arguments.is_empty() {
			return None;
		}
		let marked: Vec<usize> =
			(0..arguments.len()).filter(|index| all_marked(&arguments[*index]).is_some()).collect();
		let (index, list) = match marked.as_slice() {
			[index] => (*index, all_marked(&arguments[*index]).expect("marked").clone()),
			[] => {
				let scalar = self
					.scalar_parameters
					.get(name)
					.filter(|scalars| scalars.len() == arguments.len() && arguments.len() > 1)?;
				let lists: Vec<usize> =
					(0..arguments.len()).filter(|index| scalar[*index] && self.is_list(&arguments[*index])).collect();
				let [index] = lists.as_slice() else { return None };
				(*index, arguments[*index].clone())
			}
			_ => return None,
		};
		Some(each_item(list, |item| {
			let mut call = items.to_vec();
			call[index + 1] = item;
			parens(call)
		}))
	}

	/// `k .* v`, `2 .- v`, `xs .- floor(xs)`: an element-wise operator of a side that may be a list or a number (a
	/// parameter, a call) decides at run time, by the prelude's element_wise_any: each item of the list side, two lists
	/// paired, or both numbers plainly; `xs .* 2`, `xs .* k` of a number variable k map over xs at once
	fn element_wise_any(&self, node: &Node) -> Option<Node> {
		let (receiver, _, operand) = element_wise_parts(node)?;
		if self.is_list_expression(&receiver) && is_scalar_operand(&operand, &self.other_variables) {
			return None;
		}
		// `notes .+ 2`: a class's dispatch of a number maps over the items as written, keeping their class
		if is_dispatched(node) && is_scalar_operand(&operand, &[]) {
			return None;
		}
		let [left, right] = ELEMENT_WISE_SIDES.map(|side| Node::Symbol(side.to_string()));
		let item = element_wise_item(node, left.clone(), right.clone())?;
		// a dispatch's right side is an item, never the list the analyzer sees passed (`p .- c()`): of any kind, so the
		// class's method is not compiled for a list
		let right = match is_dispatched(node) {
			true => key(right, Op::Colon, symbol(crate::words::ANY_TYPE)),
			false => right,
		};
		let combined = key(parens(vec![left, right]), Op::Arrow, item);
		// the prelude's name for it: library_words, which names prelude words so, runs after this pass
		let word = crate::modules::prelude_name(ELEMENT_WISE_ANY).unwrap_or(ELEMENT_WISE_ANY.to_string());
		Some(call(&word, vec![receiver, operand, combined]))
	}

	/// `xs .op ys` with both sides lists: the items paired by index (lists of different lengths are an error)
	fn paired_lists(&self, node: &Node) -> Option<Node> {
		let (receiver, op, operand) = element_wise_parts(node)?;
		if !self.is_list_expression(&receiver) || !self.is_list_expression(&operand) {
			return None;
		}
		Some(self.paired(receiver, op, operand, PAIRED_TEMPLATE))
	}

	/// Two lists, their items paired by index in `template`: a list of `left op right`, or their sum
	fn paired(&self, receiver: Node, op: Op, operand: Node, template: &str) -> Node {
		paired_by(receiver, op, operand, template, self.namer())
	}

	/// The template's names made this rewrite's own: LEFT → paired_left_3 …
	fn namer(&self) -> impl Fn(&str) -> String {
		named_by(self.paired.next_number().to_string())
	}

	/// `sum(xs .op ys)`, `sum(xs .op k)`: one loop adding the items, the element-wise list never built
	fn fused_sum(&self, items: &[Node], bracket: &Bracket) -> Option<Node> {
		let [head, argument] = items else { return None };
		if self.defines_sum || *bracket != Bracket::Round || head.name() != SUM_WORD {
			return None;
		}
		let (receiver, op, operand) = element_wise_parts(ungrouped(argument))?;
		if !self.is_list_expression(&receiver) {
			return None;
		}
		let (receiver, operand) = (self.rewrite(receiver), self.rewrite(operand));
		if self.is_list_expression(&operand) {
			return Some(self.paired(receiver, op, operand, PAIRED_SUM_TEMPLATE));
		}
		let named = self.namer();
		let item = key(Node::Symbol(named("ITEM")), op, operand);
		Some(super::nodes::filled(&named(FUSED_SUM_TEMPLATE), [("fused_list", receiver), ("fused_item", item)]))
	}

	/// A list, or an element-wise expression giving one: `xs`, `[1 2]`, `(xs .* 2)`
	fn is_list_expression(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.is_list_expression(&items[0]),
			_ => {
				self.is_list(node)
					|| element_wise_parts(node).is_some_and(|(receiver, _, _)| self.is_list_expression(&receiver))
			}
		}
	}

	/// `dot(xs, ys)`: `sum(xs .* ys)`
	fn dot(&self, items: &[Node], bracket: &Bracket) -> Option<Node> {
		let [head, left, right] = items else { return None };
		if self.defines_dot || *bracket != Bracket::Round || !head.is_symbol(DOT_WORD) {
			return None;
		}
		Some(self.inner_product(left.clone(), right.clone()))
	}

	/// `xs * ys` of two lists, as `dot(xs, ys)`: `sum(xs .* ys)` (card g_n8GI)
	fn inner_product(&self, left: Node, right: Node) -> Node {
		let sum = [symbol(SUM_WORD), element_wise(left, Op::Mul, right)];
		self.fused_sum(&sum, &Bracket::Round)
			.unwrap_or_else(|| parens(vec![sum[0].clone(), self.rewrite(sum[1].clone())]))
	}

	/// A list literal that is no object, or a variable only ever assigned one
	fn is_list(&self, node: &Node) -> bool {
		is_list_literal(node) || self.is_list_value(node)
	}

	/// `abs [-1 2]`, `sqrt([4, 9])`, `sqrt xs`: a scalar prefix operator over a list literal or a list variable
	fn broadcast_operator(&self, op: Op, argument: &Node) -> Option<Node> {
		let applied = |element: Node| key(Node::Empty, op, element);
		let argument = ungrouped(argument);
		match argument {
			Node::List(elements, Bracket::Square, separator)
				if !elements.is_empty() && !elements.iter().any(is_colon_pair) =>
			{
				Some(Node::List(elements.iter().cloned().map(applied).collect(), Bracket::Square, separator.clone()))
			}
			_ if self.is_list_value(argument) => Some(each_item(argument.clone(), applied)),
			_ => None,
		}
	}

	/// The function applied to one element: to the value of a pair, broadcast again over a nested list
	fn apply(&self, name: &str, element: Node) -> Node {
		match element.drop_meta() {
			Node::Key(key, Op::Colon, value) => {
				Node::Key(key.clone(), Op::Colon, Box::new(self.apply(name, value.as_ref().clone())))
			}
			_ => {
				let items = vec![symbol(name), element];
				self.broadcast_call(&items, &Bracket::Round, &Separator::None)
					.unwrap_or_else(|| call(name, vec![items[1].clone()]))
			}
		}
	}
}

/// The prelude words this pass may call where the program writes none: element_wise_any, of any element-wise operator
/// (the parser wrote it as a map whose lambda parameter no module scan counts) or `dot` (`sum(a .* b)`)
pub(crate) fn implied_words(statements: &[Node]) -> Vec<String> {
	fn has_element_wise(node: &Node) -> bool {
		element_wise_parts(node).is_some()
			|| node.is_symbol(DOT_WORD)
			|| node.drop_meta().parts().into_iter().any(has_element_wise)
	}
	match statements.iter().any(has_element_wise) {
		true => vec![ELEMENT_WISE_ANY.to_string()],
		false => vec![],
	}
}

/// P166 (user): an element-wise operator on a plain number is the plain operator, `6 ./ 2` is 3. The parser wrote
/// `x .^ 2` as `x.map(each_element => each_element ^ 2)`; where x is a number literal, or a parameter of its function
/// or lambda used only as an operand, it becomes `x ^ 2`, which that function's broadcasting maps over a list
/// (`sq = @(x) x.^2`: `sq(3)` is 9, `sq([1,2,3])` is [1 4 9])
pub fn lower_scalar_element_wise(program: Node) -> Node {
	scalar_element_wise(program, &[])
}

fn scalar_element_wise(node: Node, parameters: &[String]) -> Node {
	// a class's dispatch of the operator (`notes .+ 2` of Notes) stays element-wise: its kind test would keep the
	// function from broadcasting
	if let Some((receiver, op, operand)) = element_wise_parts(&node).filter(|_| !is_dispatched(&node)) {
		// a number and no list: `h .+ [0, 1]` of a parameter h keeps its dot, and so does `a .+ b` of two parameters,
		// which pair two lists instead of the function broadcasting over each (card compile-shaders)
		let numbers = [&receiver, &operand].map(|side| is_scalar_operand(side, &[]));
		let scalars = [&receiver, &operand].map(|side| is_scalar_operand(side, parameters));
		if scalars == [true, true] && numbers.contains(&true) {
			return key(receiver, op, scalar_element_wise(operand, parameters));
		}
	}
	match node {
		// a function or lambda: its own parameters, the outer ones are out of scope
		Node::Key(head, op @ (Op::Define | Op::Assign | Op::Arrow), body) if is_function_head(&head, op) => {
			let inner: Vec<String> =
				untyped_parameters(&head, op).into_iter().filter(|name| only_operand(&body, name)).collect();
			Node::Key(head, op, Box::new(scalar_element_wise(*body, &inner)))
		}
		other => other.map_children(|child| scalar_element_wise(child, parameters)),
	}
}

/// A number, a parameter used only as an operand, or arithmetic of them: no list can be there
fn is_scalar_operand(node: &Node, parameters: &[String]) -> bool {
	match ungrouped(node) {
		Node::Number(_) => true,
		Node::Symbol(name) => parameters.contains(name),
		Node::Key(left, op, right) if ARITHMETIC.contains(op) || *op == Op::Neg => {
			(left.is_empty_node() || is_scalar_operand(left, parameters)) && is_scalar_operand(right, parameters)
		}
		_ => false,
	}
}

/// `x.map(each_element => each_element op operand)`: x, op, operand
/// `(x)` → x
fn ungrouped(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => ungrouped(&items[0]),
		other => other,
	}
}

/// Two lists, their items paired by index in `template`: a list of `left op right`, or their sum; `named` makes the
/// template's names the rewrite's own
pub(crate) fn paired_by(receiver: Node, op: Op, operand: Node, template: &str, named: impl Fn(&str) -> String) -> Node {
	let item = match crate::warp_parser::parse(&named(PAIRED_ITEM)).drop_meta() {
		Node::Key(left, _, right) => Node::Key(left.clone(), op, right.clone()),
		other => unreachable!("{other:?}"),
	};
	let bindings = [("paired_left", receiver), ("paired_right", operand), ("paired_item", item)];
	super::nodes::filled(&named(&template.replace("COUNT", PAIRED_COUNT)), bindings)
}

/// The templates' names with `unique` appended: LEFT → paired_left_3 …
pub(crate) fn named_by(unique: String) -> impl Fn(&str) -> String {
	move |text: &str| {
		[
			("LEFT", "paired_left"),
			("RIGHT", "paired_right"),
			("SUM", "fused_sum"),
			("ITEMS", "fused_items"),
			("ITEM", "fused_item"),
		]
		.iter()
		.fold(text.to_string(), |text, (placeholder, name)| text.replace(placeholder, &format!("{name}_{unique}")))
	}
}

/// The element name of the lambda an element-wise operator maps with: no warp program writes it
pub const EACH_ELEMENT: &str = "each_element";

/// `xs .+ n` (also `.-`, `.*`, `./`): the operator applied to each element, `xs.map(each_element => each_element + n)`
pub fn element_wise(list: Node, op: Op, operand: Node) -> Node {
	let element = || Box::new(Node::Symbol(EACH_ELEMENT.to_string()));
	let lambda = Node::Key(element(), Op::Arrow, Box::new(Node::Key(element(), op, Box::new(operand))));
	Node::Key(Box::new(list), Op::Dot, Box::new(call("map", vec![lambda])))
}

/// `xs .* 2` as the parser keeps it, the phrase [xs .* 2]: the operator applied to each element (element_wise)
pub fn lower_element_wise_operators(node: Node) -> Node {
	let node = node.map_children(lower_element_wise_operators);
	let Node::List(items, Bracket::None, Separator::Space) = node.drop_meta() else {
		return node;
	};
	match items.as_slice() {
		[list, word, operand]
			if let Some(op) = word.symbol_name().and_then(crate::operators::element_wise_operator) =>
		{
			element_wise(list.clone(), op, operand.clone()).with_meta_of(&node)
		}
		_ => node,
	}
}

pub(crate) fn element_wise_parts(node: &Node) -> Option<(Node, Op, Node)> {
	let (receiver, item) = element_wise_map(node)?;
	let Node::Key(left, op, operand) = instance_dispatch(item).unwrap_or(item).drop_meta() else {
		return None;
	};
	is_each_element(left).then(|| (receiver.clone(), *op, operand.as_ref().clone()))
}

/// `xs.map(each_element -> item)`: xs and the item
fn element_wise_map(node: &Node) -> Option<(&Node, &Node)> {
	let Node::Key(receiver, Op::Dot, call) = node.drop_meta() else {
		return None;
	};
	let Node::List(items, Bracket::Round, _) = call.drop_meta() else {
		return None;
	};
	let [word, lambda] = items.as_slice() else { return None };
	let Node::Key(element, Op::Arrow, item) = lambda.drop_meta() else {
		return None;
	};
	(word.name() == MAP_WORD && is_each_element(element)).then_some((receiver.as_ref(), item.as_ref()))
}

fn is_each_element(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == EACH_ELEMENT)
}

/// An element-wise item of a program whose class defines the operator (`use sound`: Note.plus) as class_methods
/// dispatches it, `if each_element is Note then each_element.plus(operand) else each_element + operand`: the built-in
/// operator's item
fn instance_dispatch(item: &Node) -> Option<&Node> {
	let (condition, then, otherwise) = super::nodes::branches(item)?;
	(first_operand(condition).is_some_and(is_each_element) && first_operand(then).is_some_and(is_each_element))
		.then_some(otherwise)
}

/// `x is Note` and `x.plus(o)` as written, `is_type(x, "Note")` and `plus(x, o)` once type tests and methods are
/// lowered: x
fn first_operand(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Key(subject, Op::Eq | Op::Dot, _) => Some(subject),
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() > 1 => items.get(1),
		_ => None,
	}
}

fn is_dispatched(node: &Node) -> bool {
	element_wise_map(node).is_some_and(|(_, item)| instance_dispatch(item).is_some())
}

/// The item an element-wise operator computes of `left` and `right`: `left op right`, or the dispatch of a class
/// defining the operator (instance_dispatch), so `notes .+ 2` still transposes each note
fn element_wise_item(node: &Node, left: Node, right: Node) -> Option<Node> {
	let (_, op, operand) = element_wise_parts(node)?;
	let (_, item) = element_wise_map(node)?;
	if !is_dispatched(node) {
		return Some(key(left, op, right));
	}
	let each = HashMap::from([(EACH_ELEMENT.to_string(), left)]);
	Some(crate::law::substitute(&replaced(item.clone(), &operand, &right), &each))
}

/// `node` with every `from` in it `to`
fn replaced(node: Node, from: &Node, to: &Node) -> Node {
	match node.drop_meta() == from.drop_meta() {
		true => to.clone(),
		false => node.map_children(|child| replaced(child, from, to)),
	}
}

fn is_function_head(head: &Node, op: Op) -> bool {
	matches!(op, Op::Arrow) || matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() > 1)
}

/// The untyped parameters of `f(a, b) := …`, `(a, b) => …`, `x => …`
fn untyped_parameters(head: &Node, op: Op) -> Vec<String> {
	let flattened = |items: &[Node]| -> Vec<Node> {
		items
			.iter()
			.flat_map(|item| match item.drop_meta() {
				Node::List(group, Bracket::Round, _) => group.clone(),
				other => vec![other.clone()],
			})
			.collect()
	};
	let parameters = match (head.drop_meta(), op) {
		(Node::List(items, Bracket::Round, _), Op::Define | Op::Assign) => flattened(&items[1..]),
		(Node::List(items, _, _), _) => flattened(items),
		(single, _) => vec![single.clone()],
	};
	parameters.iter().filter_map(|parameter| parameter.symbol_name().map(String::from)).collect()
}

/// Whether `name` appears in body only as an operand of arithmetic or as an element-wise receiver
fn only_operand(body: &Node, name: &str) -> bool {
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name);
	if let Some((receiver, _, operand)) = element_wise_parts(body) {
		return (is_name(&receiver) || only_operand(&receiver, name)) && only_operand(&operand, name);
	}
	match body.drop_meta() {
		Node::Symbol(symbol) => symbol != name,
		Node::Key(left, op, right) if ARITHMETIC.contains(op) => {
			[left, right].into_iter().all(|side| is_name(side) || only_operand(side, name))
		}
		Node::Key(left, _, right) => only_operand(left, name) && only_operand(right, name),
		Node::List(items, _, _) => items.iter().all(|item| only_operand(item, name)),
		_ => true,
	}
}
