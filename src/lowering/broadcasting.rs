//! Functions broadcast over lists and pairs (wiki/broadcasting.md, notes/broadcasting.md): `square [1 2 3]` is
//! `[square(1) square(2) square(3)]`, `square [a:1 b:2]` is `[a:square(1) b:square(2)]`, and a variable only ever assigned
//! a list literal is mapped (`square xs` → `map xs square`). A function of one parameter broadcasts when the parameter is
//! declared with a scalar type (`x:int`, `square number`, P50) or, undeclared, is an arithmetic operand of the body; a
//! function of a list (`count(xs)`, `xs#2`, `xs:list`) takes the list whole. Operators never broadcast (`[1 2 3]*2`).

use crate::analyzer::{annotated_kind, list_element_type};
use crate::function_values::{definitions, Definition};
use crate::type_kinds::Kind;
use crate::lambdas::IMPLICIT_PARAMETER;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const ARITHMETIC: [Op; 6] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Pow];
const MAP_WORD: &str = "map";
const ALL_WORD: &str = "all";
/// Methods that append to a list variable (analyzer APPEND_METHODS)
pub(crate) const APPEND_METHODS: [&str; 3] = ["add", "append", "push"];
const EXTREMUM_WORDS: [&str; 2] = ["max", "min"];
/// Parameter types a comparison's result fits
const TRUTH_TYPES: [&str; 3] = ["bool", "boolean", "any"];
const BROADCAST_ITEM: &str = "broadcast_item";
/// Library words of one text or number that broadcast over a list like a user function of a scalar
const SCALAR_LIBRARY_WORDS: [&str; 6] = ["upper", "lower", "trim", "floor", "ceil", "round"];
/// Prefix operators of one number: `abs [-1 2]`, `sqrt xs`
const SCALAR_OPERATORS: [Op; 3] = [Op::Abs, Op::Sqrt, Op::Cbrt];
/// The item `square all xs` applies square to
const ALL_ITEM: &str = "all_item";
/// Declared parameter kinds that take one element of a list (P50)
const SCALAR_KINDS: [Kind; 4] = [Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint];

/// P84 (user: "This should have already been done with broadcasting"): several juxtaposed arguments of a function of one
/// parameter are one list, `sum 1 2 3` is `sum [1 2 3]`, and a scalar function broadcasts over it (`square 1 2 3`).
/// After the lambda passes, which make `sum := fold +` a function of one parameter; `f(1, 2, 3)` stays an arity error.
pub fn lower_several_arguments(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
	implicit_definitions(&program, &mut found);
	let single: HashSet<String> = found.iter().filter(|definition| definition.params.len() == 1).map(|definition| definition.name.clone()).collect();
	if single.is_empty() {
		return program;
	}
	let mut gathered = false;
	let program = gather_arguments(program, &single, &mut gathered);
	if gathered { lower(program) } else { program }
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
	let mut arities: HashMap<String, Arity> = HashMap::new();
	for definition in &found {
		let count = definition.params.len();
		// P149 (user): an untyped parameter the body uses in arithmetic (`x + 1`, `it*it`) is no truth value either
		let typed = definition.params.first().is_some_and(|first| rules_out_a_truth_value(first) || is_arithmetic_operand(&definition.body, &first.name()));
		let arity = arities.entry(definition.name.clone()).or_insert(Arity { fewest: count, most: count, typed_first: typed });
		*arity = Arity { fewest: arity.fewest.min(count), most: arity.most.max(count), typed_first: arity.typed_first && typed };
	}
	if arities.is_empty() {
		return program;
	}
	// `puts add 1 2`: the output words take one value
	for word in crate::wasm_emitter::OUTPUT_WORDS {
		arities.entry(word.to_string()).or_insert(Arity { fewest: 1, most: 1, typed_first: false });
	}
	nest_prefix_calls(program, &arities)
}

/// `square all xs` (wiki/all.md): `all` splices the list into its items, the function applies to each
fn all_calls(node: Node) -> Node {
	match node {
		// also `square(all xs)`, whose call items are `square all xs`
		Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) if all_call_parts(&items).is_some() => {
			let (function, list) = all_call_parts(&items).expect("guarded");
			// a list literal spliced at compile time: `square all [[1, 2], [3]]` is `[square [1, 2], square [3]]`,
			// each call broadcast again
			if let Node::List(elements, Bracket::Square, element_separator) = list.drop_meta() {
				let calls = elements.iter().map(|element| Node::List(vec![function.clone(), all_calls(element.clone())], Bracket::None, Separator::Space)).collect();
				return Node::List(calls, Bracket::Square, element_separator.clone());
			}
			let item = Node::Symbol(ALL_ITEM.to_string());
			let call = Node::List(vec![function.clone(), item.clone()], Bracket::Round, Separator::None);
			let each = Node::Key(Box::new(item), Op::FatArrow, Box::new(call));
			Node::List(vec![Node::Symbol(MAP_WORD.to_string()), all_calls(list.clone()), each], Bracket::Round, Separator::None)
		}
		other => other.map_children(all_calls),
	}
}

/// `all xs` as an argument: xs
fn all_marked(argument: &Node) -> Option<&Node> {
	match argument.drop_meta() {
		Node::List(words, Bracket::None, Separator::Space) if words.len() == 2 && matches!(words[0].drop_meta(), Node::Symbol(word) if word == ALL_WORD) => Some(&words[1]),
		_ => None,
	}
}

/// The function and the list of `f all xs`, also as the parser nests a known function's argument, `f (all xs)`
pub(crate) fn all_call_parts(items: &[Node]) -> Option<(&Node, &Node)> {
	let is_all = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if word == ALL_WORD);
	match items {
		[function, all, list] if is_all(all) => Some((function, list)),
		// `square (all xs)` as the parser nests a known function's argument, `(square all) xs` as a value
		[first, second] => match (first.drop_meta(), second.drop_meta()) {
			(_, Node::List(words, Bracket::None, Separator::Space)) if words.len() == 2 && is_all(&words[0]) => Some((first, &words[1])),
			(Node::List(words, Bracket::None, Separator::Space), _) if words.len() == 2 && is_all(&words[1]) => Some((&words[0], second)),
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

/// `sum`, `max`, `upper` …: a library word of one value, not redefined by the program
fn is_one_value_word(node: &Node, arities: &HashMap<String, Arity>) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if !arities.contains_key(name) && (crate::library_words::is_library_word(name) || EXTREMUM_WORDS.contains(&name.as_str())))
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
	crate::diagnostic::Diagnostic::at(at, format!("{written} is ambiguous; write {call_compared} or {comparison_argument}"))
		.offer("the call compared", &written, &call_compared)
		.offer("the comparison as the argument", &written, &comparison_argument)
		.into_error()
}

fn nest_prefix_calls(node: Node, arities: &HashMap<String, Arity>) -> Node {
	let arity = |node: &Node| arity_of(node, arities);
	match node {
		Node::List(items, bracket @ (Bracket::None | Bracket::Round), separator @ (Separator::Space | Separator::None))
			if items.len() > 2 && arity(&items[0]).is_some_and(|takes| items.len() - 1 > takes.most) =>
		{
			let mut items: Vec<Node> = items.into_iter().map(|item| nest_prefix_calls(item, arities)).collect();
			let Some(inner) = (1..items.len() - 1).rev().find(|index| arity(&items[*index]).is_some()) else {
				return Node::List(items, bracket, separator);
			};
			let nested = Node::List(items.split_off(inner), Bracket::None, Separator::Space);
			items.push(nest_prefix_calls(nested, arities));
			nest_prefix_calls(Node::List(items, bracket, separator), arities)
		}
		// `sum square [1 2 3]`, `sum(square xs)`: a library word of one value takes the call of a user function after it
		Node::List(items, bracket @ (Bracket::None | Bracket::Round), separator @ (Separator::Space | Separator::None))
			if items.len() > 2 && is_one_value_word(&items[0], arities) && arity(&items[1]).is_some_and(|takes| items.len() - 2 <= takes.most) =>
		{
			let mut items = items;
			let call = Node::List(items.split_off(1), Bracket::None, Separator::Space);
			items.push(nest_prefix_calls(call, arities));
			Node::List(items, bracket, separator)
		}
		// `square 3 == 9` parses as `square (3 == 9)`. P143 (user): a parameter typed as no truth value takes the call,
		// `(square 3) == 9` (wiki/all.md `square [1 2 3] == [1 4 9]`); untyped, both readings work: a loud error
		Node::List(items, Bracket::None, Separator::Space) if matches!(items.as_slice(), [head, argument] if arity(head).is_some() && matches!(argument.drop_meta(), Node::Key(_, op, _) if op.is_comparison())) => {
			let [head, argument]: [Node; 2] = items.try_into().expect("guarded");
			let Node::Key(left, op, right) = argument.drop_meta().clone() else { unreachable!("guarded") };
			if !arity(&head).expect("guarded").typed_first {
				return ambiguous_call(&argument, &head.drop_meta().name(), &left.serialize(), &op, &right.serialize());
			}
			let call = Node::List(vec![head, *left], Bracket::None, Separator::Space);
			Node::Key(Box::new(nest_prefix_calls(call, arities)), op, Box::new(nest_prefix_calls(*right, arities)))
		}
		// the same phrase the parser grouped the other way, `(f 3) == 9`, when it did not know f takes a value: as
		// ambiguous when the body accepts anything (`f := print it`)
		Node::Key(left, op, right) if op.is_comparison() && matches!(left.drop_meta(), Node::List(items, Bracket::None, Separator::Space) if matches!(items.as_slice(), [head, _] if arity(head).is_some_and(|takes| !takes.typed_first))) => {
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
			let fits = arity_of(&items[0], arities).is_some_and(|takes| given < takes.fewest && given + rest.len() <= takes.most);
			if fits {
				return Ok(Node::List([items, rest.to_vec()].concat(), Bracket::None, Separator::Space));
			}
			// `puts add 1, 2`: the short call is the last argument
			let last = items.pop().expect("a call has its function");
			match with_comma_arguments(last, rest, arities) {
				Ok(call) if !items.is_empty() => Ok(Node::List([items, vec![call]].concat(), Bracket::None, Separator::Space)),
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
		Node::List(items, Bracket::None, Separator::Space) if items.len() > 2 && matches!(items[0].drop_meta(), Node::Symbol(name) if single.contains(name)) => {
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
	let mut broadcasting: HashSet<String> = found.iter().filter(|definition| needs_a_scalar(definition)).map(|definition| definition.name.clone()).collect();
	// the library words of one scalar: `upper ["ab" "cd"]`, unless the program defines the name itself
	let defined: HashSet<&String> = found.iter().map(|definition| &definition.name).collect();
	broadcasting.extend(SCALAR_LIBRARY_WORDS.iter().map(|word| word.to_string()).filter(|word| !defined.contains(word)));
	let mut assigned = HashMap::new();
	collect_list_variables(&program, &broadcasting, &mut assigned);
	let list_variables = assigned.into_iter().filter(|(_, only_lists)| *only_lists).map(|(name, _)| name).collect();
	let scalar_parameters = found.iter().map(|definition| (definition.name.clone(), definition.params.iter().map(|param| takes_a_scalar(param, &definition.body)).collect())).collect();
	Broadcast { functions: broadcasting, list_variables, scalar_parameters }.rewrite(program)
}

/// `map(list, broadcast_item => applied(broadcast_item))`
fn each_item(list: Node, applied: impl Fn(Node) -> Node) -> Node {
	let item = Node::Symbol(BROADCAST_ITEM.to_string());
	let each = Node::Key(Box::new(item.clone()), Op::FatArrow, Box::new(applied(item)));
	Node::List(vec![Node::Symbol(MAP_WORD.to_string()), list, each], Bracket::Round, Separator::None)
}

/// `square := it*it`: a function of the implicit parameter `it`
fn implicit_definitions(node: &Node, found: &mut Vec<Definition>) {
	node.visit(&mut |part| {
		let Node::Key(target, op @ (Op::Define | Op::Assign), body) = part else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		// `f = x => x + 1`: lambdas.rs makes it the definition f(x) later
		if let Some(lambda) = crate::lambdas::arrow_lambda(body) {
			let params = lambda.params.iter().map(|param| Node::Symbol(param.clone())).collect();
			found.push(Definition { name: name.clone(), params, body: lambda.body });
		} else if *op == Op::Define {
			let params = vec![Node::Symbol(IMPLICIT_PARAMETER.to_string())];
			found.push(Definition { name: name.clone(), params, body: body.as_ref().clone() });
		}
	});
}

fn needs_a_scalar(definition: &Definition) -> bool {
	let [param] = definition.params.as_slice() else { return false };
	takes_a_scalar(param, &definition.body)
}

/// A parameter declared with a scalar type, or undeclared and an arithmetic operand of the body
fn takes_a_scalar(param: &Node, body: &Node) -> bool {
	match param.drop_meta() {
		Node::Key(_, Op::Colon, type_node) => annotated_kind(type_node).is_some_and(|kind| SCALAR_KINDS.contains(&kind)),
		Node::Symbol(name) => is_arithmetic_operand(body, name),
		_ => false,
	}
}

fn is_arithmetic_operand(body: &Node, name: &str) -> bool {
	let is_param = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name);
	let is_list_literal = |node: &Node| matches!(node.drop_meta(), Node::List(_, Bracket::Square, _));
	let mut found = false;
	body.visit(&mut |node| {
		if let Node::Key(left, op, right) = node {
			// `ys + [x]` concatenates: ys is a list
			let concatenates = *op == Op::Add && (is_list_literal(left) || is_list_literal(right));
			found |= ARITHMETIC.contains(op) && !concatenates && (is_param(left) || is_param(right));
		}
	});
	found
}

/// The function and argument of `f x` or `f(x)` when f broadcasts
fn broadcasting_call<'a>(items: &'a [Node], bracket: &Bracket, separator: &Separator, functions: &HashSet<String>) -> Option<(&'a str, &'a Node)> {
	let is_call = matches!((bracket, separator), (Bracket::Round, Separator::None) | (Bracket::None | Bracket::Round, Separator::Space)); // also `(f [1 2])`
	let [head, argument] = items else { return None };
	let Node::Symbol(name) = head.drop_meta() else { return None };
	(is_call && functions.contains(name)).then_some((name.as_str(), argument))
}

/// Variable → whether every value assigned to it is a list literal that is no object; a broadcast over the variable itself
/// (`ps = moved ps`) keeps what it is, so it needs a list assigned elsewhere (`up(x) := { x = upper x }` maps no text)
fn collect_list_variables(node: &Node, functions: &HashSet<String>, assigned: &mut HashMap<String, bool>) {
	// `xs = []` (which parses as ø) is a list once the program appends to xs: `xs.add(i)`, `xs = xs + [i]`
	let mut appended: HashSet<String> = HashSet::new();
	node.visit(&mut |part| match part {
		Node::Key(list, Op::Dot, call) => {
			if let (Node::Symbol(name), Node::List(items, _, _)) = (list.drop_meta(), call.drop_meta()) {
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(method)) if APPEND_METHODS.contains(&method.as_str())) {
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
				let declared_list = declared_type.is_some_and(|type_node| list_element_type(&type_node.serialize()).is_some());
				let is_list = declared_list || match value.drop_meta() {
					Node::List(items, Bracket::Square, _) => !items.iter().any(is_pair),
					_ if is_range(value) => true,
					Node::Empty => appended.contains(name),
					// `xs = xs + [i]`
					Node::Key(left, Op::Add, right) => matches!(left.drop_meta(), Node::Symbol(same) if same == name) && matches!(right.drop_meta(), Node::List(_, Bracket::Square, _)),
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

/// `1 to 4`, `(1 to 4)`, `1..4`
fn is_range(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Range | Op::To, _) => true,
		Node::List(items, Bracket::Round, _) => matches!(items.as_slice(), [only] if is_range(only)),
		_ => false,
	}
}

/// `moved ps`, `moved(ps)` of a broadcasting function
fn is_broadcast_over(value: &Node, variable: &str, functions: &HashSet<String>) -> bool {
	let Node::List(items, bracket, separator) = value.drop_meta() else { return false };
	broadcasting_call(items, bracket, separator, functions).is_some_and(|(_, argument)| matches!(argument.drop_meta(), Node::Symbol(same) if same == variable))
}

fn is_pair(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, _))
}

fn call(name: &str, argument: Node) -> Node {
	Node::List(vec![Node::Symbol(name.to_string()), argument], Bracket::Round, Separator::None)
}

struct Broadcast {
	functions: HashSet<String>,
	list_variables: HashSet<String>,
	/// Per function of several parameters: which parameters take one value (`add(a, b) := a + b`: both)
	scalar_parameters: HashMap<String, Vec<bool>>,
}

impl Broadcast {
	fn rewrite(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				// `map square [1 2 3]`: the iteration word applies square itself (lambdas.rs), no broadcast inside it
				let iterates = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::function_values::ITERATION_WORDS.contains(&word.as_str()));
				let items: Vec<Node> = items.into_iter().map(|item| match item.drop_meta() {
					Node::List(inner, inner_bracket, inner_separator) if iterates => {
						Node::List(inner.iter().cloned().map(|part| self.rewrite(part)).collect(), inner_bracket.clone(), inner_separator.clone())
					}
					_ => self.rewrite(item),
				}).collect();
				self.broadcast_call(&items, &bracket, &separator)
					.or_else(|| self.broadcast_one_argument(&items, &bracket, &separator))
					.unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Key(left, op, right) if matches!(left.drop_meta(), Node::Empty) && SCALAR_OPERATORS.contains(&op) => {
				let right = self.rewrite(*right);
				self.broadcast_operator(op, &right).unwrap_or(Node::Key(left, op, Box::new(right)))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
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
			_ if self.is_list_value(argument) => Some(each_item(argument.clone(), |item| call(name, item))),
			_ => None,
		}
	}

	/// A list variable or a range: `sqrt xs`, `sqrt (1 to 4)` map over its items
	fn is_list_value(&self, node: &Node) -> bool {
		is_range(node) || matches!(node.drop_meta(), Node::Symbol(variable) if self.list_variables.contains(variable))
	}

	/// `add [1 2] 10`, `add(10, xs)`, `add(all xs, 10)`, `square(all xs)`: a call whose one list argument goes to a
	/// parameter of one value (or is marked `all`, wiki/all.md) maps over that list, the other arguments as they are
	fn broadcast_one_argument(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let is_call = matches!((bracket, separator), (Bracket::Round, Separator::None) | (Bracket::None | Bracket::Round, Separator::Space));
		let (head, arguments) = items.split_first()?;
		let Node::Symbol(name) = head.drop_meta() else { return None };
		if !is_call || arguments.is_empty() {
			return None;
		}
		let marked: Vec<usize> = (0..arguments.len()).filter(|index| all_marked(&arguments[*index]).is_some()).collect();
		let (index, list) = match marked.as_slice() {
			[index] => (*index, all_marked(&arguments[*index]).expect("marked").clone()),
			[] => {
				let scalar = self.scalar_parameters.get(name).filter(|scalars| scalars.len() == arguments.len() && arguments.len() > 1)?;
				let lists: Vec<usize> = (0..arguments.len()).filter(|index| scalar[*index] && self.is_list(&arguments[*index])).collect();
				let [index] = lists.as_slice() else { return None };
				(*index, arguments[*index].clone())
			}
			_ => return None,
		};
		Some(each_item(list, |item| {
			let mut call = items.to_vec();
			call[index + 1] = item;
			Node::List(call, Bracket::Round, Separator::None)
		}))
	}

	/// A list literal that is no object, or a variable only ever assigned one
	fn is_list(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::List(elements, Bracket::Square, _) => !elements.is_empty() && !elements.iter().any(is_pair),
			_ => self.is_list_value(node),
		}
	}

	/// `abs [-1 2]`, `sqrt xs`: a scalar prefix operator over a list literal or a list variable
	fn broadcast_operator(&self, op: Op, argument: &Node) -> Option<Node> {
		let applied = |element: Node| Node::Key(Box::new(Node::Empty), op, Box::new(element));
		match argument.drop_meta() {
			Node::List(elements, Bracket::Square, separator) if !elements.is_empty() && !elements.iter().any(is_pair) => {
				Some(Node::List(elements.iter().cloned().map(applied).collect(), Bracket::Square, separator.clone()))
			}
			_ if self.is_list_value(argument) => Some(each_item(argument.clone(), applied)),
			_ => None,
		}
	}

	/// The function applied to one element: to the value of a pair, broadcast again over a nested list
	fn apply(&self, name: &str, element: Node) -> Node {
		match element.drop_meta() {
			Node::Key(key, Op::Colon, value) => Node::Key(key.clone(), Op::Colon, Box::new(self.apply(name, value.as_ref().clone()))),
			_ => {
				let items = vec![Node::Symbol(name.to_string()), element];
				self.broadcast_call(&items, &Bracket::Round, &Separator::None).unwrap_or_else(|| call(name, items[1].clone()))
			}
		}
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
	if let Some((receiver, op, operand)) = element_wise_parts(&node) {
		let is_scalar = matches!(receiver.drop_meta(), Node::Number(_)) || matches!(receiver.drop_meta(), Node::Symbol(symbol) if parameters.contains(symbol));
		if is_scalar {
			return Node::Key(Box::new(receiver), op, Box::new(scalar_element_wise(operand, parameters)));
		}
	}
	match node {
		// a function or lambda: its own parameters, the outer ones are out of scope
		Node::Key(head, op @ (Op::Define | Op::Assign | Op::FatArrow | Op::Arrow), body) if is_function_head(&head, op) => {
			let inner: Vec<String> = untyped_parameters(&head, op).into_iter().filter(|name| only_operand(&body, name)).collect();
			Node::Key(head, op, Box::new(scalar_element_wise(*body, &inner)))
		}
		other => other.map_children(|child| scalar_element_wise(child, parameters)),
	}
}

/// `x.map(each_element => each_element op operand)`: x, op, operand
fn element_wise_parts(node: &Node) -> Option<(Node, Op, Node)> {
	let Node::Key(receiver, Op::Dot, call) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, _) = call.drop_meta() else { return None };
	let [word, lambda] = items.as_slice() else { return None };
	let Node::Key(element, Op::FatArrow, body) = lambda.drop_meta() else { return None };
	let Node::Key(left, op, operand) = body.drop_meta() else { return None };
	let is_element = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if name == crate::analyzer::EACH_ELEMENT);
	(word.name() == MAP_WORD && is_element(element) && is_element(left)).then(|| (receiver.as_ref().clone(), *op, operand.as_ref().clone()))
}

fn is_function_head(head: &Node, op: Op) -> bool {
	matches!(op, Op::FatArrow | Op::Arrow) || matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() > 1)
}

/// The untyped parameters of `f(a, b) := …`, `(a, b) => …`, `x => …`
fn untyped_parameters(head: &Node, op: Op) -> Vec<String> {
	let flattened = |items: &[Node]| -> Vec<Node> {
		items.iter().flat_map(|item| match item.drop_meta() {
			Node::List(group, Bracket::Round, _) => group.clone(),
			other => vec![other.clone()],
		}).collect()
	};
	let parameters = match (head.drop_meta(), op) {
		(Node::List(items, Bracket::Round, _), Op::Define | Op::Assign) => flattened(&items[1..]),
		(Node::List(items, _, _), _) => flattened(items),
		(single, _) => vec![single.clone()],
	};
	parameters.iter().filter_map(|parameter| match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		_ => None,
	}).collect()
}

/// Whether `name` appears in body only as an operand of arithmetic or as an element-wise receiver
fn only_operand(body: &Node, name: &str) -> bool {
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name);
	if let Some((receiver, _, operand)) = element_wise_parts(body) {
		return (is_name(&receiver) || only_operand(&receiver, name)) && only_operand(&operand, name);
	}
	match body.drop_meta() {
		Node::Symbol(symbol) => symbol != name,
		Node::Key(left, op, right) if ARITHMETIC.contains(op) => [left, right].into_iter().all(|side| is_name(side) || only_operand(side, name)),
		Node::Key(left, _, right) => only_operand(left, name) && only_operand(right, name),
		Node::List(items, _, _) => items.iter().all(|item| only_operand(item, name)),
		_ => true,
	}
}
