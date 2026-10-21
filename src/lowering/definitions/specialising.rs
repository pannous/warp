//! A function passed by name to a higher-order function gets a copy of that function made for it, which calls it by
//! name: `each(f, n) := { … f(i, i + 1) … }; each(ink, 3000000)` adds `each·ink(n) := { … ink(i, i + 1) … }` and calls
//! `each·ink(3000000)`. Called as a value its parameters would lose their types (a closure call, 2–3× slower).
//! Only top-level functions defined once, whose parameter is only called or passed on, and only names no assignment,
//! parameter or one-parameter lambda binds too (a loop variable or a destructured name of the same name is not looked
//! for: a call's head is the function anyway); a copy's own calls are specialised in turn (`fold(f, xs) := … fold(f,
//! rest)`), up to a cap. A library word the program doesn't bind itself and an operator are passed so too:
//! `each(abs, n)` adds `each·abs(n)`, which calls abs as written code does, `fold(+, xs)` `fold·+(xs)`, which adds
//! (card specialise-library-functions). In a run an original left uncalled is dropped; a module for another host keeps
//! it exported (decision exports-kept, card specialise-dead-originals).

use super::nodes::{children_rewritten, function_head, key};
use crate::lowering::function_values::ITERATION_WORDS;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// The most copies one program gets
const MAX_COPIES: usize = 64;
const NAME_SEPARATOR: &str = "·";

#[derive(Clone)]
struct Definition {
	name: String,
	params: Vec<String>,
	body: Node,
	op: Op,
}

impl Definition {
	fn from(node: &Node) -> Option<Definition> {
		let Node::Key(head, op @ (Op::Define | Op::Assign), body) = node.drop_meta() else {
			return None;
		};
		let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else {
			return None;
		};
		let (name, params) = items.split_first()?;
		let params = params.iter().map(|param| param.symbol_name().map(String::from)).collect::<Option<_>>()?;
		Some(Definition { name: name.symbol_name()?.to_string(), params, body: body.as_ref().clone(), op: *op })
	}

	fn node(&self) -> Node {
		let head = [Node::Symbol(self.name.clone())].into_iter().chain(self.params.iter().cloned().map(Node::Symbol));
		key(Node::List(head.collect(), Bracket::Round, Separator::None), self.op, self.body.clone())
	}

	/// The copy of this function for the function `passed` given as its parameter at `index`
	fn specialised(&self, index: usize, passed: &Passed) -> Definition {
		let mut params = self.params.clone();
		let param = params.remove(index);
		Definition {
			name: copy_name(&self.name, &passed.name),
			params,
			body: passed.put_in(self.body.clone(), &param),
			op: self.op,
		}
	}

	/// Is the parameter at `index` one the body only calls or passes on to where a function is taken: a parameter of
	/// `function_parameters` or an iteration word's argument (`type(f)` takes it as a value)
	fn takes_function_at(&self, index: usize, function_parameters: &FunctionParameters) -> bool {
		let mut used = false;
		let mut other_use = false;
		visit_uses(&self.body, &self.params[index], &mut |use_| match use_ {
			Use::Called => used = true,
			Use::Passed { callee, position }
				if ITERATION_WORDS.contains(&callee)
					|| function_parameters.contains(&(callee.to_string(), position)) =>
			{
				used = true
			}
			Use::Passed { .. } | Use::Other => other_use = true,
		});
		used && !other_use
	}
}

/// What a copy is made for, and how it calls it
enum Calling {
	/// a function of the program, by name
	Function,
	/// a library word, as the lambda it would become calls it (function_values.rs library_word_lambda): `abs x` an
	/// operator's
	LibraryWord,
	/// `+`: `a + b`
	Operator(Op),
}

/// A function given as an argument: its name in the copy's, the node passing it on, how it is called
struct Passed {
	name: String,
	argument: Node,
	calling: Calling,
}

impl Passed {
	/// The body with each call `param(a, b)` the passed function's, any other use (passing it on) the argument
	fn put_in(&self, node: Node, param: &str) -> Node {
		match node {
			Node::Symbol(name) if name == param => self.argument.clone(),
			Node::List(items, Bracket::Round, Separator::None)
				if items.first().is_some_and(|head| head.is_symbol(param)) =>
			{
				let arguments: Vec<Node> = items.into_iter().skip(1).map(|item| self.put_in(item, param)).collect();
				self.called(arguments)
			}
			other => children_rewritten(other, |child| self.put_in(child, param)),
		}
	}

	fn called(&self, mut arguments: Vec<Node>) -> Node {
		match self.calling {
			Calling::Function => {
				let head = Node::Symbol(self.name.clone());
				Node::List([vec![head], arguments].concat(), Bracket::Round, Separator::None)
			}
			Calling::LibraryWord => crate::closures::word_applied(&self.name, arguments),
			Calling::Operator(op) if arguments.len() == 2 => {
				let right = arguments.pop().expect("two arguments");
				key(arguments.pop().expect("two arguments"), op, right)
			}
			// a binary operator called with another count, as its closure would be: refused alike
			Calling::Operator(_) => {
				Node::List([vec![self.argument.clone()], arguments].concat(), Bracket::Round, Separator::None)
			}
		}
	}
}

/// `+` given as a function, the parser's operator with nothing on either side (`(+ ø ø)`)
fn binary_operator(argument: &Node) -> Option<Op> {
	let Node::Key(left, op, right) = argument.drop_meta() else {
		return None;
	};
	(left.is_nothing() && right.is_nothing() && crate::lambdas::operator_value_text(*op).is_some()).then_some(*op)
}

/// `abs` given as a function, the parser's bare operator `‖` that ambiguous_forms.rs operator_values made the lambda
/// `value => ‖value`: its word
fn operator_word(argument: &Node) -> Option<&'static str> {
	let Node::Key(parameter, Op::Arrow, body) = argument.drop_meta() else {
		return None;
	};
	let Node::Key(nothing, op, operand) = body.drop_meta() else {
		return None;
	};
	let applied =
		nothing.is_nothing() && operand.drop_meta() == parameter.drop_meta() && parameter.symbol_name().is_some();
	let words = crate::warp_parser::PREFIX_OPERATOR_WORDS.iter();
	words.filter(|_| applied).find(|(_, word_op)| word_op == op).map(|(word, _)| *word)
}

fn copy_name(function: &str, passed: &str) -> String {
	// the program's `each·any` beside the library's each copies to `each·ink`, the name as written
	format!("{}{NAME_SEPARATOR}{passed}", crate::lowering::typed_library_words::written(function))
}

enum Use<'a> {
	Called,
	/// as the argument at `position` of a call of `callee`
	Passed {
		callee: &'a str,
		position: usize,
	},
	Other,
}

/// The functions and positions of their parameters that take a function
type FunctionParameters = HashSet<(String, usize)>;

/// The parameters of `definitions` that take a function: those only called or passed on to one that does, the most
/// that hold together (a recursive `fold(f, xs) := … fold(f, rest)` passes f on to itself)
fn function_parameters(definitions: &[&Definition]) -> FunctionParameters {
	let mut found: FunctionParameters = definitions
		.iter()
		.flat_map(|definition| (0..definition.params.len()).map(|index| (definition.name.clone(), index)))
		.collect();
	loop {
		let kept: FunctionParameters = definitions
			.iter()
			.flat_map(|definition| (0..definition.params.len()).map(move |index| (definition, index)))
			.filter(|(definition, index)| definition.takes_function_at(*index, &found))
			.map(|(definition, index)| (definition.name.clone(), index))
			.collect();
		if kept.len() == found.len() {
			return kept;
		}
		found = kept;
	}
}

fn visit_uses<'a>(node: &'a Node, name: &str, found: &mut dyn FnMut(Use<'a>)) {
	match node.drop_meta() {
		Node::Symbol(word) if word == name => found(Use::Other),
		Node::List(items, Bracket::Round, Separator::None) if items.first().and_then(Node::symbol_name).is_some() => {
			let callee = items[0].symbol_name().expect("a named call");
			if callee == name {
				found(Use::Called);
			}
			for (position, argument) in items[1..].iter().enumerate() {
				match argument.drop_meta() {
					Node::Symbol(word) if word == name => found(Use::Passed { callee, position }),
					_ => visit_uses(argument, name, found),
				}
			}
		}
		Node::Key(left, _, right) => {
			visit_uses(left, name, found);
			visit_uses(right, name, found);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| visit_uses(item, name, found)),
		_ => {}
	}
}

/// Bound other than by a definition: assigned a value, a parameter, a loop variable
fn bound_as_value(bindings: &crate::program_names::Bindings, name: &str) -> bool {
	use crate::program_names::Meaning::{Loop, Parameter, Variable};
	[Variable, Parameter, Loop].into_iter().any(|meaning| bindings.binds_as(name, meaning))
}

struct Specialiser {
	definitions: HashMap<String, Definition>,
	function_parameters: FunctionParameters,
	/// The functions a call may pass by name: defined once at the top level, their names bound nowhere else
	passable: HashSet<String>,
	/// Every name the program defines or binds: a library word of such a name is not the library's
	own_names: HashSet<String>,
	/// The copies made, in order, each after the function it copies
	copies: Vec<(String, Definition)>,
}

impl Specialiser {
	fn rewrite(&mut self, node: Node) -> Node {
		match node {
			Node::List(items, Bracket::Round, Separator::None) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.rewrite(item)).collect();
				self.specialised_call(items)
			}
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}

	/// `each(ink, n)` as `each·ink(n)`, making that copy once
	fn specialised_call(&mut self, mut items: Vec<Node>) -> Node {
		loop {
			let Some((index, copy)) = self.copy_for(&items) else {
				return Node::List(items, Bracket::Round, Separator::None);
			};
			items.remove(index + 1);
			items[0] = Node::Symbol(copy);
		}
	}

	fn copy_for(&mut self, items: &[Node]) -> Option<(usize, String)> {
		let (head, arguments) = items.split_first()?;
		let function = self.definitions.get(head.symbol_name()?)?.clone();
		if arguments.len() != function.params.len() {
			return None;
		}
		let (index, passed) = arguments.iter().enumerate().find_map(|(index, argument)| {
			let passed = self.passed(argument)?;
			let free = !function.params.iter().any(|param| *param == passed.name);
			(free && function.takes_function_at(index, &self.function_parameters)).then_some((index, passed))
		})?;
		let name = copy_name(&function.name, &passed.name);
		if !self.definitions.contains_key(&name) {
			if self.copies.len() >= MAX_COPIES {
				return None;
			}
			let copy = function.specialised(index, &passed);
			let original = self.original_of(&function.name);
			self.definitions.insert(name.clone(), copy.clone());
			self.copies.push((original, copy));
		}
		Some((index, name))
	}

	/// The argument as a function a copy can call: a function of the program, a library word or an operator
	fn passed(&self, argument: &Node) -> Option<Passed> {
		if let Some(op) = binary_operator(argument) {
			let text = crate::lambdas::operator_value_text(op)?;
			return Some(Passed { name: text.to_string(), argument: argument.clone(), calling: Calling::Operator(op) });
		}
		let name = argument.symbol_name().or_else(|| operator_word(argument))?;
		let calling = if self.passable.contains(name) {
			Calling::Function
		} else if !self.own_names.contains(name)
			&& crate::lowering::function_values::library_function_arity(name).is_some()
		{
			Calling::LibraryWord
		} else {
			return None;
		};
		Some(Passed { name: name.to_string(), argument: argument.clone(), calling })
	}

	/// The top-level function a copy (of a copy) was made from: its copies follow it in the program
	fn original_of(&self, name: &str) -> String {
		self.copies
			.iter()
			.find(|(_, copy)| copy.name == name)
			.map_or(name.to_string(), |(original, _)| original.clone())
	}
}

/// The statements of a statement list, those of the lists nested in it too (`f(x) := …; g(x) := …` on one line); a
/// block's own definitions stay its own
fn statements_of(node: &Node) -> Vec<&Node> {
	match node.drop_meta() {
		Node::List(items, bracket, Separator::Semicolon | Separator::Newline) if *bracket != Bracket::Curly => {
			items.iter().flat_map(statements_of).collect()
		}
		_ => vec![node],
	}
}

/// The originals nothing names any more but their own definitions: every call of them was specialised
fn uncalled_originals(program: &Node, copies: &[(String, Definition)]) -> HashSet<String> {
	let originals: HashSet<&String> = copies.iter().map(|(original, _)| original).collect();
	let statements = statements_of(program);
	let named_elsewhere = |name: &str| {
		let own = |statement: &&Node| Definition::from(statement).is_some_and(|definition| definition.name == name);
		statements.iter().filter(|statement| !own(statement)).any(|statement| statement.mentions(name))
			|| copies.iter().any(|(_, copy)| copy.body.mentions(name))
	};
	originals.into_iter().filter(|name| !named_elsewhere(name)).cloned().collect()
}

/// Each copy right after the definition of the function it copies, which goes when it is `dropped`
fn with_copies(node: Node, copies: &[(String, Definition)], dropped: &HashSet<String>) -> Node {
	match node {
		Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline))
			if bracket != Bracket::Curly =>
		{
			let mut statements = vec![];
			for item in items {
				let defined = Definition::from(&item).map(|definition| definition.name);
				if !defined.as_ref().is_some_and(|name| dropped.contains(name)) {
					statements.push(with_copies(item, copies, dropped));
				}
				if let Some(name) = defined {
					let made = copies.iter().filter(|(original, _)| *original == name);
					statements.extend(made.map(|(_, copy)| copy.node()));
				}
			}
			Node::List(statements, bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_copies(*node, copies, dropped)), data },
		other => other,
	}
}

pub fn lower(program: Node) -> Node {
	let statements = statements_of(&program);
	let found: Vec<Definition> = statements.iter().copied().filter_map(Definition::from).collect();
	// every definition of a name counts, a method's typed one too (`take_while(self:Cursor, ok)` beside list's)
	let mut counts: HashMap<&str, usize> = HashMap::new();
	for head in statements.iter().copied().filter_map(function_head) {
		*counts.entry(head.name).or_default() += 1;
	}
	let bindings = crate::program_names::Bindings::of(&program);
	let once: Vec<&Definition> =
		found.iter().filter(|definition| counts.get(definition.name.as_str()) == Some(&1)).collect();
	let passable =
		once.iter().map(|definition| definition.name.clone()).filter(|name| !bound_as_value(&bindings, name)).collect();
	let own_names = counts.keys().map(|name| name.to_string()).chain(bindings.bound()).collect();
	let function_parameters = function_parameters(&once);
	let higher_order: HashMap<String, Definition> = once
		.into_iter()
		.filter(|definition| function_parameters.iter().any(|(name, _)| *name == definition.name))
		.map(|definition| (definition.name.clone(), definition.clone()))
		.collect();
	if higher_order.is_empty() {
		return program;
	}
	let mut specialiser =
		Specialiser { definitions: higher_order, function_parameters, passable, own_names, copies: vec![] };
	let rewritten = specialiser.rewrite(program.clone());
	if specialiser.copies.is_empty() {
		return program;
	}
	// the copies' bodies are rewritten until no new copy appears (a copy calling itself, or another function by name)
	let mut done = 0;
	while done < specialiser.copies.len() {
		let (original, copy) = specialiser.copies[done].clone();
		let body = specialiser.rewrite(copy.body);
		specialiser.copies[done] = (original, Definition { body, ..copy });
		done += 1;
	}
	let dropped = match crate::pipeline::compile_mode().keeps_exports() {
		true => HashSet::new(),
		false => uncalled_originals(&rewritten, &specialiser.copies),
	};
	with_copies(rewritten, &specialiser.copies, &dropped)
}
