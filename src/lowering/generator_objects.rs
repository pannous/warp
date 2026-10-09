//! Generator objects (card generators-function): `counter = count_to(3)` whose variable is advanced by `next(counter)`
//! or `counter.next()`, and `iter(count_to(3))`, are resumable objects. Such a generator also becomes the class
//! `count_to·generator`: its parameters and locals are fields (each `any`), plus the state `generator·state`; its method
//! `next()` runs from where the last call stopped to the next yield and gives the yielded value, ø once the generator
//! ends. The body is cut into states at its yields: `while 1 { if generator·state == 0 { …; generator·state = 2;
//! continue }; …; return ø }`. A loop or an if that yields inside becomes jumps between states, a `for` loop is lowered
//! to its `while` first; every other statement stays as it is. A loop walks the object with next() (lower_iterators).
//! `send(v)` (card generators-send) resumes it like next() and gives `x = yield …` the value v where it resumes; next()
//! gives it ø.

use crate::for_loop::block_items;
use crate::generators::{assign, generators, yield_statement, holds_own, holds_stop, is_return, is_word, number, statements, symbol, symbol_name, yielded_value, Generator, BREAK_WORD, CONTINUE_WORD, FOR_WORD, NAME_SEPARATOR, NEXT_METHOD, RETURN_WORD};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::ruby_blocks::arguments;
use crate::wasm_emitter::{is_step, split_step};
use crate::warp_parser::{parse, while_do};
use std::collections::{BTreeSet, HashMap, HashSet};

/// `iter(count_to(3))` makes the object explicitly
pub(crate) const ITER_WORD: &str = "iter";
const CLASS_SUFFIX: &str = "generator";
const STATE_FIELD: &str = "generator·state";
/// The state of an ended generator: no state block matches it, `next()` gives ø
const DONE: i64 = -1;
/// What `send(v)` gives the generator, which `x = yield …` receives where it resumes (ø after `next()`)
const SENT_FIELD: &str = "generator·sent";
const SENT_VALUE: &str = "generator·value";
const SEND: &str = "send(VALUE) := { SENT = VALUE; self.next() }";
const FIELD_PLACEHOLDER: &str = "generator_field";
const FIELD_TYPE: &str = "any";

pub fn lower(node: Node) -> Node {
	let generators = generators(&node);
	if generators.is_empty() {
		return node;
	}
	let node = crate::generator_consumers::lower(node, &generators);
	let advanced = advanced_variables(&node);
	let mut classes = HashMap::new();
	let node = with_objects(node, &generators, &advanced, &mut classes);
	if classes.is_empty() {
		return node;
	}
	let mut items = match node {
		Node::List(items, Bracket::None | Bracket::Curly, Separator::Semicolon | Separator::Newline) => items,
		other => vec![other],
	};
	items.splice(0..0, classes.into_values());
	let node = statements(items, Bracket::None);
	crate::class_methods::lower(crate::generators::lower_iterators(node))
}

/// The variables `next(v)` or `v.next()` advances
pub(crate) fn advanced_variables(node: &Node) -> HashSet<String> {
	let mut advanced = HashSet::new();
	node.visit(&mut |part| match part {
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 && is_word(&items[0], NEXT_METHOD) => {
			advanced.extend(arguments(&items[1]).first().and_then(symbol_name).cloned());
		}
		Node::Key(object, Op::Dot, call) if is_next_call(call) => advanced.extend(symbol_name(object).cloned()),
		_ => {}
	});
	advanced
}

fn is_next_call(call: &Node) -> bool {
	matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1 && is_word(&items[0], NEXT_METHOD)) || is_word(call, NEXT_METHOD)
}

/// `g(args)` of a generator: its name and arguments; an argument `naturals()` is parsed as the bare name
pub(crate) fn generator_call<'a>(node: &Node, generators: &'a HashMap<String, Generator>) -> Option<(&'a String, &'a Generator, Vec<Node>)> {
	let (name, arguments) = match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, _) => items.split_first()?,
		Node::Symbol(_) => (node, &[][..]),
		_ => return None,
	};
	let (name, generator) = generators.get_key_value(symbol_name(name)?)?;
	let arguments: Vec<Node> = arguments.iter().flat_map(crate::ruby_blocks::arguments).collect();
	(arguments.len() == generator.parameters.len()).then_some((name, generator, arguments))
}

/// `v = g(args)` of an advanced v and `iter(g(args))` as the construction of g's object, `next(v)` as `v.next()`
fn with_objects(node: Node, generators: &HashMap<String, Generator>, advanced: &HashSet<String>, classes: &mut HashMap<String, Node>) -> Node {
	let node = node.map_children(|child| with_objects(child, generators, advanced, classes));
	let constructed = |call: &Node, classes: &mut HashMap<String, Node>| -> Option<Node> {
		let (name, generator, arguments) = generator_call(call, generators)?;
		if !classes.contains_key(name) {
			classes.insert(name.clone(), generator_class(name, generator)?);
		}
		Some(construction(name, generator, arguments))
	};
	let rebuilt = match node.drop_meta() {
		Node::Key(target, Op::Assign, value) if symbol_name(target).is_some_and(|name| advanced.contains(name)) => {
			constructed(value, classes).map(|object| assign(target.as_ref().clone(), object))
		}
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 && is_word(&items[0], ITER_WORD) => {
			arguments(&items[1]).first().and_then(|call| constructed(call, classes))
		}
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 && is_word(&items[0], NEXT_METHOD) => {
			let call = Node::List(vec![symbol(NEXT_METHOD)], Bracket::Round, Separator::None);
			arguments(&items[1]).first().map(|object| Node::Key(Box::new(object.clone()), Op::Dot, Box::new(call)))
		}
		_ => None,
	};
	match rebuilt {
		Some(rebuilt) => rebuilt.with_meta_of(&node),
		None => node,
	}
}

fn class_name(generator: &str) -> String {
	[generator, CLASS_SUFFIX].join(NAME_SEPARATOR)
}

/// `count_to·generator(args…, ø for each local, 0)`
fn construction(name: &str, generator: &Generator, arguments: Vec<Node>) -> Node {
	let locals = fields(generator).len() - generator.parameters.len();
	let values = arguments.into_iter().chain(std::iter::repeat_n(Node::Empty, locals)).chain([number(0), Node::Empty]);
	Node::List(std::iter::once(symbol(&class_name(name))).chain(values).collect(), Bracket::Round, Separator::None)
}

/// The fields in order: the parameters, the locals, (then the state and what was sent)
fn fields(generator: &Generator) -> Vec<String> {
	let mut locals: BTreeSet<String> = assigned_names(&machine_body(generator).unwrap_or(Node::Empty));
	locals.extend(generator.locals.iter().cloned());
	locals.remove(STATE_FIELD);
	locals.remove(SENT_FIELD);
	let parameters = generator.parameters.iter().cloned();
	parameters.clone().chain(locals.into_iter().filter(|local| !generator.parameters.contains(local))).collect()
}

/// The names a body assigns: the `for` lowering's `x·items` and `x·index` too
fn assigned_names(body: &Node) -> BTreeSet<String> {
	let mut names = BTreeSet::new();
	body.visit(&mut |part| if let Node::Key(target, op, _) = part {
		if *op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) {
			names.extend(symbol_name(target).cloned());
		}
	});
	names
}

/// `class count_to·generator { n: any; k: any; generator·state: any; next() := machine }`
fn generator_class(name: &str, generator: &Generator) -> Option<Node> {
	let body = machine_body(generator)?;
	let template = parse(&format!("class {FIELD_PLACEHOLDER} {{ {FIELD_PLACEHOLDER}: {FIELD_TYPE} }}"));
	let field_template = template_field(&template)?;
	let field = |field_name: &str| match field_template.drop_meta() {
		Node::Key(_, op, kind) => Node::Key(Box::new(symbol(field_name)), *op, kind.clone()),
		other => other.clone(),
	};
	let mut members: Vec<Node> = fields(generator).iter().map(|name| field(name)).collect();
	members.extend([field(STATE_FIELD), field(SENT_FIELD)]);
	let head = Node::List(vec![symbol(NEXT_METHOD)], Bracket::Round, Separator::None);
	members.push(Node::Key(Box::new(head), Op::Define, Box::new(statements(vec![body], Bracket::Curly))));
	members.extend(crate::generator_consumers::template(SEND, &[("VALUE", &symbol(SENT_VALUE)), ("SENT", &symbol(SENT_FIELD))]));
	Some(Node::Type { name: Box::new(symbol(&class_name(name))), body: Box::new(statements(members, Bracket::Curly)) })
}

fn template_field(template: &Node) -> Option<Node> {
	let mut found = None;
	template.visit(&mut |part| if let Node::Type { body, .. } = part {
		found = match body.drop_meta() {
			Node::List(items, _, _) => items.first().cloned(),
			single => Some(single.clone()),
		};
	});
	found
}

/// The jumps out of a loop being cut into states: `break` to after it, `continue` to its step or test
#[derive(Clone, Copy)]
struct Exits {
	after: usize,
	again: usize,
}

/// The states of a generator's body: the statements of each, ending in a jump or a return
struct Machine {
	states: Vec<Vec<Node>>,
}

fn state_field() -> Node {
	symbol(STATE_FIELD)
}

fn set_state(state: i64) -> Node {
	assign(state_field(), number(state))
}

fn condition_jump(condition: Node, state: usize) -> Node {
	let test = Node::Key(Box::new(Node::Empty), Op::If, Box::new(condition));
	Node::Key(Box::new(test), Op::Then, Box::new(statements(vec![set_state(state as i64), symbol(CONTINUE_WORD)], Bracket::Curly)))
}

fn returned(value: Node) -> Node {
	Node::List(vec![symbol(RETURN_WORD), value], Bracket::None, Separator::Space)
}

/// A sequence's statements: `a; b` of a body or of a lowered `for`
fn sequence_items(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None | Bracket::Round, Separator::Semicolon | Separator::Newline) => items.clone(),
		_ => block_items(node),
	}
}

impl Machine {
	fn state(&mut self) -> usize {
		self.states.push(vec![]);
		self.states.len() - 1
	}

	fn jump(&mut self, from: usize, to: usize) {
		self.states[from].extend([set_state(to as i64), symbol(CONTINUE_WORD)]);
	}

	fn compile(&mut self, statements: Vec<Node>, mut at: usize, exits: Option<Exits>) -> Option<usize> {
		for statement in statements {
			at = self.statement(statement, at, exits)?;
		}
		Some(at)
	}

	/// Where the code goes on after the statement
	fn statement(&mut self, statement: Node, at: usize, exits: Option<Exits>) -> Option<usize> {
		if let Some((values, target)) = yield_statement(&statement) {
			let resume = self.state();
			self.states[at].extend([set_state(resume as i64), returned(yielded_value(values))]);
			if let Some(target) = target {
				self.states[resume].extend([assign(target, symbol(SENT_FIELD)), assign(symbol(SENT_FIELD), Node::Empty)]);
			}
			return Some(resume);
		}
		if is_return(&statement) {
			self.states[at].extend([set_state(DONE), returned(Node::Empty)]);
			return Some(self.state());
		}
		if let Some(exits) = exits {
			for (word, target) in [(BREAK_WORD, exits.after), (CONTINUE_WORD, exits.again)] {
				if is_word(&statement, word) {
					self.jump(at, target);
					return Some(self.state());
				}
			}
		}
		let jumps_out = exits.is_some() && (holds_own(&statement, BREAK_WORD) || holds_own(&statement, CONTINUE_WORD));
		if !holds_stop(&statement) && !jumps_out {
			self.states[at].push(statement);
			return Some(at);
		}
		match statement.drop_meta() {
			Node::Key(head, Op::Do, body) => match head.drop_meta() {
				Node::Key(_, Op::While, condition) => self.while_loop(condition.as_ref().clone(), body, at),
				_ => None,
			},
			Node::Key(head, Op::Then, then) => {
				let Node::Key(_, Op::If, condition) = head.drop_meta() else { return None };
				self.branches(condition.as_ref().clone(), then, None, at, exits)
			}
			Node::Key(head, Op::Else, otherwise) => {
				let Node::Key(test, Op::Then, then) = head.drop_meta() else { return None };
				let Node::Key(_, Op::If, condition) = test.drop_meta() else { return None };
				self.branches(condition.as_ref().clone(), then, Some(otherwise), at, exits)
			}
			Node::List(items, _, _) if items.first().is_some_and(|word| is_word(word, FOR_WORD)) => {
				let lowered = crate::for_loop::lower(statement.clone()).ok()?;
				self.compile(sequence_items(&lowered), at, exits)
			}
			Node::List(_, Bracket::None | Bracket::Round | Bracket::Curly, Separator::Semicolon | Separator::Newline) => {
				self.compile(sequence_items(&statement), at, exits)
			}
			_ => None,
		}
	}

	/// `while c { body }`: the test a state of its own, the body jumping back to it (through the step of a `for`)
	fn while_loop(&mut self, condition: Node, body: &Node, at: usize) -> Option<usize> {
		let (test, after, start) = (self.state(), self.state(), self.state());
		self.jump(at, test);
		self.states[test].push(condition_jump(condition, start));
		self.jump(test, after);
		let (body, step) = split_step(body);
		let again = match step {
			Some(step) => {
				let stepping = self.state();
				self.states[stepping].push(step);
				self.jump(stepping, test);
				stepping
			}
			None => test,
		};
		let end = self.compile(block_items(&body).into_iter().filter(|item| !is_step(item)).collect(), start, Some(Exits { after, again }))?;
		self.jump(end, again);
		Some(after)
	}

	fn branches(&mut self, condition: Node, then: &Node, otherwise: Option<&Node>, at: usize, exits: Option<Exits>) -> Option<usize> {
		let (then_start, after) = (self.state(), self.state());
		self.states[at].push(condition_jump(condition, then_start));
		let then_end = self.compile(block_items(then), then_start, exits)?;
		self.jump(then_end, after);
		match otherwise {
			Some(otherwise) => {
				let else_start = self.state();
				self.jump(at, else_start);
				let else_end = self.compile(block_items(otherwise), else_start, exits)?;
				self.jump(else_end, after);
			}
			None => self.jump(at, after),
		}
		Some(after)
	}

	/// `while 1 { if generator·state == 0 {…}; …; return ø }`
	fn dispatch(self) -> Node {
		let mut cases: Vec<Node> = self.states.into_iter().enumerate().filter(|(_, code)| !code.is_empty()).map(|(state, code)| {
			let test = Node::Key(Box::new(state_field()), Op::Eq, Box::new(number(state as i64)));
			let condition = Node::Key(Box::new(Node::Empty), Op::If, Box::new(test));
			Node::Key(Box::new(condition), Op::Then, Box::new(statements(code, Bracket::Curly)))
		}).collect();
		cases.push(returned(Node::Empty));
		while_do(number(1), statements(cases, Bracket::Curly))
	}
}

/// The body of `next()`: the generator's body cut into states, None for a yield it cannot resume at (one inside an
/// expression)
fn machine_body(generator: &Generator) -> Option<Node> {
	let mut machine = Machine { states: vec![] };
	let start = machine.state();
	let end = machine.compile(block_items(&generator.body), start, None)?;
	machine.states[end].extend([set_state(DONE), returned(Node::Empty)]);
	Some(machine.dispatch())
}
