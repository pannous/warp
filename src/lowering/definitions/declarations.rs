//! `enum color {red green blue}` declares the object `color={red:0 green:1 blue:2}`: a case is its index, `color.green` is 1.
//! `flags virtues={fast, safe}` (WIT flags, a record of bools) declares `virtues={fast:false safe:false}`, and
//! `virtues goal = fast+safe` the record `goal={fast:true safe:true}`: the flags named are true, the others false.
//! `real f(real x, int n) { … }`, the C way, defines `f(x:real, n:int) := { … }`.

use super::nodes::{
	block, call, children_rewritten, grouped_parameters, if_then, if_then_else, is_kinded_type_word, key, Counter,
};
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};
use crate::words::{IMPLICIT_PARAMETER, ON_WORD, RETURN_WORD, SELF_WORD};

mod tasks;
pub use tasks::*;
mod function_heads;
pub(crate) use function_heads::*;

/// The receiver of an extension method, as Kotlin (`this`) and Swift (`self`) name it
const THIS_WORD: &str = "this";
/// `map data = {…}`: the collection words a typed declaration takes, as the annotation they mean (type() answers map,
/// list, tuple); dict and array are other languages' words for map and list (card declaration-type)
const COLLECTION_DECLARATION_WORDS: [(&str, &str); 5] =
	[("map", "map"), ("list", "list"), ("tuple", "tuple"), ("dict", "map"), ("array", "list")];

/// `tuple`, the annotation `tuple t = (…)` lowers to, and the other collection words a declaration takes
pub(crate) fn is_collection_type_word(word: &str) -> bool {
	collection_type_meant(word).is_some()
}

/// The collection type a declaration word means: `dict` means map, `array` list
pub(crate) fn collection_type_meant(word: &str) -> Option<&'static str> {
	COLLECTION_DECLARATION_WORDS
		.iter()
		.find(|(written, meant)| *written == word || *meant == word)
		.map(|(_, meant)| *meant)
}

/// Swift's block of methods added to a type: `extension Int {…}`
const EXTENSION_WORD: &str = "extension";

/// The implicit parameter of a function: `double := it * 2`
/// `each xs: body`, `all xs: body`: a for loop over xs, the item is `it` (wiki/iteration.md)
const COLON_ITERATION_WORDS: [&str; 2] = ["each", "all"];
/// Ruby's `def f(x) … end`
const END_WORD: &str = "end";
const FLAGS_WORD: &str = "flags";
/// The argument left out of a partial application: `add(1, _)`
const PLACEHOLDER: &str = "_";

pub fn lower(node: Node) -> Node {
	lower_flags(node)
}

/// The flags types the program declares (`flags virtues={fast, safe}`), their declarations as the empty set, and the
/// typed declarations `virtues goal = fast+safe` as records of bools
fn lower_flags(program: Node) -> Node {
	let mut types: Vec<(String, Vec<String>)> = vec![];
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			types.extend(flags_declaration(items));
		}
	});
	if types.is_empty() {
		return program;
	}
	rewrite_flags(program, &types)
}

/// `flags virtues={fast, safe}`, `flags virtues {fast safe}`: the type and its flags
fn flags_declaration(items: &[Node]) -> Option<(String, Vec<String>)> {
	let (name, members) = match items {
		[word, declaration] if is_flags_word(word) => match declaration.drop_meta() {
			Node::Key(name, Op::Assign, members) => (name.as_ref(), members.as_ref()),
			_ => return None,
		},
		[word, name, members] if is_flags_word(word) => (name, members),
		_ => return None,
	};
	let (Node::Symbol(name), Node::List(members, Bracket::Curly, _)) = (name.drop_meta(), members.drop_meta()) else {
		return None;
	};
	let members: Option<Vec<String>> = members.iter().map(|member| member.symbol_name().map(String::from)).collect();
	Some((name.clone(), members?))
}

fn is_flags_word(word: &Node) -> bool {
	word.is_symbol(FLAGS_WORD)
}

fn rewrite_flags(node: Node, types: &[(String, Vec<String>)]) -> Node {
	let record = |name: &Node, members: &[String], set: &[String]| {
		let entries = members
			.iter()
			.map(|member| {
				let value = if set.contains(member) { Node::True } else { Node::False };
				key(Node::Symbol(member.clone()), Op::Colon, value)
			})
			.collect();
		key(name.clone(), Op::Assign, Node::List(entries, Bracket::Curly, Separator::Space))
	};
	match node {
		Node::List(items, bracket, separator) => {
			if let Some((name, members)) = flags_declaration(&items) {
				return record(&Node::Symbol(name), &members, &[]);
			}
			// `virtues goal = fast+safe`
			if let [type_name, declaration] = items.as_slice() {
				if let (Node::Symbol(type_name), Node::Key(variable, Op::Assign, value)) =
					(type_name.drop_meta(), declaration.drop_meta())
				{
					if let Some((_, members)) = types.iter().find(|(name, _)| name == type_name) {
						let set = named_flags(value);
						if let Some(unknown) = set.iter().find(|flag| !members.contains(flag)) {
							return crate::node::error(&format!(
								"{type_name} has no flag {unknown}; its flags: {}",
								members.join(", ")
							));
						}
						return record(variable, members, &set);
					}
				}
			}
			Node::List(items.into_iter().map(|item| rewrite_flags(item, types)).collect(), bracket, separator)
		}
		other => children_rewritten(other, |child| rewrite_flags(child, types)),
	}
}

/// `fast+safe`, `fast|safe`, `{fast, safe}`, `fast`: the flags a value names
fn named_flags(value: &Node) -> Vec<String> {
	match value.drop_meta() {
		Node::Symbol(flag) => vec![flag.clone()],
		Node::Key(left, _, right) => [named_flags(left), named_flags(right)].concat(),
		Node::List(items, _, _) => items.iter().flat_map(named_flags).collect(),
		_ => vec![],
	}
}

/// C definitions, before any pass reads `real f(…)` as a conversion of a call; and keyword definitions
/// (`def f(x) {…}`, `fun`, `function`) as `f(x) := {…}`, so every pass reads one definition form
/// The spaced definition `square x := x*x` (wiki/examples.md), parsed as the items `square`, `x := x*x`, is the
/// definition `square(x) := x*x`; `x y z := y*y+u` names x of the parameters y and z. A braceless call ending the body
/// keeps its argument (`fibonacci number := … + fibonacci it - 2`, wiki/Home.md), and in a definition of one named
/// parameter `it` is that parameter
/// P28 (user, 2026-10-05: "Zero value (Go)"): `real x;` declares x with the zero value of its type, `x:real = 0.0`.
/// Only a fresh name: after `x = "5"`, `int x` is the conversion of x
pub fn lower_bare_declarations(program: Node) -> Node {
	let names = crate::soft_keywords::names_given(&program);
	let named: Vec<&str> =
		COLLECTION_DECLARATION_WORDS.iter().map(|(word, _)| *word).filter(|word| names.contains(*word)).collect();
	let program = collection_declarations(program, &named);
	// a program of only `int n` is one statement
	match zero_declaration(&program, &std::collections::HashSet::new()) {
		Some(declaration) => declaration,
		None => declarations_in(program),
	}
}

/// `map data = {key: "v"}` is `data:map = {key: "v"}`, `dict d = …` `d:map = …` with a note; a word the program names
/// itself keeps its meaning
fn collection_declarations(node: Node, named: &[&str]) -> Node {
	let declared = match node.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[word, assignment] => match assignment.drop_meta() {
				Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Node::Symbol(_)) => {
					COLLECTION_DECLARATION_WORDS
						.iter()
						.find(|(written, _)| word.drop_meta().name() == *written && !named.contains(written))
						.map(|(written, meaning)| (*written, *meaning, name.clone(), value.clone()))
				}
				_ => None,
			},
			_ => None,
		},
		_ => None,
	};
	match declared {
		Some((written, meaning, name, value)) => {
			if written != meaning {
				crate::normalize::set_position_of(&node);
				crate::errors::diagnostic::note_alias(written, meaning);
			}
			let target = Node::Key(name, Op::Colon, Box::new(symbol(meaning)));
			key(target, Op::Assign, collection_declarations(*value, named))
		}
		None => node.map_children(|child| collection_declarations(child, named)),
	}
}

fn declarations_in(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let statements = matches!(separator, Separator::Semicolon | Separator::Newline);
			let mut assigned = std::collections::HashSet::new();
			let items = items
				.into_iter()
				.map(|item| {
					let item = declarations_in(item);
					let declared = if statements { zero_declaration(&item, &assigned) } else { None };
					crate::library_words::collect_assigned_names(&item, &mut assigned);
					declared.unwrap_or(item)
				})
				.collect();
			Node::List(items, bracket, separator)
		}
		other => children_rewritten(other, declarations_in),
	}
}

/// `T name` of a type word with a zero value and a name not assigned before: `name:T = zero`
fn zero_declaration(statement: &Node, assigned: &std::collections::HashSet<String>) -> Option<Node> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else {
		return None;
	};
	let [type_word, name] = items.as_slice() else {
		return None;
	};
	let (Node::Symbol(type_name), Node::Symbol(variable)) = (type_word.drop_meta(), name.drop_meta()) else {
		return None;
	};
	if assigned.contains(variable) || crate::analyzer::type_word_kind(variable).is_some() {
		return None;
	}
	// a number type held as a Node (`real`) starts at the exact 0 of the int it covers
	let exact_zero = || crate::type_tests::type_matches("int", type_name).then(|| Node::int(0));
	let zero = zero_value(crate::analyzer::type_word_kind(type_name)?).or_else(exact_zero)?;
	let typed = key(name.clone(), Op::Colon, type_word.clone());
	Some(key(typed, Op::Assign, zero))
}

pub(crate) fn zero_value(kind: crate::type_kinds::Kind) -> Option<Node> {
	use crate::type_kinds::Kind;
	Some(match kind {
		Kind::Int => Node::int(0),
		Kind::Float => Node::Number(crate::extensions::numbers::Number::Float(0.0)),
		Kind::Text => Node::Text(String::new()),
		Kind::List => Node::List(vec![], Bracket::Square, Separator::Space),
		_ => return None,
	})
}

/// `surface = height*width int` and `x : n*2 float`, which the parser reads as `(surface = height*width) int`: a
/// zero-filled typed array of that many elements, written `surface = height*width * int` (issue #15)
pub fn lower_sized_arrays(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if sized_array(&items).is_some() => {
			sized_array(&items).expect("guarded")
		}
		other => children_rewritten(other, lower_sized_arrays),
	}
}

fn sized_array(items: &[Node]) -> Option<Node> {
	let [binding, element_type] = items else { return None };
	let Node::Symbol(type_name) = element_type.drop_meta() else {
		return None;
	};
	let Node::Key(name, Op::Assign | Op::Colon, count) = binding.drop_meta() else {
		return None;
	};
	let is_count = !matches!(
		count.drop_meta(),
		Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square | Bracket::Curly, _)
	);
	(matches!(name.drop_meta(), Node::Symbol(_)) && is_count && crate::analyzer::type_word_kind(type_name).is_some())
		.then(|| {
			let array = key(parens(vec![count.as_ref().clone()]), Op::Mul, element_type.clone());
			Node::Key(name.clone(), Op::Assign, Box::new(array))
		})
}

pub fn lower_spaced_definitions(node: Node) -> Node {
	match node {
		Node::List(items, _, _) if colon_iteration(&items).is_some() => {
			lower_spaced_definitions(colon_iteration(&items).expect("guarded"))
		}
		// `1…5 do print it` (wiki/range.md): a loop over the range, the item is `it`
		Node::Key(range, Op::Do, body) if matches!(range.drop_meta(), Node::Key(_, Op::Range | Op::To, _)) => {
			let block = Node::List(vec![lower_spaced_definitions(*body)], Bracket::Curly, Separator::Semicolon);
			Node::List(vec![symbol("for"), *range, block], Bracket::None, Separator::Space)
		}
		// a phrase `name p… = body`, never the items of a comma list (`(a, b = a * 2) => …` has a default)
		Node::List(items, bracket, separator @ (Separator::Space | Separator::None))
			if spaced_definition(&items).is_some() =>
		{
			let (name, parameters, body) = spaced_definition(&items).expect("guarded");
			let body = lower_spaced_definitions(body);
			let head = parens([vec![name], parameters].concat());
			let body = match one_parameter(&head) {
				Some(parameter) => bind_it(body, &parameter, false),
				None => body,
			};
			let _ = (bracket, separator);
			key(head, Op::Define, body)
		}
		// `f(x) := x + it`: `it` is the one parameter too
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if one_parameter(&head).is_some() => {
			let parameter = one_parameter(&head).expect("guarded");
			let body = bind_it(lower_spaced_definitions(*body), &parameter, false);
			Node::Key(head, op, Box::new(body))
		}
		Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let items = haskell_definitions(items);
			Node::List(items.into_iter().map(lower_spaced_definitions).collect(), bracket, separator)
		}
		other => children_rewritten(other, lower_spaced_definitions),
	}
}

/// Haskell's `twice x = x * 2` among statements: the definition `twice x := x * 2` when a later statement uses
/// `twice` and the body uses every parameter (`int x = 3`, `print x = 5`, `my x = 3` stay assignments)
fn haskell_definitions(statements: Vec<Node>) -> Vec<Node> {
	let defined: Vec<Option<Node>> = statements
		.iter()
		.enumerate()
		.map(|(index, statement)| haskell_definition(statement, &statements[index + 1..]))
		.collect();
	statements.into_iter().zip(defined).map(|(statement, definition)| definition.unwrap_or(statement)).collect()
}

fn haskell_definition(statement: &Node, later: &[Node]) -> Option<Node> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else {
		return None;
	};
	let [name, parameters @ .., last] = items.as_slice() else {
		return None;
	};
	let Node::Key(last_name, Op::Assign, body) = last.drop_meta() else {
		return None;
	};
	let Node::Symbol(function) = name.drop_meta() else {
		return None;
	};
	let is_parameter = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if word == PLACEHOLDER || crate::warp_parser::mentions(body, word));
	let is_free_name = !is_function_keyword(function)
		&& !crate::analyzer::is_declaration_word(function) // `let x = p.x; let y = …` declares x
		&& !crate::library_words::is_library_word(function)
		&& crate::analyzer::type_word_kind(function).is_none();
	let is_used_later = later.iter().any(|statement| crate::warp_parser::mentions(statement, function));
	if !is_free_name || !is_used_later || !parameters.iter().chain([last_name.as_ref()]).all(is_parameter) {
		return None;
	}
	let mut words = items[..items.len() - 1].to_vec();
	words.push(Node::Key(last_name.clone(), Op::Define, body.clone()));
	Some(Node::List(words, Bracket::None, Separator::Space))
}

/// `each [1,2,3]: print it`, `all xs: …` (wiki/iteration.md): a for loop over the list whose item is `it`
fn colon_iteration(items: &[Node]) -> Option<Node> {
	let [word, list_and_body, rest @ ..] = items else {
		return None;
	};
	if !matches!(word.drop_meta(), Node::Symbol(word) if COLON_ITERATION_WORDS.contains(&word.as_str())) {
		return None;
	}
	let (list, first) = split_at_colon(list_and_body)?;
	let body = match rest {
		[] => first,
		_ => Node::List([vec![first], rest.to_vec()].concat(), Bracket::None, Separator::Space),
	};
	// `all numbers > 2: body`: the comparison acts on every item (wiki/iteration.md), a filter loop (P46, its warning)
	let (list, body) = match list.drop_meta() {
		Node::Key(collection, op, value) if op.is_comparison() => {
			let condition = Node::Key(Box::new(symbol(IMPLICIT_PARAMETER)), *op, value.clone());
			let written = format!("{} {op} {}", word.drop_meta().name(), crate::normalize::operand_text(value));
			let question = crate::errors::diagnostic::Ask::new(
				crate::for_loop::headers::FILTER_LOOP_TOPIC,
				format!("`{written}: …` visits only the items that pass its filter"),
				vec![crate::errors::diagnostic::reading(
					"filter the items",
					&format!("for it in … {{ if it {op} {} {{ … }} }}", crate::normalize::operand_text(value)),
				)],
				crate::errors::diagnostic::Fallback::Warning,
			)
			.written(&written)
			.at_node(word);
			if let Err(error) = crate::errors::diagnostic::ask(&question) {
				return Some(error);
			}
			let guarded = if_then(condition, body);
			(collection.as_ref().clone(), guarded)
		}
		_ => (list, body),
	};
	let block = Node::List(vec![body], Bracket::Curly, Separator::Semicolon);
	Some(Node::List(vec![symbol("for"), list, block], Bracket::None, Separator::Space))
}

/// `[1,2,3]: s += it`, parsed as `([1,2,3]: s) += it`: the part before the colon and the body after it
fn split_at_colon(node: &Node) -> Option<(Node, Node)> {
	match node.drop_meta() {
		Node::Key(before, Op::Colon, after) => Some((before.as_ref().clone(), after.as_ref().clone())),
		Node::Key(left, op, right) => {
			let (before, body_start) = split_at_colon(left)?;
			Some((before, Node::Key(Box::new(body_start), *op, right.clone())))
		}
		_ => None,
	}
}

/// The words of `name p… last := body …` before the definition: the name, the leading parameters and the rest, from the
/// definition on (only the shape: spaced_definition decides)
pub(crate) fn spaced_definition_words(items: &[Node]) -> Option<(&Node, &[Node], &[Node])> {
	let definition = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(target, Op::Define | Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(_))))?;
	let (words, rest) = items.split_at(definition);
	let (name, parameters) = words.split_first()?;
	let is_word = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if !is_function_keyword(word));
	(is_word(name) && parameters.iter().all(is_word)).then_some((name, parameters, rest))
}

/// `name p… last := body extra…`: the function name, its parameters and its body (the extra items the argument of
/// the braceless call that ends the body)
fn spaced_definition(items: &[Node]) -> Option<(Node, Vec<Node>, Node)> {
	let (name, parameters, rest) = spaced_definition_words(items)?;
	let (definition, extra) = rest.split_first()?;
	let Node::Key(last, op, body) = definition.drop_meta() else {
		return None;
	};
	// with `=` only a recursive definition (wiki/Home.md `fibonacci number = … fibonacci …`): `print x = 5` stays
	if *op == Op::Assign && !crate::warp_parser::mentions(body, &name.name()) {
		return None;
	}
	let words: Vec<Node> = parameters
		.iter()
		.map(|parameter| parameter.drop_meta().clone())
		.chain(std::iter::once(last.drop_meta().clone()))
		.collect();
	let names: Vec<String> = words.iter().map(Node::name).collect();
	let names: Vec<&str> = names.iter().map(String::as_str).collect();
	// one parse with type_name_matching: `square of a number := …` takes the same slots as with `=`
	let (parameters, body) = match crate::type_name_matching::spaced_parameters(&names, body) {
		None => (words, body.as_ref().clone()),
		Some(Ok(slots)) => slots,
		Some(Err(_)) => return None, // type_name_matching reports it
	};
	Some((name.drop_meta().clone(), parameters, applied_to_last(body, extra)))
}

/// The one named parameter of a definition head `f(x)` (`f(x:int)`), not `it`
/// `it` in the body of a function of one parameter is that parameter, except inside a block with `it` given as a value
/// (returned, assigned, an argument): `mk(k) := { return {it * k} }` returns the function `it => it * k`
fn bind_it(node: Node, parameter: &str, is_value: bool) -> Node {
	let values = |items: Vec<Node>| -> Vec<Node> {
		let head = items.first().cloned().unwrap_or(Node::Empty);
		let mut items = items.into_iter();
		let bound = |item: Node| {
			if crate::lambdas::has_own_it(&head, &item) {
				item
			} else {
				bind_it(item, parameter, true)
			}
		};
		items.next().map(|head| bind_it(head, parameter, false)).into_iter().chain(items.map(bound)).collect()
	};
	match node {
		Node::Symbol(name) if name == IMPLICIT_PARAMETER => symbol(parameter),
		// a lambda of its own `it`: `mk(k) := it => it * k`
		Node::Key(ref parameters, Op::Arrow, _) if names_it(parameters) => node,
		// `{it * k}`, Swift's `{ $0 + k }`
		Node::List(_, Bracket::Curly, _) if is_value && crate::lambdas::block_as_arrow(&node).is_some() => {
			crate::lambdas::block_as_arrow(&node).unwrap_or(node)
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
			key(bind_it(*target, parameter, false), op, bind_it(*value, parameter, true))
		}
		// `return {…}`, `f({…})`, `f {…}`
		Node::List(
			items,
			bracket @ (Bracket::Round | Bracket::None),
			separator @ (Separator::Space | Separator::None | Separator::Colon),
		) if items.len() > 1 && is_callee(&items[0]) => Node::List(values(items), bracket, separator),
		// `xs.map{ it + x }` glued: the block is the method's lambda, its `it` is its own
		Node::Key(receiver, Op::Dot, method) if glued_block(&method).is_some() => {
			Node::Key(Box::new(bind_it(*receiver, parameter, false)), Op::Dot, method)
		}
		// the last statement of a body is its value
		Node::List(items, bracket, separator) => {
			let last = items.len().saturating_sub(1);
			let is_body = bracket == Bracket::Curly || separator == Separator::Semicolon;
			Node::List(
				items
					.into_iter()
					.enumerate()
					.map(|(index, item)| bind_it(item, parameter, is_body && index == last && !is_value))
					.collect(),
				bracket,
				separator,
			)
		}
		Node::Key(left, op, right) => key(bind_it(*left, parameter, false), op, bind_it(*right, parameter, false)),
		other => children_rewritten(other, |child| bind_it(child, parameter, is_value)),
	}
}

/// `it` or `(it, …)`: lambda parameters that bind `it`
fn names_it(parameters: &Node) -> bool {
	match parameters.drop_meta() {
		Node::Symbol(name) => name == IMPLICIT_PARAMETER,
		Node::List(items, _, _) => items.iter().any(names_it),
		_ => false,
	}
}

/// `map{ it + x }` after a dot: the method name and its block
pub(crate) fn glued_block(method: &Node) -> Option<(Node, Node)> {
	let Node::Key(word, Op::Colon, block) = method.drop_meta() else {
		return None;
	};
	let Node::Symbol(_) = word.drop_meta() else { return None };
	matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))
		.then(|| (word.as_ref().clone(), block.as_ref().clone()))
}

/// The function of a call: a name `f`, or a method `xs.map`
pub(crate) fn is_callee(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(_) | Node::Key(_, Op::Dot, _))
}

fn one_parameter(head: &Node) -> Option<String> {
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else {
		return None;
	};
	let [name, parameter] = items.as_slice() else {
		return None;
	};
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let parameter = match parameter.drop_meta() {
		Node::Key(parameter, Op::Colon, _) => parameter.drop_meta(),
		other => other,
	};
	match parameter {
		Node::Symbol(parameter) if parameter != IMPLICIT_PARAMETER => Some(parameter.clone()),
		_ => None,
	}
}

/// `a + f` with the extra items `x - 2`: the last operand called with them, `a + f(x - 2)`
fn applied_to_last(body: Node, extra: &[Node]) -> Node {
	if extra.is_empty() {
		return body;
	}
	match body {
		Node::Key(left, op, right) => Node::Key(left, op, Box::new(applied_to_last(*right, extra))),
		meta @ Node::Meta { .. } => meta.under_meta(|node| applied_to_last(node, extra)),
		last => parens([vec![last], extra.to_vec()].concat()),
	}
}

pub fn lower_c_functions(node: Node) -> Node {
	lower_lists(end_definitions(node), |items| {
		definition_then_statements(items)
			.or_else(|| function_definition(items))
			.or_else(|| extension_block(items))
			.or_else(|| smart_scope(items))
			.or_else(|| partial_application(items))
	})
}

/// The definition keeps the comment and position of its first word (`// doubles` before `def f(x) {…}`)
fn function_definition(items: &[Node]) -> Option<Node> {
	let definition = c_function(items).or_else(|| keyword_definition(items))?;
	Some(definition.with_meta_of(items.first()?))
}

/// `function f(x) { return x * 2 } f(3)` (JS, PHP, Go, Rust): the body's closing brace ends the definition, what
/// follows is the next statement
fn definition_then_statements(items: &[Node]) -> Option<Node> {
	let [keyword, ..] = items else { return None };
	if !matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)) {
		return None;
	}
	let body_end = items.iter().skip(1).position(ends_in_block)? + 1;
	if body_end + 1 >= items.len() {
		return None;
	}
	let (definition, rest) = items.split_at(body_end + 1);
	let definition = function_definition(definition)
		.unwrap_or_else(|| Node::List(definition.to_vec(), Bracket::None, Separator::Space));
	let rest = match rest {
		[single] => single.clone(),
		several => definition_then_statements(several)
			.unwrap_or_else(|| Node::List(several.to_vec(), Bracket::None, Separator::Space)),
	};
	Some(Node::List(vec![definition, rest], Bracket::None, Separator::Semicolon))
}

/// A `{…}` block, or a group or arrow whose last part is one: `((f (x)) {…})`, `(f x) -> i32 {…}`
fn ends_in_block(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(_, Bracket::Curly, _) => true,
		Node::List(items, _, _) => items.last().is_some_and(ends_in_block),
		Node::Key(_, _, right) => ends_in_block(right),
		_ => false,
	}
}

/// `add(1, _)`: a call with placeholders is the lambda of the missing arguments, `partial_1 => add(1, partial_1)`
fn partial_application(items: &[Node]) -> Option<Node> {
	let [Node::Symbol(head), arguments @ ..] = items else {
		return None;
	};
	// `for _ in 0..3 {…}`: a keyword is no call, its `_` an unused loop variable
	if crate::soft_keywords::HARD_KEYWORDS.contains(&head.as_str()) {
		return None;
	}
	let is_placeholder = |argument: &Node| argument.is_symbol(PLACEHOLDER);
	if !arguments.iter().any(is_placeholder) {
		return None;
	}
	// Swift's `func twice(_ x: Int)`, `func sum(_ xs: Int...)`: `_` labels a typed parameter, no placeholder
	let is_type = |type_node: &Node| {
		is_kinded_type_word(type_node)
			|| matches!(type_node.drop_meta(), Node::Key(element, Op::To, end) if is_kinded_type_word(element) && matches!(end.drop_meta(), Node::Empty))
	};
	let typed_parameter =
		|node: &Node| matches!(node.drop_meta(), Node::Key(_, Op::Colon, type_node) if is_type(type_node));
	if arguments.windows(2).any(|pair| is_placeholder(&pair[0]) && typed_parameter(&pair[1])) {
		return None;
	}
	let mut parameters = vec![];
	let arguments = arguments.iter().map(|argument| match is_placeholder(argument) {
		true => {
			parameters.push(Node::Symbol(format!("partial_{}", parameters.len() + 1)));
			parameters.last().expect("pushed").clone()
		}
		false => argument.clone(),
	});
	let call = parens(std::iter::once(items[0].clone()).chain(arguments).collect());
	let head = match parameters.as_slice() {
		[single] => single.clone(),
		_ => Node::List(parameters, Bracket::Round, Separator::Colon),
	};
	Some(key(head, Op::Arrow, call))
}

/// `def f(a, b) { body }`, `def f(x) := body`, `function g() { … }`: the definition `f(a, b) := body`
pub(crate) fn keyword_definition(items: &[Node]) -> Option<Node> {
	let (keyword, definition, mut result_type) = keyword_form(items)?;
	if !matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)) {
		return None;
	}
	let definition = function_typed_result(extension_definition(definition));
	let definition = match typed_result(&definition) {
		Some((untyped, written_type)) => {
			if matches!(definition, Node::Key(_, Op::Arrow, _)) {
				hint_as_result(&definition, &written_type);
			}
			result_type = Some(written_type);
			untyped
		}
		None => definition,
	};
	let (head, op, body) = definition_parts(definition)?;
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else {
		return None;
	};
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let mut labeled_names = vec![];
	let parameters = written_parameters(arguments, keyword.is_symbol(SWIFT_FUNCTION_KEYWORD), &mut labeled_names);
	let body = with_label_names(body, labeled_names);
	// Kotlin/Scala `def square(x) = x*x` defines as `:=` does, so the passes reading definitions see it (P143)
	let op = if parameters.is_empty() { op } else { Op::Define };
	let head = parens(std::iter::once(name.clone()).chain(parameters).collect());
	let target = match result_type {
		Some(result_type) => key(head, Op::Colon, result_type),
		None => head,
	};
	Some(key(target, op, body))
}

/// The keyword, definition and result type of the ways a definition after a keyword is written: glued or curried
/// heads, a tuple result type, `as int` before a block, Go's result types between the head and the body
fn keyword_form(items: &[Node]) -> Option<(&Node, Node, Option<Node>)> {
	Some(match items {
		[keyword, glued] if glued_curried_definition(glued).is_some() => {
			(keyword, glued_curried_definition(glued).expect("guarded"), None)
		}
		[keyword, definition] => (keyword, definition.drop_meta().clone(), None),
		// a tuple result type `-> (Int, Int) {…}` (Swift): the body's tuple as it is
		[keyword, head, body] if tuple_result(head).is_some() => {
			let Node::Key(_, _, result_types) = head.drop_meta() else { unreachable!("guarded") };
			(
				keyword,
				parens(vec![tuple_result(head).expect("guarded").clone(), typed_returns(body.clone(), result_types)]),
				None,
			)
		}
		[keyword, head, rest] if curried_definition(head, rest).is_some() => {
			(keyword, curried_definition(head, rest).expect("guarded"), None)
		}
		// `def f(x) as int { … }`: the result type before a block body, read as `-> int { … }` is (user 2026-10-12, return-as)
		[keyword, head, body]
			if let Node::Key(head, Op::As, result_type) = head.drop_meta()
				&& matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) =>
		{
			let typed_body =
				Node::List(vec![result_type.as_ref().clone(), body.clone()], Bracket::None, Separator::Space);
			(keyword, key(head.as_ref().clone(), Op::As, typed_body), None)
		}
		[keyword, head, body] => (keyword, parens(vec![head.clone(), body.clone()]), None),
		// Go's `func add1(x int) int {…}`: the result type between the head and the body
		[keyword, head, result_type, body] if is_kinded_type_word(result_type) => {
			(keyword, parens(vec![head.clone(), body.clone()]), Some(result_type.clone()))
		}
		// Go's `func divmod(a, b int) (int, int) {…}`: a tuple of result types, each returned value converted
		[keyword, head, result_types, body] if is_type_tuple(result_types) => {
			(keyword, parens(vec![head.clone(), typed_returns(body.clone(), result_types)]), None)
		}
		_ => return None,
	})
}

/// The head, operator and body of a keyword's definition
fn definition_parts(definition: Node) -> Option<(Node, Op, Node)> {
	Some(match definition {
		// `def test: print "test"` (wiki/signal.md), Ruby's endless `def twice = x * 2`: a function without parameters;
		// `def square := it * it` and `def add := a + b` keep their implicit parameters (later passes)
		Node::Key(name, op @ (Op::Colon | Op::Assign), body)
			if matches!(name.drop_meta(), Node::Symbol(_)) && (op == Op::Colon || !crate::analyzer::uses_it(&body)) =>
		{
			(parens(vec![*name]), Op::Define, *body)
		}
		Node::Key(head, op @ (Op::Define | Op::Assign), body) => (*head, op, *body),
		// Python's `def apply(f, x): return f(x)`, early, so the function value passes see a definition; without
		// parameters it stays a def form for late_binding, which would read `z() := e` as a getter (P71); a generator
		// (`def numbers(): yield 1`) is no getter
		Node::Key(head, Op::Colon, body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if (items.len() > 1 || crate::blocks::contains_yield(&body)) && matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))) => {
			(*head, Op::Define, *body)
		}
		// `def hello {…}` without parentheses has no parameters, as `def hello() {…}`
		Node::List(parts, Bracket::Round, _)
			if parts.len() == 2 && matches!(parts[1].drop_meta(), Node::List(_, Bracket::Curly, _)) =>
		{
			let head = match parts[0].drop_meta() {
				Node::Symbol(_) => parens(vec![parts[0].clone()]),
				_ => parts[0].clone(),
			};
			(head, Op::Define, parts[1].clone())
		}
		_ => return None,
	})
}

/// The parameters of a keyword's definition as written, their labels collected into `labeled_names`
fn written_parameters(arguments: &[Node], is_swift: bool, labeled_names: &mut Vec<Node>) -> Vec<Node> {
	// Swift's one labeled parameter `greet(person name: String)` arrives as the two words; a call's arguments come flat,
	// so only Swift's keyword reads them so: `def add(a, b: int) -> int` has two parameters
	let arguments = match arguments {
		[label, typed] if is_swift && argument_label(label, typed).is_some() => {
			vec![Node::List(arguments.to_vec(), Bracket::None, Separator::Space)]
		}
		_ => arguments.to_vec(),
	};
	// the parameters may come as one group: `f (a, b)`, `f (m)`, `f ø`
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, Separator::Space) if matches!(group.as_slice(), [label, typed] if argument_label(label, typed).is_some()) => vec![argument.clone()],
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![argument.clone()],
	})
	// Python's markers `*` (keyword-only after it) and `/` (positional-only before it) take no argument
	.filter(|parameter| !matches!(parameter.drop_meta(), Node::Key(left, Op::Mul | Op::Div, right) if matches!((left.drop_meta(), right.drop_meta()), (Node::Empty, Node::Empty))))
	.map(name_then_type);
	let parameters: Vec<Node> = parameters.map(|parameter| labeled_parameter(parameter, labeled_names)).collect();
	// `func add1(x int)`: the one parameter and its type arrive as two words; `func total(s Shape)` of a declared type;
	// a comma separates parameters, so `def f(p, number)` keeps two
	let comma_separated = arguments
		.iter()
		.any(|argument| matches!(argument.drop_meta(), Node::List(_, Bracket::Round, Separator::Colon)));
	if let (false, [parameter, type_word]) = (comma_separated, parameters.as_slice()) {
		let is_type_name = matches!(type_word.drop_meta(), Node::Symbol(word) if word.starts_with(char::is_uppercase));
		if matches!(parameter.drop_meta(), Node::Symbol(_)) && (is_kinded_type_word(type_word) || is_type_name) {
			return vec![key(parameter.clone(), Op::Colon, type_word.clone())];
		}
	}
	parameters
}

/// Kotlin's extension method `fun Int.twice() = this * 2`: the function `twice(this:Int) := this * 2`, its receiver the
/// first parameter, named as the body names it (`this`, or Swift's `self`), so `3.twice()` calls it as a method
fn extension_definition(definition: Node) -> Node {
	let Node::Key(head, op, rest) = definition else {
		return definition;
	};
	let Node::Key(receiver_type, Op::Dot, call) = head.drop_meta() else {
		return Node::Key(head, op, rest);
	};
	let Node::List(call, Bracket::Round, separator) = call.drop_meta() else {
		return Node::Key(head, op, rest);
	};
	if !matches!(receiver_type.drop_meta(), Node::Symbol(_))
		|| !matches!(call.first().map(Node::drop_meta), Some(Node::Symbol(_)))
	{
		return Node::Key(head, op, rest);
	}
	let head = with_receiver(call, separator, receiver_type, &rest);
	Node::Key(Box::new(head), op, rest)
}

/// The head `name(receiver:T, parameters…)` of a method on T whose call is `name(parameters…)`
fn with_receiver(call: &[Node], separator: &Separator, receiver_type: &Node, body: &Node) -> Node {
	let receiver = if body.mentions_any(&[SELF_WORD]) { SELF_WORD } else { THIS_WORD };
	let receiver = key(symbol(receiver), Op::Colon, receiver_type.clone());
	let parameters = call[1..].iter().flat_map(grouped_parameters);
	Node::List(
		std::iter::once(call[0].clone()).chain(std::iter::once(receiver)).chain(parameters).collect(),
		Bracket::Round,
		separator.clone(),
	)
}

/// A smart scope (wiki/inventions.md) `Number { Square = it*it }`: in the scope of a type each `name = body` defines a
/// method of the type, `it` the value it is called on: `Square(self:Number) := self*self`, so `3.Square` is 9
fn smart_scope(items: &[Node]) -> Option<Node> {
	let [receiver_type, block] = items else { return None };
	let Node::Symbol(type_name) = receiver_type.drop_meta() else {
		return None;
	};
	let Node::List(definitions, Bracket::Curly, separator) = block.drop_meta() else {
		return None;
	};
	let separated =
		matches!(separator, Separator::Semicolon | Separator::Newline | Separator::Colon) || definitions.len() == 1;
	if !separated {
		return None;
	}
	let is_method = |definition: &Node| matches!(definition.drop_meta(), Node::Key(name, Op::Assign | Op::Define, _) if matches!(name.drop_meta(), Node::Symbol(_)));
	if crate::analyzer::type_word_kind(&type_name.to_lowercase()).is_none()
		|| definitions.is_empty()
		|| !definitions.iter().all(is_method)
	{
		return None;
	}
	let receiver = symbol(SELF_WORD);
	let methods = definitions.iter().map(|definition| {
		let Node::Key(name, _, body) = definition.drop_meta() else { unreachable!("guarded") };
		let head = parens(vec![name.as_ref().clone(), key(receiver.clone(), Op::Colon, receiver_type.clone())]);
		key(head, Op::Define, renamed_it(body.as_ref().clone(), &receiver))
	});
	Some(Node::List(methods.collect(), Bracket::None, Separator::Semicolon))
}

/// The body with `it` read as the receiver
fn renamed_it(node: Node, receiver: &Node) -> Node {
	match node {
		Node::Symbol(word) if word == crate::words::IMPLICIT_PARAMETER => receiver.clone(),
		other => other.map_children(|child| renamed_it(child, receiver)),
	}
}

/// The (type, method) pairs of the extension methods a program defines, `fun P.twice() = …` and `extension P { func
/// twice() … }`, read before lower_c_functions turns them into functions (reflection lists them among P's methods)
pub(crate) fn extension_methods(node: &Node) -> Vec<(String, String)> {
	let mut methods = vec![];
	let is_function_keyword = |item: &Node| item.symbol_name().is_some_and(crate::operators::is_function_keyword);
	node.visit(&mut |part| {
		let Node::List(items, _, _) = part else {
			return;
		};
		match items.as_slice() {
			[keyword, definition, ..] if is_function_keyword(keyword) => {
				methods.extend(receiver_and_method(definition));
			}
			[word, receiver_type, block] if word.is_symbol(EXTENSION_WORD) => {
				let Some(receiver_type) = receiver_type.symbol_name() else {
					return;
				};
				let definitions = match block.drop_meta() {
					Node::List(items, Bracket::Curly, _) if items.first().is_some_and(is_function_keyword) => {
						vec![block.drop_meta()]
					}
					Node::List(items, Bracket::Curly, _) => items.iter().map(Node::drop_meta).collect(),
					_ => vec![],
				};
				for definition in definitions {
					let Node::List(items, _, _) = definition else {
						continue;
					};
					if let [keyword, head, ..] = items.as_slice() {
						if let (true, Node::Symbol(method)) = (is_function_keyword(keyword), leftmost(head)) {
							methods.push((receiver_type.to_string(), method.clone()));
						}
					}
				}
			}
			_ => {}
		}
	});
	methods
}

/// `P.twice` in the head of `fun P.twice() = …` (`def P.twice() {…}`, `fun P.twice(): int {…}`)
fn receiver_and_method(definition: &Node) -> Option<(String, String)> {
	match definition.drop_meta() {
		Node::Key(receiver_type, Op::Dot, call) if matches!(receiver_type.drop_meta(), Node::Symbol(_)) => {
			match leftmost(call) {
				Node::Symbol(method) => Some((receiver_type.drop_meta().name(), method.clone())),
				_ => None,
			}
		}
		Node::Key(head, _, _) => receiver_and_method(head),
		Node::List(items, _, _) => receiver_and_method(items.first()?),
		_ => None,
	}
}

/// The first word of a head: `twice` of `(twice ø) {…}`
fn leftmost(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::Key(head, _, _) => leftmost(head),
		Node::List(items, _, _) if !items.is_empty() => leftmost(&items[0]),
		other => other,
	}
}

/// Swift's `extension Int { func twice() -> Int { self * 2 } }`: each function of the block a method on the type, as
/// `fun Int.twice()` defines it
fn extension_block(items: &[Node]) -> Option<Node> {
	let [word, receiver_type, block] = items else {
		return None;
	};
	let is_extension = word.is_symbol(EXTENSION_WORD) && matches!(receiver_type.drop_meta(), Node::Symbol(_));
	// a block of one function arrives as that definition
	let definitions = match block.drop_meta() {
		Node::List(definitions, Bracket::Curly, _) => definitions.clone(),
		definition @ Node::Key(_, Op::Define | Op::Assign, _) => vec![definition.clone()],
		_ => return None,
	};
	if !is_extension {
		return None;
	}
	let method = |definition: &Node| {
		let Node::Key(head, op @ (Op::Define | Op::Assign), body) = definition.drop_meta() else {
			return definition.clone();
		};
		let (call, result_type) = match head.drop_meta() {
			Node::Key(call, Op::Colon, result_type) => (call.drop_meta(), Some(result_type)),
			call => (call, None),
		};
		let Node::List(call, Bracket::Round, separator) = call else {
			return definition.clone();
		};
		let head = with_receiver(call, separator, receiver_type, body);
		let head = match result_type {
			Some(result_type) => Node::Key(Box::new(head), Op::Colon, result_type.clone()),
			None => head,
		};
		Node::Key(Box::new(head), *op, body.clone())
	};
	Some(Node::List(definitions.iter().map(method).collect(), Bracket::None, Separator::Semicolon))
}

fn lower_lists(node: Node, lowering: impl Fn(&[Node]) -> Option<Node> + Copy) -> Node {
	// `f(_) := …`: the definition lowered in place, its head as a head; a keyword form `func f() -> T {…}` is a list,
	// lowered from its items as any list (Swift's extension block reads its methods so)
	if !matches!(node.drop_meta(), Node::List(..)) && crate::lowering::nodes::function_head(&node).is_some() {
		return lower_definition(node, lowering);
	}
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| lower_lists(item, lowering)).collect();
			lowering(&items).unwrap_or(Node::List(items, bracket, separator))
		}
		other => children_rewritten(other, |child| lower_lists(child, lowering)),
	}
}

fn lower_definition(node: Node, lowering: impl Fn(&[Node]) -> Option<Node> + Copy) -> Node {
	match node {
		Node::Key(head, op, body) => {
			Node::Key(Box::new(lower_definition_head(*head, lowering)), op, Box::new(lower_lists(*body, lowering)))
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| lower_definition(node, lowering)),
		other => other,
	}
}

fn lower_definition_head(node: Node, lowering: impl Fn(&[Node]) -> Option<Node> + Copy) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			Node::List(items.into_iter().map(|item| lower_lists(item, lowering)).collect(), bracket, separator)
		}
		Node::Key(head, op, result) => {
			Node::Key(Box::new(lower_definition_head(*head, lowering)), op, Box::new(lower_lists(*result, lowering)))
		}
		meta @ Node::Meta { .. } => meta.under_meta(|node| lower_definition_head(node, lowering)),
		other => other,
	}
}

/// Swift's label for an argument without a label
const WILDCARD_LABEL: &str = "_";
const DART_REQUIRED: &str = "required";
const SWIFT_FUNCTION_KEYWORD: &str = "func";

/// `real f(real x) { … }`: the definition `f(x:real) := { … }` (the parser reads the type word, then the call and its
/// block); the result kind is inferred as for any definition
pub(crate) fn c_function(items: &[Node]) -> Option<Node> {
	let [result_type, definition] = items else { return None };
	let Node::Symbol(result_type) = result_type.drop_meta() else {
		return None;
	};
	if crate::analyzer::type_word_kind(result_type).is_none() && result_type != "void" {
		return None;
	}
	let (head, body) = match definition.drop_meta() {
		Node::List(parts, Bracket::Round, _) => match parts.as_slice() {
			[head, body] if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => (head, body),
			_ => return None,
		},
		// C#'s expression-bodied member `int Add(int a, int b) => a + b`
		Node::Key(head, Op::Arrow, body) => (head.as_ref(), body.as_ref()),
		_ => return None,
	};
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else {
		return None;
	};
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, Separator::Colon) => group.clone(), // `(real a, int b)`
		Node::List(group, Bracket::Curly, _) => group.clone(), // Dart's named parameters `{required int a, int b = 2}`
		Node::Empty => vec![],                                 // `f()`
		_ => vec![argument.clone()],
	});
	let parameters: Option<Vec<Node>> = parameters.map(|parameter| c_parameter(&parameter)).collect();
	let head = Node::List([vec![name.clone()], parameters?].concat(), Bracket::Round, Separator::Colon);
	Some(key(head, Op::Define, body.clone()))
}

/// `real x` is `x:real`; a bare name stays untyped, as does a parameter the parser typed already
fn c_parameter(parameter: &Node) -> Option<Node> {
	match parameter.drop_meta() {
		Node::Symbol(_) | Node::Key(_, Op::Colon, _) => Some(parameter.clone()),
		// Dart's `int b = 2` among the named parameters
		Node::Key(declared, Op::Assign, default) => {
			Some(Node::Key(Box::new(c_parameter(declared)?), Op::Assign, default.clone()))
		}
		Node::List(words, _, Separator::Space) => match words.as_slice() {
			// Dart's `required int a`: every parameter needs its value anyway
			[required, rest @ ..] if required.is_symbol(DART_REQUIRED) => {
				c_parameter(&Node::List(rest.to_vec(), Bracket::None, Separator::Space))
			}
			[kind, name] if matches!((kind.drop_meta(), name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) => {
				Some(key(name.clone(), Op::Colon, kind.clone()))
			}
			_ => None,
		},
		_ => None,
	}
}
