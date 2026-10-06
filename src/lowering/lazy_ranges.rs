//! A range is a descriptor, not its list (card lazy-range, notes/lazy_ranges.md): `count`, `#`, `sum` and indexing of
//! a range of plain bounds (integer literals or variables) are worked out from the bounds, and a range variable that is
//! never changed is replaced by its range wherever it is used, so `xs = 1..100001; xs#5` builds no list. `for`, `map`,
//! `filter` and the other iteration words already loop over a range's bounds (for_loop.rs, lambdas.rs). Any other use
//! (printing, passing it on, changing it) keeps the variable a list, collected once (declaration_lowering.rs).

use crate::analyzer::{extract_user_functions, TEMPORARY_SEPARATOR};
use crate::context::Context;
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;

const COUNT: &str = "count";
const SUM: &str = "sum";
/// Words a range variable may stand behind a dot for: they read the range without collecting it
const READING_METHODS: [&str; 10] = [COUNT, SUM, "map", "filter", "each", "fold", "find", "any", "all", "reduce"];
const START: &str = "range_start";
const END: &str = "range_end";
const INDEX: &str = "range_index";
const WHOLE: &str = "range_whole";
const LENGTH: &str = "range_length";
const LENGTH_TEMPLATE: &str = "(if range_end - range_start > 0 then range_end - range_start else 0)";
const INDEX_TEMPLATE: &str = "if range_index >= 1 and range_index <= range_length then range_start + range_index - 1 else range_whole#range_index";
const SUM_TEMPLATE: &str = "range_length * (2 * range_start + range_length - 1) / 2";

pub fn lower(node: Node) -> Node {
	if !has_range(&node) {
		return node;
	}
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let own = |word: &str| context.user_functions.contains_key(word);
	let mut lowering = Lowering { counts: !own(COUNT), sums: !own(SUM), temporaries: 0 };
	lowering.rewrite(replace_range_variables(node))
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

fn plain_bound(bound: &Node) -> bool {
	matches!(bound.drop_meta(), Node::Symbol(_) | Node::Number(crate::extensions::numbers::Number::Int(_)))
}

fn range_of(node: &Node) -> Option<Range> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => range_of(&items[0]),
		Node::Key(start, op @ (Op::Range | Op::To), end) if plain_bound(start) && plain_bound(end) => {
			// `a to b` ends after b: its end is b + 1
			let end = if *op == Op::To { add_one(end) } else { end.drop_meta().clone() };
			Some(Range { start: start.drop_meta().clone(), end, whole: node.drop_meta().clone() })
		}
		_ => None,
	}
}

fn add_one(bound: &Node) -> Node {
	match bound.drop_meta() {
		Node::Number(crate::extensions::numbers::Number::Int(n)) => Node::int(n + 1),
		other => Node::Key(Box::new(other.clone()), Op::Add, Box::new(Node::int(1))),
	}
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == word)
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
			Node::List(items, _, separator) if is_call(items, separator) && self.counts && is_word(&items[0], COUNT) => Some(self.length(&range_of(&items[1])?)),
			Node::List(items, _, separator) if is_call(items, separator) && self.sums && is_word(&items[0], SUM) => Some(self.sum(&range_of(&items[1])?)),
			Node::Key(empty, Op::Hash, counted) if matches!(empty.drop_meta(), Node::Empty) => Some(self.length(&range_of(counted)?)),
			Node::Key(range, Op::Dot, word) if self.counts && is_word(word, COUNT) => Some(self.length(&range_of(range)?)),
			Node::Key(range, Op::Dot, word) if self.sums && is_word(word, SUM) => Some(self.sum(&range_of(range)?)),
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
		let binding = Node::Key(Box::new(temporary.clone()), Op::Assign, Box::new(value));
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
fn replace_range_variables(node: Node) -> Node {
	let node = node.map_children(replace_range_variables);
	let Node::List(statements, bracket, separator @ (Separator::Semicolon | Separator::Newline)) = node else { return node };
	// `{a = 1..2000; b = #a}` ending in a field is an object: each of its assignments is a field it keeps
	if bracket == Bracket::Curly && statements.last().is_some_and(|last| matches!(last.drop_meta(), Node::Key(_, Op::Assign | Op::Define | Op::Colon, _))) {
		return Node::List(statements, bracket, separator);
	}
	let mut statements = statements;
	let mut position = 0;
	while position + 1 < statements.len() {
		match replaceable(&statements, position) {
			Some((name, range)) => {
				statements.remove(position);
				for statement in &mut statements[position..] {
					*statement = substitute(std::mem::replace(statement, Node::Empty), &name, &range);
				}
			}
			None => position += 1,
		}
	}
	Node::List(statements, bracket, separator)
}

/// The variable and range statement `position` assigns, when every other statement only reads it as a range
fn replaceable(statements: &[Node], position: usize) -> Option<(String, Node)> {
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
	let read_only = before.iter().all(|statement| !statement.mentions_any(&[name])) && after.iter().all(|statement| only_read(statement, name));
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
		found |= changing && is_word(target, name);
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
/// word; never inside a function definition, which would read it as a global
fn only_read(node: &Node, name: &str) -> bool {
	let is_name = |node: &Node| is_word(node, name);
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol != name,
		Node::List(items, _, separator) if is_call(items, separator) && is_name(&items[1]) && (is_word(&items[0], COUNT) || is_word(&items[0], SUM)) => true,
		Node::List(items, _, _) => items.iter().enumerate().all(|(at, item)| (is_name(item) && at > 0 && is_word(&items[at - 1], "in")) || only_read(item, name)),
		Node::Key(empty, Op::Hash, counted) if matches!(empty.drop_meta(), Node::Empty) && is_name(counted) => true,
		Node::Key(list, Op::Hash, index) if is_name(list) => is_position(index) && only_read(index, name),
		Node::Key(list, Op::Dot, method) if is_name(list) => reading_method(method) && only_read(method, name),
		Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(..)) => !body.mentions_any(&[name]),
		Node::Key(left, _, right) => only_read(left, name) && only_read(right, name),
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
