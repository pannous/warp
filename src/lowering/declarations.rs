//! `enum color {red green blue}` declares the object `color={red:0 green:1 blue:2}`: a case is its index, `color.green` is 1.
//! `real f(real x, int n) { … }`, the C way, defines `f(x:real, n:int) := { … }`.

use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};

/// The implicit parameter of a function: `double := it * 2`
/// `each xs: body`, `all xs: body`: a for loop over xs, the item is `it` (wiki/iteration.md)
const COLON_ITERATION_WORDS: [&str; 2] = ["each", "all"];
const IT_PARAMETER: &str = "it";
const ENUM_WORD: &str = "enum";
const FIRST_CASE_INDEX: i64 = 0;
/// The argument left out of a partial application: `add(1, _)`
const PLACEHOLDER: &str = "_";
/// `go f(x)` starts a task, `await job` waits for it
const TASK_WORDS: [&str; 2] = ["go", "await"];
const ONCE_WORD: &str = "once";
const ON_WORD: &str = "on";
/// What a task signals when it is done, and the controls that would interrupt one
const FINISH_EVENTS: [&str; 5] = ["finishes", "finished", "completes", "ends", "done"];
const TASK_CONTROLS: [&str; 4] = ["stop", "pause", "cancel", "resume"];
/// `go f(x)` of a user function and a read of its task variable, until resolve_tasks knows whether f runs on a thread
pub(crate) const TASK_GO: &str = "task·go";
const TASK_VALUE: &str = "task·value";
/// `stop job`, `job.pause()`: until resolve_tasks knows whether the task runs on a thread
const TASK_CONTROL_MARK: &str = "task·control";

pub fn lower(node: Node) -> Node {
	lower_lists(node, enum_object)
}

/// `job = go f(x)` starts a task, `await job` waits for its value (wiki/async.md). A module runs on one thread, so a
/// task runs to its end where it starts: `go` gives the value of its call, and `await` of a finished task (or of any
/// value, which a task auto-casts to) is that value. Its signals follow: `once job finishes: …` (or `once the download
/// finishes:` for `go download(url)`) runs at once, as the task is done; a handler of a pause or stop never runs and
/// `stop job` has nothing left to stop: both warn. A program defining its own `go` or `await` keeps them.
pub fn lower_tasks(node: Node) -> Node {
	let mut defined = std::collections::HashSet::new();
	let mut functions = std::collections::HashSet::new();
	let mut tasks = std::collections::HashSet::new();
	let mut started = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::Key(head, Op::Define | Op::Assign, value) = part {
		let name = match head.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.first().map(|name| name.drop_meta().name()),
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		};
		if let (Node::Symbol(variable), Node::List(items, _, _)) = (head.drop_meta(), value.drop_meta()) {
			if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == TASK_WORDS[0]) {
				tasks.insert(variable.clone());
				if let Some((function, _)) = started_call(&items[1..]) {
					started.insert(variable.clone(), function);
				}
			}
		}
		if matches!(head.drop_meta(), Node::List(..)) {
			functions.extend(name.clone());
		}
		defined.extend(name);
	});
	let words: Vec<&str> = TASK_WORDS.into_iter().filter(|word| !defined.contains(*word)).collect();
	if words.is_empty() {
		return node;
	}
	// the functions a `go` starts: `once the download finishes` names the task by its function
	node.visit(&mut |part| if let Node::List(items, _, _) = part {
		if let [word, started, ..] = items.as_slice() {
			if matches!(word.drop_meta(), Node::Symbol(name) if name == TASK_WORDS[0]) {
				let function = match started.drop_meta() {
					Node::List(call, _, _) => call.first().map(|name| name.drop_meta().name()),
					other => Some(other.name()),
				};
				tasks.extend(function);
			}
		}
	});
	started.retain(|_, function| functions.contains(function));
	Tasks { words, tasks, started, functions }.lower(node)
}

/// `f(a, b)` or `f a b` after `go`: the function and its arguments
fn started_call(rest: &[Node]) -> Option<(String, Vec<Node>)> {
	match rest {
		[call] => match call.drop_meta() {
			Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => Some((items[0].name(), items[1..].to_vec())),
			Node::List(items, Bracket::None, Separator::Space) => started_call(items),
			_ => None,
		},
		[function, arguments @ ..] if matches!(function.drop_meta(), Node::Symbol(_)) => Some((function.name(), arguments.to_vec())),
		_ => None,
	}
}

fn marker(word: &str, parts: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(word.to_string())], parts].concat(), Bracket::Round, Separator::None)
}

struct Tasks<'a> {
	words: Vec<&'a str>,
	/// task variables and the functions `go` starts
	tasks: std::collections::HashSet<String>,
	/// task variable → the user function its `go` started
	started: std::collections::HashMap<String, String>,
	/// the program's functions
	functions: std::collections::HashSet<String>,
}

impl Tasks<'_> {
	fn is_task(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Symbol(name) => self.tasks.contains(name),
			Node::List(items, _, _) if word(&items[0]) == TASK_VALUE => self.is_task(&items[1]),
			_ => false,
		}
	}

	/// `stop job`: a control of a task that may run on a thread, else the warning that a finished task has nothing to stop
	fn control(&self, subject: &Node, control: &str) -> Node {
		let variable = match subject.drop_meta() {
			Node::List(items, _, _) if word(&items[0]) == TASK_VALUE => word(&items[1]),
			other => other.name(),
		};
		match self.started.get(&variable) {
			Some(function) => marker(TASK_CONTROL_MARK, vec![Node::Symbol(variable), Node::Symbol(function.clone()), Node::Symbol(control.to_string())]),
			None => nothing_to_stop(&Node::Symbol(variable), control),
		}
	}

	/// A read of a task variable: its value once the task is done (`await job`, `job + 1`)
	fn value_of(&self, variable: &str) -> Option<Node> {
		let function = self.started.get(variable)?;
		Some(marker(TASK_VALUE, vec![Node::Symbol(variable.to_string()), Node::Symbol(function.clone())]))
	}

	fn lower(&self, node: Node) -> Node {
		match node {
			Node::Symbol(name) if self.started.contains_key(&name) => self.value_of(&name).expect("a started task"),
			// `job = go f(x)`: the variable holds the task
			Node::Key(target, op @ (Op::Assign | Op::Define), value) if matches!(target.drop_meta(), Node::Symbol(name) if self.started.contains_key(name)) => {
				Node::Key(target, op, Box::new(self.lower(*value)))
			}
			Node::List(items, bracket, separator) => {
				// a signal handler first, before its `download.stop` reads as a control
				if let Some(handled) = self.signal_handler(&items) {
					return self.lower(handled);
				}
				let items: Vec<Node> = items.into_iter().map(|item| self.lower(item)).collect();
				self.task_statement(&items).unwrap_or(Node::List(items, bracket, separator))
			}
			// `job.stop()`, `job.pause`
			Node::Key(subject, Op::Dot, word) if self.is_task(&subject) && TASK_CONTROLS.contains(&control_word(&word).as_str()) => {
				self.control(&subject, &control_word(&word))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.lower(*left)), op, Box::new(self.lower(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.lower(*node)), data },
			other => other,
		}
	}

	fn task_statement(&self, items: &[Node]) -> Option<Node> {
		match items {
			// `go f(x)` of a user function: a task, on a thread when resolve_tasks finds f runs there
			[head, rest @ ..] if word(head) == TASK_WORDS[0] && self.words.contains(&TASK_WORDS[0]) && started_call(rest).is_some_and(|(f, _)| self.functions.contains(&f)) => {
				let (function, arguments) = started_call(rest).expect("guarded");
				Some(marker(TASK_GO, [vec![Node::Symbol(function)], arguments].concat()))
			}
			[head, rest @ ..] if !rest.is_empty() && self.words.contains(&word(head).as_str()) => Some(match rest {
				[single] => single.clone(),
				_ => Node::List(rest.to_vec(), Bracket::None, Separator::Space),
			}),
			// `stop job`
			[control, subject] if TASK_CONTROLS.contains(&word(control).as_str()) && self.is_task(subject) => Some(self.control(subject, &word(control))),
			_ => None,
		}
	}

	fn signal_handler(&self, items: &[Node]) -> Option<Node> {
		match items {
			// `once job finishes: body`, `once the download finishes: body`
			[once, subject @ .., handler] if word(once) == ONCE_WORD => {
				let subject = subject.iter().find(|item| word(item) != "the")?;
				let (event, body) = handler_parts(handler)?;
				self.is_task(subject).then(|| self.handler(subject, &word(&event), &body))
			}
			// `on download.stop : body`
			[on, handler] if word(on) == ON_WORD => {
				let (signal, body) = handler_parts(handler)?;
				let Node::Key(subject, Op::Dot, event) = signal.drop_meta() else { return None };
				self.is_task(subject).then(|| self.handler(subject, &control_word(event), &body))
			}
			_ => None,
		}
	}

	/// A finished task's handler runs once the task is done (it waits for a task on a thread); one of a pause or stop
	/// never runs
	fn handler(&self, subject: &Node, event: &str, body: &Node) -> Node {
		if FINISH_EVENTS.contains(&event) {
			return match self.value_of(&subject.name()) {
				Some(value) => Node::List(vec![value, body.clone()], Bracket::Curly, Separator::Semicolon),
				None => body.clone(),
			};
		}
		never_happens(subject, &format!("`{event}` of {} never happens: a task finishes where it starts, its handler never runs", subject.name()))
	}
}

/// How a started function runs: on a thread with Int words, on a thread with value words, or where it starts
#[derive(Clone, PartialEq)]
enum TaskPath {
	Ints,
	/// with the kinds of the parameters: a Float parameter gets its argument converted, as a call does
	Values(Vec<crate::type_kinds::Kind>),
	Inline,
}

/// The kinds a value of a task may have to cross between instances (tasks::TaskValue); a list parameter takes another
/// convention (list_abi), a function value cannot leave its instance
const CROSSING_KINDS: [crate::type_kinds::Kind; 7] = {
	use crate::type_kinds::Kind::*;
	[Int, Float, Text, Codepoint, Symbol, Empty, List]
};

/// `task·go(f, args)`, `task·value(job, f)` and `task·control(job, f, word)` (lower_tasks). Natively a function runs on
/// its own thread (tasks.rs): one of at most four Ints that gives an Int through task_spawn / task_await, one of
/// numbers, texts and characters through task_spawn_values / task_await_value. Any other runs where it starts, as a
/// plain call, and its task variable is its value
pub fn resolve_tasks(node: Node) -> Node {
	use crate::type_kinds::Kind;
	let mut has_task = false;
	node.visit(&mut |part| has_task |= matches!(part, Node::List(items, _, _) if matches!(word(items.first().unwrap_or(&Node::Empty)).as_str(), TASK_GO | TASK_VALUE | TASK_CONTROL_MARK)));
	if !has_task {
		return node;
	}
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, &node);
	let int_starts = int_starts(&node, &crate::analyzer::literal_variable_kinds(&node, &context));
	let path = |function: &str| {
		let Some(definition) = context.user_functions.get(function).filter(|_| cfg!(feature = "native")) else { return TaskPath::Inline };
		let parameters: Vec<Kind> = definition.params.iter().map(crate::analyzer::param_kind).collect();
		if !definition.tuple_kinds.is_empty() || !CROSSING_KINDS.contains(&definition.return_kind) || !parameters.iter().all(|kind| CROSSING_KINDS.contains(kind)) {
			return TaskPath::Inline;
		}
		let ints = definition.return_kind == Kind::Int && parameters.len() <= crate::host::MAX_TASK_ARGUMENTS && parameters.iter().all(|kind| *kind == Kind::Int)
			&& int_starts.get(function).copied().unwrap_or(false);
		if ints { TaskPath::Ints } else { TaskPath::Values(parameters) }
	};
	let wrapped = std::cell::RefCell::new(std::collections::BTreeMap::new());
	let node = resolved(node, &path, &wrapped);
	with_node_wrappers(node, &wrapped.into_inner())
}

/// Per started function: whether every `go` of it passes surely Ints (Int literals, variables only ever assigned Int
/// literals, arithmetic of them), which the Int words carry raw; anything else, like a list a `.map` gives, takes the
/// value words
fn int_starts(node: &Node, literal_kinds: &std::collections::HashMap<String, crate::type_kinds::Kind>) -> std::collections::HashMap<String, bool> {
	fn surely_int(argument: &Node, literal_kinds: &std::collections::HashMap<String, crate::type_kinds::Kind>) -> bool {
		match argument.drop_meta() {
			Node::Number(crate::extensions::numbers::Number::Int(_)) => true,
			Node::Symbol(name) => literal_kinds.get(name) == Some(&crate::type_kinds::Kind::Int),
			Node::Key(left, op, right) if matches!(op, Op::Add | Op::Sub | Op::Mul) => surely_int(left, literal_kinds) && surely_int(right, literal_kinds),
			_ => false,
		}
	}
	let mut starts = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::List(items, _, _) = part {
		if word(items.first().unwrap_or(&Node::Empty)) == TASK_GO {
			let all_int = items[2..].iter().all(|argument| surely_int(argument, literal_kinds));
			let entry = starts.entry(word(&items[1])).or_insert(true);
			*entry &= all_int;
		}
	});
	starts
}

/// The suffix of the wrapper a task of values is started through: `total·node(a) := { r·node = total(a); r·node }`
/// takes and gives plain Nodes, where total itself may take and give arrays (list_abi)
const NODE_WRAPPER_SUFFIX: &str = "·node";
const WRAPPER_RESULT: &str = "r·node";

/// The program with the wrappers of the started functions (name → parameter count) defined first
fn with_node_wrappers(node: Node, wrapped: &std::collections::BTreeMap<String, usize>) -> Node {
	if wrapped.is_empty() {
		return node;
	}
	let mut items: Vec<Node> = wrapped.iter().map(|(function, count)| {
		let parameters: Vec<Node> = (0..*count).map(|index| Node::Symbol(format!("p·{index}"))).collect();
		let head = Node::List([vec![Node::Symbol(format!("{function}{NODE_WRAPPER_SUFFIX}"))], parameters.clone()].concat(), Bracket::Round, Separator::None);
		let call = Node::List([vec![Node::Symbol(function.clone())], parameters].concat(), Bracket::Round, Separator::None);
		let result = Node::Symbol(WRAPPER_RESULT.to_string());
		let body = Node::List(vec![Node::Key(Box::new(result.clone()), Op::Assign, Box::new(call)), result], Bracket::Curly, Separator::Semicolon);
		Node::Key(Box::new(head), Op::Define, Box::new(body))
	}).collect();
	match node {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			items.extend(statements);
			Node::List(items, Bracket::None, separator)
		}
		single => {
			items.push(single);
			Node::List(items, Bracket::None, Separator::Semicolon)
		}
	}
}

/// `task·check(task_join(job), task_failure(job)); task_await(job)`: a failed task raises its error from wasm, which
/// `try` catches, before the result is read
fn checked_await(await_word: &str, job: &Node) -> Node {
	use crate::host::{TASK_CHECK, TASK_FAILURE, TASK_JOIN};
	let check = marker(TASK_CHECK, vec![marker(TASK_JOIN, vec![job.clone()]), marker(TASK_FAILURE, vec![job.clone()])]);
	Node::List(vec![check, marker(await_word, vec![job.clone()])], Bracket::None, Separator::Semicolon)
}

fn resolved(node: Node, path: &dyn Fn(&str) -> TaskPath, wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>) -> Node {
	use crate::host::{TASK_AWAIT, TASK_AWAIT_VALUE, TASK_CONTROL, TASK_SPAWN, TASK_SPAWN_VALUES};
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| resolved(item, path, wrapped)).collect();
			match word(items.first().unwrap_or(&Node::Empty)).as_str() {
				TASK_GO => {
					let function = word(&items[1]);
					let arguments = items[2..].to_vec();
					match path(&function) {
						TaskPath::Ints => marker(TASK_SPAWN, [vec![Node::Text(function)], arguments].concat()),
						TaskPath::Values(parameters) => {
							let float = |argument: Node| Node::Key(Box::new(argument), Op::As, Box::new(Node::Symbol("float".to_string())));
							let arguments = arguments.into_iter().zip(parameters.iter().chain(std::iter::repeat(&crate::type_kinds::Kind::Empty)))
								.map(|(argument, kind)| if *kind == crate::type_kinds::Kind::Float { float(argument) } else { argument }).collect();
							// through the wrapper: whether f takes a list as an array is decided later (list_abi)
							wrapped.borrow_mut().insert(function.clone(), parameters.len());
							let started = format!("{function}{NODE_WRAPPER_SUFFIX}");
							marker(TASK_SPAWN_VALUES, vec![Node::Text(started), Node::List(arguments, Bracket::Square, Separator::Colon)])
						}
						TaskPath::Inline => Node::List(items[1..].to_vec(), Bracket::Round, Separator::None),
					}
				}
				TASK_VALUE => match path(&word(&items[2])) {
					TaskPath::Ints => checked_await(TASK_AWAIT, &items[1]),
					TaskPath::Values(_) => checked_await(TASK_AWAIT_VALUE, &items[1]),
					TaskPath::Inline => items[1].clone(),
				},
				TASK_CONTROL_MARK => match path(&word(&items[2])) {
					TaskPath::Inline => nothing_to_stop(&items[1], &word(&items[3])),
					_ => marker(TASK_CONTROL, vec![items[1].clone(), Node::Number(crate::extensions::numbers::Number::Int(control_operation(&word(&items[3]))))]),
				},
				_ => Node::List(items, bracket, separator),
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(resolved(*left, path, wrapped)), op, Box::new(resolved(*right, path, wrapped))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(resolved(*node, path, wrapped)), data },
		other => other,
	}
}

/// `event: body`; `event: x = 7` parses as `(event: x) = 7`
pub(crate) fn handler_parts(handler: &Node) -> Option<(Node, Node)> {
	match handler.drop_meta() {
		Node::Key(event, Op::Colon, body) => Some((event.drop_meta().clone(), body.as_ref().clone())),
		Node::Key(head, op, value) => match head.drop_meta() {
			Node::Key(event, Op::Colon, target) => Some((event.drop_meta().clone(), Node::Key(target.clone(), *op, value.clone()))),
			_ => None,
		},
		_ => None,
	}
}

pub(crate) fn word(node: &Node) -> String {
	match node.drop_meta() {
		Node::Symbol(name) => name.clone(),
		_ => String::new(),
	}
}

/// The word of `stop`, `stop()`, `(stop)`
fn control_word(node: &Node) -> String {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => items[0].drop_meta().name(),
		other => other.name(),
	}
}

fn nothing_to_stop(subject: &Node, control: &str) -> Node {
	never_happens(subject, &format!("{control} {} has nothing to stop: a task finishes where it starts", subject.name()))
}

/// The task_control operation of a control word
fn control_operation(control: &str) -> i64 {
	match control {
		"pause" => crate::host::TASK_PAUSE,
		"resume" => crate::host::TASK_RESUME,
		_ => crate::host::TASK_STOP, // stop, cancel
	}
}

/// A warning about task control that cannot happen on one thread, and the statement's value ø (an error under
/// `use strict`)
fn never_happens(located: &Node, message: &str) -> Node {
	match crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(located, message.to_string())]) {
		Ok(()) => Node::Empty,
		Err(error) => error,
	}
}

/// C definitions, before any pass reads `real f(…)` as a conversion of a call; and keyword definitions
/// (`def f(x) {…}`, `fun`, `function`) as `f(x) := {…}`, so every pass reads one definition form
/// The spaced definition `square x := x*x` (wiki/examples.md), parsed as the items `square`, `x := x*x`, is the
/// definition `square(x) := x*x`; `x y z := y*y+u` names x of the parameters y and z. A braceless call ending the body
/// keeps its argument (`fibonacci number := … + fibonacci it - 2`, wiki/Home.md), and in a definition of one named
/// parameter `it` is that parameter
pub fn lower_spaced_definitions(node: Node) -> Node {
	match node {
		Node::List(items, _, _) if colon_iteration(&items).is_some() => lower_spaced_definitions(colon_iteration(&items).expect("guarded")),
		// `1…5 do print it` (wiki/range.md): a loop over the range, the item is `it`
		Node::Key(range, Op::Do, body) if matches!(range.drop_meta(), Node::Key(_, Op::Range | Op::To, _)) => {
			let block = Node::List(vec![lower_spaced_definitions(*body)], Bracket::Curly, Separator::Semicolon);
			Node::List(vec![Node::Symbol("for".into()), *range, block], Bracket::None, Separator::Space)
		}
		Node::List(items, bracket, separator) if spaced_definition(&items).is_some() => {
			let (name, parameters, body) = spaced_definition(&items).expect("guarded");
			let body = lower_spaced_definitions(body);
			let body = match parameters.as_slice() {
				[Node::Symbol(parameter)] if parameter != IT_PARAMETER => crate::law::substitute(&body, &[(IT_PARAMETER.to_string(), Node::Symbol(parameter.clone()))].into()),
				_ => body,
			};
			let head = Node::List([vec![name], parameters].concat(), Bracket::Round, Separator::None);
			let _ = (bracket, separator);
			Node::Key(Box::new(head), Op::Define, Box::new(body))
		}
		// `f(x) := x + it`: `it` is the one parameter too
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if one_parameter(&head).is_some() => {
			let parameter = one_parameter(&head).expect("guarded");
			let body = crate::law::substitute(&lower_spaced_definitions(*body), &[(IT_PARAMETER.to_string(), Node::Symbol(parameter))].into());
			Node::Key(head, op, Box::new(body))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower_spaced_definitions(*left)), op, Box::new(lower_spaced_definitions(*right))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower_spaced_definitions).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_spaced_definitions(*node)), data },
		other => other,
	}
}

/// `each [1,2,3]: print it`, `all xs: …` (wiki/iteration.md): a for loop over the list whose item is `it`
fn colon_iteration(items: &[Node]) -> Option<Node> {
	let [word, list_and_body, rest @ ..] = items else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(word) if COLON_ITERATION_WORDS.contains(&word.as_str())) {
		return None;
	}
	let (list, first) = split_at_colon(list_and_body)?;
	let body = match rest {
		[] => first,
		_ => Node::List([vec![first], rest.to_vec()].concat(), Bracket::None, Separator::Space),
	};
	let block = Node::List(vec![body], Bracket::Curly, Separator::Semicolon);
	Some(Node::List(vec![Node::Symbol("for".into()), list, block], Bracket::None, Separator::Space))
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

/// `name p… last := body extra…`: the function name, its parameters and its body (the extra items the argument of
/// the braceless call that ends the body)
fn spaced_definition(items: &[Node]) -> Option<(Node, Vec<Node>, Node)> {
	let definition = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(target, Op::Define, _) if matches!(target.drop_meta(), Node::Symbol(_))))?;
	let (words, rest) = items.split_at(definition);
	let (name, parameters) = words.split_first()?;
	let is_word = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if !is_function_keyword(word));
	if !is_word(name) || !parameters.iter().all(is_word) {
		return None;
	}
	let (definition, extra) = rest.split_first()?;
	let Node::Key(last, _, body) = definition.drop_meta() else { return None };
	let words: Vec<Node> = parameters.iter().map(|parameter| parameter.drop_meta().clone()).chain(std::iter::once(last.drop_meta().clone())).collect();
	Some((name.drop_meta().clone(), typed_words(words), applied_to_last(body.as_ref().clone(), extra)))
}

/// `int x y` as the parameters `x:int`, `y`: a type word types the name after it
fn typed_words(words: Vec<Node>) -> Vec<Node> {
	let mut parameters = vec![];
	let mut words = words.into_iter().peekable();
	while let Some(word) = words.next() {
		let is_type = matches!(&word, Node::Symbol(name) if crate::analyzer::type_word_kind(name).is_some());
		match words.peek() {
			Some(Node::Symbol(_)) if is_type => {
				let name = words.next().expect("peeked");
				parameters.push(Node::Key(Box::new(name), Op::Colon, Box::new(word)));
			}
			_ => parameters.push(word),
		}
	}
	parameters
}

/// The one named parameter of a definition head `f(x)` (`f(x:int)`), not `it`
fn one_parameter(head: &Node) -> Option<String> {
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let [name, parameter] = items.as_slice() else { return None };
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let parameter = match parameter.drop_meta() {
		Node::Key(parameter, Op::Colon, _) => parameter.drop_meta(),
		other => other,
	};
	match parameter {
		Node::Symbol(parameter) if parameter != IT_PARAMETER => Some(parameter.clone()),
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
		Node::Meta { node, data } => Node::Meta { node: Box::new(applied_to_last(*node, extra)), data },
		last => Node::List([vec![last], extra.to_vec()].concat(), Bracket::Round, Separator::None),
	}
}

pub fn lower_c_functions(node: Node) -> Node {
	lower_lists(node, |items| c_function(items).or_else(|| keyword_definition(items)).or_else(|| partial_application(items)))
}

/// `add(1, _)`: a call with placeholders is the lambda of the missing arguments, `partial_1 => add(1, partial_1)`
fn partial_application(items: &[Node]) -> Option<Node> {
	let [Node::Symbol(_), arguments @ ..] = items else { return None };
	let is_placeholder = |argument: &Node| matches!(argument.drop_meta(), Node::Symbol(name) if name == PLACEHOLDER);
	if !arguments.iter().any(is_placeholder) {
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
	let call = Node::List(std::iter::once(items[0].clone()).chain(arguments).collect(), Bracket::Round, Separator::None);
	let head = match parameters.as_slice() {
		[single] => single.clone(),
		_ => Node::List(parameters, Bracket::Round, Separator::Colon),
	};
	Some(Node::Key(Box::new(head), Op::FatArrow, Box::new(call)))
}

/// `def f(a, b) { body }`, `def f(x) := body`, `function g() { … }`: the definition `f(a, b) := body`
fn keyword_definition(items: &[Node]) -> Option<Node> {
	let (keyword, definition) = match items {
		[keyword, definition] => (keyword, definition.drop_meta().clone()),
		[keyword, head, body] => (keyword, Node::List(vec![head.clone(), body.clone()], Bracket::Round, Separator::None)),
		// Go's `func add1(x int) int {…}`: the result type between the head and the body
		[keyword, head, result_type, body] if is_type_word(result_type) => (keyword, Node::List(vec![head.clone(), body.clone()], Bracket::Round, Separator::None)),
		_ => return None,
	};
	if !matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)) {
		return None;
	}
	let (head, op, body) = match definition {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) => (*head, op, *body),
		// `def test: print "test"` (wiki/signal.md): a function without parameters
		Node::Key(name, Op::Colon, body) if matches!(name.drop_meta(), Node::Symbol(_)) => {
			(Node::List(vec![*name], Bracket::Round, Separator::None), Op::Define, *body)
		}
		Node::List(parts, Bracket::Round, _) if parts.len() == 2 && matches!(parts[1].drop_meta(), Node::List(_, Bracket::Curly, _)) => (parts[0].clone(), Op::Define, parts[1].clone()),
		_ => return None,
	};
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	// the parameters may come as one group: `f (a, b)`, `f (m)`, `f ø`
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![argument.clone()],
	}).map(name_then_type);
	let mut parameters: Vec<Node> = parameters.collect();
	// `func add1(x int)`: the one parameter and its type arrive as two words
	if let [parameter, type_word] = parameters.as_slice() {
		if matches!(parameter.drop_meta(), Node::Symbol(_)) && is_type_word(type_word) {
			parameters = vec![Node::Key(Box::new(parameter.clone()), Op::Colon, Box::new(type_word.clone()))];
		}
	}
	let head = Node::List(std::iter::once(name.clone()).chain(parameters).collect(), Bracket::Round, Separator::None);
	Some(Node::Key(Box::new(head), op, Box::new(body)))
}

fn is_type_word(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if crate::analyzer::type_word_kind(word).is_some())
}

/// Go's parameter `x int` (the name, then its type) is `x:int`
fn name_then_type(parameter: Node) -> Node {
	match parameter.drop_meta() {
		Node::List(parts, _, Separator::Space) if parts.len() == 2 && matches!(parts[0].drop_meta(), Node::Symbol(_)) && is_type_word(&parts[1]) => {
			Node::Key(Box::new(parts[0].clone()), Op::Colon, Box::new(parts[1].clone()))
		}
		_ => parameter,
	}
}

fn lower_lists(node: Node, lowering: impl Fn(&[Node]) -> Option<Node> + Copy) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| lower_lists(item, lowering)).collect();
			lowering(&items).unwrap_or(Node::List(items, bracket, separator))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower_lists(*left, lowering)), op, Box::new(lower_lists(*right, lowering))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_lists(*node, lowering)), data },
		other => other,
	}
}

/// The assignment `name={case:index …}` of the items `enum name {case …}`
fn enum_object(items: &[Node]) -> Option<Node> {
	let [word, name, cases] = items else { return None };
	let is_enum = matches!(word.drop_meta(), Node::Symbol(word) if word == ENUM_WORD);
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let Node::List(case_names, Bracket::Curly, _) = cases.drop_meta() else { return None };
	if !is_enum || case_names.iter().any(|case| !matches!(case.drop_meta(), Node::Symbol(_))) {
		return None;
	}
	let entries = case_names
		.iter()
		.zip(FIRST_CASE_INDEX..)
		.map(|(case, index)| Node::Key(Box::new(case.clone()), Op::Colon, Box::new(Node::int(index))))
		.collect();
	let object = Node::List(entries, Bracket::Curly, Separator::Space);
	Some(Node::Key(Box::new(name.clone()), Op::Assign, Box::new(object)))
}


/// `real f(real x) { … }`: the definition `f(x:real) := { … }` (the parser reads the type word, then the call and its
/// block); the result kind is inferred as for any definition
fn c_function(items: &[Node]) -> Option<Node> {
	let [result_type, definition] = items else { return None };
	let Node::Symbol(result_type) = result_type.drop_meta() else { return None };
	if crate::analyzer::type_word_kind(result_type).is_none() && result_type != "void" {
		return None;
	}
	let Node::List(parts, Bracket::Round, _) = definition.drop_meta() else { return None };
	let [head, body] = parts.as_slice() else { return None };
	let Node::List(_, Bracket::Curly, _) = body.drop_meta() else { return None };
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, Separator::Colon) => group.clone(), // `(real a, int b)`
		_ => vec![argument.clone()],
	});
	let parameters: Option<Vec<Node>> = parameters.map(|parameter| c_parameter(&parameter)).collect();
	let head = Node::List([vec![name.clone()], parameters?].concat(), Bracket::Round, Separator::Colon);
	Some(Node::Key(Box::new(head), Op::Define, Box::new(body.clone())))
}

/// `real x` is `x:real`; a bare name stays untyped
fn c_parameter(parameter: &Node) -> Option<Node> {
	match parameter.drop_meta() {
		Node::Symbol(_) => Some(parameter.clone()),
		Node::List(words, _, Separator::Space) => match words.as_slice() {
			[kind, name] if matches!((kind.drop_meta(), name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) => {
				Some(Node::Key(Box::new(name.clone()), Op::Colon, Box::new(kind.clone())))
			}
			_ => None,
		},
		_ => None,
	}
}
