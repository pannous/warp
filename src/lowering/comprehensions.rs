//! `[x * x for x in 1..10]` and `[x for x in xs if x % 2 == 0]`: a list built by a loop,
//! `(made = []; for x in xs { if c { made.push(x * x) } }; made)`. Lowered first, so the loop and the push go through
//! every later pass like written ones. `xs where it > 1` filters like `[it for it in xs if it > 1]`.

use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
const IF_WORD: &str = "if";
const WHERE_WORD: &str = "where";
/// The element a `where` condition reads as `it`
const WHERE_ELEMENT: &str = "where·element";
pub(crate) const MADE: &str = "comprehension_list";
const VARIABLE: &str = "comprehension_variable";
const SEQUENCE: &str = "comprehension_sequence";
const CONDITION: &str = "comprehension_condition";
const ELEMENT: &str = "comprehension_element";

pub fn lower(node: Node) -> Node {
	Lowering { count: Cell::new(0) }.lower(node)
}

/// `xs where it > 1` as the comprehension `[it for it in xs if it > 1]`; before welcome_forms reads `xs where …` as
/// juxtaposed words. A condition not reading `it` is an error: it would filter nothing it can name.
pub fn lower_where(node: Node) -> Node {
	let node = where_reassociated(node);
	let mut lists = Lists::of(&node);
	let filtered = where_filters(node, &mut lists);
	crate::database_tables::with_filter_functions(filtered, lists.tables)
}

/// What a filter's condition may name besides `it`: the fields of each element of a list declared of a class
/// (`people: [Person]`), and the variables, which win over a field of the same name (P223)
struct Lists {
	fields: HashMap<String, Vec<String>>,
	variables: HashSet<String>,
	tables: crate::database_tables::Tables,
}

impl Lists {
	fn of(program: &Node) -> Self {
		let classes: HashMap<String, Vec<String>> = crate::database_tables::class_bodies(program).into_iter()
			.map(|(class, body)| (class, crate::class_methods::field_declarations(&body).into_iter().map(|(field, _, _)| field).collect())).collect();
		let (mut fields, mut variables) = (HashMap::new(), HashSet::new());
		program.visit(&mut |node| match node {
			Node::Key(declared, Op::Colon, list_type) => {
				if let (Node::Symbol(list), Node::List(element, Bracket::Square, _)) = (declared.drop_meta(), list_type.drop_meta()) {
					if let Some(class_fields) = element.first().filter(|_| element.len() == 1).and_then(|class| classes.get(&class.drop_meta().name())) {
						fields.insert(list.clone(), class_fields.clone());
					}
				}
			}
			Node::Key(target, Op::Assign | Op::Define, _) => variables.extend(bound_names(target)),
			Node::List(words, _, _) if words.first().is_some_and(|word| is_word(word, FOR_WORD)) => variables.extend(words.get(1).map(Node::name)),
			_ => {}
		});
		Lists { fields, variables, tables: crate::database_tables::registered(program) }
	}

	/// `age > 20` of `people where …` as `it.age > 20` where each person has a field age and no variable age is bound
	fn with_fields_of_it(&self, subject: &Node, condition: Node) -> Node {
		let Some(fields) = self.fields.get(&subject.drop_meta().name()) else { return condition };
		fields_of_it(condition, fields, &self.variables)
	}
}

/// The names a definition or assignment binds: `x`, `x: int`, the parameters of `f(a, b: int)`
fn bound_names(target: &Node) -> Vec<String> {
	match target.drop_meta() {
		Node::Symbol(name) => vec![name.clone()],
		Node::Key(name, Op::Colon, _) => bound_names(name),
		Node::List(items, _, _) => items.iter().skip(1).flat_map(bound_names).collect(),
		_ => vec![],
	}
}

fn fields_of_it(condition: Node, fields: &[String], variables: &HashSet<String>) -> Node {
	match condition {
		Node::Symbol(name) if fields.contains(&name) => {
			let field_of_it = Node::Key(Box::new(Node::Symbol(crate::lambdas::IMPLICIT_PARAMETER.to_string())), Op::Dot, Box::new(Node::Symbol(name.clone())));
			if !variables.contains(&name) {
				return field_of_it;
			}
			let warning = format!("{name} is the variable {name} here, not the field of each element: write {} for the field", field_of_it.serialize());
			match crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(&Node::Symbol(name.clone()), warning)]) {
				Ok(()) => Node::Symbol(name),
				Err(error) => error,
			}
		}
		// `it.age`, `name.reverse()`: what follows a dot is no variable
		Node::Key(left, Op::Dot, right) => Node::Key(Box::new(fields_of_it(*left, fields, variables)), Op::Dot, right),
		Node::Meta { node, data } => Node::Meta { node: Box::new(fields_of_it(*node, fields, variables)), data },
		other => other.map_children(|child| fields_of_it(child, fields, variables)),
	}
}

fn where_filters(node: Node, lists: &mut Lists) -> Node {
	if let Node::List(items, bracket, separator) = node.drop_meta() {
		if let Some(filtered_loop) = loop_over_filtered(items, lists) {
			return Node::List(filtered_loop, bracket.clone(), separator.clone());
		}
	}
	let node = match node {
		// a method's filter as a function's (a class body is no child of map_children)
		Node::Type { name, body } => return Node::Type { name, body: Box::new(where_filters(*body, lists)) },
		other => other.map_children(|child| where_filters(child, lists)),
	};
	let Node::List(items, bracket, separator) = node else { return node };
	let Some(at) = where_position(&items) else { return Node::List(items, bracket, separator) };
	let filtered = match items[at + 1].drop_meta() {
		// Haskell's binding `x * 2 where x = 3`: the assignment, then the expression
		Node::Key(_, Op::Assign, _) => Node::List(vec![items[at + 1].clone(), items[at - 1].clone()], Bracket::Round, Separator::Semicolon),
		_ => {
			let condition = lists.with_fields_of_it(&items[at - 1], items[at + 1].clone());
			match crate::database_tables::queried(&items[at - 1], &condition, &lists.variables, &mut lists.tables) {
				Some(Ok(found) | Err(found)) => found,
				None => where_comprehension(&items[at - 1], &condition),
			}
		}
	};
	// `xs where c` alone, or the last argument of a call `count(xs where c)`
	if at == 1 && bracket == Bracket::None {
		return filtered;
	}
	let mut items = items;
	items.truncate(at - 1);
	items.push(filtered);
	Node::List(items, bracket, separator)
}

/// `for p in people where p.age > 18 { … }` walks `people where it.age > 18`, the loop variable read as `it` (a
/// table's filter stays its SQL)
fn loop_over_filtered(items: &[Node], lists: &mut Lists) -> Option<Vec<Node>> {
	// `for x in (xs where c) {…}` as the parser groups the filter
	let items = crate::list_phrases::words(items);
	let [for_word, variable, in_word, sequence, where_word, condition, body] = items.as_slice() else { return None };
	if !is_word(for_word, FOR_WORD) || !is_word(in_word, IN_WORD) || !is_word(where_word, WHERE_WORD) || !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return None;
	}
	let Node::Symbol(name) = variable.drop_meta() else { return None };
	let condition = substitute(condition.clone(), name, &Node::Symbol(crate::lambdas::IMPLICIT_PARAMETER.to_string()));
	let filter = Node::List(vec![sequence.clone(), where_word.clone(), condition], Bracket::None, Separator::Space);
	Some(vec![for_word.clone(), variable.clone(), in_word.clone(), where_filters(filter, lists), where_filters(body.clone(), lists)])
}

/// `[it, (for it in xs if), condition]`, as the parser groups a comprehension
fn where_comprehension(subject: &Node, condition: &Node) -> Node {
	if !crate::lambdas::mentions(condition, crate::lambdas::IMPLICIT_PARAMETER) {
		let subject = subject.serialize();
		let field_hint = field_condition(condition).map(|(field, written)| format!(", or {subject} where {written} for a field {field} of each element")).unwrap_or_default();
		return crate::node::error(&format!("`{subject} where {}` filters by each element `it`: write {subject} where it > 1{field_hint}", condition.serialize()));
	}
	// a name of its own: in a function of one parameter `it` is that parameter
	let element = Node::Symbol(WHERE_ELEMENT.to_string());
	let condition = substitute(condition.clone(), crate::lambdas::IMPLICIT_PARAMETER, &element);
	let word = |word: &str| Node::Symbol(word.to_string());
	let clause = Node::List(vec![word(FOR_WORD), element.clone(), word(IN_WORD), subject.clone(), word(IF_WORD)], Bracket::None, Separator::Space);
	Node::List(vec![element, clause, condition], Bracket::Square, Separator::Space)
}

/// The field `country` a condition `country is germany` may mean of each element, with the condition written so:
/// `it.country==germany`
fn field_condition(condition: &Node) -> Option<(String, String)> {
	let Node::Key(field, op, value) = condition.drop_meta() else { return None };
	let Node::Symbol(name) = field.drop_meta() else { return None };
	let it = Node::Symbol(crate::lambdas::IMPLICIT_PARAMETER.to_string());
	let field_of_it = Node::Key(Box::new(it), Op::Dot, field.clone());
	Some((name.clone(), Node::Key(Box::new(field_of_it), *op, value.clone()).serialize()))
}

/// `xs where it > 1` parses as `(xs where it) > 1`: the operators right of `where` join its condition,
/// `[xs, where, it > 1]`, so the filter takes the whole condition (`it > 1 and it < 5` too)
fn where_reassociated(node: Node) -> Node {
	match where_flattened(node) {
		Node::Key(left, op, right) if !matches!(op, Op::Assign | Op::Define | Op::Colon) && !op.is_compound_assign() => {
			let (left, right) = (where_reassociated(*left), where_reassociated(*right));
			match left.drop_meta() {
				// a parenthesized filter is closed: `(xs where it > 0).slice(0, 1)` slices the filtered list
				Node::List(items, bracket, separator) if *bracket != Bracket::Round && where_position(items).is_some() => {
					let mut items = items.clone();
					let condition = items.pop().expect("where has a condition");
					items.push(Node::Key(Box::new(condition), op, Box::new(right)));
					Node::List(items, bracket.clone(), separator.clone())
				}
				_ => Node::Key(Box::new(left), op, Box::new(right)),
			}
		}
		Node::Type { name, body } => Node::Type { name, body: Box::new(where_reassociated(*body)) },
		other => other.map_children(where_reassociated),
	}
}

/// `[[xs, where], it]` and `[xs, [where, 4 + it]]`, as the right side of an assignment parses, as `[xs, where, it]`
fn where_flattened(node: Node) -> Node {
	let Node::List(items, bracket, separator) = node else { return node };
	let words_of = |item: Option<&Node>| match item.map(Node::drop_meta) {
		Some(Node::List(inner, Bracket::None, _)) => Some(inner.clone()),
		_ => None,
	};
	let flat = match (words_of(items.first()), words_of(items.last())) {
		(Some(inner), _) if inner.last().is_some_and(|word| is_word(word, WHERE_WORD)) => inner.into_iter().chain(items[1..].iter().cloned()).collect(),
		(_, Some(inner)) if items.len() > 1 && inner.len() == 2 && is_word(&inner[0], WHERE_WORD) => items[..items.len() - 1].iter().cloned().chain(inner).collect(),
		_ => items,
	};
	Node::List(flat, bracket, separator)
}

/// The position of `where` in `[… subject, where, condition]`
fn where_position(items: &[Node]) -> Option<usize> {
	(items.len() >= 3).then(|| items.len() - 2).filter(|&at| matches!(items[at].drop_meta(), Node::Symbol(symbol) if symbol == WHERE_WORD))
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == word)
}

/// The clause `for v in xs …`
fn starts_with_for(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(words, _, _) if words.first().is_some_and(|word| is_word(word, FOR_WORD)))
}

/// One node of several juxtaposed words: `upper w`
fn phrase(words: &[Node]) -> Node {
	match words {
		[single] => single.clone(),
		_ => Node::List(words.to_vec(), Bracket::None, Separator::Space),
	}
}

struct Lowering {
	count: Cell<usize>,
}

impl Lowering {
	fn lower(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.lower(item)).collect();
				match bracket {
					Bracket::Square => self.comprehension(&items).unwrap_or(Node::List(items, bracket, separator)),
					// Python's generator argument `sum(x * x for x in xs)`: the call of the comprehension's list
					Bracket::Round if separator == Separator::None && items.len() > 1 => match self.comprehension(&items[1..]) {
						Some(list) => Node::List(vec![items[0].clone(), list], bracket, separator),
						None => Node::List(items, bracket, separator),
					},
					_ => Node::List(items, bracket, separator),
				}
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.lower(*left)), op, Box::new(self.lower(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.lower(*node)), data },
			other => other,
		}
	}

	/// The items `[element, (for v in xs), …]` as the parser groups them; with a filter `(for v in xs if) condition`.
	/// An element or condition of several words is their phrase: `[upper w for w in words]`
	fn comprehension(&self, items: &[Node]) -> Option<Node> {
		let at = items.iter().position(starts_with_for).filter(|&at| at > 0)?;
		let (element, clause, rest) = (phrase(&items[..at]), &items[at], &items[at + 1..]);
		let Node::List(words, _, _) = clause.drop_meta() else { return None };
		let [_, variable, in_word, sequence, tail @ ..] = words.as_slice() else { return None };
		if !is_word(in_word, IN_WORD) || !matches!(variable.drop_meta(), Node::Symbol(_)) {
			return None;
		}
		let condition = match (tail, rest) {
			([], []) => None,
			([nothing], []) if matches!(nothing.drop_meta(), Node::Empty) => None,
			([filter_word], [_, ..]) if is_word(filter_word, IF_WORD) || is_word(filter_word, WHERE_WORD) => Some(phrase(rest)),
			_ => return None,
		};
		Some(self.built(variable, sequence, &element, condition.as_ref()))
	}

	/// `(made = []; for variable in sequence { if condition { made.push(element) } }; made)`
	fn built(&self, variable: &Node, sequence: &Node, element: &Node, condition: Option<&Node>) -> Node {
		let number = self.count.get();
		self.count.set(number + 1);
		let made = format!("{MADE}_{number}");
		let push = format!("{made}.push({ELEMENT})");
		let body = if condition.is_some() { format!("if {CONDITION} {{ {push} }}") } else { push };
		let template = format!("(var {made} = []; for {VARIABLE} in {SEQUENCE} {{ {body} }}; {made})");
		let mut program = parse(&template);
		for (placeholder, replacement) in [(VARIABLE, variable), (SEQUENCE, sequence), (ELEMENT, element)] {
			program = substitute(program, placeholder, replacement);
		}
		if let Some(condition) = condition {
			program = substitute(program, CONDITION, condition);
		}
		program
	}
}
