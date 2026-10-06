//! `enum color {red green blue}` declares the object `color={red:0 green:1 blue:2}`: a case is its index, `color.green` is 1.
//! `flags virtues={fast, safe}` (WIT flags, a record of bools) declares `virtues={fast:false safe:false}`, and
//! `virtues goal = fast+safe` the record `goal={fast:true safe:true}`: the flags named are true, the others false.
//! `real f(real x, int n) { … }`, the C way, defines `f(x:real, n:int) := { … }`.

use crate::node::{Bracket, Node, Separator};
use crate::operators::{is_function_keyword, Op};

/// The receiver of an extension method, as Kotlin (`this`) and Swift (`self`) name it
const THIS_WORD: &str = "this";
const SELF_WORD: &str = "self";
/// Swift's block of methods added to a type: `extension Int {…}`
const EXTENSION_WORD: &str = "extension";

/// The implicit parameter of a function: `double := it * 2`
/// `each xs: body`, `all xs: body`: a for loop over xs, the item is `it` (wiki/iteration.md)
const COLON_ITERATION_WORDS: [&str; 2] = ["each", "all"];
const IT_PARAMETER: &str = "it";
const ENUM_WORD: &str = "enum";
/// Ruby's `def f(x) … end`
const END_WORD: &str = "end";
const FLAGS_WORD: &str = "flags";
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
/// A read of a job list (`jobs.add(go f(i))`, P47): `task·list(jobs, f…)` is every result, `task·element(jobs#i, f…)` one;
/// until resolve_tasks knows whether the started functions run on threads
const TASK_LIST: &str = "task·list";
/// The reads `await` names: `await job`, `await all jobs`; the others (`job + 1`) wait unsaid, which warns
const AWAITED_MARKERS: [(&str, &str); 3] = [(TASK_VALUE, "task·awaited"), (TASK_LIST, "task·awaited·list"), (TASK_ELEMENT, "task·awaited·element")];
const IMPLICIT_WAIT_TOPIC: &str = "implicit-await";
const TASK_ELEMENT: &str = "task·element";
/// `await all jobs`: the results of every job of a list
const ALL_WORD: &str = "all";
const ADD_WORD: &str = "add";
/// The words that count a list: counting a job list waits for no job
const JOB_COUNTERS: [&str; 4] = ["count", "size", "length", "len"];
/// The parameter of the await map over a job list (resolve_tasks)
const AWAITED_JOB: &str = "awaited_job";
/// `stop job`, `job.pause()`: until resolve_tasks knows whether the task runs on a thread
const TASK_CONTROL_MARK: &str = "task·control";
/// `once job finishes: body`, `on job.stop: body` of a task that may run on a thread: `task·on(job, f, event, body)`
const TASK_ON: &str = "task·on";
const FINISH_EVENT: &str = "finishes";
/// `handled·0`: whether the first task handler ran
const HANDLED_PREFIX: &str = "handled·";

pub fn lower(node: Node) -> Node {
	lower_lists(lower_flags(node), enum_object)
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
	let (Node::Symbol(name), Node::List(members, Bracket::Curly, _)) = (name.drop_meta(), members.drop_meta()) else { return None };
	let members: Option<Vec<String>> = members.iter().map(|member| match member.drop_meta() {
		Node::Symbol(member) => Some(member.clone()),
		_ => None,
	}).collect();
	Some((name.clone(), members?))
}

fn is_flags_word(word: &Node) -> bool {
	matches!(word.drop_meta(), Node::Symbol(word) if word == FLAGS_WORD)
}

fn rewrite_flags(node: Node, types: &[(String, Vec<String>)]) -> Node {
	let record = |name: &Node, members: &[String], set: &[String]| {
		let entries = members.iter().map(|member| {
			let value = if set.contains(member) { Node::True } else { Node::False };
			Node::Key(Box::new(Node::Symbol(member.clone())), Op::Colon, Box::new(value))
		}).collect();
		Node::Key(Box::new(name.clone()), Op::Assign, Box::new(Node::List(entries, Bracket::Curly, Separator::Space)))
	};
	match node {
		Node::List(items, bracket, separator) => {
			if let Some((name, members)) = flags_declaration(&items) {
				return record(&Node::Symbol(name), &members, &[]);
			}
			// `virtues goal = fast+safe`
			if let [type_name, declaration] = items.as_slice() {
				if let (Node::Symbol(type_name), Node::Key(variable, Op::Assign, value)) = (type_name.drop_meta(), declaration.drop_meta()) {
					if let Some((_, members)) = types.iter().find(|(name, _)| name == type_name) {
						let set = named_flags(value);
						if let Some(unknown) = set.iter().find(|flag| !members.contains(flag)) {
							return crate::node::error(&format!("{type_name} has no flag {unknown}; its flags: {}", members.join(", ")));
						}
						return record(variable, members, &set);
					}
				}
			}
			Node::List(items.into_iter().map(|item| rewrite_flags(item, types)).collect(), bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(rewrite_flags(*left, types)), op, Box::new(rewrite_flags(*right, types))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite_flags(*node, types)), data },
		other => other,
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

/// `job = go f(x)` starts a task, `await job` waits for its value (wiki/async.md). A module runs on one thread, so a
/// task runs to its end where it starts: `go` gives the value of its call, and `await` of a finished task (or of any
/// value, which a task auto-casts to) is that value. Its signals follow: `once job finishes: …` (or `once the download
/// finishes:` for `go download(url)`) runs at once, as the task is done; a handler of a pause or stop never runs and
/// `stop job` has nothing left to stop: both warn. A program defining its own `go` or `await` keeps them.
pub fn lower_tasks(node: Node) -> Node {
	let node = awaited_starts(node, &std::cell::Cell::new(0));
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
	// `def hi: {…}`, `fun f(x) {…}` too
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, &node);
	functions.extend(context.user_functions.into_keys());
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
	let job_lists = job_lists(&node, &functions);
	Tasks { words, tasks, started, functions, job_lists }.lower(node)
}

/// `await go f(x)`: the task gets a name, `(go·job·1 = go f(x); await go·job·1)`, so it is awaited like any other
fn awaited_starts(node: Node, counter: &std::cell::Cell<usize>) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let words: Vec<Node> = match items.as_slice() {
				[pair, rest @ ..] if matches!(pair.drop_meta(), Node::List(inner, Bracket::None, _) if inner.len() == 2) => {
					let Node::List(inner, _, _) = pair.drop_meta() else { unreachable!("guarded") };
					[inner.clone(), rest.to_vec()].concat()
				}
				_ => items.clone(),
			};
			match words.as_slice() {
				[await_word, go_word, started @ ..] if !started.is_empty() && await_word.name() == TASK_WORDS[1] && go_word.name() == TASK_WORDS[0] => {
					let job = Node::Symbol(format!("go·job·{}", counter.replace(counter.get() + 1)));
					let start = Node::List([vec![go_word.clone()], started.to_vec()].concat(), Bracket::None, Separator::Space);
					let assignment = Node::Key(Box::new(job.clone()), Op::Assign, Box::new(start));
					let awaited = Node::List(vec![await_word.clone(), job], Bracket::None, Separator::Space);
					Node::List(vec![assignment, awaited], Bracket::Round, Separator::Semicolon)
				}
				_ => Node::List(items.into_iter().map(|item| awaited_starts(item, counter)).collect(), bracket, separator),
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(awaited_starts(*left, counter)), op, Box::new(awaited_starts(*right, counter))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(awaited_starts(*node, counter)), data },
		other => other,
	}
}

/// The list variables that collect started tasks, `jobs.add(go f(i))` (P47), with the functions they start
fn job_lists(node: &Node, functions: &std::collections::HashSet<String>) -> std::collections::HashMap<String, Vec<String>> {
	let mut lists: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
	node.visit(&mut |part| if let Node::Key(receiver, Op::Dot, method) = part {
		if let (Node::Symbol(list), Some((function, _))) = (receiver.drop_meta(), added_start(method)) {
			if functions.contains(&function) {
				let started = lists.entry(list.clone()).or_default();
				if !started.contains(&function) {
					started.push(function);
				}
			}
		}
	});
	lists
}

/// `add(go f(x))`, `add go f(x)`: the `go …` items after `add`
fn added_task(method: &Node) -> Option<Vec<Node>> {
	let Node::List(items, _, _) = method.drop_meta() else { return None };
	let [add, rest @ ..] = items.as_slice() else { return None };
	if word(add) != ADD_WORD {
		return None;
	}
	let rest = match rest {
		[single] => match single.drop_meta() {
			Node::List(inner, _, _) => inner.clone(),
			_ => return None,
		},
		several => several.to_vec(),
	};
	(word(rest.first()?) == TASK_WORDS[0]).then_some(rest)
}

fn added_start(method: &Node) -> Option<(String, Vec<Node>)> {
	started_call(&added_task(method)?[1..])
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

/// A read of a task that `await` names: no warning that it waits
fn awaited(node: Node) -> Node {
	match node {
		Node::List(mut items, bracket, separator) if AWAITED_MARKERS.iter().any(|(read, _)| word(items.first().unwrap_or(&Node::Empty)) == *read) => {
			let read = word(&items[0]);
			let (_, named) = AWAITED_MARKERS.iter().find(|(marker, _)| *marker == read).expect("guarded");
			items[0] = Node::Symbol(named.to_string());
			Node::List(items, bracket, separator)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(awaited(*node)), data },
		other => other,
	}
}

/// The read as `await` would name it, and whether it was named so: `task·awaited` is `task·value` said
fn read_marker(head: &str) -> (&str, bool) {
	AWAITED_MARKERS.iter().find(|(_, named)| *named == head).map_or((head, false), |(read, _)| (*read, true))
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
	/// list variables that collect started tasks → the functions they start (P47)
	job_lists: std::collections::HashMap<String, Vec<String>>,
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

	/// `count(jobs)`, `count jobs`, `#jobs`, `jobs.size` (count, length) of a job list: a count needs no result (P47)
	fn counted_job_list(&self, node: &Node) -> bool {
		let is_job_list = |list: &Node| matches!(list.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name));
		match node.drop_meta() {
			Node::List(items, _, _) => matches!(items.as_slice(), [counter, list] if JOB_COUNTERS.contains(&word(counter).as_str()) && is_job_list(list)),
			Node::Key(empty, Op::Hash, list) => matches!(empty.drop_meta(), Node::Empty) && is_job_list(list),
			Node::Key(list, Op::Dot, counter) => is_job_list(list) && JOB_COUNTERS.contains(&word(counter).as_str()),
			_ => false,
		}
	}

	/// A read of a job list or of one of its jobs: its results once the jobs are done
	fn job_list_read(&self, list: &str, read: Node, marker_word: &str) -> Node {
		let functions = self.job_lists[list].iter().map(|function| Node::Symbol(function.clone()));
		marker(marker_word, [vec![read], functions.collect()].concat())
	}

	fn lower(&self, node: Node) -> Node {
		match node {
			Node::Symbol(name) if self.started.contains_key(&name) => self.value_of(&name).expect("a started task"),
			Node::Symbol(name) if self.job_lists.contains_key(&name) => self.job_list_read(&name, Node::Symbol(name.clone()), TASK_LIST),
			// `count(jobs)`, `#jobs`, `jobs.size`: how many jobs, without waiting for any
			node if self.counted_job_list(&node) => node,
			// `jobs.add(go f(i))`: the list keeps the task, unawaited
			Node::Key(receiver, Op::Dot, method) if matches!(receiver.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name)) && added_task(&method).is_some() => {
				let started = self.task_statement(&added_task(&method).expect("guarded")).expect("a go of a user function");
				Node::Key(receiver, Op::Dot, Box::new(Node::List(vec![Node::Symbol(ADD_WORD.to_string()), started], Bracket::Round, Separator::None)))
			}
			// `jobs#2`: the result of that job
			Node::Key(list, Op::Hash, index) if matches!(list.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name)) => {
				let name = word(&list);
				self.job_list_read(&name, Node::Key(list, Op::Hash, Box::new(self.lower(*index))), TASK_ELEMENT)
			}
			// `jobs = []`: the variable itself
			Node::Key(target, op @ (Op::Assign | Op::Define), value) if matches!(target.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name)) => {
				Node::Key(target, op, Box::new(self.lower(*value)))
			}
			// `job = go f(x)`: the variable holds the task
			Node::Key(target, op @ (Op::Assign | Op::Define), value) if matches!(target.drop_meta(), Node::Symbol(name) if self.started.contains_key(name)) => {
				Node::Key(target, op, Box::new(self.lower(*value)))
			}
			Node::List(items, bracket, separator) => {
				// a signal handler first, before its `download.stop` reads as a control
				if let Some(handled) = self.signal_handler(&items) {
					// a task·on marker holds the task variable itself, its body is lowered already
					let is_task_handler = matches!(&handled, Node::List(parts, _, _) if word(parts.first().unwrap_or(&Node::Empty)) == TASK_ON);
					return if is_task_handler { handled } else { self.lower(handled) };
				}
				let items: Vec<Node> = items.into_iter().map(|item| self.lower(item)).collect();
				match self.task_statement(&items) {
					// `{ go f(x) }` stays a block: a body in braces is no getter
					Some(statement) if bracket == Bracket::Curly => Node::List(vec![statement], bracket, separator),
					Some(statement) => statement,
					None => Node::List(items, bracket, separator),
				}
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
			// `await all jobs` (also read as `await (all jobs)`): every result, a job list read awaits them all
			[head, phrase] if word(head) == TASK_WORDS[1] && self.words.contains(&TASK_WORDS[1]) && matches!(phrase.drop_meta(), Node::List(inner, Bracket::None, _) if inner.len() == 2 && word(&inner[0]) == ALL_WORD) => {
				let Node::List(inner, _, _) = phrase.drop_meta() else { unreachable!("guarded") };
				Some(awaited(inner[1].clone()))
			}
			[head, all, rest @ ..] if word(head) == TASK_WORDS[1] && word(all) == ALL_WORD && !rest.is_empty() && self.words.contains(&TASK_WORDS[1]) => Some(awaited(match rest {
				[single] => single.clone(),
				_ => Node::List(rest.to_vec(), Bracket::None, Separator::Space),
			})),
			[head, rest @ ..] if !rest.is_empty() && self.words.contains(&word(head).as_str()) => {
				let value = match rest {
					[single] => single.clone(),
					_ => Node::List(rest.to_vec(), Bracket::None, Separator::Space),
				};
				Some(if word(head) == TASK_WORDS[1] { awaited(value) } else { value })
			}
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
		if let Some(function) = self.started.get(&subject.name()) {
			let event = if FINISH_EVENTS.contains(&event) { FINISH_EVENT } else { event };
			return marker(TASK_ON, vec![Node::Symbol(subject.name()), Node::Symbol(function.clone()), Node::Symbol(event.to_string()), self.lower(body.clone())]);
		}
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

/// The kinds a value of a task may have to cross between instances (tasks::TaskValue); a function value crosses as its
/// target's name and captured values (closure_rebuild)
const CROSSING_KINDS: [crate::type_kinds::Kind; 8] = {
	use crate::type_kinds::Kind::*;
	[Int, Float, Text, Codepoint, Symbol, Empty, List, Function]
};

/// `task·go(f, args)`, `task·value(job, f)` and `task·control(job, f, word)` (lower_tasks). A function runs in a fresh
/// instance, natively on its own thread (tasks.rs), in the browser through host.js: one of at most four surely-Int
/// arguments that gives an Int through task_spawn / task_await, one of numbers, texts, characters and lists through
/// task_spawn_values / task_await_value. Any other runs where it starts, as a plain call, and its task variable is its
/// value
pub fn resolve_tasks(node: Node) -> Node {
	use crate::type_kinds::Kind;
	let mut has_task = false;
	node.visit(&mut |part| has_task |= matches!(part, Node::List(items, _, _) if matches!(read_marker(&word(items.first().unwrap_or(&Node::Empty))).0, TASK_GO | TASK_VALUE | TASK_CONTROL_MARK | TASK_LIST | TASK_ELEMENT | crate::wasp_parser::TRY_MARKER)));
	if !has_task {
		return node;
	}
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, &node);
	let int_starts = int_starts(&node, &crate::analyzer::literal_variable_kinds(&node, &context));
	let path = |function: &str| {
		let Some(definition) = context.user_functions.get(function) else { return TaskPath::Inline };
		let parameters: Vec<Kind> = definition.params.iter().map(crate::analyzer::param_kind).collect();
		if !definition.tuple_kinds.is_empty() || !CROSSING_KINDS.contains(&definition.return_kind) || !parameters.iter().all(|kind| CROSSING_KINDS.contains(kind)) {
			return TaskPath::Inline;
		}
		let ints = definition.return_kind == Kind::Int && parameters.len() <= crate::host::MAX_TASK_ARGUMENTS && parameters.iter().all(|kind| *kind == Kind::Int)
			&& int_starts.get(function).copied().unwrap_or(false);
		if ints { TaskPath::Ints } else { TaskPath::Values(parameters) }
	};
	let node = TaskHandlers { path: &path, count: std::cell::Cell::new(0) }.lower(node, &[]);
	let wrapped = std::cell::RefCell::new(std::collections::BTreeMap::new());
	// the guardable functions, with the type word their Node result converts back to (it comes back from the host as a Node)
	let guardable = |function: &str| context.user_functions.get(function).filter(|definition| definition.tuple_kinds.is_empty())
		.map(|definition| match definition.return_kind {
			Kind::Int => Some("int"),
			Kind::Float => Some("float"),
			_ => None, // a Node (a text, a list, an Error) is used as it comes
		});
	let node = guarded_calls(node, &guardable, &wrapped);
	let node = resolved(node, &path, &wrapped);
	let node = forwarded_raises(node, &wrapped);
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
			Node::Key(left, Op::Add | Op::Sub | Op::Mul, right) => surely_int(left, literal_kinds) && surely_int(right, literal_kinds),
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

/// A handler of a task on a thread: after every later statement of its block (loop bodies too) it runs once the task's
/// status shows its event; a final check at the block's end waits for the task (a stopped or finished one)
#[derive(Clone)]
struct TaskHandler {
	job: Node,
	status: i64,
	body: Node,
	flag: String,
}

impl TaskHandler {
	fn check(&self) -> Node {
		use crate::variable_signals::{assign, block, if_then};
		let not_handled = Node::Key(Box::new(Node::Empty), Op::Not, Box::new(Node::Symbol(self.flag.clone())));
		let status = Node::Key(Box::new(marker(crate::host::TASK_STATUS, vec![self.job.clone()])), Op::Eq, Box::new(Node::Number(crate::extensions::numbers::Number::Int(self.status))));
		let condition = Node::Key(Box::new(not_handled), Op::And, Box::new(status));
		if_then(condition, block(vec![assign(&self.flag, Node::True), self.body.clone()]))
	}

	/// At the end of the block: a finish or stop handler waits for the task, a pause handler does not
	fn final_check(&self) -> Option<Node> {
		(self.status != crate::host::TASK_PAUSED).then(|| {
			crate::variable_signals::block(vec![marker(crate::host::TASK_JOIN, vec![self.job.clone()]), self.check()])
		})
	}
}

struct TaskHandlers<'a> {
	path: &'a dyn Fn(&str) -> TaskPath,
	count: std::cell::Cell<usize>,
}

impl TaskHandlers<'_> {
	fn lower(&self, node: Node, active: &[TaskHandler]) -> Node {
		match node {
			Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(&bracket, &separator) => {
				Node::List(self.statements(items, active), bracket, separator)
			}
			other => other.map_children(|child| self.lower(child, active)),
		}
	}

	/// The checks follow each statement; the last statement keeps the block's value, so the checks of the block (and
	/// the final ones) come before it
	fn statements(&self, items: Vec<Node>, outer: &[TaskHandler]) -> Vec<Node> {
		let (mut active, mut own, mut out) = (outer.to_vec(), vec![], vec![]);
		let count = items.len();
		for (index, item) in items.into_iter().enumerate() {
			let is_handler = matches!(item.drop_meta(), Node::List(parts, _, _) if word(parts.first().unwrap_or(&Node::Empty)) == TASK_ON);
			if !is_handler {
				let lowered = self.lower(item, &active);
				if index + 1 == count {
					out.extend(active.iter().map(TaskHandler::check));
					out.extend(own.iter().filter_map(TaskHandler::final_check));
					own.clear();
					out.push(lowered);
				} else {
					out.push(lowered);
					out.extend(active.iter().map(TaskHandler::check));
				}
				continue;
			}
			let Node::List(parts, _, _) = item.drop_meta() else { unreachable!("a handler is a list") };
			let (job, function, event, body) = (parts[1].clone(), word(&parts[2]), word(&parts[3]), self.lower(parts[4].clone(), &[]));
			if (self.path)(&function) == TaskPath::Inline {
				// the task finished where it started: a finish handler runs now, one of a stop or pause never
				out.push(match event.as_str() {
					FINISH_EVENT => body,
					_ => never_happens(&job, &format!("`{event}` of {} never happens: a task finishes where it starts, its handler never runs", job.name())),
				});
				continue;
			}
			let status = match event.as_str() {
				FINISH_EVENT => crate::host::TASK_FINISHED,
				"pause" | "pauses" | "paused" => crate::host::TASK_PAUSED,
				_ => crate::host::TASK_STOPPED, // stop, cancel
			};
			let flag = format!("{HANDLED_PREFIX}{}", self.count.get());
			self.count.set(self.count.get() + 1);
			out.push(crate::variable_signals::assign(&flag, Node::False));
			let handler = TaskHandler { job, status, body, flag };
			active.push(handler.clone());
			own.push(handler);
		}
		out.extend(own.iter().filter_map(TaskHandler::final_check));
		out
	}
}

/// The suffix of the wrapper a task of values is started through: `total·node(arguments·node) := { r·node =
/// total(arguments·node#1); r·node }` takes the argument list and gives a plain value, where total itself may take and
/// give arrays (list_abi)
pub(crate) const NODE_WRAPPER_SUFFIX: &str = "·node";
const WRAPPER_RESULT: &str = "r·node";
/// The one parameter of the wrapper: the list of the arguments, as task_spawn_values carries it (the host passes one
/// Node and needs no parameter types: a browser cannot read them)
const WRAPPER_ARGUMENTS: &str = "arguments·node";

/// The program with the wrappers of the started functions (name → parameter count) defined first
pub(crate) fn with_node_wrappers(node: Node, wrapped: &std::collections::BTreeMap<String, usize>) -> Node {
	// a page event's handler has its wrapper already (event_signals.rs), a task may want it too
	let mut defined = std::collections::HashSet::new();
	node.visit(&mut |part| if let Node::Key(head, Op::Define, _) = part {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			defined.insert(word(&items[0]));
		}
	});
	let wrapped: Vec<(&String, &usize)> = wrapped.iter().filter(|(function, _)| !defined.contains(&format!("{function}{NODE_WRAPPER_SUFFIX}"))).collect();
	if wrapped.is_empty() {
		return node;
	}
	let items: Vec<Node> = wrapped.into_iter().map(|(function, count)| {
		let arguments = Node::Symbol(WRAPPER_ARGUMENTS.to_string());
		// declared a list: a function of no arguments leaves it unused, which would make it an Int (the host passes a Node)
		let declared = Node::Key(Box::new(arguments.clone()), Op::Colon, Box::new(Node::Symbol("list".to_string())));
		let head = Node::List(vec![Node::Symbol(format!("{function}{NODE_WRAPPER_SUFFIX}")), declared], Bracket::Round, Separator::None);
		let argument = |index: usize| Node::Key(Box::new(arguments.clone()), Op::Hash, Box::new(Node::Number(crate::extensions::numbers::Number::Int(index as i64 + 1))));
		let call = Node::List([vec![Node::Symbol(function.clone())], (0..*count).map(argument).collect()].concat(), Bracket::Round, Separator::None);
		let result = Node::Symbol(WRAPPER_RESULT.to_string());
		let body = Node::List(vec![Node::Key(Box::new(result.clone()), Op::Assign, Box::new(call)), result], Bracket::Curly, Separator::Semicolon);
		Node::Key(Box::new(head), Op::Define, Box::new(body))
	}).collect();
	with_definitions_first(node, items)
}

/// The program with `definitions` as its first statements
pub(crate) fn with_definitions_first(node: Node, mut definitions: Vec<Node>) -> Node {
	if definitions.is_empty() {
		return node;
	}
	match node {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			definitions.extend(statements);
			Node::List(definitions, Bracket::None, separator)
		}
		single => {
			definitions.push(single);
			Node::List(definitions, Bracket::None, Separator::Semicolon)
		}
	}
}

/// A raise inside a task reaches the starting thread's handlers (P110, notes/signals.md phase 6): each handler
/// `on·alarm(event) := body` (event_signals.rs) becomes `if task_inside() { signal_send("on·alarm·node", [event]) } else
/// { body }`, so in a task's instance it forwards the event, and the program runs it through the node wrapper
fn forwarded_raises(node: Node, wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>) -> Node {
	let handlers: std::collections::HashSet<String> = crate::event_signals::handled_signals(&node).into_iter().map(|(_, function)| function).collect();
	if handlers.is_empty() {
		return node;
	}
	fn forward(node: Node, handlers: &std::collections::HashSet<String>, wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>) -> Node {
		match node {
			Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if handlers.contains(&word(&items[0]))) => {
				let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
				let function = word(&items[0]);
				wrapped.borrow_mut().insert(function.clone(), items.len() - 1);
				let arguments = Node::List(items[1..].to_vec(), Bracket::Square, Separator::Colon);
				let send = marker(crate::host::SIGNAL_SEND, vec![Node::Text(format!("{function}{NODE_WRAPPER_SUFFIX}")), arguments]);
				let template = crate::wasp_parser::parse(&format!("if {}() {{ {FORWARD_PLACEHOLDER} }} else {{ {BODY_PLACEHOLDER} }}", crate::host::TASK_INSIDE));
				let body = crate::library_words::substitute(crate::library_words::substitute(template, FORWARD_PLACEHOLDER, &send), BODY_PLACEHOLDER, &body);
				Node::Key(head, Op::Define, Box::new(body))
			}
			other => other.map_children(|child| forward(child, handlers, wrapped)),
		}
	}
	forward(node, &handlers, wrapped)
}

const FORWARD_PLACEHOLDER: &str = "forwarded_raise_placeholder";
const BODY_PLACEHOLDER: &str = "handler_body_placeholder";

/// `task·check(task_join(job), task_failure(job)); task_await(job)`: a failed task raises its error from wasm, which
/// `try` catches, before the result is read
fn checked_await(await_word: &str, job: &Node) -> Node {
	use crate::host::{TASK_CHECK, TASK_FAILURE, TASK_JOIN};
	let check = marker(TASK_CHECK, vec![marker(TASK_JOIN, vec![job.clone()]), marker(TASK_FAILURE, vec![job.clone()])]);
	Node::List(vec![check, marker(await_word, vec![job.clone()])], Bracket::None, Separator::Semicolon)
}

/// `try f(a, b) else Y` of a user function f: the guarded call goes through the host, `guarded_call("f·node", [a, b])`,
/// whose stack overflow is an Error `try` catches (host.rs guarded_call); f's node wrapper is the one tasks use. Any other
/// guarded expression stays as it is
fn guarded_calls(node: Node, guardable: &dyn Fn(&str) -> Option<Option<&'static str>>, wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>) -> Node {
	match node {
		Node::List(items, bracket, separator) if items.len() >= 3 && word(&items[0]) == crate::wasp_parser::TRY_MARKER => {
			let items: Vec<Node> = items.into_iter().map(|item| guarded_calls(item, guardable, wrapped)).collect();
			let guarded = match items[1].drop_meta() {
				Node::List(call, Bracket::Round, _) if matches!(call.first().map(Node::drop_meta), Some(Node::Symbol(function)) if guardable(function).is_some()) => {
					let function = word(&call[0]);
					wrapped.borrow_mut().insert(function.clone(), call.len() - 1);
					let arguments = Node::List(call[1..].to_vec(), Bracket::Square, Separator::Colon);
					let guarded = marker(crate::host::GUARDED_CALL, vec![Node::Text(format!("{function}{NODE_WRAPPER_SUFFIX}")), arguments]);
					// an Int or Float result is that again (a caught overflow's Error fails the conversion, which `try` catches)
					match guardable(&function).flatten() {
						Some(type_word) => Node::Key(Box::new(guarded), Op::As, Box::new(Node::Symbol(type_word.to_string()))),
						None => guarded,
					}
				}
				_ => items[1].clone(),
			};
			Node::List([vec![items[0].clone(), guarded], items[2..].to_vec()].concat(), bracket, separator)
		}
		other => other.map_children(|child| guarded_calls(child, guardable, wrapped)),
	}
}

/// Every result of a job list: `jobs.map(awaited_job => <checked await of awaited_job>)`
fn awaited_jobs(list: &Node) -> Node {
	use crate::host::TASK_AWAIT_VALUE;
	let template = crate::wasp_parser::parse(&format!("{TASK_LIST_PLACEHOLDER}.map({AWAITED_JOB} => {AWAITED_PLACEHOLDER})"));
	let awaited = checked_await(TASK_AWAIT_VALUE, &Node::Symbol(AWAITED_JOB.to_string()));
	let template = crate::library_words::substitute(template, TASK_LIST_PLACEHOLDER, list);
	crate::library_words::substitute(template, AWAITED_PLACEHOLDER, &awaited)
}

const TASK_LIST_PLACEHOLDER: &str = "awaited_jobs_list";
const AWAITED_PLACEHOLDER: &str = "awaited_job_result";

fn resolved(node: Node, path: &dyn Fn(&str) -> TaskPath, wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>) -> Node {
	use crate::host::{TASK_AWAIT, TASK_AWAIT_VALUE, TASK_CONTROL, TASK_SPAWN, TASK_SPAWN_VALUES};
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| resolved(item, path, wrapped)).collect();
			let head = word(items.first().unwrap_or(&Node::Empty));
			let (read, said) = read_marker(&head);
			if let Some(error) = (!said).then(|| unsaid_wait(read, &items, path)).flatten() {
				return error;
			}
			match read {
				TASK_GO => {
					let function = word(&items[1]);
					let arguments = items[2..].to_vec();
					match path(&function) {
						TaskPath::Ints => marker(TASK_SPAWN, [vec![Node::Text(function)], arguments].concat()),
						TaskPath::Values(parameters) => {
							let float = |argument: Node| Node::Key(Box::new(argument), Op::As, Box::new(Node::Symbol("float".to_string())));
							// a character crosses as a one-character text, as a variable holds it
							let arguments = arguments.into_iter().zip(parameters.iter().chain(std::iter::repeat(&crate::type_kinds::Kind::Empty)))
								.map(|(argument, kind)| match argument.drop_meta() {
									Node::Char(character) => Node::Text(character.to_string()),
									_ if *kind == crate::type_kinds::Kind::Float => float(argument),
									_ => argument,
								}).collect();
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
				TASK_LIST | TASK_ELEMENT => {
					let threaded = items[2..].iter().all(|function| path(&word(function)) != TaskPath::Inline);
					match (threaded, read == TASK_LIST) {
						(false, _) => items[1].clone(),
						(true, false) => checked_await(TASK_AWAIT_VALUE, &items[1]),
						(true, true) => awaited_jobs(&items[1]),
					}
				}
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

/// A task read as its value without `await` (`job + 1`, `starts = jobs`) on a thread: the program waits there, unsaid.
/// A got-it warning naming `await`; the error a strict program makes of it
fn unsaid_wait(read: &str, items: &[Node], path: &dyn Fn(&str) -> TaskPath) -> Option<Node> {
	let functions = match read {
		TASK_VALUE => &items[2..3],
		TASK_LIST | TASK_ELEMENT => &items[2..],
		_ => return None,
	};
	if functions.iter().any(|function| path(&word(function)) == TaskPath::Inline) {
		return None; // it ran where it started: nothing to wait for
	}
	let written = crate::diagnostic::written_text(&items[1]);
	let (all, verb) = if read == TASK_LIST { ("all ", "waits for every task in it") } else { ("", "waits for the task") };
	let question = crate::diagnostic::Ask::new(IMPLICIT_WAIT_TOPIC, format!("{written} is used as its value: the program {verb} here; write `await {all}{written}` to say so"),
		vec![crate::diagnostic::reading("wait here", &format!("await {all}{written}"))], crate::diagnostic::Fallback::Warning)
		.written(&written).at_node(&items[1]);
	crate::diagnostic::ask(&question).err()
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
/// P28 (user, 2026-10-05: "Zero value (Go)"): `real x;` declares x with the zero value of its type, `x:real = 0.0`.
/// Only a fresh name: after `x = "5"`, `int x` is the conversion of x
pub fn lower_bare_declarations(program: Node) -> Node {
	// a program of only `int n` is one statement
	match zero_declaration(&program, &std::collections::HashSet::new()) {
		Some(declaration) => declaration,
		None => declarations_in(program),
	}
}

fn declarations_in(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let statements = matches!(separator, Separator::Semicolon | Separator::Newline);
			let mut assigned = std::collections::HashSet::new();
			let items = items.into_iter().map(|item| {
				let item = declarations_in(item);
				let declared = if statements { zero_declaration(&item, &assigned) } else { None };
				crate::library_words::collect_assigned_names(&item, &mut assigned);
				declared.unwrap_or(item)
			}).collect();
			Node::List(items, bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(declarations_in(*left)), op, Box::new(declarations_in(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(declarations_in(*node)), data },
		other => other,
	}
}

/// `T name` of a type word with a zero value and a name not assigned before: `name:T = zero`
fn zero_declaration(statement: &Node, assigned: &std::collections::HashSet<String>) -> Option<Node> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else { return None };
	let [type_word, name] = items.as_slice() else { return None };
	let (Node::Symbol(type_name), Node::Symbol(variable)) = (type_word.drop_meta(), name.drop_meta()) else { return None };
	if assigned.contains(variable) || crate::analyzer::type_word_kind(variable).is_some() {
		return None;
	}
	let zero = zero_value(crate::analyzer::type_word_kind(type_name)?)?;
	let typed = Node::Key(Box::new(name.clone()), Op::Colon, Box::new(type_word.clone()));
	Some(Node::Key(Box::new(typed), Op::Assign, Box::new(zero)))
}

fn zero_value(kind: crate::type_kinds::Kind) -> Option<Node> {
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
		Node::List(items, Bracket::None, Separator::Space) if sized_array(&items).is_some() => sized_array(&items).expect("guarded"),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower_sized_arrays).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(lower_sized_arrays(*left)), op, Box::new(lower_sized_arrays(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_sized_arrays(*node)), data },
		other => other,
	}
}

fn sized_array(items: &[Node]) -> Option<Node> {
	let [binding, element_type] = items else { return None };
	let Node::Symbol(type_name) = element_type.drop_meta() else { return None };
	let Node::Key(name, Op::Assign | Op::Colon, count) = binding.drop_meta() else { return None };
	let is_count = !matches!(count.drop_meta(), Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square | Bracket::Curly, _));
	(matches!(name.drop_meta(), Node::Symbol(_)) && is_count && crate::analyzer::type_word_kind(type_name).is_some()).then(|| {
		let array = Node::Key(Box::new(Node::List(vec![count.as_ref().clone()], Bracket::Round, Separator::None)), Op::Mul, Box::new(element_type.clone()));
		Node::Key(name.clone(), Op::Assign, Box::new(array))
	})
}

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
			let head = Node::List([vec![name], parameters].concat(), Bracket::Round, Separator::None);
			let body = match one_parameter(&head) {
				Some(parameter) => bind_it(body, &parameter, false),
				None => body,
			};
			let _ = (bracket, separator);
			Node::Key(Box::new(head), Op::Define, Box::new(body))
		}
		// `f(x) := x + it`: `it` is the one parameter too
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if one_parameter(&head).is_some() => {
			let parameter = one_parameter(&head).expect("guarded");
			let body = bind_it(lower_spaced_definitions(*body), &parameter, false);
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
	// `all numbers > 2: body`: the comparison acts on every item (wiki/iteration.md), a filter loop (P46, its warning)
	let (list, body) = match list.drop_meta() {
		Node::Key(collection, op, value) if op.is_comparison() => {
			let condition = Node::Key(Box::new(Node::Symbol(IT_PARAMETER.to_string())), *op, value.clone());
			let written = format!("{} {op} {}", word.drop_meta().name(), crate::normalize::operand_text(value));
			let question = crate::diagnostic::Ask::new(crate::wasp_parser::FILTER_LOOP_TOPIC, format!("`{written}: …` visits only the items that pass its filter"),
				vec![crate::diagnostic::reading("filter the items", &format!("for it in … {{ if it {op} {} {{ … }} }}", crate::normalize::operand_text(value)))],
				crate::diagnostic::Fallback::Warning).written(&written).at_node(word);
			if let Err(error) = crate::diagnostic::ask(&question) {
				return Some(error);
			}
			let guarded = Node::Key(Box::new(Node::Key(Box::new(Node::Empty), Op::If, Box::new(condition))), Op::Then, Box::new(body));
			(collection.as_ref().clone(), guarded)
		}
		_ => (list, body),
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
	let definition = items.iter().position(|item| matches!(item.drop_meta(), Node::Key(target, Op::Define | Op::Assign, _) if matches!(target.drop_meta(), Node::Symbol(_))))?;
	let (words, rest) = items.split_at(definition);
	let (name, parameters) = words.split_first()?;
	let is_word = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if !is_function_keyword(word));
	if !is_word(name) || !parameters.iter().all(is_word) {
		return None;
	}
	let (definition, extra) = rest.split_first()?;
	let Node::Key(last, op, body) = definition.drop_meta() else { return None };
	// with `=` only a recursive definition (wiki/Home.md `fibonacci number = … fibonacci …`): `print x = 5` stays
	if *op == Op::Assign && !crate::wasp_parser::mentions(body, &name.name()) {
		return None;
	}
	let words: Vec<Node> = parameters.iter().map(|parameter| parameter.drop_meta().clone()).chain(std::iter::once(last.drop_meta().clone())).collect();
	let names: Vec<String> = words.iter().map(Node::name).collect();
	let names: Vec<&str> = names.iter().map(String::as_str).collect();
	// one parse with type_name_matching: `square of a number := …` takes the same slots as with `=`
	let parameters = match crate::type_name_matching::spaced_parameters(&names, body) {
		None => words,
		Some(Ok(parameters)) => parameters,
		Some(Err(_)) => return None, // type_name_matching reports it
	};
	Some((name.drop_meta().clone(), parameters, applied_to_last(body.as_ref().clone(), extra)))
}

/// The one named parameter of a definition head `f(x)` (`f(x:int)`), not `it`
/// `it` in the body of a function of one parameter is that parameter, except inside a block with `it` given as a value
/// (returned, assigned, an argument): `mk(k) := { return {it * k} }` returns the function `it => it * k`
fn bind_it(node: Node, parameter: &str, is_value: bool) -> Node {
	let values = |items: Vec<Node>| -> Vec<Node> {
		let mut items = items.into_iter();
		items.next().map(|head| bind_it(head, parameter, false)).into_iter().chain(items.map(|item| bind_it(item, parameter, true))).collect()
	};
	match node {
		Node::Symbol(name) if name == IT_PARAMETER => Node::Symbol(parameter.to_string()),
		Node::List(_, Bracket::Curly, _) if is_value && crate::wasp_parser::mentions(&node, IT_PARAMETER) => crate::lambdas::block_as_arrow(&node).unwrap_or(node),
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => Node::Key(Box::new(bind_it(*target, parameter, false)), op, Box::new(bind_it(*value, parameter, true))),
		// `return {…}`, `f({…})`, `f {…}`
		Node::List(items, bracket @ (Bracket::Round | Bracket::None), separator @ (Separator::Space | Separator::None | Separator::Colon))
			if items.len() > 1 && matches!(items[0].drop_meta(), Node::Symbol(_)) =>
		{
			Node::List(values(items), bracket, separator)
		}
		// the last statement of a body is its value
		Node::List(items, bracket, separator) => {
			let last = items.len().saturating_sub(1);
			let is_body = bracket == Bracket::Curly || separator == Separator::Semicolon;
			Node::List(items.into_iter().enumerate().map(|(index, item)| bind_it(item, parameter, is_body && index == last && !is_value)).collect(), bracket, separator)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(bind_it(*left, parameter, false)), op, Box::new(bind_it(*right, parameter, false))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(bind_it(*node, parameter, is_value)), data },
		other => other,
	}
}

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
	lower_lists(end_definitions(node), |items| c_function(items).or_else(|| keyword_definition(items)).or_else(|| extension_block(items)).or_else(|| partial_application(items)))
}

/// `add(1, _)`: a call with placeholders is the lambda of the missing arguments, `partial_1 => add(1, partial_1)`
fn partial_application(items: &[Node]) -> Option<Node> {
	let [Node::Symbol(_), arguments @ ..] = items else { return None };
	let is_placeholder = |argument: &Node| matches!(argument.drop_meta(), Node::Symbol(name) if name == PLACEHOLDER);
	if !arguments.iter().any(is_placeholder) {
		return None;
	}
	// Swift's `func twice(_ x: Int)`: `_` labels a typed parameter, no placeholder
	let typed_parameter = |node: &Node| matches!(node.drop_meta(), Node::Key(_, Op::Colon, type_node) if is_type_word(type_node));
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
	let call = Node::List(std::iter::once(items[0].clone()).chain(arguments).collect(), Bracket::Round, Separator::None);
	let head = match parameters.as_slice() {
		[single] => single.clone(),
		_ => Node::List(parameters, Bracket::Round, Separator::Colon),
	};
	Some(Node::Key(Box::new(head), Op::FatArrow, Box::new(call)))
}

/// `def f(a, b) { body }`, `def f(x) := body`, `function g() { … }`: the definition `f(a, b) := body`
pub(crate) fn keyword_definition(items: &[Node]) -> Option<Node> {
	let (keyword, definition, mut result_type) = match items {
		[keyword, definition] => (keyword, definition.drop_meta().clone(), None),
		// a tuple result type `-> (Int, Int) {…}` (Swift): the body's tuple as it is
		[keyword, head, body] if tuple_result(head).is_some() => (keyword, Node::List(vec![tuple_result(head).expect("guarded").clone(), body.clone()], Bracket::Round, Separator::None), None),
		[keyword, head, body] => (keyword, Node::List(vec![head.clone(), body.clone()], Bracket::Round, Separator::None), None),
		// Go's `func add1(x int) int {…}`: the result type between the head and the body
		[keyword, head, result_type, body] if is_type_word(result_type) => (keyword, Node::List(vec![head.clone(), body.clone()], Bracket::Round, Separator::None), Some(result_type.clone())),
		_ => return None,
	};
	if !matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)) {
		return None;
	}
	let definition = extension_definition(definition);
	let definition = match typed_result(&definition) {
		Some((untyped, written_type)) => {
			result_type = Some(written_type);
			untyped
		}
		None => definition,
	};
	let (head, op, body) = match definition {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) => (*head, op, *body),
		// `def test: print "test"` (wiki/signal.md): a function without parameters
		Node::Key(name, Op::Colon, body) if matches!(name.drop_meta(), Node::Symbol(_)) => {
			(Node::List(vec![*name], Bracket::Round, Separator::None), Op::Define, *body)
		}
		// Python's `def apply(f, x): return f(x)`, early, so the function value passes see a definition; without
		// parameters it stays a def form for late_binding, which would read `z() := e` as a getter (P71)
		Node::Key(head, Op::Colon, body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() > 1 && matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))) => {
			(*head, Op::Define, *body)
		}
		Node::List(parts, Bracket::Round, _) if parts.len() == 2 && matches!(parts[1].drop_meta(), Node::List(_, Bracket::Curly, _)) => (parts[0].clone(), Op::Define, parts[1].clone()),
		_ => return None,
	};
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	// Swift's one labeled parameter `greet(person name: String)` arrives as the two words
	let arguments = match arguments {
		[label, typed] if argument_label(label, typed).is_some() => vec![Node::List(arguments.to_vec(), Bracket::None, Separator::Space)],
		_ => arguments.to_vec(),
	};
	// the parameters may come as one group: `f (a, b)`, `f (m)`, `f ø`
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, Separator::Space) if matches!(group.as_slice(), [label, typed] if argument_label(label, typed).is_some()) => vec![argument.clone()],
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![argument.clone()],
	}).map(name_then_type);
	let mut labeled_names = vec![];
	let mut parameters: Vec<Node> = parameters.map(|parameter| labeled_parameter(parameter, &mut labeled_names)).collect();
	let body = with_label_names(body, labeled_names);
	// `func add1(x int)`: the one parameter and its type arrive as two words
	if let [parameter, type_word] = parameters.as_slice() {
		if matches!(parameter.drop_meta(), Node::Symbol(_)) && is_type_word(type_word) {
			parameters = vec![Node::Key(Box::new(parameter.clone()), Op::Colon, Box::new(type_word.clone()))];
		}
	}
	let head = Node::List(std::iter::once(name.clone()).chain(parameters).collect(), Bracket::Round, Separator::None);
	let target = match result_type {
		Some(result_type) => Node::Key(Box::new(head), Op::Colon, Box::new(result_type)),
		None => head,
	};
	Some(Node::Key(Box::new(target), op, Box::new(body)))
}

/// Kotlin's extension method `fun Int.twice() = this * 2`: the function `twice(this:Int) := this * 2`, its receiver the
/// first parameter, named as the body names it (`this`, or Swift's `self`), so `3.twice()` calls it as a method
fn extension_definition(definition: Node) -> Node {
	let Node::Key(head, op, rest) = definition else { return definition };
	let Node::Key(receiver_type, Op::Dot, call) = head.drop_meta() else { return Node::Key(head, op, rest) };
	let Node::List(call, Bracket::Round, separator) = call.drop_meta() else { return Node::Key(head, op, rest) };
	if !matches!(receiver_type.drop_meta(), Node::Symbol(_)) || !matches!(call.first().map(Node::drop_meta), Some(Node::Symbol(_))) {
		return Node::Key(head, op, rest);
	}
	let head = with_receiver(call, separator, receiver_type, &rest);
	Node::Key(Box::new(head), op, rest)
}

/// The head `name(receiver:T, parameters…)` of a method on T whose call is `name(parameters…)`
fn with_receiver(call: &[Node], separator: &Separator, receiver_type: &Node, body: &Node) -> Node {
	let receiver = if body.mentions_any(&[SELF_WORD]) { SELF_WORD } else { THIS_WORD };
	let receiver = Node::Key(Box::new(Node::Symbol(receiver.to_string())), Op::Colon, Box::new(receiver_type.clone()));
	let parameters = call[1..].iter().flat_map(|parameter| match parameter.drop_meta() {
		Node::List(group, Bracket::Round, _) => group.clone(),
		Node::Empty => vec![],
		_ => vec![parameter.clone()],
	});
	Node::List(std::iter::once(call[0].clone()).chain(std::iter::once(receiver)).chain(parameters).collect(), Bracket::Round, separator.clone())
}

/// Swift's `extension Int { func twice() -> Int { self * 2 } }`: each function of the block a method on the type, as
/// `fun Int.twice()` defines it
fn extension_block(items: &[Node]) -> Option<Node> {
	let [word, receiver_type, block] = items else { return None };
	let is_extension = matches!(word.drop_meta(), Node::Symbol(word) if word == EXTENSION_WORD) && matches!(receiver_type.drop_meta(), Node::Symbol(_));
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
		let Node::Key(head, op @ (Op::Define | Op::Assign), body) = definition.drop_meta() else { return definition.clone() };
		let (call, result_type) = match head.drop_meta() {
			Node::Key(call, Op::Colon, result_type) => (call.drop_meta(), Some(result_type)),
			call => (call, None),
		};
		let Node::List(call, Bracket::Round, separator) = call else { return definition.clone() };
		let head = with_receiver(call, separator, receiver_type, body);
		let head = match result_type {
			Some(result_type) => Node::Key(Box::new(head), Op::Colon, result_type.clone()),
			None => head,
		};
		Node::Key(Box::new(head), op.clone(), body.clone())
	};
	Some(Node::List(definitions.iter().map(method).collect(), Bracket::None, Separator::Semicolon))
}

/// `f(x) -> int { body }`, `f(x): int { body }` (Rust/Swift/Python, TypeScript/Kotlin) and `f(x) -> int: body`
/// (Python): the definition `(f(x) { body })` without its result type, and the type
fn typed_result(definition: &Node) -> Option<(Node, Node)> {
	let Node::Key(head, Op::Arrow | Op::Colon, typed_body) = definition else { return None };
	if !matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))) {
		return None;
	}
	let (result_type, body) = match typed_body.drop_meta() {
		Node::List(parts, _, Separator::Space) if parts.len() == 2 && is_type_word(&parts[0]) && matches!(parts[1].drop_meta(), Node::List(_, Bracket::Curly, _)) => (parts[0].clone(), parts[1].clone()),
		Node::Key(result_type, Op::Colon, body) if is_type_word(result_type) => (result_type.as_ref().clone(), body.as_ref().clone()),
		_ => return None,
	};
	Some((Node::Key(head.clone(), Op::Define, Box::new(body)), result_type))
}

/// `f(a, b) -> (Int, Int)`: the head of a function whose result type is a tuple of types
fn tuple_result(head: &Node) -> Option<&Node> {
	let Node::Key(head, Op::Arrow, result) = head.drop_meta() else { return None };
	let Node::List(types, Bracket::Round, _) = result.drop_meta() else { return None };
	(types.len() > 1 && types.iter().all(is_type_word)).then_some(head.as_ref())
}

/// Swift's `label name: T`: the label (`_` for none)
fn argument_label<'a>(label: &'a Node, typed: &Node) -> Option<&'a str> {
	let Node::Symbol(label) = label.drop_meta() else { return None };
	matches!(typed.drop_meta(), Node::Key(name, Op::Colon, _) if matches!(name.drop_meta(), Node::Symbol(_))).then_some(label.as_str())
}

/// `person name: String` is the parameter `person: String` whose value the body reads as name (`labeled_names` gets
/// `name = person`); `_ name: String` is `name: String`
fn labeled_parameter(parameter: Node, labeled_names: &mut Vec<Node>) -> Node {
	let Node::List(words, _, _) = parameter.drop_meta() else { return parameter };
	let [label, typed] = words.as_slice() else { return parameter };
	let Some(label) = argument_label(label, typed) else { return parameter };
	let Node::Key(name, Op::Colon, type_node) = typed.drop_meta() else { return parameter };
	if label == WILDCARD_LABEL {
		return typed.clone();
	}
	labeled_names.push(Node::Key(name.clone(), Op::Assign, Box::new(Node::Symbol(label.to_string()))));
	Node::Key(Box::new(Node::Symbol(label.to_string())), Op::Colon, type_node.clone())
}

/// The body with `name = label` first for every labeled parameter
fn with_label_names(body: Node, labeled_names: Vec<Node>) -> Node {
	if labeled_names.is_empty() {
		return body;
	}
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, separator) if items.len() == 1 || matches!(separator, Separator::Semicolon | Separator::Newline) => {
			Node::List([labeled_names, items.clone()].concat(), Bracket::Curly, Separator::Semicolon)
		}
		_ => Node::List([labeled_names, vec![body]].concat(), Bracket::Curly, Separator::Semicolon),
	}
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

/// Ruby's `def f(a, b: 2) a * b end` and `def f(a)` ⏎ statements ⏎ `end`: the definition `def f(a, b: 2) {…}`
fn end_definitions(node: Node) -> Node {
	let node = match node {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(end_definitions).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(end_definitions(*left)), op, Box::new(end_definitions(*right))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(end_definitions(*node)), data },
		other => other,
	};
	let Node::List(items, bracket, separator) = node else { return node };
	// one line: `def head body… end`
	if let ([keyword, head, body @ .., end], Separator::Space) = (items.as_slice(), &separator) {
		if is_function_keyword(&keyword.drop_meta().name()) && is_end(end) && !body.is_empty() {
			return Node::List(vec![keyword.clone(), function_head(head), body_block(body.to_vec())], bracket, separator);
		}
	}
	if !matches!(separator, Separator::Newline | Separator::Semicolon) || !items.iter().any(is_end) {
		return Node::List(items, bracket, separator);
	}
	// statements: `def head`, the body's statements, `end`
	let mut statements: Vec<Node> = vec![];
	let mut rest = items.into_iter();
	while let Some(statement) = rest.next() {
		let Some((keyword, head)) = bodiless_definition(&statement) else {
			statements.push(statement);
			continue;
		};
		let body: Vec<Node> = rest.by_ref().take_while(|statement| !is_end(statement)).collect();
		statements.push(Node::List(vec![keyword, function_head(&head), body_block(body)], Bracket::None, Separator::Space));
	}
	Node::List(statements, bracket, separator)
}

fn is_end(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if word == END_WORD)
}

/// `def f(a)` or `def h` without a body
fn bodiless_definition(statement: &Node) -> Option<(Node, Node)> {
	let Node::List(items, Bracket::None, Separator::Space) = statement.drop_meta() else { return None };
	let [keyword, head] = items.as_slice() else { return None };
	let is_head = matches!(head.drop_meta(), Node::Symbol(_)) || matches!(head.drop_meta(), Node::List(parts, Bracket::Round, _) if matches!(parts.first().map(Node::drop_meta), Some(Node::Symbol(_))));
	(is_function_keyword(&keyword.drop_meta().name()) && is_head).then(|| (keyword.clone(), head.clone()))
}

/// `h` → `h()`; `(f a b:2)` → `(f (a, b:2))`, the parameters as one comma group as `def f(a, b: 2) {…}` has them (two
/// loose words would read as Swift's labeled parameter)
fn function_head(head: &Node) -> Node {
	match head.drop_meta() {
		Node::Symbol(_) => Node::List(vec![head.clone()], Bracket::Round, Separator::None),
		Node::List(items, Bracket::Round, separator) if items.len() > 2 => {
			let parameters = Node::List(items[1..].to_vec(), Bracket::Round, Separator::Colon);
			Node::List(vec![items[0].clone(), parameters], Bracket::Round, separator.clone())
		}
		_ => head.clone(),
	}
}

fn body_block(statements: Vec<Node>) -> Node {
	Node::List(statements, Bracket::Curly, Separator::Semicolon)
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


/// Swift's label for an argument without a label
const WILDCARD_LABEL: &str = "_";

/// `real f(real x) { … }`: the definition `f(x:real) := { … }` (the parser reads the type word, then the call and its
/// block); the result kind is inferred as for any definition
fn c_function(items: &[Node]) -> Option<Node> {
	let [result_type, definition] = items else { return None };
	let Node::Symbol(result_type) = result_type.drop_meta() else { return None };
	if crate::analyzer::type_word_kind(result_type).is_none() && result_type != "void" {
		return None;
	}
	let (head, body) = match definition.drop_meta() {
		Node::List(parts, Bracket::Round, _) => match parts.as_slice() {
			[head, body] if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => (head, body),
			_ => return None,
		},
		// C#'s expression-bodied member `int Add(int a, int b) => a + b`
		Node::Key(head, Op::FatArrow, body) => (head.as_ref(), body.as_ref()),
		_ => return None,
	};
	let Node::List(head_items, Bracket::Round, _) = head.drop_meta() else { return None };
	let (name, arguments) = head_items.split_first()?;
	let Node::Symbol(_) = name.drop_meta() else { return None };
	let parameters = arguments.iter().flat_map(|argument| match argument.drop_meta() {
		Node::List(group, Bracket::Round, Separator::Colon) => group.clone(), // `(real a, int b)`
		Node::Empty => vec![], // `f()`
		_ => vec![argument.clone()],
	});
	let parameters: Option<Vec<Node>> = parameters.map(|parameter| c_parameter(&parameter)).collect();
	let head = Node::List([vec![name.clone()], parameters?].concat(), Bracket::Round, Separator::Colon);
	Some(Node::Key(Box::new(head), Op::Define, Box::new(body.clone())))
}

/// `real x` is `x:real`; a bare name stays untyped, as does a parameter the parser typed already
fn c_parameter(parameter: &Node) -> Option<Node> {
	match parameter.drop_meta() {
		Node::Symbol(_) | Node::Key(_, Op::Colon, _) => Some(parameter.clone()),
		Node::List(words, _, Separator::Space) => match words.as_slice() {
			[kind, name] if matches!((kind.drop_meta(), name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) => {
				Some(Node::Key(Box::new(name.clone()), Op::Colon, Box::new(kind.clone())))
			}
			_ => None,
		},
		_ => None,
	}
}
