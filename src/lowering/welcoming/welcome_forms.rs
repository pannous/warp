//! Forms other languages use, lowered to warp's own (notes/welcoming.md), before any other pass:
//! - `match v { 0 => "zero"; _ => "other" }`: the cases of switch/match written with `=>`, `_` the default
//! - `loop { … }`: `while true { … }`, left by `break`
//! - anonymous functions `function(a, b) { … }`, `fn(x) { … }` (JS, Rust-ish), `f <- function(x) x * 2` (R) and
//!   `lambda x: …` (Python): lambdas
//!   (`xs |> f(b)` is read by the parser, `f(1, _)` lowered in declarations.rs)
//! - OCaml / F# `let f x = body in rest`: the definition `f(x) := body`, then rest
//! - JS destructured parameters `({a, b}) => a + b`: the object taken apart into its fields

use super::nodes::{call, children_rewritten, is_kinded_type_word, key};
use crate::lowering::signals::event_signals::PAGE_EVENTS;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::GENERIC_MARK;
use crate::words::IMPLICIT_PARAMETER;
use crate::words::IS_WORD;
use crate::words::LET_WORD;
use crate::words::{EVENT_ALIASES, ONCE_WORD, ON_WORD, WHENEVER_WORD};
use std::collections::HashSet;

use crate::words::{MATCH_WORD, SWITCH_WORDS};
const WILDCARD: &str = "_";
const SET_WORD: &str = "set";
const SET_PROSE_REASON: &str = "warp doesn't do prose, write the assignment";
const DEFAULT_CASE: &str = "default";
const LOOP_KEYWORD: &str = "loop";
/// Lua's, Ruby's and Julia's word closing a function body
const END_WORD: &str = "end";
const LAMBDA_WORD: &str = "lambda";
/// OCaml's and F#'s lambda keyword, dropped (decision foreign-spellings); Kotlin's `fun f(x) = …` stays
const OCAML_FUN: &str = "fun";
const VAR_WORD: &str = "var";
const DESTRUCTURED_OBJECT: &str = "object·";
const RUBY_LAMBDA_WORDS: [&str; 2] = ["lambda", "proc"];
const CALL_METHOD: &str = "call";
/// Swift's and C's case label `case .north:` in a switch
const CASE_WORD: &str = "case";
/// Kotlin's `when (x) { a -> 1; is T -> 2; else -> 3 }`, an if chain; `when` means on, whenever, if or match by its
/// shape, and every shape warns the same (user, 2026-10-13)
const WHEN_KEYWORD: &str = "when";
const WHEN_TOPIC: &str = "when";
const WHEN_REASON: &str = "when is on, whenever, if or match by its shape; write the one meant";
const ELSE_WORD: &str = "else";
/// The listeners a `when` without arms is: of an event, of a condition
const IF_WORD: &str = "if";
/// The if chain a `when` arm adds, its conditions joined by `or`
const WHEN_ARM: &str = "if CONDITION then VALUE else OTHERWISE";
const LAST_WHEN_ARM: &str = "if CONDITION then VALUE";
/// Modules of other languages whose words are warp's: JS's `Math.sqrt(16)` is `sqrt(16)`, F#'s `List.map f xs` is
/// `map f xs` (also `Seq.`, `Array.`), Elixir's `Enum.map(xs, f)` is `map(xs, f)`
const FOREIGN_MODULES: [&str; 5] = ["Math", "List", "Seq", "Array", "Enum"];
/// Module words written only qualified, as the module's own name for them (P183: `file.append(path, text)`; a bare
/// `append` stays the list method)
const QUALIFIED_WORDS: [(&str, &str, &str); 1] = [("file", "append", "append_file")];
/// Ruby's `xs.sum { |x| x * 2 }`, `xs.count { it > 1 }`: a reduction of the list
/// the block makes, `sum(map(xs, block))`, `count(filter(xs, block))`
const BLOCK_REDUCTIONS: [(&str, &str); 2] = [("sum", "map"), ("count", "filter")];
/// C++'s `auto x = 3`
const CPP_AUTO_WORD: &str = "auto";
/// Other languages' method names for warp's words (the alias rule: they work, with a note naming warp's word): C#'s
/// LINQ, Ruby's conversions and cases, Kotlin's `xs.sumOf {…}`
const METHOD_ALIASES: [(&str, &str); 18] = [
	("Select", "map"),
	("Where", "filter"),
	("Aggregate", "reduce"),
	("Sum", "sum"),
	("Count", "count"),
	("Max", "max"),
	("Min", "min"),
	("Any", "any"),
	("All", "all"),
	("First", "first"),
	("Last", "last"),
	("Contains", "contains"),
	("to_s", "string"),
	("to_i", "int"),
	("to_f", "float"),
	("upcase", "upper"),
	("downcase", "lower"),
	("sumOf", "sum"),
];

pub fn lower(node: Node) -> Node {
	if let Some(error) = warn_assigned_it(&node) {
		return error;
	}
	// the program's own names keep their meaning: a field `Count`, Go's method `Sum()`
	let own: HashSet<String> = crate::library_words::defined_names(&node)
		.into_iter()
		.chain(crate::soft_keywords::names_given(&node))
		.chain(type_fields(&node))
		.collect();
	let node = qualified_module_calls(module_aliases(node));
	let node = event_aliases(forms(method_aliases(node, &own)), &own);
	let mut lambda_names = HashSet::new();
	collect_lambda_names(&node, &mut lambda_names);
	lambda_calls(node, &lambda_names)
}

/// `it = 5` (user 2026-10-16): `it` is reserved, it names the parameter of a function that names none, so a variable
/// of that name warns (an error under `use strict`)
fn warn_assigned_it(node: &Node) -> Option<Node> {
	let mut assigned = None;
	node.visit(&mut |node| {
		if let Node::Key(target, Op::Assign | Op::Define, _) = node {
			if assigned.is_none() && target.drop_meta().is_symbol(IMPLICIT_PARAMETER) {
				assigned = Some(target.as_ref().clone());
			}
		}
	});
	let message = format!(
		"`{IMPLICIT_PARAMETER}` is reserved: it names the parameter of a function that names none \
		 (`square := {IMPLICIT_PARAMETER} * {IMPLICIT_PARAMETER}`); pick another name"
	);
	crate::errors::diagnostic::report(&[crate::errors::diagnostic::Diagnostic::at(&assigned?, message)]).err()
}

/// Swift's and Rust's result arrow before `:=`: `def x(p:P) -> int := p.n` is `x(p:P): int := p.n`, the one typed head
/// every pass reads (card undefined-function: it was read as a lambda, `undefined function: x`)
/// `set x to 5` is `x = 5` for any assignment target, with a style hint (decision setter-calls); after of_lookups
/// `set width of canvas to 12` is `set canvas.width to 12`, so `canvas.width = 12`. A program's own `set` wins
pub fn lower_set_phrases(node: Node) -> Node {
	if !node.mentions_any(&[SET_WORD]) || crate::library_words::defined_names(&node).contains(SET_WORD) {
		return node;
	}
	set_phrases(node)
}

fn set_phrases(node: Node) -> Node {
	match set_assignment(&node) {
		Some(assignment) => set_phrases(assignment),
		None => node.map_children(set_phrases),
	}
}

/// The assignment a phrase `set target to value` says, with its style hint
fn set_assignment(node: &Node) -> Option<Node> {
	let Node::List(items, Bracket::None, Separator::Space) = node else { return None };
	let [set, phrase] = items.as_slice() else { return None };
	let Node::Key(target, Op::To, value) = phrase.drop_meta() else { return None };
	if !set.is_symbol(SET_WORD) || !is_assignable(target) {
		return None;
	}
	let assignment = key(target.as_ref().clone(), Op::Assign, value.as_ref().clone());
	crate::normalize::set_position_of(set);
	let written = format!("{SET_WORD} {} to {}", target.serialize(), value.serialize());
	crate::normalize::hint(&written, &assignment.serialize(), SET_PROSE_REASON);
	Some(assignment)
}

/// A variable or a field `p.width`
fn is_assignable(target: &Node) -> bool {
	matches!(target.drop_meta(), Node::Symbol(_) | Node::Key(_, Op::Dot, _))
}

pub fn lower_arrow_result_types(node: Node) -> Node {
	match node {
		Node::Key(target, op @ (Op::Define | Op::Assign), body) => {
			let target = match *target {
				Node::Key(head, Op::Arrow, result) if is_named_head(&head) => key(*head, Op::Colon, *result),
				other => lower_arrow_result_types(other),
			};
			key(target, op, lower_arrow_result_types(*body))
		}
		other => other.map_children(lower_arrow_result_types),
	}
}

/// `x(p:P)`: a function's name and parameters in round brackets
fn is_named_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|name| matches!(name.drop_meta(), Node::Symbol(_))))
}

/// The names assigned a lambda: `f = x => …`, `f = { |x| … }`
fn collect_lambda_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(name, Op::Assign, value)
			if matches!(name.drop_meta(), Node::Symbol(_)) && crate::lambdas::arrow_lambda(value).is_some() =>
		{
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
		Node::Key(function, Op::Dot, call)
			if lambda_names.contains(&function.name()) && called_arguments(&call).is_some() =>
		{
			let arguments = called_arguments(&call)
				.expect("guarded")
				.into_iter()
				.map(|argument| lambda_calls(argument, lambda_names));
			parens([*function].into_iter().chain(arguments).collect())
		}
		other => other.map_children(|child| lambda_calls(child, lambda_names)),
	}
}

fn forms(node: Node) -> Node {
	if let Some(error) = ocaml_fun(&node) {
		return error;
	}
	match node {
		Node::List(items, bracket, separator) if generic_names(&items).is_some() => {
			let (function, names) = generic_names(&items).expect("guarded");
			crate::normalize::set_position_of(&items[0]);
			crate::normalize::hint(
				&format!("{function}<{}>", names.join(", ")),
				&function,
				"warp infers types: write the function without type parameters",
			);
			forms(without_type_parameters(Node::List(items, bracket, separator), &names))
		}
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = python_lambdas(braceless_lambdas(items.into_iter().map(forms).collect()));
			if let Some(lambda) = anonymous_function(&items)
				.or_else(|| assigned_braceless_function(&items))
				.or_else(|| ruby_lambda(&items))
			{
				return lambda;
			}
			if let Some(reduction) = block_reduction(&items) {
				return reduction;
			}
			// C++'s `auto x = 3`: `var x = 3`
			if items.len() > 1
				&& items[0].is_symbol(CPP_AUTO_WORD)
				&& matches!(items[1].drop_meta(), Node::Key(_, Op::Assign, _))
			{
				crate::normalize::set_position_of(&items[0]);
				crate::errors::diagnostic::note_alias(CPP_AUTO_WORD, VAR_WORD);
				return var_declaration(Node::single_or_list(items[1..].to_vec(), bracket, separator));
			}
			if let Some(variable) = let_mut(&items) {
				return variable;
			}
			let items = go_destructuring(items, &separator);
			if let Some(chain) = when_chain(&items) {
				return chain;
			}
			if let Some(listener) = when_listener(&items) {
				return Node::List(listener, bracket, separator);
			}
			let endless = match items.as_slice() {
				[word, body] => endless_loop(word, body),
				_ => None,
			};
			endless.unwrap_or_else(|| Node::List(items, bracket, separator))
		}
		// Ruby's `loop do … end`
		Node::Key(word, Op::Do, body) if endless_loop(&word, &body).is_some() => {
			endless_loop(&word, &forms(*body)).expect("guarded")
		}
		Node::Key(left, Op::Colon, body) if lambda_parameters(&left).is_some() => {
			lambda(lambda_parameters(&left).expect("guarded"), forms(*body))
		}
		Node::Key(parameters, Op::Arrow, body) if destructured_parameters(&parameters).is_some() => {
			let (parameters, fields) = destructured_parameters(&parameters).expect("guarded");
			lambda(parameters, Node::List([fields, vec![forms(*body)]].concat(), Bracket::Curly, Separator::Semicolon))
		}
		// `f = lambda *xs: …`, JS `f = (...xs) => …`: the definition `f(*xs) := …`, which variadic.rs reads
		Node::Key(name, Op::Assign, value) if starred_lambda(&name, &value).is_some() => {
			forms(starred_lambda(&name, &value).expect("guarded"))
		}
		other => children_rewritten(other, forms),
	}
}

/// Card fun-x-loud: OCaml's `fun x -> …` (`fun (x -> …)` as parsed) and `fun a b -> …` were dropped (decision
/// foreign-spellings), so they are an error naming warp's lambda instead of running as something else
fn ocaml_fun(node: &Node) -> Option<Node> {
	let names = match node.drop_meta() {
		Node::Key(head, Op::Arrow, _) => {
			let mut words = vec![];
			crate::lambdas::flatten_phrase(head, &mut words);
			match words.split_first() {
				Some((keyword, names)) if keyword.is_symbol(OCAML_FUN) => names.to_vec(),
				_ => return None,
			}
		}
		Node::List(items, _, _) => {
			let arrow = items.windows(2).find(|pair| pair[0].is_symbol(OCAML_FUN))?.get(1)?;
			let Node::Key(head, Op::Arrow, _) = arrow.drop_meta() else {
				return None;
			};
			let mut words = vec![];
			crate::lambdas::flatten_phrase(head, &mut words);
			words
		}
		_ => return None,
	};
	if names.is_empty() || !names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) {
		return None;
	}
	let names: Vec<String> = names.iter().map(Node::name).collect();
	let parameters = match names.as_slice() {
		[single] => single.clone(),
		many => format!("({})", many.join(", ")),
	};
	let message = format!("OCaml's `{OCAML_FUN} {} -> …` is not warp: write `{parameters} -> …`", names.join(" "));
	Some(crate::errors::diagnostic::Diagnostic::at(node, message).fix(format!("{parameters} -> …")).into_error())
}

/// Rust's `let mut x = 1`: `var x = 1`, with a note naming var
fn let_mut(items: &[Node]) -> Option<Node> {
	let [keyword, modifier, declaration] = items else {
		return None;
	};
	let Node::Key(name, Op::Assign, _) = declaration.drop_meta() else {
		return None;
	};
	let modifier = modifier.symbol_name().filter(|word| crate::words::MUTABLE_WORDS.contains(word))?;
	if !keyword.is_symbol(LET_WORD) {
		return None;
	}
	let name = name.serialize();
	crate::normalize::set_position_of(keyword);
	crate::errors::diagnostic::educate_once(
		&format!("{}let {modifier}", crate::errors::diagnostic::ALIAS_TOPIC_PREFIX),
		&format!("let {modifier} {name}"),
		&format!("var {name}"),
		&format!("write var {name}, not let {modifier} {name}"),
	);
	Some(var_declaration(declaration.clone()))
}

/// `var x = 1`
fn var_declaration(declaration: Node) -> Node {
	Node::List(vec![symbol(VAR_WORD), forms(declaration)], Bracket::None, Separator::None)
}

/// Go's `q, r := f(x)`: names taking the values apart, `q, r = f(x)` (`r := …` alone would define a getter)
fn go_destructuring(mut items: Vec<Node>, separator: &Separator) -> Vec<Node> {
	let Some(position) = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(_, Op::Define, _))) else {
		return items;
	};
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(_));
	let Node::Key(name, Op::Define, value) = items[position].drop_meta() else {
		return items;
	};
	if *separator != Separator::Colon || position == 0 || !is_name(name) || !items[..position].iter().all(is_name) {
		return items;
	}
	items[position] = Node::Key(name.clone(), Op::Assign, value.clone());
	items
}

/// Every match's arms in the one form the later passes read, right after parsing: `[match, subject, {k => v …}]` (or
/// `[match, (subject {k => v …})]` as an assigned match parses) with the cases as `k: v`, `_ => v` as `default: v`
pub fn lower_match_arms(node: Node) -> Node {
	colon_arms(named_switch_subjects(node))
}

fn colon_arms(node: Node) -> Node {
	match node.map_children(colon_arms) {
		Node::List(items, bracket, separator) => Node::List(with_match_cases(items, colon_cases), bracket, separator),
		other => other,
	}
}

fn is_switch_word(word: &Node) -> bool {
	SWITCH_WORDS.iter().any(|switch| word.is_symbol(switch))
}

/// `match alle = f(3) {…}` names its subject (user, card match-binding-arm) but parses as `match (alle = (f(3) {…}))`,
/// after `r =` as `r = ((match alle) = (f(3) {…}))`: as `match (alle = f(3)) {…}`, the assignment the subject
fn named_switch_subjects(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let mut result: Vec<Node> = Vec::with_capacity(items.len() + 1);
			for item in items.into_iter().map(named_switch_subjects) {
				match named_subject(&item).filter(|_| result.last().is_some_and(is_switch_word)) {
					Some((subject, cases)) => result.extend([subject, cases]),
					None => result.push(item),
				}
			}
			Node::List(result, bracket, separator)
		}
		Node::Key(left, Op::Assign, right) => match left.drop_meta() {
			Node::List(words, _, _) if words.len() == 2 && is_switch_word(&words[0]) => {
				named_switch_subjects(Node::List(
					vec![words[0].clone(), key(words[1].clone(), Op::Assign, *right)],
					Bracket::None,
					Separator::Space,
				))
			}
			_ => key(named_switch_subjects(*left), Op::Assign, named_switch_subjects(*right)),
		},
		other => children_rewritten(other, named_switch_subjects),
	}
}

/// `alle = (f(3) {cases})`: the assignment `alle = f(3)` and the case block
fn named_subject(item: &Node) -> Option<(Node, Node)> {
	let Node::Key(name, Op::Assign, value) = item.drop_meta() else {
		return None;
	};
	let Node::List(parts, bracket, separator) = value.drop_meta() else {
		return None;
	};
	let (cases, expression) = parts.split_last()?;
	if !matches!(name.drop_meta(), Node::Symbol(_)) || !super::nodes::is_block(cases) || expression.is_empty() {
		return None;
	}
	let expression = match expression {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), bracket.clone(), separator.clone()),
	};
	Some((key(name.as_ref().clone(), Op::Assign, expression), cases.clone()))
}

/// The items of a match, `[match, subject, cases]`, `[match, (subject cases)]` or, as a body or an arm parses it,
/// `[(match subject), cases]`, with `convert` of its cases block; other items as they are
pub(crate) fn with_match_cases(mut items: Vec<Node>, convert: impl Fn(&Node) -> Node) -> Vec<Node> {
	if let [head, cases] = items.as_slice() {
		if matches!(head.drop_meta(), Node::List(words, _, _) if words.len() == 2 && is_switch_word(&words[0])) {
			items[1] = convert(cases);
			return items;
		}
	}
	if !items.first().is_some_and(is_switch_word) {
		return items;
	}
	match items.len() {
		3 => items[2] = convert(&items[2]),
		2 => {
			if let Node::List(pair, bracket, separator) = items[1].drop_meta() {
				if let [subject, cases] = pair.as_slice() {
					items[1] = Node::List(vec![subject.clone(), convert(cases)], bracket.clone(), separator.clone());
				}
			}
		}
		_ => {}
	}
	items
}

fn colon_cases(block: &Node) -> Node {
	let Node::List(cases, Bracket::Curly, separator) = block.drop_meta() else {
		return block.clone();
	};
	Node::List(cases.iter().map(colon_case).collect(), Bracket::Curly, separator.clone())
}

fn colon_case(case: &Node) -> Node {
	match case.drop_meta() {
		// `1, 2 => …`: the alternatives and the case
		Node::List(items, Bracket::None, Separator::Colon) => {
			Node::List(items.iter().map(colon_case).collect(), Bracket::None, Separator::Colon)
		}
		// Swift's `case .north: 1` (read as the member `case.north`) and `case Direction.north: 1`
		Node::Key(label, Op::Colon, body) if matches!(label.drop_meta(), Node::Key(word, Op::Dot, _) if word.is_symbol(CASE_WORD)) =>
		{
			let Node::Key(_, _, member) = label.drop_meta() else { unreachable!("guarded") };
			Node::Key(Box::new(Node::Key(Box::new(Node::Empty), Op::Dot, member.clone())), Op::Colon, body.clone())
		}
		Node::List(words, _, _) if matches!(words.as_slice(), [word, _] if word.is_symbol(CASE_WORD)) => {
			words[1].clone()
		}
		// Kotlin's arms `1 -> a; else -> b` too
		Node::Key(pattern, Op::Arrow, body) => {
			let pattern = if pattern.is_symbol(WILDCARD) || pattern.is_symbol(ELSE_WORD) {
				symbol(DEFAULT_CASE)
			} else {
				pattern.as_ref().clone()
			};
			Node::Key(Box::new(pattern), Op::Colon, body.clone())
		}
		_ => case.clone(),
	}
}

/// Kotlin's `when (x) { 1, 2 -> a; is Circle -> b; else -> c }` (or `when { x > 0 -> a … }` without a subject): the
/// chain `if x == 1 or x == 2 then a else if x is Circle then b else c`
fn when_chain(items: &[Node]) -> Option<Node> {
	// `= when (s) {…}` after a definition arrives as the group `when (s)` and the arms
	if let [head, arms] = items {
		if let Node::List(words, _, Separator::Space) = head.drop_meta() {
			if words.len() == 2 && words[0].is_symbol(WHEN_KEYWORD) {
				return when_chain(&[words[0].clone(), words[1].clone(), arms.clone()]);
			}
		}
	}
	let (subject, arms) = match items {
		[word, subject, arms] if word.is_symbol(WHEN_KEYWORD) => (
			Some(match subject.drop_meta() {
				Node::List(inner, Bracket::Round, _) if inner.len() == 1 => inner[0].clone(),
				_ => subject.clone(),
			}),
			arms,
		),
		[word, arms] if word.is_symbol(WHEN_KEYWORD) => (None, arms),
		_ => return None,
	};
	let Node::List(arms, Bracket::Curly, _) = arms.drop_meta() else {
		return None;
	};
	let arms: Vec<(Vec<Node>, Node)> = arms.iter().map(when_arm).collect::<Option<_>>()?;
	when_meant(&items[0], if subject.is_some() { MATCH_WORD } else { IF_WORD });
	let condition = |pattern: Node| match (&subject, pattern) {
		(_, pattern) if pattern.is_symbol(ELSE_WORD) => Node::True,
		(Some(subject), pattern) => key(subject.clone(), Op::Eq, pattern),
		(None, pattern) => pattern,
	};
	arms.into_iter().rev().fold(None, |otherwise: Option<Node>, (patterns, value)| {
		let condition = patterns.into_iter().map(condition).reduce(|left, right| key(left, Op::Or, right))?;
		let template = if otherwise.is_some() { WHEN_ARM } else { LAST_WHEN_ARM };
		let bindings =
			[("CONDITION", condition), ("VALUE", forms(value)), ("OTHERWISE", otherwise.unwrap_or(Node::Empty))];
		Some(super::nodes::filled(template, bindings))
	})
}

/// `when click {…}` is `on click {…}`, `when x > 3 {…}` is `whenever x > 3 {…}` (card signals-shape, user 2026-10-08):
/// a `when` whose block holds no `->` arms listens, to the event a bare word names or to the condition otherwise
fn when_listener(items: &[Node]) -> Option<Vec<Node>> {
	let [word, subject, body] = items else { return None };
	let Node::List(statements, Bracket::Curly, _) = body.drop_meta() else {
		return None;
	};
	let has_arms = statements.iter().any(|statement| matches!(statement.drop_meta(), Node::Key(_, Op::Arrow, _)));
	if !word.is_symbol(WHEN_KEYWORD) || has_arms {
		return None;
	}
	let listener = if matches!(subject.drop_meta(), Node::Symbol(_)) { ON_WORD } else { WHENEVER_WORD };
	when_meant(word, listener);
	Some(vec![symbol(listener), subject.clone(), body.clone()])
}

/// `on keydown {…}`, `once keypress {…}`, `when keyup {…}` (after forms made it `on`): the listener of the warp event,
/// with the alias note; the DOM attribute `onclick {…}`, `onkeydown {…}` is `on click {…}`, `on key {…}` unless the
/// program defines that name
fn event_aliases(node: Node, own: &HashSet<String>) -> Node {
	match node.map_children(|child| event_aliases(child, own)) {
		Node::List(items, bracket, separator) => {
			let items = aliased_event(&items, own).unwrap_or(items);
			Node::List(items, bracket, separator)
		}
		other => other,
	}
}

fn aliased_event(items: &[Node], own: &HashSet<String>) -> Option<Vec<Node>> {
	if let [attribute, body] = items {
		if let Some((written, event, released)) =
			attribute_event(attribute).filter(|(written, ..)| !own.contains(written))
		{
			if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
				note_event_alias(attribute, &written, &format!("{ON_WORD} {event}"), released);
				return Some(vec![symbol(ON_WORD), symbol(event), body.clone()]);
			}
		}
	}
	let [listener, event, rest @ ..] = items else { return None };
	if !listener.is_symbol(ON_WORD) && !listener.is_symbol(ONCE_WORD) {
		return None;
	}
	let name = event.symbol_name()?;
	let (meant, released) = aliased_event_name(name)?;
	note_event_alias(event, name, meant, released);
	Some([vec![listener.clone(), symbol(meant)], rest.to_vec()].concat())
}

/// The got-it note of an event alias; a release (`keyup`) says the warp event fires on the press
fn note_event_alias(at: &Node, written: &str, meant: &str, released: bool) {
	use crate::errors::diagnostic::{educate_once, note_alias, ALIAS_TOPIC_PREFIX};
	crate::normalize::set_position_of(at);
	if released {
		let reason = format!("warp says {meant}, which fires on the press: there is no release event");
		educate_once(&format!("{ALIAS_TOPIC_PREFIX}{written}"), written, meant, &reason);
	} else {
		note_alias(written, meant);
	}
}

fn aliased_event_name(name: &str) -> Option<(&'static str, bool)> {
	EVENT_ALIASES.iter().find(|(alias, ..)| *alias == name).map(|(_, meant, released)| (*meant, *released))
}

/// `onclick`: the written word, the warp event its DOM attribute name stands for, whether it names a release
fn attribute_event(word: &Node) -> Option<(String, &'static str, bool)> {
	let written = word.symbol_name()?;
	let name = written.strip_prefix(ON_WORD)?;
	let (event, released) = match PAGE_EVENTS.iter().find(|event| **event == name) {
		Some(event) => (*event, false),
		None => aliased_event_name(name)?,
	};
	Some((written.to_string(), event, released))
}

/// The one warning of every `when`, its fix the word this shape means
fn when_meant(word: &Node, meant: &str) {
	crate::normalize::set_position_of(word);
	crate::errors::diagnostic::educate_once(WHEN_TOPIC, WHEN_KEYWORD, meant, WHEN_REASON);
}

/// The patterns and value of a `when` arm: `5 -> 50`, `1, 2 -> 10`, `is Circle -> 3`, `else -> 0`
fn when_arm(arm: &Node) -> Option<(Vec<Node>, Node)> {
	match arm.drop_meta() {
		Node::Key(pattern, Op::Arrow, value) => Some((vec![pattern.as_ref().clone()], value.as_ref().clone())),
		// `is Circle -> 3`: the type test `x is Circle`, as `is` reads
		Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [word, _] if word.is_symbol(IS_WORD)) => {
			when_arm(&words[1])
		}
		Node::List(patterns, _, _) if !patterns.is_empty() => {
			let (last, first) = patterns.split_last()?;
			let (mut all, value) = when_arm(last)?;
			all.splice(0..0, first.iter().cloned());
			Some((all, value))
		}
		_ => None,
	}
}

/// `loop { body }`, Ruby's `loop do body end`
fn endless_loop(word: &Node, body: &Node) -> Option<Node> {
	if !word.is_symbol(LOOP_KEYWORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let condition = key(Node::Empty, Op::While, Node::True);
	Some(key(condition, Op::Do, body.clone()))
}

fn lambda(parameters: Node, body: Node) -> Node {
	key(parameters, Op::Arrow, body)
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
	let is_type_parameter = |node: &Node| mentions_type_parameter(node, names);
	match node {
		Node::Meta { node, data } if matches!(data.as_ref(), Node::Key(mark, _, _) if mark.name() == GENERIC_MARK) => {
			without_type_parameters(*node, names)
		}
		// `head -> T { body }`, `head: T { body }`: the definition `(head { body })`
		Node::Key(head, Op::Arrow | Op::Colon, typed_body) if matches!(typed_body.drop_meta(), Node::List(parts, Bracket::None, Separator::Space) if parts.len() == 2 && is_type_parameter(&parts[0])) =>
		{
			let Node::List(parts, _, _) = typed_body.drop_meta() else { unreachable!("guarded") };
			parens(vec![without_type_parameters(*head, names), without_type_parameters(parts[1].clone(), names)])
		}
		// `x: T`, `xs: Vec<T>`: the parameter x
		Node::Key(name, Op::Colon, kind) if is_type_parameter(&kind) => without_type_parameters(*name, names),
		// `head -> (A, B)` before its block: the head
		Node::Key(head, Op::Arrow, kind)
			if matches!(kind.drop_meta(), Node::List(_, Bracket::Round, _)) && is_type_parameter(&kind) =>
		{
			without_type_parameters(*head, names)
		}
		// Swift's `_ x: T`: without its type `_ x` would read as a placeholder `_` (partial application), so the label goes too
		Node::List(items, bracket, separator)
			if items.windows(2).any(|pair| pair[0].is_symbol(WILDCARD) && generic_parameter(&pair[1], names)) =>
		{
			let mut kept: Vec<Node> = Vec::new();
			for item in items {
				if generic_parameter(&item, names) && kept.last().is_some_and(|label| label.is_symbol(WILDCARD)) {
					kept.pop();
					let Node::Key(name, _, _) = item.drop_meta() else { unreachable!("a generic parameter") };
					crate::normalize::set_position_of(&item);
					crate::normalize::hint(
						&format!("{WILDCARD} {}", item.serialize()),
						&name.serialize(),
						"warp names a parameter once, no label",
					);
				}
				kept.push(item);
			}
			without_type_parameters(Node::List(kept, bracket, separator), names)
		}
		other => other.map_children(|child| without_type_parameters(child, names)),
	}
}

/// `T`, `Vec<T>` (the parser's symbol `Vec of T`), `[T]` of a type parameter T
fn mentions_type_parameter(kind: &Node, names: &[String]) -> bool {
	kind.mentions_where(|words| words.split_whitespace().any(|word| names.iter().any(|name| name == word)))
}

/// `x: T`, `xs: Vec<T>` of a type parameter T
fn generic_parameter(node: &Node, names: &[String]) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, kind) if mentions_type_parameter(kind, names))
}

/// R's `function(x) x * 2` as an argument, `map(xs, function(x) x * 2)`: the keyword head and the expression after it
fn braceless_lambdas(items: Vec<Node>) -> Vec<Node> {
	let mut merged: Vec<Node> = Vec::new();
	for item in items {
		// Go's function type `f func(int) int` is no lambda: its parameters and result are type words
		let is_body = !matches!(item.drop_meta(), Node::List(_, Bracket::Curly, _)) && !is_kinded_type_word(&item);
		let parameters = merged
			.last()
			.filter(|head| is_body && matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)))
			.and_then(function_parameters);
		match parameters.filter(|parameters| !is_kinded_type_word(parameters)) {
			Some(parameters) => {
				merged.pop();
				merged.push(lambda(parameters, item));
			}
			None => merged.push(item),
		}
	}
	merged
}

/// `[xs.sum, {block}]` as the parser groups `xs.sum { |x| x * 2 }`: `sum(map(xs, {block}))` (BLOCK_REDUCTIONS)
fn block_reduction(items: &[Node]) -> Option<Node> {
	let [method, block] = items else { return None };
	let Node::Key(subject, Op::Dot, word) = method.drop_meta() else {
		return None;
	};
	let &(reduction, iteration) = BLOCK_REDUCTIONS.iter().find(|(reduction, _)| word.is_symbol(reduction))?;
	if !matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let call = |word: &str, arguments: Vec<Node>| call(word, arguments);
	Some(call(reduction, vec![call(iteration, vec![subject.as_ref().clone(), block.clone()])]))
}

/// `list.zip(a, b)`, `math.gcd(4, 6)`, JS's `Math.sqrt(16)`: a module's word called through the module's name is the
/// word itself, with a note (a program's own variable `text` keeps its methods)
fn qualified_module_calls(node: Node) -> Node {
	let names = crate::soft_keywords::names_given(&node);
	let modules: Vec<&str> =
		crate::modules::std_module_names().chain(FOREIGN_MODULES).filter(|module| !names.contains(*module)).collect();
	module_calls(node, &modules)
}

/// `use list as l; l.unique(xs)`: a standard or warp module under the program's name for it is the module, a word
/// called through that name the word itself, without the note qualified_module_calls gives
fn module_aliases(node: Node) -> Node {
	let mut aliases = vec![];
	let node = unaliased_uses(node, &mut aliases);
	let aliases: Vec<&str> = aliases.iter().map(String::as_str).collect();
	match aliases.is_empty() {
		true => node,
		false => crate::normalize::without_hints(|| module_calls(node, &aliases)),
	}
}

/// `use list as l` → `use list`, collecting the alias l
fn unaliased_uses(node: Node, aliases: &mut Vec<String>) -> Node {
	if let Node::List(items, bracket, separator) = node.drop_meta() {
		if let [keyword, used] = items.as_slice() {
			if let Some((module, alias)) = aliased_module(keyword, used) {
				aliases.push(alias);
				return Node::List(vec![keyword.clone(), module], bracket.clone(), separator.clone());
			}
		}
	}
	node.map_children(|child| unaliased_uses(child, aliases))
}

fn aliased_module(keyword: &Node, used: &Node) -> Option<(Node, String)> {
	let Node::Key(module, Op::As, alias) = used.drop_meta() else {
		return None;
	};
	let is_use =
		matches!(keyword.drop_meta(), Node::Symbol(word) if crate::modules::USE_KEYWORDS.contains(&word.as_str()));
	let alias = alias.symbol_name()?;
	(is_use && crate::modules::names_a_module(&crate::modules::path_of(module)?))
		.then(|| (module.as_ref().clone(), alias.to_string()))
}

pub(crate) fn module_calls(node: Node, modules: &[&str]) -> Node {
	match node {
		// F#'s `List.map f xs`: the word applied to the words after it
		Node::List(mut items, Bracket::None, Separator::Space)
			if items.len() > 1 && qualified_word(&items[0], modules).is_some() =>
		{
			let word = qualified_word(&items[0], modules).expect("guarded");
			crate::normalize::set_position_of(&items[0]);
			crate::normalize::hint(&items[0].serialize(), &word, "warp calls a module's word by its name");
			items[0] = symbol(&word);
			Node::List(
				items.into_iter().map(|item| module_calls(item, modules)).collect(),
				Bracket::None,
				Separator::Space,
			)
		}
		Node::Key(module, Op::Dot, call)
			if modules.iter().any(|name| module.is_symbol(name))
				&& matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|word| matches!(word.drop_meta(), Node::Symbol(_)))) =>
		{
			let word = match call.drop_meta() {
				Node::List(items, _, _) => items[0].name(),
				_ => unreachable!("guarded"),
			};
			if let Some((_, _, own_word)) =
				QUALIFIED_WORDS.iter().find(|(owner, written, _)| module.is_symbol(owner) && *written == word)
			{
				let Node::List(mut items, bracket, separator) = call.drop_meta().clone() else {
					unreachable!("guarded")
				};
				items[0] = symbol(own_word);
				return Node::List(
					items.into_iter().map(|item| module_calls(item, modules)).collect(),
					bracket,
					separator,
				);
			}
			crate::normalize::hint(
				&format!("{}.{word}(", module.name()),
				&format!("{word}("),
				"warp calls a module's word by its name",
			);
			let operator = crate::warp_parser::PREFIX_OPERATOR_WORDS
				.iter()
				.find(|(written, _)| *written == word)
				.map(|(_, op)| *op);
			match (operator, call.drop_meta()) {
				// `Math.sqrt(16)`: the parser reads `sqrt(16)` as the operator √
				(Some(op), Node::List(items, _, _)) if items.len() == 2 => {
					let argument = parens(vec![module_calls(items[1].clone(), modules)]);
					key(Node::Empty, op, argument)
				}
				_ => module_calls(*call, modules),
			}
		}
		other => other.map_children(|child| module_calls(child, modules)),
	}
}

/// Go's `func(x int) int { … }`: the body without the result type before it
fn without_result_type(rest: &[Node]) -> &[Node] {
	match rest {
		[result_type, body]
			if is_kinded_type_word(result_type) && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) =>
		{
			&rest[1..]
		}
		_ => rest,
	}
}

/// R's `f <- function(x) x * 2`, JS-ish `f = function(x) x * 2`: the assignment takes the phrase after the head as
/// the body, `f = x => x * 2`
fn assigned_braceless_function(items: &[Node]) -> Option<Node> {
	let [assignment, body @ ..] = items else { return None };
	let Node::Key(name, op @ (Op::Assign | Op::Define), head) = assignment.drop_meta() else {
		return None;
	};
	// Lua's `sq = function(x) return x * x end`: the closing word ends the body
	let body = match without_result_type(body) {
		[body @ .., end] if end.is_symbol(END_WORD) => body,
		body => body,
	};
	let body = match body {
		[] => return None,
		[single] => single.clone(),
		several => Node::List(several.to_vec(), Bracket::None, Separator::Space),
	};
	Some(Node::Key(name.clone(), *op, Box::new(lambda(function_parameters(head)?, forms(body)))))
}

/// JS `({a, b}, k) => …`: an object parameter taken apart, `(object·0, k) => { a = object·0.a; b = object·0.b; … }`.
/// The parentheses are needed, as in JS: a bare `{a, b} => …` is a map pattern arm (card map-patterns)
fn destructured_parameters(parameters: &Node) -> Option<(Node, Vec<Node>)> {
	let Node::List(items, Bracket::Round, separator) = parameters.drop_meta() else {
		return None;
	};
	let (items, separator) = (items.clone(), separator.clone());
	let mut fields = Vec::new();
	let parameters: Vec<Node> = items
		.into_iter()
		.enumerate()
		.map(|(index, parameter)| match parameter.drop_meta() {
			Node::List(names, Bracket::Curly, _)
				if !names.is_empty() && names.iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) =>
			{
				let object = Node::Symbol(format!("{DESTRUCTURED_OBJECT}{index}"));
				let field = |name: &Node| key(name.clone(), Op::Assign, key(object.clone(), Op::Dot, name.clone()));
				fields.extend(names.iter().map(field));
				object
			}
			_ => parameter,
		})
		.collect();
	(!fields.is_empty()).then(|| (Node::List(parameters, Bracket::Round, separator), fields))
}

/// Ruby's `lambda { |x| x * x }`, `proc { |x| … }`: the block, a lambda itself
fn ruby_lambda(items: &[Node]) -> Option<Node> {
	let [word, block] = items else { return None };
	let is_lambda_word = RUBY_LAMBDA_WORDS.iter().any(|lambda_word| word.is_symbol(lambda_word));
	(is_lambda_word && matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))).then(|| block.clone())
}

fn starred_lambda(name: &Node, value: &Node) -> Option<Node> {
	let Node::Key(parameters, Op::Arrow, body) = value.drop_meta() else {
		return None;
	};
	let parameters = match parameters.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.clone(),
		single => vec![single.clone()],
	};
	let is_starred = |parameter: &Node| matches!(parameter.drop_meta(), Node::Symbol(word) if word.starts_with(crate::words::STARRED) && word.len() > 1);
	if !matches!(name.drop_meta(), Node::Symbol(_)) || !parameters.iter().any(is_starred) {
		return None;
	}
	let head = parens([vec![name.clone()], parameters].concat());
	Some(Node::Key(Box::new(head), Op::Define, body.clone()))
}

/// `xs.Select(f)`: warp's `xs.map(f)`, `x.to_s`: `x.string`, with a note (METHOD_ALIASES); a name the program gives
/// itself stays
fn method_aliases(node: Node, own: &HashSet<String>) -> Node {
	match node {
		Node::Key(receiver, Op::Dot, method) => {
			let method = aliased_method(*method, own);
			key(method_aliases(*receiver, own), Op::Dot, method_aliases(method, own))
		}
		other => other.map_children(|child| method_aliases(child, own)),
	}
}

/// `Select(f)` or `to_s`: the method under warp's word
fn aliased_method(method: Node, own: &HashSet<String>) -> Node {
	let (word, called) = match method.drop_meta() {
		Node::List(items, Bracket::Round, _) if !items.is_empty() => (items[0].name(), true),
		_ => (method.name(), false),
	};
	let Some((written, warp_word)) =
		METHOD_ALIASES.iter().find(|(written, _)| *written == word && !own.contains(*written))
	else {
		return method;
	};
	crate::normalize::set_position_of(&method);
	let paren = if called { "(" } else { "" };
	crate::normalize::hint(
		&format!(".{written}{paren}"),
		&format!(".{warp_word}{paren}"),
		"warp's word for the method",
	);
	match method.drop_meta() {
		Node::List(items, bracket, separator) => {
			let arguments = items[1..].iter().cloned();
			Node::List([symbol(warp_word)].into_iter().chain(arguments).collect(), bracket.clone(), separator.clone())
		}
		_ => symbol(warp_word),
	}
}

/// The fields of `type P { Count: int }`: names the program gives
fn type_fields(node: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Type { body, .. } = part {
			let fields = match body.drop_meta() {
				Node::List(fields, _, _) => fields.as_slice(),
				single => std::slice::from_ref(single),
			};
			names.extend(fields.iter().filter_map(|field| match field.drop_meta() {
				Node::Key(name, _, _) => name.symbol_name().map(str::to_string),
				_ => None,
			}));
		}
	});
	names
}

/// `List.map`: the word a named module qualifies
fn qualified_word(node: &Node, modules: &[&str]) -> Option<String> {
	let Node::Key(module, Op::Dot, word) = node.drop_meta() else {
		return None;
	};
	let is_module = modules.iter().any(|name| module.is_symbol(name));
	word.symbol_name().filter(|_| is_module).map(str::to_string)
}

/// `call(3)`, `call 3`: the arguments of Ruby's `.call`
fn called_arguments(call: &Node) -> Option<Vec<Node>> {
	let Node::List(items, _, _) = call.drop_meta() else {
		return call.is_symbol(CALL_METHOD).then(Vec::new);
	};
	let [word, arguments @ ..] = items.as_slice() else {
		return None;
	};
	word.is_symbol(CALL_METHOD).then(|| arguments.to_vec())
}

/// `function (a, b)`: the parameters after a function keyword with no name
fn function_parameters(head: &Node) -> Option<Node> {
	let Node::List(head_items, _, _) = head.drop_meta() else {
		return None;
	};
	let [keyword, parameters @ ..] = head_items.as_slice() else {
		return None;
	};
	if !matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)) {
		return None;
	}
	match parameters {
		[single] => Some(single.clone()),
		[] => None,
		// Go's `func(x int)`, `func(a, b int)`: the names, the types after them dropped
		several => {
			let names: Vec<Node> = several.iter().filter_map(|parameter| match parameter.drop_meta() {
				Node::Symbol(_) if !is_kinded_type_word(parameter) => Some(parameter.clone()),
				Node::List(words, _, Separator::Space) if matches!(words.as_slice(), [name, kind] if is_kinded_type_word(kind) && !is_kinded_type_word(name)) => Some(words[0].clone()),
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
	let Node::List(items, Bracket::None, _) = words.drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[word, parameter] if word.is_symbol(LAMBDA_WORD) => Some(parameter.clone()),
		_ => None,
	}
}

/// `map xs lambda x: x+1` arrives as the items `…, lambda, x: x+1`: the two are one lambda
fn python_lambdas(items: Vec<Node>) -> Vec<Node> {
	let mut merged: Vec<Node> = Vec::with_capacity(items.len());
	for item in items {
		let follows_lambda = merged.last().is_some_and(|last| last.is_symbol(LAMBDA_WORD));
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
