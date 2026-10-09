//! Forms other languages use, lowered to warp's own (notes/welcoming.md), before any other pass:
//! - `match v { 0 => "zero"; _ => "other" }`: the cases of switch/match written with `=>`, `_` the default
//! - `loop { … }`: `while true { … }`, left by `break`
//! - anonymous functions `function(a, b) { … }`, `fn(x) { … }` (JS, Rust-ish), `f <- function(x) x * 2` (R) and
//!   `lambda x: …` (Python): lambdas
//!   (`xs |> f(b)` is read by the parser, `f(1, _)` lowered in declarations.rs)
//! - OCaml / F# `let f x = body in rest`: the definition `f(x) := body`, then rest
//! - JS destructured parameters `({a, b}) => a + b`: the object taken apart into its fields

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

use crate::switch::SWITCH_WORDS;
const WILDCARD: &str = "_";
const DEFAULT_CASE: &str = "default";
const LOOP_WORD: &str = "loop";
/// Lua's, Ruby's and Julia's word closing a function body
const END_WORD: &str = "end";
const LAMBDA_WORD: &str = "lambda";
const LET_WORD: &str = "let";
const IN_WORD: &str = "in";
const DESTRUCTURED_OBJECT: &str = "object·";
/// The Meta key the parser puts on the name of a function written with type parameters `fn id<T>(…)`: their names
pub const GENERIC_MARK: &str = "generic";
const RUBY_LAMBDA_WORDS: [&str; 2] = ["lambda", "proc"];
const CALL_METHOD: &str = "call";
/// Swift's and C's case label `case .north:` in a switch
const CASE_WORD: &str = "case";
/// Kotlin's `when (x) { a -> 1; is T -> 2; else -> 3 }`, an if chain
const WHEN_WORD: &str = "when";
const ELSE_WORD: &str = "else";
/// The listeners a `when` without arms is: of an event, of a condition
const ON_WORD: &str = "on";
const WHENEVER_WORD: &str = "whenever";
const WHEN_CONDITION_TOPIC: &str = "when-condition";
const IS_WORD: &str = "is";
/// The if chain a `when` arm adds, its conditions joined by `or`
const WHEN_ARM: &str = "if CONDITION then VALUE else OTHERWISE";
const LAST_WHEN_ARM: &str = "if CONDITION then VALUE";
const OCAML_FUNCTION_KEYWORD: &str = "fun";
/// F#/OCaml (and Elixir's Enum) modules whose iteration functions are warp's words: `List.map f xs` is `map f xs`
const ITERATION_MODULES: [&str; 4] = ["List", "Seq", "Array", "Enum"];
const MODULE_ITERATIONS: [&str; 5] = ["map", "filter", "fold", "reduce", "sum"];
/// C# LINQ methods and the warp words they are (the alias rule: they work, with a note naming warp's word)
/// Iteration functions of other languages and the warp word they are (alias rule, with a note): R's `sapply(xs, f)` is
/// `map(xs, f)`; PHP's `array_map(f, xs)` takes the function first (true), `array_filter(xs, f)` the list
const FOREIGN_ITERATIONS: [(&str, &str, bool, &str); 6] = [
	("sapply", "map", false, "R's apply function"), ("lapply", "map", false, "R's apply function"), ("vapply", "map", false, "R's apply function"),
	("array_map", "map", true, "PHP's array function"), ("array_filter", "filter", false, "PHP's array function"),
	("array_sum", "sum", false, "PHP's array function"),
];
/// JavaScript's math namespace: `Math.sqrt(16)` is `sqrt(16)`
const JS_MATH: &str = "Math";
/// Module words written only qualified, as the module's own name for them (P183: `file.append(path, text)`; a bare
/// `append` stays the list method)
const QUALIFIED_WORDS: [(&str, &str, &str); 1] = [("file", "append", "append_file")];
/// R's vector constructor: `c(1, 2, 3)` is the list `[1, 2, 3]` unless the program names something c
const R_VECTOR_WORD: &str = "c";
/// Ruby's `xs.sum { |x| x * 2 }`, Kotlin's `xs.sumOf { it * 2 }`, `xs.count { it > 1 }`: a reduction of the list
/// the block makes, `sum(map(xs, block))`, `count(filter(xs, block))`
const BLOCK_REDUCTIONS: [(&str, &str, &str); 3] = [("sum", "sum", "map"), ("sumOf", "sum", "map"), ("count", "count", "filter")];
/// Ruby's conversion and case methods and the warp words they are: `x.to_s` is `string(x)`
const RUBY_METHODS: [(&str, &str); 5] = [("to_s", "string"), ("to_i", "int"), ("to_f", "float"), ("upcase", "upper"), ("downcase", "lower")];
/// Elixir's `def sq(x), do: x * x` and its private `defp`
const ELIXIR_DO_WORD: &str = "do";
const ELIXIR_PRIVATE_DEF: &str = "defp";
/// C++'s `auto x = 3`
const CPP_AUTO_WORD: &str = "auto";
const LINQ_METHODS: [(&str, &str); 12] = [("Select", "map"), ("Where", "filter"), ("Aggregate", "reduce"), ("Sum", "sum"),
	("Count", "count"), ("Max", "max"), ("Min", "min"), ("Any", "any"), ("All", "all"), ("First", "first"), ("Last", "last"),
	("Contains", "contains")];

pub fn lower(node: Node) -> Node {
	let defined = defined_names(&node);
	let node = if names_vector_word(&node) { node } else { r_vectors(node) };
	let node = qualified_module_calls(node);
	let node = linq_calls(forms(node), &defined);
	let mut lambda_names = HashSet::new();
	collect_lambda_names(&node, &mut lambda_names);
	lambda_calls(node, &lambda_names)
}

/// The names assigned a lambda: `f = x => …`, `f = { |x| … }`
fn collect_lambda_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Node::Symbol(_)) && crate::lambdas::arrow_lambda(value).is_some() => {
			names.insert(name.name());
		}
		Node::Key(left, _, right) => [left, right].into_iter().for_each(|side| collect_lambda_names(side, names)),
		Node::List(items, _, _) => items.iter().for_each(|item| collect_lambda_names(item, names)),
		_ => {}
	}
}

/// Ruby's `f.call(3)` of a lambda f: the call `f(3)`; a method `call` of an object stays
fn lambda_calls(node: Node, lambda_names: &HashSet<String>) -> Node {
	match node {
		Node::Key(function, Op::Dot, call) if lambda_names.contains(&function.name()) && called_arguments(&call).is_some() => {
			let arguments = called_arguments(&call).expect("guarded").into_iter().map(|argument| lambda_calls(argument, lambda_names));
			Node::List([*function].into_iter().chain(arguments).collect(), Bracket::Round, Separator::None)
		}
		other => other.map_children(|child| lambda_calls(child, lambda_names)),
	}
}

fn forms(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) if generic_names(&items).is_some() => {
			let (function, names) = generic_names(&items).expect("guarded");
			crate::normalize::set_position_of(&items[0]);
			crate::normalize::hint(&format!("{function}<{}>", names.join(", ")), &function, "warp infers types: write the function without type parameters");
			forms(without_type_parameters(Node::List(items, bracket, separator), &names))
		}
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = foreign_iteration(module_qualified_iteration(python_lambdas(braceless_lambdas(items.into_iter().map(forms).collect()))));
			if let Some(lambda) = anonymous_function(&items).or_else(|| assigned_braceless_function(&items)).or_else(|| ruby_lambda(&items)).or_else(|| keyword_arrow(&items)) {
				return lambda;
			}
			if let Some(reduction) = block_reduction(&items) {
				return reduction;
			}
			// C++'s `auto x = 3`: wasp infers every type
			if items.len() > 1 && is_word(&items[0], CPP_AUTO_WORD) && matches!(items[1].drop_meta(), Node::Key(_, Op::Assign, _)) {
				crate::normalize::set_position_of(&items[0]);
				crate::normalize::hint(&format!("{CPP_AUTO_WORD} "), "", "warp infers types: write the assignment without auto");
				let rest = items[1..].to_vec();
				return forms(if rest.len() == 1 { rest[0].clone() } else { Node::List(rest, bracket, separator) });
			}
			if let Some(assignment) = cpp_lambda(&items) {
				return assignment;
			}
			if let Some(definition) = elixir_definition(&items) {
				return definition;
			}
			if let Some(binding) = let_binding(&items) {
				return binding;
			}
			let items = go_destructuring(items, &separator);
			if let Some(chain) = when_chain(&items) {
				return chain;
			}
			if let Some(listener) = when_listener(&items) {
				return Node::List(listener, bracket, separator);
			}
			endless_loop(&items).unwrap_or_else(|| Node::List(arrow_cases(items), bracket, separator))
		}
		Node::Key(left, Op::Colon, body) if lambda_parameters(&left).is_some() => lambda(lambda_parameters(&left).expect("guarded"), forms(*body)),
		Node::Key(parameters, Op::FatArrow, body) if destructured_parameters(&parameters).is_some() => {
			let (parameters, fields) = destructured_parameters(&parameters).expect("guarded");
			lambda(parameters, Node::List([fields, vec![forms(*body)]].concat(), Bracket::Curly, Separator::Semicolon))
		}
		// Elixir's `Enum.map(xs, f)`: the call `map(xs, f)`, with a note naming warp's word
		Node::Key(module, Op::Dot, call) if module_call(&module, &call).is_some() => {
			let iteration = module_call(&module, &call).expect("guarded");
			crate::normalize::hint(&format!("{}.{iteration}(", module.serialize()), &format!("{iteration}("), "warp's word for the module function");
			forms(*call)
		}
		// Ruby's `x.to_s`, `t.upcase`: warp's word called on x, `string(x)`, `upper(t)` (alias rule, with a note)
		Node::Key(subject, Op::Dot, method) if ruby_method(&method).is_some() => {
			let warp_word = ruby_method(&method).expect("guarded");
			crate::normalize::set_position_of(&method);
			crate::normalize::hint(&format!(".{}", method.name()), &format!("{warp_word}(…)"), "warp's word for Ruby's method");
			Node::List(vec![Node::Symbol(warp_word.to_string()), forms(*subject)], Bracket::Round, Separator::None)
		}
		// `f = lambda *xs: …`, JS `f = (...xs) => …`: the definition `f(*xs) := …`, which variadic.rs reads
		Node::Key(name, Op::Assign, value) if starred_lambda(&name, &value).is_some() => forms(starred_lambda(&name, &value).expect("guarded")),
		Node::Key(left, op, right) => Node::Key(Box::new(forms(*left)), op, Box::new(forms(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(forms(*node)), data },
		other => other,
	}
}

/// OCaml / F# `let f x = body in rest`, `let f x = body`, `let x = v in rest`: the definition `f(x) := body` (or the
/// assignment) and then rest
fn let_binding(items: &[Node]) -> Option<Node> {
	let [keyword, names @ .., last] = items else { return None };
	let Node::Key(last_name, Op::Assign, value) = last.drop_meta() else { return None };
	if !is_word(keyword, LET_WORD) {
		return None;
	}
	let names: Vec<&Node> = names.iter().chain([last_name.as_ref()]).collect();
	if !names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) {
		return None;
	}
	let (body, rest) = split_at_in(value);
	if names.len() == 1 && rest.is_none() {
		return None; // `let x = 5`: a declaration as it is
	}
	let (name, parameters) = names.split_first().expect("a name");
	let binding = match parameters.is_empty() {
		true => Node::Key(Box::new((*name).clone()), Op::Assign, Box::new(forms(body))),
		false => {
			let head = Node::List(names.iter().map(|name| (*name).clone()).collect(), Bracket::Round, Separator::None);
			Node::Key(Box::new(head), Op::Define, Box::new(forms(body)))
		}
	};
	Some(match rest {
		Some(rest) => Node::List(vec![binding, forms(rest)], Bracket::Round, Separator::Semicolon),
		None => binding,
	})
}

/// Go's `q, r := f(x)`: names taking the values apart, `q, r = f(x)` (`r := …` alone would define a getter)
fn go_destructuring(mut items: Vec<Node>, separator: &Separator) -> Vec<Node> {
	let Some(position) = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Define, _))) else { return items };
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(_));
	let Node::Key(name, Op::Define, value) = items[position].drop_meta() else { return items };
	if *separator != Separator::Colon || position == 0 || !is_name(name) || !items[..position].iter().all(is_name) {
		return items;
	}
	items[position] = Node::Key(name.clone(), Op::Assign, value.clone());
	items
}

/// `x * 2 in double 4` → (`x * 2`, `double 4`); no `in`: the value alone
fn split_at_in(value: &Node) -> (Node, Option<Node>) {
	// OCaml's `let inc = fun x -> x + 1 in inc 4`: the `in` ends the lambda's body
	if let Node::Key(head, op @ (Op::Arrow | Op::FatArrow), body) = value.drop_meta() {
		let (body, rest) = split_at_in(body);
		return (Node::Key(head.clone(), *op, Box::new(body)), rest);
	}
	let Node::List(words, Bracket::None, separator @ (Separator::Space | Separator::None)) = value.drop_meta() else { return (value.clone(), None) };
	let Some(position) = words.iter().position(|word| is_word(word, IN_WORD)) else { return (value.clone(), None) };
	let phrase = |part: &[Node]| match part {
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, separator.clone()),
	};
	(phrase(&words[..position]), Some(phrase(&words[position + 1..])))
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word)
}

/// `[match, subject, {k => v …}]`, or `[match, (subject {k => v …})]` as an assigned match parses: the cases as `k: v`,
/// `_ => v` as `default: v`
fn arrow_cases(mut items: Vec<Node>) -> Vec<Node> {
	if !items.first().is_some_and(|word| SWITCH_WORDS.iter().any(|switch| is_word(word, switch))) {
		return items;
	}
	match items.len() {
		3 => items[2] = colon_cases(&items[2]),
		2 => if let Node::List(pair, bracket, separator) = items[1].drop_meta() {
			if let [subject, cases] = pair.as_slice() {
				items[1] = Node::List(vec![subject.clone(), colon_cases(cases)], bracket.clone(), separator.clone());
			}
		},
		_ => {}
	}
	items
}

fn colon_cases(block: &Node) -> Node {
	let Node::List(cases, Bracket::Curly, separator) = block.drop_meta() else { return block.clone() };
	let cases = cases.iter().map(|case| match case.drop_meta() {
		// Swift's `case .north: 1` (read as the member `case.north`) and `case Direction.north: 1`
		Node::Key(label, Op::Colon, body) if matches!(label.drop_meta(), Node::Key(word, Op::Dot, _) if is_word(word, CASE_WORD)) => {
			let Node::Key(_, _, member) = label.drop_meta() else { unreachable!("guarded") };
			Node::Key(Box::new(Node::Key(Box::new(Node::Empty), Op::Dot, member.clone())), Op::Colon, body.clone())
		}
		Node::List(words, _, _) if matches!(words.as_slice(), [word, _] if is_word(word, CASE_WORD)) => words[1].clone(),
		Node::Key(pattern, Op::FatArrow, body) => {
			let pattern = if is_word(pattern, WILDCARD) { Node::Symbol(DEFAULT_CASE.to_string()) } else { pattern.as_ref().clone() };
			Node::Key(Box::new(pattern), Op::Colon, body.clone())
		}
		_ => case.clone(),
	});
	Node::List(cases.collect(), Bracket::Curly, separator.clone())
}

/// Kotlin's `when (x) { 1, 2 -> a; is Circle -> b; else -> c }` (or `when { x > 0 -> a … }` without a subject): the
/// chain `if x == 1 or x == 2 then a else if x is Circle then b else c`
fn when_chain(items: &[Node]) -> Option<Node> {
	// `= when (s) {…}` after a definition arrives as the group `when (s)` and the arms
	if let [head, arms] = items {
		if let Node::List(words, _, Separator::Space) = head.drop_meta() {
			if words.len() == 2 && is_word(&words[0], WHEN_WORD) {
				return when_chain(&[words[0].clone(), words[1].clone(), arms.clone()]);
			}
		}
	}
	let (subject, arms) = match items {
		[word, subject, arms] if is_word(word, WHEN_WORD) => (Some(match subject.drop_meta() {
			Node::List(inner, Bracket::Round, _) if inner.len() == 1 => inner[0].clone(),
			_ => subject.clone(),
		}), arms),
		[word, arms] if is_word(word, WHEN_WORD) => (None, arms),
		_ => return None,
	};
	let Node::List(arms, Bracket::Curly, _) = arms.drop_meta() else { return None };
	let arms: Vec<(Vec<Node>, Node)> = arms.iter().map(when_arm).collect::<Option<_>>()?;
	let condition = |pattern: Node| match (&subject, pattern) {
		(_, pattern) if is_word(&pattern, ELSE_WORD) => Node::True,
		(Some(subject), pattern) => Node::Key(Box::new(subject.clone()), Op::Eq, Box::new(pattern)),
		(None, pattern) => pattern,
	};
	let chain = arms.into_iter().rev().fold(None, |otherwise: Option<Node>, (patterns, value)| {
		let condition = patterns.into_iter().map(condition).reduce(|left, right| Node::Key(Box::new(left), Op::Or, Box::new(right)))?;
		let template = if otherwise.is_some() { WHEN_ARM } else { LAST_WHEN_ARM };
		let bindings = [("CONDITION", condition), ("VALUE", forms(value)), ("OTHERWISE", otherwise.unwrap_or(Node::Empty))];
		let bindings = bindings.into_iter().map(|(placeholder, node)| (placeholder.to_string(), node)).collect();
		Some(crate::law::substitute(&crate::warp_parser::parse(template), &bindings).drop_meta().clone())
	});
	chain
}

/// `when click {…}` is `on click {…}`, `when x > 3 {…}` is `whenever x > 3 {…}` (card signals-shape, user 2026-10-08):
/// a `when` whose block holds no `->` arms listens, to the event a bare word names or to the condition otherwise
fn when_listener(items: &[Node]) -> Option<Vec<Node>> {
	let [word, subject, body] = items else { return None };
	let Node::List(statements, Bracket::Curly, _) = body.drop_meta() else { return None };
	let has_arms = statements.iter().any(|statement| matches!(statement.drop_meta(), Node::Key(_, Op::Arrow, _)));
	if !is_word(word, WHEN_WORD) || has_arms {
		return None;
	}
	let listener = if matches!(subject.drop_meta(), Node::Symbol(_)) { ON_WORD } else { WHENEVER_WORD };
	if listener == WHENEVER_WORD {
		crate::normalize::set_position_of(word);
		crate::diagnostic::educate_once(WHEN_CONDITION_TOPIC, WHEN_WORD, WHENEVER_WORD, "it reacts to every later write that makes the condition true; write if for a one-time check now");
	}
	Some(vec![Node::Symbol(listener.to_string()), subject.clone(), body.clone()])
}

/// The patterns and value of a `when` arm: `5 -> 50`, `1, 2 -> 10`, `is Circle -> 3`, `else -> 0`
fn when_arm(arm: &Node) -> Option<(Vec<Node>, Node)> {
	match arm.drop_meta() {
		Node::Key(pattern, Op::Arrow, value) => Some((vec![pattern.as_ref().clone()], value.as_ref().clone())),
		// `is Circle -> 3`: the type test `x is Circle`, as `is` reads
		Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [word, _] if is_word(word, IS_WORD)) => when_arm(&words[1]),
		Node::List(patterns, _, _) if !patterns.is_empty() => {
			let (last, first) = patterns.split_last()?;
			let (mut all, value) = when_arm(last)?;
			all.splice(0..0, first.iter().cloned());
			Some((all, value))
		}
		_ => None,
	}
}

/// `loop { body }`
fn endless_loop(items: &[Node]) -> Option<Node> {
	let [word, body] = items else { return None };
	if !is_word(word, LOOP_WORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let condition = Node::Key(Box::new(Node::Empty), Op::While, Box::new(Node::True));
	Some(Node::Key(Box::new(condition), Op::Do, Box::new(body.clone())))
}

fn lambda(parameters: Node, body: Node) -> Node {
	Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
}

/// `[function (a, b)] {body}`: a function keyword with parameters and no name, then its body
fn anonymous_function(items: &[Node]) -> Option<Node> {
	let [head, rest @ ..] = items else { return None };
	let [body] = without_result_type(rest) else { return None };
	if !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	Some(lambda(function_parameters(head)?, forms(body.clone())))
}

/// The type parameters marked on a definition's name (`fn id<T>`), when this statement is that definition
fn generic_names(items: &[Node]) -> Option<(String, Vec<String>)> {
	items.iter().find_map(marked_generic_names)
}

/// The function's name and the names in a GENERIC_MARK anywhere in node (Node::visit looks through Meta, this reads it)
fn marked_generic_names(node: &Node) -> Option<(String, Vec<String>)> {
	match node {
		Node::Meta { node, data } => match data.as_ref() {
			Node::Key(mark, Op::Colon, written) if mark.name() == GENERIC_MARK => {
				let function = match node.drop_meta() {
					Node::List(items, _, _) => items.first().map(Node::name).unwrap_or_default(),
					other => other.name(),
				};
				Some((function, written.name().split(' ').map(str::to_string).collect()))
			}
			_ => marked_generic_names(node),
		},
		Node::Key(left, _, right) => marked_generic_names(left).or_else(|| marked_generic_names(right)),
		Node::List(items, _, _) => items.iter().find_map(marked_generic_names),
		_ => None,
	}
}

/// P157: the definition without its generics: the mark, `x: T` annotations and a `-> T` / `: T` result naming them
fn without_type_parameters(node: Node, names: &[String]) -> Node {
	let is_type_parameter = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if names.contains(name));
	match node {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Key(mark, _, _) if mark.name() == GENERIC_MARK) => without_type_parameters(*node, names),
		Node::Key(name, Op::Colon, kind) if is_type_parameter(&kind) => without_type_parameters(*name, names),
		// `head -> T { body }`, `head: T { body }`: the definition `(head { body })`
		Node::Key(head, Op::Arrow | Op::Colon, typed_body) if matches!(typed_body.drop_meta(), Node::List(parts, Bracket::None, Separator::Space) if parts.len() == 2 && is_type_parameter(&parts[0])) => {
			let Node::List(parts, _, _) = typed_body.drop_meta() else { unreachable!("guarded") };
			Node::List(vec![without_type_parameters(*head, names), without_type_parameters(parts[1].clone(), names)], Bracket::Round, Separator::None)
		}
		// Swift's `_ x: T`: without its type `_ x` would read as a placeholder `_` (partial application), so the label goes too
		Node::List(items, bracket, separator) if items.windows(2).any(|pair| is_word(&pair[0], WILDCARD) && generic_parameter(&pair[1], names)) => {
			let mut kept: Vec<Node> = Vec::new();
			for item in items {
				if generic_parameter(&item, names) && kept.last().is_some_and(|label| is_word(label, WILDCARD)) {
					kept.pop();
					let Node::Key(name, _, _) = item.drop_meta() else { unreachable!("a generic parameter") };
					crate::normalize::set_position_of(&item);
					crate::normalize::hint(&format!("{WILDCARD} {}", item.serialize()), &name.serialize(), "warp names a parameter once, no label");
				}
				kept.push(item);
			}
			without_type_parameters(Node::List(kept, bracket, separator), names)
		}
		other => other.map_children(|child| without_type_parameters(child, names)),
	}
}

/// `x: T` of a type parameter T
fn generic_parameter(node: &Node, names: &[String]) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, kind) if matches!(kind.drop_meta(), Node::Symbol(name) if names.contains(name)))
}

/// R's `function(x) x * 2` as an argument, `map(xs, function(x) x * 2)`: the keyword head and the expression after it
fn braceless_lambdas(items: Vec<Node>) -> Vec<Node> {
	let mut merged: Vec<Node> = Vec::new();
	for item in items {
		// Go's function type `f func(int) int` is no lambda: its parameters and result are type words
		let is_body = !matches!(item.drop_meta(), Node::List(_, Bracket::Curly, _)) && !is_type_word(&item);
		let parameters = merged.last().filter(|head| is_body && matches!(head.drop_meta(), Node::List(_, Bracket::Round, _))).and_then(function_parameters);
		match parameters.filter(|parameters| !is_type_word(parameters)) {
			Some(parameters) => {
				merged.pop();
				merged.push(lambda(parameters, item));
			}
			None => merged.push(item),
		}
	}
	merged
}

/// `to_s` of `x.to_s` (or `x.to_s()`): the warp word of the Ruby method (RUBY_METHODS)
fn ruby_method(method: &Node) -> Option<&'static str> {
	let word = match method.drop_meta() {
		Node::Symbol(word) => word.as_str(),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => match items[0].drop_meta() {
			Node::Symbol(word) => word.as_str(),
			_ => return None,
		},
		_ => return None,
	};
	RUBY_METHODS.iter().find(|(ruby, _)| *ruby == word).map(|(_, warp_word)| *warp_word)
}

/// Elixir's one-line `def sq(x), do: x * x`, parsed as `def sq(x)`, `do: x * x`: the definition `sq(x) := x * x`.
/// Its block form `def sq(x) do x * x end` is parsed as `def`, `sq(x) do {x * x}`
fn elixir_definition(items: &[Node]) -> Option<Node> {
	let (keyword, call, body) = match items {
		[head, body] => match (head.drop_meta(), body.drop_meta()) {
			(Node::List(words, _, _), Node::Key(do_word, Op::Colon, body)) if is_word(do_word, ELIXIR_DO_WORD) => match words.as_slice() {
				[keyword, call] => (keyword, call, body.as_ref()),
				_ => return None,
			},
			(_, Node::Key(call, Op::Do, body)) if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => (head, call.as_ref(), body.as_ref()),
			_ => return None,
		},
		_ => return None,
	};
	let is_call = matches!(call.drop_meta(), Node::List(_, Bracket::Round, _));
	let is_keyword = matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word) || word == ELIXIR_PRIVATE_DEF);
	(is_keyword && is_call).then(|| Node::Key(Box::new(call.clone()), Op::Define, Box::new(forms(body.clone()))))
}

/// C++'s `sq = [](int x) { return x * x; }`, parsed as `sq = []`, `(int x)`, `{…}`, the capture list `[]`, `[&]` or
/// `[=]` is the value assigned: the assignment of the lambda `x => {…}` (a warp closure captures what it reads)
/// A declared type before it, `std::function<int(int)> sq = …` or `function<int(int)> sq = …`, is dropped like `auto`
fn cpp_lambda(items: &[Node]) -> Option<Node> {
	let (assignment, parameters, body) = match items {
		[assignment, parameters, body] => (assignment, parameters, body),
		[declared, assignment, parameters, body] if is_cpp_type(declared) => (assignment, parameters, body),
		_ => return None,
	};
	let Node::Key(name, Op::Assign, capture) = assignment.drop_meta() else { return None };
	let is_capture = matches!(capture.drop_meta(), Node::Empty | Node::List(_, Bracket::Square, _));
	let Node::List(declared, Bracket::Round, separator) = parameters.drop_meta() else { return None };
	if !is_capture || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	// `(int x)` is one parameter, `(int a, int b)` several: each its name, the type before it dropped
	let last_word = |parameter: &Node| match parameter.drop_meta() {
		Node::List(words, _, _) => words.last().cloned(),
		_ => Some(parameter.clone()),
	};
	let names: Vec<Node> = match separator {
		Separator::Colon => declared.iter().map(last_word).collect::<Option<_>>()?,
		_ => vec![declared.last()?.clone()],
	};
	let parameters = match names.as_slice() {
		[single] => single.clone(),
		_ => Node::List(names, Bracket::Round, Separator::Colon),
	};
	Some(Node::Key(name.clone(), Op::Assign, Box::new(lambda(parameters, forms(body.clone())))))
}

/// `std::function<int(int)>` (a namespaced name) or `function<int(int)>` (by now the one symbol `function of int(int)`)
fn is_cpp_type(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Scope, _) | Node::Symbol(_))
}

/// `[xs.sum, {block}]` as the parser groups `xs.sum { |x| x * 2 }`: `sum(map(xs, {block}))` (BLOCK_REDUCTIONS)
fn block_reduction(items: &[Node]) -> Option<Node> {
	let [method, block] = items else { return None };
	let Node::Key(subject, Op::Dot, word) = method.drop_meta() else { return None };
	let &(written, reduction, iteration) = BLOCK_REDUCTIONS.iter().find(|(written, ..)| is_word(word, written))?;
	if !matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	if written != reduction {
		crate::normalize::set_position_of(method);
		crate::normalize::hint(&format!(".{written} {{"), &format!(".{reduction} {{"), "warp's word");
	}
	let call = |word: &str, arguments: Vec<Node>| Node::List([vec![Node::Symbol(word.to_string())], arguments].concat(), Bracket::Round, Separator::None);
	Some(call(reduction, vec![call(iteration, vec![subject.as_ref().clone(), block.clone()])]))
}

/// R's `sapply(xs, f)`, PHP's `array_map(f, xs)`: `map(xs, f)`, with a note naming warp's word (FOREIGN_ITERATIONS)
fn foreign_iteration(mut items: Vec<Node>) -> Vec<Node> {
	let Some(&(word, warp_word, function_first, source)) = items.first().and_then(|first| FOREIGN_ITERATIONS.iter().find(|(word, ..)| is_word(first, word))) else { return items };
	if items.len() < 2 {
		return items;
	}
	crate::normalize::set_position_of(&items[0]);
	crate::normalize::hint(&format!("{word}("), &format!("{warp_word}("), &format!("warp's word for {source}"));
	items[0] = Node::Symbol(warp_word.to_string());
	if function_first && items.len() == 3 {
		items.swap(1, 2);
	}
	items
}

/// Whether the program uses `c` other than as R's call `c(…)`: a variable, parameter or function of that name
fn names_vector_word(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => word == R_VECTOR_WORD,
		Node::List(items, Bracket::Round, _) if items.len() > 1 && is_word(&items[0], R_VECTOR_WORD) => items[1..].iter().any(names_vector_word),
		// `def c(x){…}`, `fun c(x) …`, `c(x) {…}`: a function named c
		Node::List(items, _, _) if items.windows(2).any(|pair| is_vector_call(&pair[1]) && is_function_keyword(&pair[0]) || is_vector_call(&pair[0]) && matches!(pair[1].drop_meta(), Node::List(_, Bracket::Curly, _))) => true,
		Node::List(items, _, _) => items.iter().any(names_vector_word),
		Node::Key(target, Op::Assign | Op::Define, _) if matches!(target.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|first| is_word(first, R_VECTOR_WORD))) => true,
		Node::Key(left, _, right) => names_vector_word(left) || names_vector_word(right),
		_ => false,
	}
}

fn is_vector_call(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|first| is_word(first, R_VECTOR_WORD)))
}

fn is_function_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word))
}

/// `list.zip(a, b)`, `math.gcd(4, 6)`, JS's `Math.sqrt(16)`: a module's word called through the module's name is the
/// word itself, with a note (a program's own variable `text` keeps its methods)
fn qualified_module_calls(node: Node) -> Node {
	let modules: Vec<&str> = crate::modules::std_module_names().chain([JS_MATH]).filter(|module| !crate::soft_keywords::program_names(&node, module)).collect();
	module_calls(node, &modules)
}

pub(crate) fn module_calls(node: Node, modules: &[&str]) -> Node {
	match node {
		Node::Key(module, Op::Dot, call) if modules.iter().any(|name| is_word(&module, name)) && matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|word| matches!(word.drop_meta(), Node::Symbol(_)))) => {
			let word = match call.drop_meta() {
				Node::List(items, _, _) => items[0].name(),
				_ => unreachable!("guarded"),
			};
			if let Some((_, _, own_word)) = QUALIFIED_WORDS.iter().find(|(owner, written, _)| is_word(&module, owner) && *written == word) {
				let Node::List(mut items, bracket, separator) = call.drop_meta().clone() else { unreachable!("guarded") };
				items[0] = Node::Symbol(own_word.to_string());
				return Node::List(items.into_iter().map(|item| module_calls(item, modules)).collect(), bracket, separator);
			}
			crate::normalize::hint(&format!("{}.{word}(", module.name()), &format!("{word}("), "warp calls a module's word by its name");
			let operator = crate::warp_parser::PREFIX_OPERATOR_WORDS.iter().find(|(written, _)| *written == word).map(|(_, op)| *op);
			match (operator, call.drop_meta()) {
				// `Math.sqrt(16)`: the parser reads `sqrt(16)` as the operator √
				(Some(op), Node::List(items, _, _)) if items.len() == 2 => {
					let argument = Node::List(vec![module_calls(items[1].clone(), modules)], Bracket::Round, Separator::None);
					Node::Key(Box::new(Node::Empty), op, Box::new(argument))
				}
				_ => module_calls(*call, modules),
			}
		}
		other => other.map_children(|child| module_calls(child, modules)),
	}
}

/// R's `c(1, 2, 3)`: the list `[1, 2, 3]`, with a note
fn r_vectors(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::Round, separator) if items.len() > 1 && is_word(&items[0], R_VECTOR_WORD) => {
			crate::normalize::set_position_of(&items[0]);
			crate::normalize::hint(&format!("{R_VECTOR_WORD}("), "[", "warp writes a list in brackets");
			let elements = items.into_iter().skip(1).map(r_vectors).flat_map(|element| match element {
				Node::List(group, Bracket::Round, Separator::Colon) => group, // `c(1, 2, 3)` holds its arguments as one group
				other => vec![other],
			});
			Node::List(elements.collect(), Bracket::Square, if separator == Separator::None { Separator::Colon } else { separator })
		}
		other => other.map_children(r_vectors),
	}
}

/// `Enum.map(…)`: the iteration word of a module's call
fn module_call(module: &Node, call: &Node) -> Option<&'static str> {
	let Node::List(items, Bracket::Round, _) = call.drop_meta() else { return None };
	let word = items.first()?;
	let is_module = ITERATION_MODULES.iter().any(|name| is_word(module, name));
	(is_module && items.len() > 1).then(|| MODULE_ITERATIONS.iter().find(|name| is_word(word, name)).copied()).flatten()
}

/// F#'s `List.map f xs` (also `Seq.`, `Array.`): `map f xs`, with a note naming warp's word (alias rule)
fn module_qualified_iteration(mut items: Vec<Node>) -> Vec<Node> {
	let Some(Node::Key(module, Op::Dot, word)) = items.first().map(Node::drop_meta) else { return items };
	let is_module = ITERATION_MODULES.iter().any(|name| is_word(module, name));
	let Some(iteration) = MODULE_ITERATIONS.iter().find(|name| is_word(word, name)) else { return items };
	if !is_module || items.len() < 2 {
		return items;
	}
	crate::normalize::set_position_of(&items[0]);
	crate::normalize::hint(&format!("{}.{iteration}", module.serialize()), iteration, "warp's word for the module function");
	items[0] = Node::Symbol(iteration.to_string());
	items
}

/// OCaml/F# `(fun x -> x * 2)` as the parser groups it, the keyword then the arrow: the lambda
fn keyword_arrow(items: &[Node]) -> Option<Node> {
	let [keyword, arrow] = items else { return None };
	let Node::Key(parameters, Op::Arrow, body) = arrow.drop_meta() else { return None };
	// only names: Swift's `func f(x) -> Int { … }` names a function and its result type
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(_));
	let only_names = is_name(parameters) || matches!(parameters.drop_meta(), Node::List(words, Bracket::None, Separator::Space) if words.iter().all(is_name));
	(is_word(keyword, OCAML_FUNCTION_KEYWORD) && only_names).then(|| lambda(parameters.as_ref().clone(), forms(body.as_ref().clone())))
}

/// Go's `func(x int) int { … }`: the body without the result type before it
fn without_result_type(rest: &[Node]) -> &[Node] {
	match rest {
		[result_type, body] if is_type_word(result_type) && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => &rest[1..],
		_ => rest,
	}
}

fn is_type_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if crate::analyzer::type_word_kind(word).is_some())
}

/// R's `f <- function(x) x * 2`, JS-ish `f = function(x) x * 2`: the assignment takes the phrase after the head as
/// the body, `f = x => x * 2`
fn assigned_braceless_function(items: &[Node]) -> Option<Node> {
	let [assignment, body @ ..] = items else { return None };
	let Node::Key(name, op @ (Op::Assign | Op::Define), head) = assignment.drop_meta() else { return None };
	// Lua's `sq = function(x) return x * x end`: the closing word ends the body
	let body = match without_result_type(body) {
		[body @ .., end] if is_word(end, END_WORD) => body,
		body => body,
	};
	let body = match body {
		[] => return None,
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	Some(Node::Key(name.clone(), *op, Box::new(lambda(function_parameters(head)?, forms(body)))))
}

/// JS `({a, b}, k) => …`: an object parameter taken apart, `(object·0, k) => { a = object·0.a; b = object·0.b; … }`
fn destructured_parameters(parameters: &Node) -> Option<(Node, Vec<Node>)> {
	let (items, separator) = match parameters.drop_meta() {
		Node::List(items, Bracket::Round, separator) => (items.clone(), separator.clone()),
		single => (vec![single.clone()], Separator::Colon),
	};
	let mut fields = Vec::new();
	let parameters: Vec<Node> = items.into_iter().enumerate().map(|(index, parameter)| match parameter.drop_meta() {
		Node::List(names, Bracket::Curly, _) if !names.is_empty() && names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) => {
			let object = Node::Symbol(format!("{DESTRUCTURED_OBJECT}{index}"));
			let field = |name: &Node| Node::Key(Box::new(name.clone()), Op::Assign, Box::new(Node::Key(Box::new(object.clone()), Op::Dot, Box::new(name.clone()))));
			fields.extend(names.iter().map(field));
			object
		}
		_ => parameter,
	}).collect();
	(!fields.is_empty()).then(|| (Node::List(parameters, Bracket::Round, separator), fields))
}

/// Ruby's `lambda { |x| x * x }`, `proc { |x| … }`: the block, a lambda itself
fn ruby_lambda(items: &[Node]) -> Option<Node> {
	let [word, block] = items else { return None };
	let is_lambda_word = RUBY_LAMBDA_WORDS.iter().any(|lambda_word| is_word(word, lambda_word));
	(is_lambda_word && matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))).then(|| block.clone())
}

fn starred_lambda(name: &Node, value: &Node) -> Option<Node> {
	let Node::Key(parameters, Op::FatArrow, body) = value.drop_meta() else { return None };
	let parameters = match parameters.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.clone(),
		single => vec![single.clone()],
	};
	let is_starred = |parameter: &Node| matches!(parameter.drop_meta(), Node::Symbol(word) if word.starts_with(crate::tuples::STARRED) && word.len() > 1);
	if !matches!(name.drop_meta(), Node::Symbol(_)) || !parameters.iter().any(is_starred) {
		return None;
	}
	let head = Node::List([vec![name.clone()], parameters].concat(), Bracket::Round, Separator::None);
	Some(Node::Key(Box::new(head), Op::Define, body.clone()))
}

/// C#'s `xs.Sum()`, `xs.Select(x => x * 2)`: the LINQ name, warp's word and the arguments (a called method only:
/// `obj.Count` may be a field)
/// `xs.Select(f)`: warp's `xs.map(f)`, with a note; a method the program defines itself (Go's `Sum()`) stays
fn linq_calls(node: Node, defined: &HashSet<String>) -> Node {
	match node {
		Node::Key(receiver, Op::Dot, method) if linq_method(&method).is_some_and(|(written, _, _)| !defined.contains(written)) => {
			let (written, word, arguments) = linq_method(&method).expect("guarded");
			crate::normalize::set_position_of(&method);
			crate::normalize::hint(&format!(".{written}("), &format!(".{word}("), "warp's word for the LINQ method");
			let arguments = arguments.into_iter().map(|argument| linq_calls(argument, defined));
			let call = Node::List([vec![Node::Symbol(word.to_string())], arguments.collect()].concat(), Bracket::Round, Separator::None);
			Node::Key(Box::new(linq_calls(*receiver, defined)), Op::Dot, Box::new(call))
		}
		other => other.map_children(|child| linq_calls(child, defined)),
	}
}

/// The names the program defines: `f(x) := …`, `def f`, `func f`, Go's method `func (p Point) Sum()`
pub(crate) fn defined_names(node: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| match part {
		Node::Key(head, Op::Define, _) => {
			names.insert(crate::lowering::class_methods::leading_name(head));
		}
		Node::List(words, _, _) if words.first().is_some_and(|keyword| matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word))) => {
			let name = match words.get(1).map(Node::drop_meta) {
				Some(Node::List(_, Bracket::Round, _)) if words.len() > 2 => words.get(2),
				_ => words.get(1),
			};
			names.extend(name.map(crate::lowering::class_methods::leading_name));
		}
		_ => {}
	});
	names
}

fn linq_method(method: &Node) -> Option<(&'static str, &'static str, Vec<Node>)> {
	let Node::List(items, Bracket::Round, _) = method.drop_meta() else { return None };
	let (word, arguments) = items.split_first()?;
	let (written, warp_word) = LINQ_METHODS.iter().find(|(linq, _)| is_word(word, linq))?;
	Some((written, warp_word, arguments.to_vec()))
}

/// `call(3)`, `call 3`: the arguments of Ruby's `.call`
fn called_arguments(call: &Node) -> Option<Vec<Node>> {
	let Node::List(items, _, _) = call.drop_meta() else { return is_word(call, CALL_METHOD).then(Vec::new) };
	let [word, arguments @ ..] = items.as_slice() else { return None };
	is_word(word, CALL_METHOD).then(|| arguments.to_vec())
}

/// `function (a, b)`: the parameters after a function keyword with no name
fn function_parameters(head: &Node) -> Option<Node> {
	let Node::List(head_items, _, _) = head.drop_meta() else { return None };
	let [keyword, parameters @ ..] = head_items.as_slice() else { return None };
	if !matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)) {
		return None;
	}
	match parameters {
		[single] => Some(single.clone()),
		[] => None,
		// Go's `func(x int)`, `func(a, b int)`: the names, the types after them dropped
		several => {
			let names: Vec<Node> = several.iter().filter_map(|parameter| match parameter.drop_meta() {
				Node::Symbol(_) if !is_type_word(parameter) => Some(parameter.clone()),
				Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [name, kind] if is_type_word(kind) && !is_type_word(name)) => Some(words[0].clone()),
				_ => None,
			}).collect();
			match names.as_slice() {
				[single] => Some(single.clone()),
				_ => Some(Node::List(names, Bracket::Round, Separator::Colon)),
			}
		}
	}
}

/// `lambda x` (the words before Python's colon): the parameters
fn lambda_parameters(words: &Node) -> Option<Node> {
	let Node::List(items, Bracket::None, _) = words.drop_meta() else { return None };
	match items.as_slice() {
		[word, parameter] if is_word(word, LAMBDA_WORD) => Some(parameter.clone()),
		_ => None,
	}
}

/// `map xs lambda x: x+1` arrives as the items `…, lambda, x: x+1`: the two are one lambda
fn python_lambdas(items: Vec<Node>) -> Vec<Node> {
	let mut merged: Vec<Node> = Vec::with_capacity(items.len());
	for item in items {
		let follows_lambda = merged.last().is_some_and(|last| is_word(last, LAMBDA_WORD));
		match item.drop_meta() {
			Node::Key(parameters, Op::Colon, body) if follows_lambda => {
				merged.pop();
				merged.push(lambda(parameters.as_ref().clone(), body.as_ref().clone()));
			}
			_ => merged.push(item),
		}
	}
	merged
}
