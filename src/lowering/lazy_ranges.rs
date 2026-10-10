//! A range is a descriptor, not its list (card lazy-range, notes/lazy_ranges.md): `count`, `#`, `sum` and indexing of
//! a range of plain bounds (integer literals or variables) are worked out from the bounds, and a range variable that is
//! never changed is replaced by its range wherever it is used, so `xs = 1..100001; xs#5` builds no list. `for`, `map`,
//! `filter` and the other iteration words already loop over a range's bounds (for_loop.rs, lambdas.rs). A range passed
//! to a function that only reads its parameter so is passed as its bounds: `f(1..n)` calls `f·range·0(1, n)`, a copy of
//! f whose parameter is the range of its two new parameters. Any other use (printing, returning, changing it) keeps the
//! variable a list, collected once (declaration_lowering.rs). A function whose value is a range of its parameters
//! (`f(n) := 1..n`) returns that range: a call with plain arguments is the range itself, read as above.

use super::words::{COUNT_WORD, SUM_WORD};
use super::nodes::{call, key};
use crate::analyzer::{call_name, TEMPORARY_SEPARATOR};
use crate::effects::call_arguments;
use crate::memoization::definition_parts;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;

/// Words a range variable may stand behind a dot for: they read the range without collecting it
const READING_METHODS: [&str; 10] = [COUNT_WORD, SUM_WORD, "map", "filter", "each", "fold", "find", "any", "all", "reduce"];
const START: &str = "range_start";
const END: &str = "range_end";
const INDEX: &str = "range_index";
const WHOLE: &str = "range_whole";
const LENGTH: &str = "range_length";
const LENGTH_TEMPLATE: &str = "(if range_end - range_start > 0 then range_end - range_start else 0)";
const INDEX_TEMPLATE: &str = "if range_index >= 1 and range_index <= range_length then range_start + range_index - 1 else range_whole#range_index";
const BOUND_ARITHMETIC: [Op; 5] = [Op::Add, Op::Sub, Op::Mul, Op::Pow, Op::Mod];
const SUM_TEMPLATE: &str = "range_length * (2 * range_start + range_length - 1) / 2";

pub fn lower(node: Node) -> Node {
	if !has_range(&node) {
		return node;
	}
	let context = crate::analyzer::function_context(&node);
	let own = |word: &str| context.user_functions.contains_key(word);
	let mut lowering = Lowering { counts: !own(COUNT_WORD), sums: !own(SUM_WORD), temporaries: 0 };
	let producers = range_producers(&node);
	let node = inline_produced_ranges(node, &producers);
	let readers = range_readers(&node);
	let node = replace_range_variables(node, &readers);
	lowering.rewrite(pass_ranges_as_bounds(node, &readers))
}

fn has_range(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Key(_, Op::Range | Op::To, _)));
	found
}

/// A range of plain bounds, seen through parentheses: its start, its end past the last number, and the range itself
struct Range {
	start: Node,
	end: Node,
	whole: Node,
}

/// An integer literal, a variable, or arithmetic of them (`10^12`, `n + 1`): worked out wherever the range is read
fn plain_bound(bound: &Node) -> bool {
	match bound.drop_meta() {
		Node::Symbol(_) | Node::Number(crate::extensions::numbers::Number::Int(_)) => true,
		Node::Key(left, op, right) if BOUND_ARITHMETIC.contains(op) => plain_bound(left) && plain_bound(right),
		Node::List(items, Bracket::Round, _) if items.len() == 1 => plain_bound(&items[0]),
		_ => false,
	}
}

/// A range seen through parentheses, of any bounds: the range itself, its start and its end past the last number
fn range_parts(node: &Node) -> Option<(&Node, Node, Node)> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => range_parts(&items[0]),
		whole @ Node::Key(start, op @ (Op::Range | Op::To), end) => {
			// `a to b` ends after b: its end is b + 1
			let end = if *op == Op::To { add_one(end) } else { end.drop_meta().clone() };
			Some((whole, start.drop_meta().clone(), end))
		}
		_ => None,
	}
}

fn range_of(node: &Node) -> Option<Range> {
	let (whole, start, end) = range_parts(node)?;
	let Node::Key(written_start, _, written_end) = whole else { return None };
	(plain_bound(written_start) && plain_bound(written_end)).then(|| Range { start, end, whole: whole.clone() })
}

fn add_one(bound: &Node) -> Node {
	match bound.drop_meta() {
		Node::Number(crate::extensions::numbers::Number::Int(n)) => Node::int(n + 1),
		other => key(other.clone(), Op::Add, Node::int(1)),
	}
}

struct Lowering {
	counts: bool,
	sums: bool,
	temporaries: usize,
}

impl Lowering {
	fn rewrite(&mut self, node: Node) -> Node {
		let node = node.map_children(|child| self.rewrite(child));
		self.read(&node).unwrap_or(node)
	}

	/// `count r`, `#r`, `r.count`, `sum r`, `r.sum`, `r#i` from the bounds of r
	fn read(&mut self, node: &Node) -> Option<Node> {
		match node.drop_meta() {
			Node::List(items, _, separator) if is_call(items, separator) && self.counts && items[0].is_symbol(COUNT_WORD) => Some(self.length(&range_of(&items[1])?)),
			Node::List(items, _, separator) if is_call(items, separator) && self.sums && items[0].is_symbol(SUM_WORD) => Some(self.sum(&range_of(&items[1])?)),
			Node::Key(empty, Op::Hash, counted) if matches!(empty.drop_meta(), Node::Empty) => Some(self.length(&range_of(counted)?)),
			Node::Key(range, Op::Dot, word) if self.counts && word.is_symbol(COUNT_WORD) => Some(self.length(&range_of(range)?)),
			Node::Key(range, Op::Dot, word) if self.sums && word.is_symbol(SUM_WORD) => Some(self.sum(&range_of(range)?)),
			Node::Key(range, Op::Hash, index) if is_position(index) => Some(self.element(&range_of(range)?, index)),
			_ => None,
		}
	}

	fn bounds(&self, template: &str, range: &Range) -> Node {
		substitute(substitute(parse(template), START, &range.start), END, &range.end)
	}

	fn length(&self, range: &Range) -> Node {
		use crate::extensions::numbers::Number::Int;
		match (range.start.drop_meta(), range.end.drop_meta()) {
			(Node::Number(Int(start)), Node::Number(Int(end))) => Node::int((end - start).max(0)),
			_ => self.bounds(LENGTH_TEMPLATE, range),
		}
	}

	fn temporary(&mut self, what: &str) -> Node {
		self.temporaries += 1;
		Node::Symbol(format!("range{TEMPORARY_SEPARATOR}{}{TEMPORARY_SEPARATOR}{what}", self.temporaries))
	}

	/// `(t = value; body)` where body reads t
	fn bound(&mut self, what: &str, value: Node, body: &dyn Fn(&Node) -> Node) -> Node {
		if plain_bound(&value) {
			return body(&value);
		}
		let temporary = self.temporary(what);
		let binding = key(temporary.clone(), Op::Assign, value);
		Node::List(vec![binding, body(&temporary)], Bracket::Round, Separator::Semicolon)
	}

	fn sum(&mut self, range: &Range) -> Node {
		let length = self.length(range);
		let summed = self.bounds(SUM_TEMPLATE, range);
		self.bound("length", length, &|length| substitute(summed.clone(), LENGTH, length))
	}

	/// `r#i`: the number at i, or what the list would do there (the error `index out of range`)
	fn element(&mut self, range: &Range, index: &Node) -> Node {
		let length = self.length(range);
		let picked = substitute(substitute(self.bounds(INDEX_TEMPLATE, range), LENGTH, &length), WHOLE, &range.whole);
		self.bound("index", index.drop_meta().clone(), &|index| substitute(picked.clone(), INDEX, index))
	}
}

/// `count r`, `count(r)`, `sum r`: a word and its one argument, not two statements
fn is_call(items: &[Node], separator: &Separator) -> bool {
	items.len() == 2 && matches!(separator, Separator::Space | Separator::None)
}

/// An index by position, not a key (`xs#"a"`) or a slice (`xs#(2..4)`)
fn is_position(index: &Node) -> bool {
	!matches!(index.drop_meta(), Node::Text(_) | Node::Char(_) | Node::Key(_, Op::Range | Op::To, _))
		&& !matches!(index.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1 && range_of(&items[0]).is_some())
}

/// In every statement list: `xs = a..b` that nothing changes and that is only read (count, index, sum, `for … in xs`,
/// xs.map …) is replaced by its range at each use, and the assignment goes
fn replace_range_variables(node: Node, readers: &RangeReaders) -> Node {
	let node = node.map_children(|child| replace_range_variables(child, readers));
	let Node::List(statements, bracket, separator @ (Separator::Semicolon | Separator::Newline)) = node else { return node };
	// `{a = 1..2000; b = #a}` ending in a field is an object: each of its assignments is a field it keeps
	if bracket == Bracket::Curly && statements.last().is_some_and(|last| matches!(last.drop_meta(), Node::Key(_, Op::Assign | Op::Define | Op::Colon, _))) {
		return Node::List(statements, bracket, separator);
	}
	let mut statements = statements;
	let mut position = 0;
	while position + 1 < statements.len() {
		match replaceable(&statements, position, readers) {
			Some((name, range)) => {
				statements.remove(position);
				for statement in &mut statements[position..] {
					*statement = substitute_variable(std::mem::replace(statement, Node::Empty), &name, &range);
				}
			}
			None => position += 1,
		}
	}
	Node::List(statements, bracket, separator)
}

/// The variable and range statement `position` assigns, when every other statement only reads it as a range
fn replaceable(statements: &[Node], position: usize, readers: &RangeReaders) -> Option<(String, Node)> {
	let Node::Key(target, Op::Assign | Op::Define, value) = statements[position].drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	let range = range_of(value)?;
	// a short range of literals is its list already (counting.rs range_elements)
	if crate::analyzer::range_elements(&range.whole).is_some() {
		return None;
	}
	let (before, after) = (&statements[..position], &statements[position + 1..]);
	let bound_names: Vec<String> = [&range.start, &range.end].into_iter().flat_map(symbols).collect();
	let unchanged = |name: &str| !after.iter().any(|statement| changes(statement, name));
	let read_only = before.iter().all(|statement| has_parameter(statement, name) || !statement.mentions_any(&[name])) && after.iter().all(|statement| only_read(statement, name, readers));
	(read_only && unchanged(name) && bound_names.iter().all(|bound| unchanged(bound) && !after.iter().any(|statement| is_parameter(statement, bound))))
		.then(|| (name.clone(), Node::List(vec![range.whole.clone()], Bracket::Round, Separator::None)))
}

fn symbols(node: &Node) -> Vec<String> {
	let mut names = vec![];
	node.visit(&mut |part| if let Node::Symbol(name) = part { names.push(name.clone()) });
	names
}

/// Whether `node` assigns or updates `name`, its items or its fields
fn changes(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| {
		let Node::Key(target, op, _) = part else { return };
		let changing = matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign();
		let target = match target.drop_meta() {
			Node::Key(owner, Op::Hash | Op::Dot, _) => owner.drop_meta(),
			other => other,
		};
		found |= changing && target.is_symbol(name);
	});
	found
}

/// Whether `name` is a parameter of a lambda in `node`, which would hide the variable a substituted range reads
fn is_parameter(node: &Node, name: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Key(parameters, Op::FatArrow, _) if parameters.mentions_any(&[name])));
	found
}

/// Whether every use of `name` in `node` reads it as a range: count, sum, index, the list of a `for` or of an iteration
/// word, an argument a range reader takes as its bounds; never inside a function definition, which would read it as a
/// global
fn only_read(node: &Node, name: &str, readers: &RangeReaders) -> bool {
	let is_name = |node: &Node| node.is_symbol(name);
	let reads = |node: &Node| only_read(node, name, readers);
	match node.drop_meta() {
		_ if has_parameter(node, name) => true,
		Node::Symbol(symbol) => symbol != name,
		Node::List(items, _, separator) if is_call(items, separator) && is_name(&items[1]) && (items[0].is_symbol(COUNT_WORD) || items[0].is_symbol(SUM_WORD)) => true,
		Node::List(items, bracket, separator) if let Some(reading) = reading_parameters(items, bracket, separator, readers) => {
			call_arguments(&items[1..]).into_iter().zip(reading).all(|(argument, reads_range)| if is_name(argument) { *reads_range } else { reads(argument) })
		}
		Node::List(items, _, _) => items.iter().enumerate().all(|(at, item)| (is_name(item) && at > 0 && items[at - 1].is_symbol("in")) || reads(item)),
		Node::Key(empty, Op::Hash, counted) if matches!(empty.drop_meta(), Node::Empty) && is_name(counted) => true,
		Node::Key(list, Op::Hash, index) if is_name(list) => is_position(index) && reads(index),
		Node::Key(list, Op::Dot, method) if is_name(list) => reading_method(method) && reads(method),
		Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(..)) => !body.mentions_any(&[name]),
		Node::Key(left, _, right) => reads(left) && reads(right),
		_ => true,
	}
}

/// `count`, `sum`, `map(f)`, `filter(f)` …: a method that reads a range without collecting it
fn reading_method(method: &Node) -> bool {
	match method.drop_meta() {
		Node::Symbol(word) => READING_METHODS.contains(&word.as_str()),
		Node::List(items, _, _) => items.first().is_some_and(|head| matches!(head.drop_meta(), Node::Symbol(word) if READING_METHODS.contains(&word.as_str()))),
		_ => false,
	}
}

/// Per user function defined once: whether its body only reads each parameter as a range (`only_read`, unchanged)
type RangeReaders = HashMap<String, Vec<bool>>;

/// The name and parameters of a definition's head `(f a b)`
fn signature(head: &Node) -> Option<(String, Vec<&Node>)> {
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let Node::Symbol(name) = items.first()?.drop_meta() else { return None };
	Some((name.clone(), call_arguments(&items[1..])))
}

/// A definition with a parameter `name`: its body reads that parameter, not the variable
fn has_parameter(node: &Node, name: &str) -> bool {
	definition_parts(node).and_then(|(head, _, _)| signature(&head).map(|(_, parameters)| parameters.iter().any(|parameter| parameter.is_symbol(name)))).unwrap_or(false)
}

/// `name` replaced by `range`, except in a definition with a parameter `name`
fn substitute_variable(node: Node, name: &str, range: &Node) -> Node {
	match node {
		_ if has_parameter(&node, name) => node,
		Node::Symbol(symbol) if symbol == name => range.clone(),
		other => other.map_children(|child| substitute_variable(child, name, range)),
	}
}

/// Until nothing changes: a parameter passed on only to range readers is read as a range too (`f(xs) := g(xs)`); a
/// recursive call is no reader yet, so `f(xs) := … f(xs)` keeps its list
fn range_readers(program: &Node) -> RangeReaders {
	let mut readers = RangeReaders::new();
	loop {
		let next = readers_given(program, &readers);
		if next == readers {
			return readers;
		}
		readers = next;
	}
}

fn readers_given(program: &Node, known: &RangeReaders) -> RangeReaders {
	let mut definitions: HashMap<String, Vec<Vec<bool>>> = HashMap::new();
	program.visit(&mut |part| {
		let Some((head, body, _)) = definition_parts(part) else { return };
		let Some((name, parameters)) = signature(&head) else { return };
		let reads_range = |parameter: &&Node| matches!(parameter.drop_meta(), Node::Symbol(parameter) if !changes(&body, parameter) && only_read(&body, parameter, known));
		definitions.entry(name).or_default().push(parameters.iter().map(reads_range).collect());
	});
	definitions.into_iter().filter(|(_, found)| found.len() == 1).map(|(name, mut found)| (name, found.remove(0))).collect()
}

/// Per parameter of the range reader `items` calls: whether it reads it as a range
fn reading_parameters<'a>(items: &[Node], bracket: &Bracket, separator: &Separator, readers: &'a RangeReaders) -> Option<&'a Vec<bool>> {
	let reading = readers.get(call_name(items, bracket, separator)?)?;
	(call_arguments(&items[1..]).len() == reading.len() && reading.contains(&true)).then_some(reading)
}

/// The copy of `function` taking the ranges at `positions` as their start and end
fn bounds_name(function: &str, positions: &[usize]) -> String {
	let positions: Vec<String> = positions.iter().map(usize::to_string).collect();
	format!("{function}{TEMPORARY_SEPARATOR}range{TEMPORARY_SEPARATOR}{}", positions.join(TEMPORARY_SEPARATOR))
}

fn bound_names(parameter: &str) -> [String; 2] {
	[format!("{parameter}{TEMPORARY_SEPARATOR}start"), format!("{parameter}{TEMPORARY_SEPARATOR}end")]
}

/// Every call passing a range to a reader of it calls its bounds copy, which follows the reader's definition
fn pass_ranges_as_bounds(program: Node, readers: &RangeReaders) -> Node {
	if readers.values().all(|reading| !reading.contains(&true)) {
		return program;
	}
	let mut copies = Copies::new();
	let program = call_bounds_copies(program, readers, &mut copies);
	// a copy's body can pass its range on to another reader, which then needs its copy too
	loop {
		let wanted = copies.clone();
		let with_copies = add_bounds_copies(program.clone(), readers, &mut copies);
		if copies == wanted {
			return with_copies;
		}
	}
}

/// Per range reader: the parameter positions of each of its copies that is called
type Copies = BTreeMap<String, BTreeSet<Vec<usize>>>;

fn call_bounds_copies(node: Node, readers: &RangeReaders, copies: &mut Copies) -> Node {
	let node = node.map_children(|child| call_bounds_copies(child, readers, copies));
	let Node::List(items, bracket, separator) = node.drop_meta() else { return node };
	let Some(reading) = reading_parameters(items, bracket, separator, readers) else { return node };
	let arguments = call_arguments(&items[1..]);
	let ranges: Vec<Option<(Node, Node)>> = arguments.iter().zip(reading).map(|(argument, reads_range)| {
		range_parts(argument).filter(|_| *reads_range).map(|(_, start, end)| (start, end))
	}).collect();
	let positions: Vec<usize> = (0..ranges.len()).filter(|at| ranges[*at].is_some()).collect();
	if positions.is_empty() {
		return node;
	}
	let function = items[0].name();
	let name = bounds_name(&function, &positions);
	let arguments: Vec<Node> = arguments.into_iter().zip(ranges).flat_map(|(argument, range)| match range {
		Some((start, end)) => vec![start, end],
		None => vec![argument.clone()],
	}).collect();
	copies.entry(function).or_default().insert(positions);
	call(&name, arguments)
}

/// After each definition of a function whose copies are called: those copies
fn add_bounds_copies(node: Node, readers: &RangeReaders, copies: &mut Copies) -> Node {
	let node = node.map_children(|child| add_bounds_copies(child, readers, copies));
	let Node::List(statements, bracket, separator @ (Separator::Semicolon | Separator::Newline)) = node else { return node };
	let statements = statements.into_iter().flat_map(|statement| {
		let added = bounds_copies(&statement, readers, copies);
		std::iter::once(statement).chain(added)
	}).collect();
	Node::List(statements, bracket, separator)
}

fn bounds_copies(statement: &Node, readers: &RangeReaders, copies: &mut Copies) -> Vec<Node> {
	let Some((head, body, make)) = definition_parts(statement) else { return vec![] };
	let Some((function, parameters)) = signature(&head) else { return vec![] };
	let Some(wanted) = copies.get(&function).cloned() else { return vec![] };
	wanted.iter().map(|positions| {
		let mut new_parameters = vec![Node::Symbol(bounds_name(&function, positions))];
		let mut new_body = body.clone();
		for (at, parameter) in parameters.iter().enumerate() {
			if !positions.contains(&at) {
				new_parameters.push(parameter.drop_meta().clone());
				continue;
			}
			let [start, end] = bound_names(&parameter.name()).map(Node::Symbol);
			let range = Node::List(vec![key(start.clone(), Op::Range, end.clone())], Bracket::Round, Separator::None);
			new_body = substitute(new_body, &parameter.name(), &range);
			new_parameters.extend([start, end]);
		}
		make(Node::List(new_parameters, Bracket::Round, Separator::None), call_bounds_copies(new_body, readers, copies))
	}).collect()
}

/// Per function defined once whose value is a range of its parameters (`f(n) := 1..n`, `upto(a, b) := { a to b }`): its
/// parameters and that range
type RangeProducers = HashMap<String, Produced>;
/// A producer's parameters and the range its body is
type Produced = (Vec<String>, Node);

fn range_producers(program: &Node) -> RangeProducers {
	let mut definitions: HashMap<String, Vec<Option<Produced>>> = HashMap::new();
	program.visit(&mut |part| {
		let Some((head, body, _)) = definition_parts(part) else { return };
		let Some((name, parameters)) = signature(&head) else { return };
		let parameters: Option<Vec<String>> = parameters.iter().map(|parameter| match parameter.drop_meta() {
			Node::Symbol(parameter) => Some(parameter.clone()),
			_ => None,
		}).collect();
		let produced = parameters.and_then(|parameters| {
			let range = produced_range(&body)?;
			symbols(&range).iter().all(|symbol| parameters.contains(symbol)).then_some((parameters, range))
		});
		definitions.entry(name).or_default().push(produced);
	});
	definitions.into_iter().filter_map(|(name, mut found)| (found.len() == 1).then(|| found.remove(0)).flatten().map(|produced| (name, produced))).collect()
}

/// The range a body is: `1..n`, `(1..n)`, `{ 1..n }`, with plain bounds
fn produced_range(body: &Node) -> Option<Node> {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) if items.len() == 1 => produced_range(&items[0]),
		other => range_of(other).map(|range| range.whole),
	}
}

/// Each call of a range producer with plain arguments (`count f(10^12)`, `r = f(n)`) is its range, the parameters
/// replaced by the arguments; definition heads stay
fn inline_produced_ranges(node: Node, producers: &RangeProducers) -> Node {
	if producers.is_empty() {
		return node;
	}
	if let Some((head, body, make)) = definition_parts(&node) {
		return make(head, inline_produced_ranges(body, producers));
	}
	let node = node.map_children(|child| inline_produced_ranges(child, producers));
	let Node::List(items, bracket, separator) = node.drop_meta() else { return node };
	let Some((parameters, range)) = call_name(items, bracket, separator).and_then(|name| producers.get(name)) else { return node };
	// `f(g())` is `(f (g))`: one argument, the call, not the arguments of f
	let called = |argument: &Node| matches!(argument.drop_meta(), Node::List(inner, bracket, separator) if call_name(inner, bracket, separator).is_some());
	let arguments = if items.len() == 2 && called(&items[1]) { vec![&items[1]] } else { call_arguments(&items[1..]) };
	// an argument is evaluated where the range reads its parameter, maybe twice: numbers and variables only, no call
	let calls_nothing = |argument: &Node| {
		let mut calls = false;
		argument.visit(&mut |part| calls |= called(part));
		!calls
	};
	if arguments.len() != parameters.len() || !arguments.iter().all(|argument| plain_bound(argument) && calls_nothing(argument)) {
		return node;
	}
	let bindings: HashMap<String, Node> = parameters.iter().cloned().zip(arguments.into_iter().map(|argument| argument.drop_meta().clone())).collect();
	Node::List(vec![crate::law::substitute(range, &bindings)], Bracket::Round, Separator::None)
}
