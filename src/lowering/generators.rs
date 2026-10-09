//! Generators (card generators-function): a function whose body yields, `count_to(n) := { i = 1; while i <= n { yield i;
//! i += 1 } }`, is a generator.
//! A loop over its call runs lazily: `for x in count_to(10^9) { … }` inlines the generator's body (its parameters and
//! locals renamed `count_to·i·1`), each `yield v` becoming `x = v` followed by the loop's body. A `break` in the loop's
//! body stops the generator, `continue` goes on after the yield, the generator's `return` ends the loop: a stop flag
//! `count_to·stop·1` breaks out of each of the generator's loops, all of them inside one loop run once.
//! Any other call collects what it yields into a list: `count_to(3)` is [1 2 3], `return` ends the list.
//! Runs after ruby_blocks, which takes the yielding functions some call passes a block to.

use crate::for_loop::{block_items, loop_variables};
use crate::inlining::renamed_names;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::ruby_blocks::{arguments, contains_yield, yielded};
use crate::warp_parser::while_do;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

pub(crate) const FOR_WORD: &str = "for";
pub(crate) const IN_WORD: &str = "in";
const IMPLICIT_VARIABLE: &str = "it";
pub(crate) const RETURN_WORD: &str = "return";
pub(crate) const BREAK_WORD: &str = "break";
pub(crate) const CONTINUE_WORD: &str = "continue";
const GLOBAL_WORD: &str = "global";
pub(crate) const NAME_SEPARATOR: &str = "·";
/// The list a collecting generator returns, `count_to·yielded`
const YIELDED_SUFFIX: &str = "yielded";
/// Set when an inlined generator is to stop, `count_to·stop·1`
const STOP_SUFFIX: &str = "stop";
/// The method an iterator object gives its next item with, ø at the end
pub(crate) const NEXT_METHOD: &str = "next";
/// The object a loop walks with next(), `x·iterator`
const ITERATOR_SUFFIX: &str = "iterator";

pub(crate) struct Generator {
	pub(crate) parameters: Vec<String>,
	pub(crate) body: Node,
	/// The parameters and locals, renamed per inlined loop
	pub(crate) locals: HashSet<String>,
	/// Inlined into a loop over its call: not recursive, no `global`
	inlinable: bool,
}

/// What a statement rewrite does with one node
enum Step {
	Keep,
	Descend,
	Replace(Vec<Node>),
}

pub fn lower(node: Node) -> Node {
	let node = crate::generator_objects::lower(node);
	let generators = generators(&node);
	if generators.is_empty() {
		return node;
	}
	let counter = Cell::new(0);
	collected_definitions(lazy_loops(node, &generators, &counter), &generators)
}

pub(crate) fn symbol(name: &str) -> Node {
	Node::Symbol(name.to_string())
}

pub(crate) fn symbol_name(node: &Node) -> Option<&String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name),
		_ => None,
	}
}

pub(crate) fn is_word(node: &Node, word: &str) -> bool {
	symbol_name(node).is_some_and(|name| name == word)
}

pub(crate) fn assign(target: Node, value: Node) -> Node {
	Node::Key(Box::new(target), Op::Assign, Box::new(value))
}

pub(crate) fn statements(items: Vec<Node>, bracket: Bracket) -> Node {
	Node::List(items, bracket, Separator::Semicolon)
}

pub(crate) fn number(value: i64) -> Node {
	Node::Number(crate::extensions::numbers::Number::Int(value))
}

/// `if flag { break }`
fn break_if(flag: &Node) -> Node {
	let condition = Node::Key(Box::new(Node::Empty), Op::If, Box::new(flag.clone()));
	Node::Key(Box::new(condition), Op::Then, Box::new(statements(vec![symbol(BREAK_WORD)], Bracket::Curly)))
}

/// `while 1 { statements; break }`: a loop run once, which `break` leaves early
fn run_once(mut body: Vec<Node>) -> Node {
	body.push(symbol(BREAK_WORD));
	while_do(number(1), statements(body, Bracket::Curly))
}

/// `name(params) := body` of a body that yields: its name, parameters and body
fn definition(node: &Node) -> Option<(String, Vec<String>, &Node)> {
	let Node::Key(head, Op::Define, body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, parameters) = items.split_first()?;
	let parameters = parameters.iter().map(|parameter| symbol_name(parameter).cloned()).collect::<Option<Vec<_>>>()?;
	let name = symbol_name(name)?.clone();
	contains_yield(body).then(|| (name, parameters, body.as_ref()))
}

pub(crate) fn generators(node: &Node) -> HashMap<String, Generator> {
	let mut generators = HashMap::new();
	node.visit(&mut |part| if let Some((name, parameters, body)) = definition(part) {
		let mut locals: HashSet<String> = parameters.iter().cloned().collect();
		locals.extend(loop_variables(body));
		let mut inlinable = true;
		body.visit(&mut |inner| match inner {
			Node::Key(target, op, _) if *op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) => {
				locals.extend(symbol_name(target).cloned());
			}
			Node::List(items, Bracket::Round, _) if items.first().is_some_and(|callee| is_word(callee, &name)) => inlinable = false,
			Node::Symbol(word) if word == GLOBAL_WORD => inlinable = false,
			_ => {}
		});
		generators.insert(name, Generator { parameters, body: body.clone(), locals, inlinable });
	});
	generators
}

pub(crate) fn is_return(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(items, _, _) => items.first().is_some_and(|word| is_word(word, RETURN_WORD)),
		other => is_word(other, RETURN_WORD),
	}
}

fn is_loop(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(head, Op::Do, _) => matches!(head.drop_meta(), Node::Key(_, Op::While, _)),
		Node::List(items, _, _) => items.first().is_some_and(|word| is_word(word, FOR_WORD)),
		_ => false,
	}
}

/// A definition or a lambda: its statements are its own
fn is_own_scope(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Define | Op::FatArrow | Op::Arrow, _))
}

/// The value a `yield` gives: one value, the list of several, or nothing
pub(crate) fn yielded_value(values: Vec<Node>) -> Node {
	match values.len() {
		0 => Node::Empty,
		1 => values.into_iter().next().expect("one value"),
		_ => Node::List(values, Bracket::Square, Separator::Colon),
	}
}

/// A list of statements: a block's, a sequence's, or a block of one
pub(crate) fn is_statement_list(items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
	matches!(separator, Separator::Semicolon | Separator::Newline) || (*bracket == Bracket::Curly && items.len() == 1)
}

/// The node with each statement `step` replaces spliced into its statement list (a block of its own elsewhere)
fn rewritten(node: Node, step: &dyn Fn(&Node) -> Step) -> Node {
	match node {
		Node::List(items, bracket, separator) if is_statement_list(&items, &bracket, &separator) => {
			let separator = if matches!(separator, Separator::Newline) { separator } else { Separator::Semicolon };
			let items = items.into_iter().flat_map(|item| match step(&item) {
				Step::Keep => vec![item],
				Step::Descend => vec![item.map_children(|child| rewritten(child, step))],
				Step::Replace(replacement) => replacement,
			});
			Node::List(items.collect(), bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewritten(*node, step)), data },
		other => match step(&other) {
			Step::Keep => other,
			Step::Descend => other.map_children(|child| rewritten(child, step)),
			Step::Replace(replacement) => statements(replacement, Bracket::Curly),
		},
	}
}

/// The direct parts of a node: a list's items, a key's two sides
fn parts(node: &Node) -> Vec<&Node> {
	match node {
		Node::List(items, _, _) => items.iter().collect(),
		Node::Key(left, _, right) => vec![left, right],
		_ => vec![],
	}
}

/// Whether `node` holds `word` outside its loops and own scopes: a `break` or `continue` of the loop body itself
pub(crate) fn holds_own(node: &Node, word: &str) -> bool {
	match node.drop_meta() {
		inner if is_word(inner, word) => true,
		inner if is_loop(inner) || is_own_scope(inner) => false,
		inner => parts(inner).into_iter().any(|child| holds_own(child, word)),
	}
}

/// Whether `node` yields or returns outside its own scopes: where an inlined generator may stop
pub(crate) fn holds_stop(node: &Node) -> bool {
	match node.drop_meta() {
		inner if yielded(inner).is_some() || is_return(inner) => true,
		inner if is_own_scope(inner) => false,
		inner => parts(inner).into_iter().any(holds_stop),
	}
}

/// `for x in g(args) {body}` and `for g(args) {body}`: the loop variable, the generator, its arguments and the body
fn generator_loop<'a>(node: &Node, generators: &'a HashMap<String, Generator>) -> Option<(Node, &'a String, &'a Generator, Vec<Node>, Node)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	if !items.first().is_some_and(|keyword| is_word(keyword, FOR_WORD)) {
		return None;
	}
	let (variable, call, body) = match items.as_slice() {
		[_, variable, within, call, body] if is_word(within, IN_WORD) && symbol_name(variable).is_some() => (variable.clone(), call, body),
		[_, call, body] => (symbol(IMPLICIT_VARIABLE), call, body),
		_ => return None,
	};
	if !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	// `for f in fib`: a generator without parameters is called by its name alone
	let call_items = match call.drop_meta() {
		Node::List(call_items, Bracket::Round | Bracket::None, _) => call_items.as_slice(),
		Node::Symbol(_) => std::slice::from_ref(call),
		_ => return None,
	};
	let (name, arguments) = call_items.split_first()?;
	let (name, generator) = generators.get_key_value(symbol_name(name)?)?;
	let arguments: Vec<Node> = arguments.iter().flat_map(self::arguments).collect();
	(generator.inlinable && arguments.len() == generator.parameters.len()).then(|| (variable, name, generator, arguments, body.clone()))
}

fn lazy_loops(node: Node, generators: &HashMap<String, Generator>, counter: &Cell<usize>) -> Node {
	let node = node.map_children(|child| lazy_loops(child, generators, counter));
	match generator_loop(&node, generators) {
		Some((variable, name, generator, arguments, body)) => {
			counter.set(counter.get() + 1);
			inlined_loop(variable, name, generator, arguments, body, counter.get()).with_meta_of(&node)
		}
		None => node,
	}
}

/// The generator's body inlined, each `yield v` running the loop's body with the variable `v`
fn inlined_loop(variable: Node, name: &str, generator: &Generator, arguments: Vec<Node>, body: Node, loop_number: usize) -> Node {
	let rename = |word: &str| [name, word, &loop_number.to_string()].join(NAME_SEPARATOR);
	let stop = symbol(&rename(STOP_SUFFIX));
	let breaks = holds_own(&body, BREAK_WORD);
	let continues = holds_own(&body, CONTINUE_WORD);
	let generator_body = renamed_names(generator.body.clone(), &generator.locals, &rename);
	let stops = breaks || holds_own_return(&generator_body);
	let loop_body = |values: Vec<Node>| {
		let statements = block_items(&rewritten(body.clone(), &|node| loop_body_step(node, &stop, continues)));
		let mut run = vec![assign(variable.clone(), yielded_value(values))];
		match continues {
			true => run.push(run_once(statements)),
			false => run.extend(statements),
		}
		if breaks {
			run.push(break_if(&stop));
		}
		run
	};
	let generator_statements = block_items(&rewritten(generator_body, &|node| generator_step(node, &loop_body, &stop, stops)));
	let bindings = generator.parameters.iter().zip(arguments).map(|(parameter, argument)| assign(symbol(&rename(parameter)), argument));
	let mut all: Vec<Node> = bindings.collect();
	match stops {
		true => {
			all.push(assign(stop.clone(), number(0)));
			all.push(run_once(generator_statements));
		}
		false => all.extend(generator_statements),
	}
	statements(all, Bracket::None)
}

/// In the generator's body: `yield v` runs the loop's body, `return` stops, a loop that may stop is followed by
/// `if stop { break }`
fn generator_step(node: &Node, loop_body: &dyn Fn(Vec<Node>) -> Vec<Node>, stop: &Node, stops: bool) -> Step {
	if let Some(values) = yielded(node) {
		return Step::Replace(loop_body(values));
	}
	match node {
		_ if is_return(node) => Step::Replace(vec![assign(stop.clone(), number(1)), symbol(BREAK_WORD)]),
		_ if is_own_scope(node) => Step::Keep,
		_ if stops && is_loop(node) && holds_stop(node) => Step::Replace(vec![node.drop_meta().clone().map_children(|child| rewritten(child, &|inner| generator_step(inner, loop_body, stop, stops))), break_if(stop)]),
		_ => Step::Descend,
	}
}

fn holds_own_return(node: &Node) -> bool {
	match node.drop_meta() {
		inner if is_return(inner) => true,
		inner if is_own_scope(inner) => false,
		inner => parts(inner).into_iter().any(holds_own_return),
	}
}

/// In the loop's body: `break` stops the generator, `continue` leaves the run-once loop around the body
fn loop_body_step(node: &Node, stop: &Node, continues: bool) -> Step {
	match node.drop_meta() {
		inner if is_word(inner, BREAK_WORD) => Step::Replace(vec![assign(stop.clone(), number(1)), symbol(BREAK_WORD)]),
		inner if continues && is_word(inner, CONTINUE_WORD) => Step::Replace(vec![symbol(BREAK_WORD)]),
		inner if is_loop(inner) || is_own_scope(inner) => Step::Keep,
		_ => Step::Descend,
	}
}

/// Iterator objects (card generators-function): `for x in c {body}` of an object whose class has a method `next()`,
/// `c = Countdown(3)` or `for x in Countdown(3)`, walks what `next()` gives until it gives ø:
/// `x·iterator = c; x = x·iterator.next(); while x != ø { body; x = x·iterator.next() }` (the last a marked step,
/// which `continue` runs too). Before class_methods, which lowers the `next()` calls.
pub fn lower_iterators(node: Node) -> Node {
	let classes = iterator_classes(&node);
	if classes.is_empty() {
		return node;
	}
	let mut objects = HashSet::new();
	node.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		if constructed_iterator(value, &classes) {
			objects.extend(symbol_name(target).cloned());
		}
	});
	iterator_loops(node, &classes, &objects)
}

/// The classes with a method `next()`
fn iterator_classes(node: &Node) -> HashSet<String> {
	let mut classes = HashSet::new();
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		let is_next = |member: &Node| matches!(member.drop_meta(), Node::Key(head, Op::Define, _)
			if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1 && is_word(&items[0], NEXT_METHOD)));
		let members = match body.drop_meta() {
			Node::List(items, _, _) => items.clone(),
			single => vec![single.clone()],
		};
		if members.iter().any(is_next) {
			classes.extend(symbol_name(name).cloned());
		}
	});
	classes
}

/// `Countdown(3)` of an iterator class
fn constructed_iterator(value: &Node, classes: &HashSet<String>) -> bool {
	matches!(value.drop_meta(), Node::List(items, Bracket::Round | Bracket::None, _)
		if items.first().and_then(symbol_name).is_some_and(|class| classes.contains(class)))
}

fn iterator_loops(node: Node, classes: &HashSet<String>, objects: &HashSet<String>) -> Node {
	let node = node.map_children(|child| iterator_loops(child, classes, objects));
	let Node::List(items, _, _) = node.drop_meta() else { return node };
	let (variable, iterable, body) = match items.as_slice() {
		[keyword, variable, within, iterable, body] if is_word(keyword, FOR_WORD) && is_word(within, IN_WORD) => (variable, iterable, body),
		_ => return node,
	};
	let is_object = symbol_name(iterable).is_some_and(|name| objects.contains(name)) || constructed_iterator(iterable, classes);
	let Some(name) = symbol_name(variable).filter(|_| is_object && matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _))) else { return node };
	let iterator = symbol(&[name.as_str(), ITERATOR_SUFFIX].join(NAME_SEPARATOR));
	let next = || assign(variable.clone(), Node::Key(Box::new(iterator.clone()), Op::Dot, Box::new(Node::List(vec![symbol(NEXT_METHOD)], Bracket::Round, Separator::None))));
	let mut statements_of_body = block_items(body);
	statements_of_body.push(crate::wasm_emitter::mark_step(next()));
	let more = Node::Key(Box::new(variable.clone()), Op::Ne, Box::new(Node::Empty));
	statements(vec![assign(iterator.clone(), iterable.clone()), next(), while_do(more, statements(statements_of_body, Bracket::Curly))], Bracket::None).with_meta_of(&node)
}

/// Each generator's definition collects what it yields: `{ g·yielded = []; …; g·yielded += [v]; …; g·yielded }`
fn collected_definitions(node: Node, generators: &HashMap<String, Generator>) -> Node {
	match node {
		Node::Key(head, Op::Define, body) if definition(&Node::Key(head.clone(), Op::Define, body.clone())).is_some_and(|(name, _, _)| generators.contains_key(&name)) => {
			let (name, _, _) = definition(&Node::Key(head.clone(), Op::Define, body.clone())).expect("a generator");
			let list = symbol(&[name.as_str(), YIELDED_SUFFIX].join(NAME_SEPARATOR));
			let step = |node: &Node| -> Step {
				if let Some(values) = yielded(node) {
					let item = Node::List(vec![yielded_value(values)], Bracket::Square, Separator::None);
					return Step::Replace(vec![Node::Key(Box::new(list.clone()), Op::AddAssign, Box::new(item))]);
				}
				match node {
					_ if is_return(node) => Step::Replace(vec![Node::List(vec![symbol(RETURN_WORD), list.clone()], Bracket::None, Separator::Space)]),
					_ if is_own_scope(node) => Step::Keep,
					_ => Step::Descend,
				}
			};
			let mut collected = vec![assign(list.clone(), crate::warp_parser::parse("[]"))];
			collected.extend(block_items(&rewritten(*body, &step)));
			collected.push(list);
			Node::Key(head, Op::Define, Box::new(statements(collected, Bracket::Curly)))
		}
		other => other.map_children(|child| collected_definitions(child, generators)),
	}
}
