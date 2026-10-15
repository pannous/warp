//! Tasks: `job = go f(x)`, `await job`, `await any`/`all`, `within` deadlines, task controls and handlers (wiki/async.md)
use super::*;
use crate::lowering::nodes::{fresh_number, phrase_words};
use crate::node::parens;
use crate::words::{ADD_WORD, ALL_WORD, RAISE_WORD};

/// `go f(x)` starts a task, `await job` waits for it
pub(super) const TASK_WORDS: [&str; 2] = ["go", "await"];
/// What a task signals when it is done, and the controls that would interrupt one
pub(super) const FINISH_EVENTS: [&str; 5] = ["finishes", "finished", "completes", "ends", "done"];
pub(super) const TASK_CONTROLS: [&str; 4] = ["stop", "pause", "cancel", "resume"];
/// `go f(x)` of a user function and a read of its task variable, until resolve_tasks knows whether f runs on a thread
pub(crate) const TASK_GO: &str = "task·go";
pub(super) const TASK_VALUE: &str = "task·value";
/// A read of a job list (`jobs.add(go f(i))`, P47): `task·list(jobs, f…)` is every result, `task·element(jobs#i, f…)` one;
/// until resolve_tasks knows whether the started functions run on threads
pub(super) const TASK_LIST: &str = "task·list";
/// The reads `await` names: `await job`, `await all jobs`; the others (`job + 1`) wait unsaid, which warns
pub(super) const AWAITED_MARKERS: [(&str, &str); 3] =
	[(TASK_VALUE, "task·awaited"), (TASK_LIST, "task·awaited·list"), (TASK_ELEMENT, "task·awaited·element")];
pub(super) const IMPLICIT_WAIT_TOPIC: &str = "implicit-await";
pub(super) const TASK_ELEMENT: &str = "task·element";
/// `await any [a, b]`: the result of the first task to finish (Promise.race)
pub(super) const ANY_WORD: &str = "any";
/// `race·0`: which task of an `await any` finished first (1-based, 0 while none)
pub(super) const RACE_PREFIX: &str = "race·";
/// how often an `await any` looks at its tasks
pub(super) const RACE_POLL_MILLISECONDS: i64 = 1;
pub(super) const SLEEP_WORD: &str = "sleep";
/// `await job within 100 ms or 0` (P154)
pub(super) const WITHIN_WORD: &str = "within";
pub(super) const WITHIN_TOPIC: &str = "await-within";
pub(super) const FIRST_WORD: &str = "first";
pub(super) const FIRST_TOPIC: &str = "await-first";
/// `started·0`: when an `await … within` began waiting
pub(super) const STARTED_PREFIX: &str = "started·";
/// task_status of a task still running (tasks.rs: finished 1, failed 2, stopped 3, paused 4)
pub(super) const TASK_RUNNING: i64 = 0;
/// the event an `await … within` without `or` raises at its deadline: try catches it
pub(super) const TIMEOUT_EVENT: &str = "timeout";
/// The parameter of the await map over a job list (resolve_tasks)
pub(super) const AWAITED_JOB: &str = "awaited_job";
/// `stop job`, `job.pause()`: until resolve_tasks knows whether the task runs on a thread
pub(super) const TASK_CONTROL_MARK: &str = "task·control";
/// `once job finishes: body`, `on job.stop: body` of a task that may run on a thread: `task·on(job, f, event, body)`
pub(super) const TASK_ON: &str = "task·on";
pub(super) const FINISH_EVENT: &str = "finishes";
/// `handled·0`: whether the first task handler ran
pub(super) const HANDLED_PREFIX: &str = "handled·";

/// `job = go f(x)` starts a task, `await job` waits for its value (wiki/async.md). A module runs on one thread, so a
/// task runs to its end where it starts: `go` gives the value of its call, and `await` of a finished task (or of any
/// value, which a task auto-casts to) is that value. Its signals follow: `once job finishes: …` (or `once the download
/// finishes:` for `go download(url)`) runs at once, as the task is done; a handler of a pause or stop never runs and
/// `stop job` has nothing left to stop: both warn. A program defining its own `go` or `await` keeps them.
pub fn lower_tasks(node: Node) -> Node {
	let node = awaited_starts(node, &Counter::default());
	let mut defined = std::collections::HashSet::new();
	let mut functions = std::collections::HashSet::new();
	let mut tasks = std::collections::HashSet::new();
	let mut started = std::collections::HashMap::new();
	node.visit(&mut |part| {
		if let Node::Key(head, Op::Define | Op::Assign, _) = part {
			let name = match head.drop_meta() {
				Node::List(items, Bracket::Round, _) => items.first().map(|name| name.drop_meta().name()),
				Node::Symbol(name) => Some(name.clone()),
				_ => None,
			};
			if let Some((variable, items)) = task_start(part) {
				tasks.insert(variable.clone());
				if let Some((function, _)) = started_call(&items[1..]) {
					started.insert(variable.clone(), function);
				}
			}
			if matches!(head.drop_meta(), Node::List(..)) {
				functions.extend(name.clone());
			}
			defined.extend(name);
		}
	});
	let words: Vec<&str> = TASK_WORDS.into_iter().filter(|word| !defined.contains(*word)).collect();
	if words.is_empty() {
		return node;
	}
	// `def hi: {…}`, `fun f(x) {…}` too
	let context = crate::analyzer::function_definitions(&node);
	functions.extend(context.user_functions.into_keys());
	// the functions a `go` starts: `once the download finishes` names the task by its function
	node.visit(&mut |part| {
		if let Node::List(items, _, _) = part {
			if let [word, started, ..] = items.as_slice() {
				if matches!(word.drop_meta(), Node::Symbol(name) if name == TASK_WORDS[0]) {
					let function = match started.drop_meta() {
						Node::List(call, _, _) => call.first().map(|name| name.drop_meta().name()),
						other => Some(other.name()),
					};
					tasks.extend(function);
				}
			}
		}
	});
	started.retain(|_, function| functions.contains(function));
	let job_lists = job_lists(&node, &functions);
	Tasks { words, tasks, started, functions, job_lists }.lower(node)
}

/// `job = go f(x)`: the task variable and the words of its start
pub(super) fn task_start(node: &Node) -> Option<(&String, &Vec<Node>)> {
	let Node::Key(head, Op::Define | Op::Assign, value) = node else {
		return None;
	};
	match (head.drop_meta(), value.drop_meta()) {
		(Node::Symbol(variable), Node::List(items, _, _)) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == TASK_WORDS[0]) => {
			Some((variable, items))
		}
		_ => None,
	}
}

/// The words of a statement the parser paired, flat: `(await any) [a, b]`, `await (all [go f(1), …])`
pub(super) fn spoken_words(items: &[Node]) -> Vec<Node> {
	let pair = |node: &Node| match node.drop_meta() {
		Node::List(inner, Bracket::None, _) if inner.len() == 2 => Some(inner.clone()),
		_ => None,
	};
	match items {
		[first, rest @ ..] if pair(first).is_some() => [pair(first).expect("guarded"), rest.to_vec()].concat(),
		[head, second] if head.name() == TASK_WORDS[1] && pair(second).is_some() => {
			[vec![head.clone()], pair(second).expect("guarded")].concat()
		}
		_ => items.to_vec(),
	}
}

/// `await go f(x)`: the task gets a name, `(go·job·1 = go f(x); await go·job·1)`, so it is awaited like any other
pub(super) fn awaited_starts(node: Node, counter: &Counter) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			match spoken_words(&items).as_slice() {
				[await_word, go_word, started @ ..]
					if !started.is_empty() && await_word.name() == TASK_WORDS[1] && go_word.name() == TASK_WORDS[0] =>
				{
					let job = fresh_job(counter);
					let start =
						Node::List([vec![go_word.clone()], started.to_vec()].concat(), Bracket::None, Separator::Space);
					let assignment = key(job.clone(), Op::Assign, start);
					let awaited = Node::List(vec![await_word.clone(), job], Bracket::None, Separator::Space);
					Node::List(vec![assignment, awaited], Bracket::Round, Separator::Semicolon)
				}
				// `await all [go f(1), go f(2)]` (Promise.all, asyncio.gather): every task starts, then each is awaited;
				// `await any [go f(1), go f(2)]` (Promise.race): every task starts, then the first to finish is awaited
				// `await first [go a(), go b()]`: the first task's result, whichever ends first (P153, with a note naming the race)
				[await_word, all_word, list]
					if await_word.name() == TASK_WORDS[1]
						&& [ALL_WORD, ANY_WORD, FIRST_WORD].contains(&all_word.name().as_str())
						&& matches!(list.drop_meta(), Node::List(listed, Bracket::Square, _) if listed.iter().any(is_start)) =>
				{
					let racing = all_word.name() == ANY_WORD;
					let first = all_word.name() == FIRST_WORD;
					if first {
						crate::normalize::set_position_of(list);
						crate::errors::diagnostic::educate_once(
							FIRST_TOPIC,
							"await first […]",
							"await any […]",
							"await first is the first task's result; for the first to finish: await any […]",
						);
					}
					let Node::List(listed, list_bracket, list_separator) = list.drop_meta().clone() else {
						unreachable!("guarded")
					};
					let mut starts = vec![];
					let results: Vec<Node> = listed
						.into_iter()
						.map(|item| match is_start(&item) {
							true => {
								let job = fresh_job(counter);
								starts.push(key(job.clone(), Op::Assign, item));
								if racing {
									job
								} else {
									Node::List(vec![await_word.clone(), job], Bracket::None, Separator::Space)
								}
							}
							false => item,
						})
						.collect();
					if first {
						let first_result = results.into_iter().next().expect("a started task");
						return Node::List([starts, vec![first_result]].concat(), Bracket::Round, Separator::Semicolon);
					}
					let results = Node::List(results, list_bracket, list_separator);
					let value = if racing {
						Node::List(vec![await_word.clone(), all_word.clone(), results], Bracket::None, Separator::Space)
					} else {
						results
					};
					Node::List([starts, vec![value]].concat(), Bracket::Round, Separator::Semicolon)
				}
				_ => Node::List(
					items.into_iter().map(|item| awaited_starts(item, counter)).collect(),
					bracket,
					separator,
				),
			}
		}
		other => children_rewritten(other, |child| awaited_starts(child, counter)),
	}
}

fn fresh_job(counter: &Counter) -> Node {
	Node::Symbol(format!("go·job·{}", counter.next_number()))
}

/// `go f(x)`, `go { … }`: a task started where it stands
pub(super) fn is_start(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(words, Bracket::None, Separator::Space) if words.len() > 1 && words[0].name() == TASK_WORDS[0])
}

/// The list variables that collect started tasks, `jobs.add(go f(i))` (P47) or `jobs = [go f(3), go f(1)]`, with the
/// functions they start
pub(super) fn job_lists(
	node: &Node,
	functions: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, Vec<String>> {
	let mut lists: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
	node.visit(&mut |part| {
		let (list, started_functions) = match part {
			Node::Key(receiver, Op::Dot, method) => {
				(receiver, added_start(method).map(|(function, _)| function).into_iter().collect())
			}
			Node::Key(target, Op::Assign | Op::Define, value) => (target, listed_starts(value)),
			_ => return,
		};
		let Node::Symbol(list) = list.drop_meta() else { return };
		for function in started_functions.into_iter().filter(|function| functions.contains(function)) {
			let started = lists.entry(list.clone()).or_default();
			if !started.contains(&function) {
				started.push(function);
			}
		}
	});
	lists
}

/// `[go f(3), go g(1)]`: the functions its items start
pub(super) fn listed_starts(value: &Node) -> Vec<String> {
	let Node::List(items, Bracket::Square, _) = value.drop_meta() else {
		return vec![];
	};
	items
		.iter()
		.filter(|item| is_start(item))
		.filter_map(|item| match item.drop_meta() {
			Node::List(words, _, _) => started_call(&words[1..]).map(|(function, _)| function),
			_ => None,
		})
		.collect()
}

/// `add(go f(x))`, `add go f(x)`: the `go …` items after `add`
pub(super) fn added_task(method: &Node) -> Option<Vec<Node>> {
	let Node::List(items, _, _) = method.drop_meta() else {
		return None;
	};
	let [add, rest @ ..] = items.as_slice() else {
		return None;
	};
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

pub(super) fn added_start(method: &Node) -> Option<(String, Vec<Node>)> {
	started_call(&added_task(method)?[1..])
}

/// `f(a, b)` or `f a b` after `go`: the function and its arguments
pub(super) fn started_call(rest: &[Node]) -> Option<(String, Vec<Node>)> {
	match rest {
		[call] => match call.drop_meta() {
			Node::List(items, Bracket::Round, _)
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) =>
			{
				Some((items[0].name(), items[1..].to_vec()))
			}
			Node::List(items, Bracket::None, Separator::Space) => started_call(items),
			_ => None,
		},
		[function, arguments @ ..] if matches!(function.drop_meta(), Node::Symbol(_)) => {
			Some((function.name(), arguments.to_vec()))
		}
		_ => None,
	}
}

/// A read of a task that `await` names: no warning that it waits
pub(super) fn awaited(node: Node) -> Node {
	match node {
		Node::List(mut items, bracket, separator) => {
			let head = word(items.first().unwrap_or(&Node::Empty));
			if let Some((_, named)) = AWAITED_MARKERS.iter().find(|(read, _)| *read == head) {
				items[0] = symbol(named);
			}
			Node::List(items, bracket, separator)
		}
		meta @ Node::Meta { .. } => meta.under_meta(awaited),
		other => other,
	}
}

/// The read as `await` would name it, and whether it was named so: `task·awaited` is `task·value` said
pub(super) fn read_marker(head: &str) -> (&str, bool) {
	AWAITED_MARKERS.iter().find(|(_, named)| *named == head).map_or((head, false), |(read, _)| (*read, true))
}

#[derive(Clone)]
pub(super) struct Tasks<'a> {
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
	pub(super) fn is_task(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Symbol(name) => self.tasks.contains(name),
			Node::List(items, _, _) => read_job(items).is_some_and(|job| self.is_task(job)),
			_ => false,
		}
	}

	/// `stop job`: a control of a task that may run on a thread, else the warning that a finished task has nothing to stop
	pub(super) fn control(&self, subject: &Node, control: &str) -> Node {
		let variable = task_variable(subject).name();
		match self.started.get(&variable) {
			Some(function) => {
				call(TASK_CONTROL_MARK, vec![Node::Symbol(variable), Node::Symbol(function.clone()), symbol(control)])
			}
			None => nothing_to_stop(&Node::Symbol(variable), control),
		}
	}

	/// A read of a task variable: its value once the task is done (`await job`, `job + 1`)
	pub(super) fn value_of(&self, variable: &str) -> Option<Node> {
		let function = self.started.get(variable)?;
		Some(call(TASK_VALUE, vec![symbol(variable), Node::Symbol(function.clone())]))
	}

	/// `count(jobs)`, `count jobs`, `#jobs`, `jobs.size` (count, length) of a job list: a count needs no result (P47)
	pub(super) fn counted_job_list(&self, node: &Node) -> bool {
		let is_job_list =
			|list: &Node| matches!(list.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name));
		match node.drop_meta() {
			Node::List(items, _, _) => {
				matches!(items.as_slice(), [counter, list] if crate::analyzer::is_counting_word(&word(counter)) && is_job_list(list))
			}
			Node::Key(empty, Op::Hash, list) => matches!(empty.drop_meta(), Node::Empty) && is_job_list(list),
			Node::Key(list, Op::Dot, counter) => is_job_list(list) && crate::analyzer::is_counting_word(&word(counter)),
			_ => false,
		}
	}

	/// A read of a job list or of one of its jobs: its results once the jobs are done
	pub(super) fn job_list_read(&self, list: &str, read: Node, marker_word: &str) -> Node {
		let functions = self.job_lists[list].iter().map(|function| Node::Symbol(function.clone()));
		call(marker_word, [vec![read], functions.collect()].concat())
	}

	/// Inside a function that binds a task variable's name itself (a parameter, a local, a loop variable: a joined
	/// module's `for c in …` under a main-level `c = go f()`) other than by starting a task, that name is its own (card
	/// task-name)
	pub(super) fn lower(&self, node: Node) -> Node {
		let mut own_tasks = vec![];
		node.visit(&mut |part| own_tasks.extend(task_start(part).map(|(variable, _)| variable.clone())));
		let shadowed: Vec<String> = crate::lowering::getters::shadowed_names(&node)
			.into_iter()
			.filter(|name| !own_tasks.contains(name))
			.collect();
		if !shadowed.iter().any(|name| self.started.contains_key(name) || self.job_lists.contains_key(name)) {
			return self.lower_unshadowed(node);
		}
		let mut narrowed = self.clone();
		for name in &shadowed {
			narrowed.started.remove(name);
			narrowed.job_lists.remove(name);
			narrowed.tasks.remove(name);
		}
		narrowed.lower_unshadowed(node)
	}

	pub(super) fn lower_unshadowed(&self, node: Node) -> Node {
		match node {
			Node::Symbol(name) if self.started.contains_key(&name) => self.value_of(&name).expect("a started task"),
			Node::Symbol(name) if self.job_lists.contains_key(&name) => {
				self.job_list_read(&name, Node::Symbol(name.clone()), TASK_LIST)
			}
			// `count(jobs)`, `#jobs`, `jobs.size`: how many jobs, without waiting for any
			node if self.counted_job_list(&node) => node,
			// `jobs.add(go f(i))`: the list keeps the task, unawaited
			Node::Key(receiver, Op::Dot, method)
				if matches!(receiver.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name))
					&& added_task(&method).is_some() =>
			{
				let started =
					self.task_statement(&added_task(&method).expect("guarded")).expect("a go of a user function");
				Node::Key(receiver, Op::Dot, Box::new(call(ADD_WORD, vec![started])))
			}
			// `jobs#2`: the result of that job
			Node::Key(list, Op::Hash, index) if matches!(list.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name)) =>
			{
				let name = word(&list);
				self.job_list_read(&name, Node::Key(list, Op::Hash, Box::new(self.lower(*index))), TASK_ELEMENT)
			}
			// `jobs = []`, `job = go f(x)`: the variable itself holds the jobs, the task
			Node::Key(target, op @ (Op::Assign | Op::Define), value) if matches!(target.drop_meta(), Node::Symbol(name) if self.job_lists.contains_key(name) || self.started.contains_key(name)) => {
				Node::Key(target, op, Box::new(self.lower(*value)))
			}
			Node::List(items, bracket, separator) => {
				// a signal handler first, before its `download.stop` reads as a control
				if let Some(handled) = self.signal_handler(&items) {
					// a task·on marker holds the task variable itself, its body is lowered already
					let is_task_handler = matches!(&handled, Node::List(parts, _, _) if word(parts.first().unwrap_or(&Node::Empty)) == TASK_ON);
					return if is_task_handler { handled } else { self.lower(handled) };
				}
				// `await any [a, b]` and `await job within 100 ms` read the tasks themselves, before a read of one waits
				if let Some(race) =
					self.raced(&items).or_else(|| self.within(&items, None)).or_else(|| self.within_or(&items))
				{
					return race;
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
			Node::Key(subject, Op::Dot, word)
				if self.is_task(&subject) && TASK_CONTROLS.contains(&control_word(&word).as_str()) =>
			{
				self.control(&subject, &control_word(&word))
			}
			// `x = await job within 100 ms or 0`
			Node::Key(left, Op::Or, right) if matches!(left.drop_meta(), Node::List(items, _, _) if self.within(items, Some(right.as_ref().clone())).is_some()) =>
			{
				let Node::List(items, _, _) = left.drop_meta() else { unreachable!("guarded") };
				self.within(items, Some(*right)).expect("guarded")
			}
			other => children_rewritten(other, |child| self.lower(child)),
		}
	}

	/// `x = await job within 100 ms or 5` arrives as go_blocks' `try·else((await job) (within 100 ms), 5)`
	pub(super) fn within_or(&self, items: &[Node]) -> Option<Node> {
		let [marker_word, awaited, fallback] = items else {
			return None;
		};
		if word(marker_word) != crate::warp_parser::TRY_MARKER {
			return None;
		}
		self.within(&phrase_words(std::slice::from_ref(awaited)), Some(fallback.clone()))
	}

	/// `await job within 100 ms or 0` (P154, asyncio.wait_for, Kotlin withTimeout): the job's result if it ends in
	/// time, else the job is stopped and the value is the `or` one (an error `timeout` without it). A got-it note says so
	/// where it is written: `(started·0 = clock(); while job runs and clock() - started·0 < 100 {sleep(1)}; if job
	/// runs then {stop job; 0} else await job)`
	pub(super) fn within(&self, items: &[Node], fallback: Option<Node>) -> Option<Node> {
		use crate::variable_signals::assign;
		// `(await job) within …`: the await arrives grouped
		let items: Vec<Node> = match items.split_first() {
			Some((first, rest)) => match first.drop_meta() {
				Node::List(inner, Bracket::None, _)
					if inner.first().is_some_and(|head| word(head) == TASK_WORDS[1]) =>
				{
					[inner.clone(), rest.to_vec()].concat()
				}
				_ => items.to_vec(),
			},
			None => return None,
		};
		let [head, job, within, deadline @ ..] = items.as_slice() else {
			return None;
		};
		if word(head) != TASK_WORDS[1] || word(within) != WITHIN_WORD || !self.is_task(job) || deadline.is_empty() {
			return None;
		}
		// `await job within 100 ms or 0` as one statement: the `or` holds the duration and the fallback
		let (deadline, fallback) = match (deadline, fallback) {
			([single], None) if matches!(single.drop_meta(), Node::Key(_, Op::Or, _)) => {
				let Node::Key(duration, _, value) = single.drop_meta() else { unreachable!("guarded") };
				(duration.as_ref().clone(), Some(value.as_ref().clone()))
			}
			([single], fallback) => (single.clone(), fallback),
			(several, fallback) => (Node::List(several.to_vec(), Bracket::None, Separator::Space), fallback),
		};
		let milliseconds = crate::numbers::units::milliseconds(&deadline)?;
		let job_name = task_variable(job);
		let started = format!("{STARTED_PREFIX}{}", fresh_number());
		let status = call(crate::host::TASK_STATUS, vec![job_name.clone()]);
		let runs = key(status, Op::Eq, crate::node::int(TASK_RUNNING));
		let elapsed = key(call(crate::host::CLOCK, vec![]), Op::Sub, Node::Symbol(started.clone()));
		let in_time = key(elapsed, Op::Lt, crate::node::int(milliseconds));
		let waiting = key(
			key(Node::Empty, Op::While, key(runs.clone(), Op::And, in_time)),
			Op::Do,
			block(vec![call(SLEEP_WORD, vec![crate::node::int(RACE_POLL_MILLISECONDS)])]),
		);
		let timed_out = match &fallback {
			Some(value) => value.clone(),
			None => Node::List(vec![symbol(RAISE_WORD), symbol(TIMEOUT_EVENT)], Bracket::None, Separator::Space),
		};
		let written = format!("await {} within {milliseconds} ms", job_name.serialize().trim());
		let outcome = fallback.as_ref().map_or_else(
			|| "it fails with timeout".to_string(),
			|value| format!("the result is {}", value.serialize().trim()),
		);
		crate::errors::diagnostic::educate_once(
			WITHIN_TOPIC,
			&written,
			&written,
			&format!("on timeout the job is stopped and {outcome}"),
		);
		let awaited_job = self.value_of(&word(&job_name))?;
		let result = if_then_else(
			runs,
			// a sequence, not a block: a Node result would take `{stop; 0}` as data
			Node::List(vec![self.control(job, TASK_CONTROLS[0]), timed_out], Bracket::None, Separator::Semicolon),
			awaited_job,
		);
		Some(Node::List(
			vec![assign(&started, call(crate::host::CLOCK, vec![])), waiting, result],
			Bracket::Round,
			Separator::Semicolon,
		))
	}

	pub(super) fn task_statement(&self, items: &[Node]) -> Option<Node> {
		match items {
			// `go f(x)` of a user function: a task, on a thread when resolve_tasks finds f runs there
			[head, rest @ ..]
				if word(head) == TASK_WORDS[0]
					&& self.words.contains(&TASK_WORDS[0])
					&& started_call(rest).is_some_and(|(f, _)| self.functions.contains(&f)) =>
			{
				let (function, arguments) = started_call(rest).expect("guarded");
				Some(call(TASK_GO, [vec![Node::Symbol(function)], arguments].concat()))
			}
			[head, rest @ ..] if !rest.is_empty() && self.words.contains(&word(head).as_str()) => {
				let one = |nodes: &[Node]| match nodes {
					[single] => single.clone(),
					_ => Node::List(nodes.to_vec(), Bracket::None, Separator::Space),
				};
				if word(head) != TASK_WORDS[1] {
					return Some(one(rest));
				}
				// `await all jobs` (also read as `await (all jobs)`): every result, a job list read awaits them all
				let spoken = match rest {
					[phrase] => match phrase.drop_meta() {
						Node::List(inner, Bracket::None, _) if inner.len() == 2 => inner.as_slice(),
						_ => rest,
					},
					_ => rest,
				};
				Some(awaited(one(match spoken {
					[all, jobs @ ..] if word(all) == ALL_WORD && !jobs.is_empty() => jobs,
					_ => rest,
				})))
			}
			// `stop job`
			[control, subject] if TASK_CONTROLS.contains(&word(control).as_str()) && self.is_task(subject) => {
				Some(self.control(subject, &word(control)))
			}
			_ => None,
		}
	}

	/// `await any [a, b]` (also read as `(await any) [a, b]` or `await (any [a, b])`): the first of the tasks to finish
	pub(super) fn raced(&self, items: &[Node]) -> Option<Node> {
		match spoken_words(items).as_slice() {
			[head, any, list] if word(head) == TASK_WORDS[1] && word(any) == ANY_WORD => self.race(list),
			[head, phrase] if word(head) == TASK_WORDS[1] => match phrase.drop_meta() {
				// `any [a, b]` read as an index of any
				Node::Key(any, _, list) if word(any) == ANY_WORD => self.race(list),
				_ => None,
			},
			_ => None,
		}
	}

	/// `(race·0 = 0; while race·0 == 0 { if task_status(a) is finished or failed {race·0 = 1}; …; if race·0 == 0
	/// {sleep(1)} }; if race·0 == 1 then await a else …)`: the tasks keep running, the first one's result (or failure)
	pub(super) fn race(&self, list: &Node) -> Option<Node> {
		use crate::variable_signals::assign;
		let Node::List(listed, Bracket::Square, _) = list.drop_meta() else {
			return None;
		};
		if listed.is_empty() || !listed.iter().all(|item| self.is_task(item)) {
			return None;
		}
		let winner = format!("{RACE_PREFIX}{}", fresh_number());
		let winner_is = |index: usize| key(Node::Symbol(winner.clone()), Op::Eq, crate::node::int(index as i64));
		// `go 1+1` is computed right away, no task is started: it has ended (card await-hang)
		let ended = |job: &Node| {
			if !self.started.contains_key(&word(&task_variable(job))) {
				return Node::True;
			}
			let status = call(crate::host::TASK_STATUS, vec![task_variable(job)]);
			let is = |code: i64| key(status.clone(), Op::Eq, crate::node::int(code));
			key(is(crate::host::TASK_FINISHED), Op::Or, is(crate::host::TASK_FAILED))
		};
		let mut checks: Vec<Node> = listed
			.iter()
			.enumerate()
			.map(|(index, job)| {
				if_then(
					key(winner_is(0), Op::And, ended(job)),
					block(vec![assign(&winner, crate::node::int(index as i64 + 1))]),
				)
			})
			.collect();
		checks
			.push(if_then(winner_is(0), block(vec![call(SLEEP_WORD, vec![crate::node::int(RACE_POLL_MILLISECONDS)])])));
		let waiting = key(key(Node::Empty, Op::While, winner_is(0)), Op::Do, block(checks));
		// the race is the `await`: reading the winner waits as said, no warning (card await-warns)
		let awaited_job =
			|job: &Node| awaited(self.value_of(&word(&task_variable(job))).unwrap_or_else(|| job.clone()));
		let (last, first) = listed.split_last().expect("not empty");
		let result = first.iter().enumerate().rev().fold(awaited_job(last), |otherwise, (index, job)| {
			if_then_else(winner_is(index + 1), awaited_job(job), otherwise)
		});
		Some(Node::List(
			vec![assign(&winner, crate::node::int(0)), waiting, result],
			Bracket::Round,
			Separator::Semicolon,
		))
	}

	pub(super) fn signal_handler(&self, items: &[Node]) -> Option<Node> {
		match items {
			// `once job finishes: body`, `once the download finishes: body`
			[once, subject @ .., handler] if word(once) == crate::words::ONCE_WORD => {
				let subject = subject.iter().find(|item| word(item) != "the")?;
				let (event, body) = handler_parts(handler)?;
				self.is_task(subject).then(|| self.handler(subject, &word(&event), &body))
			}
			// `on download.stop : body`
			[on, handler] if word(on) == ON_WORD => {
				let (signal, body) = handler_parts(handler)?;
				let Node::Key(subject, Op::Dot, event) = signal.drop_meta() else {
					return None;
				};
				self.is_task(subject).then(|| self.handler(subject, &control_word(event), &body))
			}
			_ => None,
		}
	}

	/// A finished task's handler runs once the task is done (it waits for a task on a thread); one of a pause or stop
	/// never runs
	pub(super) fn handler(&self, subject: &Node, event: &str, body: &Node) -> Node {
		if let Some(function) = self.started.get(&subject.name()) {
			let event = if FINISH_EVENTS.contains(&event) { FINISH_EVENT } else { event };
			return call(
				TASK_ON,
				vec![
					Node::Symbol(subject.name()),
					Node::Symbol(function.clone()),
					symbol(event),
					self.lower(body.clone()),
				],
			);
		}
		if FINISH_EVENTS.contains(&event) {
			return match self.value_of(&subject.name()) {
				Some(value) => Node::List(vec![value, body.clone()], Bracket::Curly, Separator::Semicolon),
				None => body.clone(),
			};
		}
		never_happens(
			subject,
			&format!(
				"`{event}` of {} never happens: a task finishes where it starts, its handler never runs",
				subject.name()
			),
		)
	}
}

/// How a started function runs: on a thread, or where it starts
#[derive(Clone, PartialEq)]
pub(super) enum TaskPath {
	/// with the kinds of the parameters (a Float parameter gets its argument converted, as a call does) and of the result
	Values(Vec<crate::type_kinds::Kind>, crate::type_kinds::Kind),
	Inline,
}

/// The kinds a value of a task may have to cross between instances (tasks::TaskValue); a function value crosses as its
/// target's name and captured values (closure_rebuild)
pub(super) const CROSSING_KINDS: [crate::type_kinds::Kind; 8] = {
	use crate::type_kinds::Kind::*;
	[Int, Float, Text, Codepoint, Symbol, Empty, List, Function]
};

/// `task·go(f, args)`, `task·value(job, f)` and `task·control(job, f, word)` (lower_tasks). A function runs in a fresh
/// instance, natively on the fixed worker pool (tasks.rs), in the browser through host.js: one of numbers, texts, characters
/// and lists through task_spawn_values / task_await_value. Any other runs where it starts, as a plain call, and its task variable is its
/// value
pub fn resolve_tasks(node: Node) -> Node {
	use crate::type_kinds::Kind;
	let mut has_task = false;
	node.visit(&mut |part| has_task |= matches!(part, Node::List(items, _, _) if matches!(read_marker(&word(items.first().unwrap_or(&Node::Empty))).0, TASK_GO | TASK_VALUE | TASK_CONTROL_MARK | TASK_LIST | TASK_ELEMENT | crate::warp_parser::TRY_MARKER)));
	if !has_task {
		return node;
	}
	let mut context = crate::compiler::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, &node);
	crate::analyzer::note_bool_functions(context.user_functions.values());
	let path = |function: &str| {
		let Some(definition) = context.user_functions.get(function) else {
			return TaskPath::Inline;
		};
		let parameters: Vec<Kind> = definition.params.iter().map(crate::analyzer::param_kind).collect();
		if !definition.tuple_kinds.is_empty()
			|| !CROSSING_KINDS.contains(&definition.return_kind)
			|| !parameters.iter().all(|kind| CROSSING_KINDS.contains(kind))
		{
			return TaskPath::Inline;
		}
		TaskPath::Values(parameters, definition.return_kind)
	};
	let node = TaskHandlers { path: &path, count: Counter::default() }.lower(node, &[]);
	let wrapped = std::cell::RefCell::new(std::collections::BTreeMap::new());
	// the guardable functions, with the type word their Node result converts back to (it comes back from the host as a Node)
	let guardable = |function: &str| {
		context.user_functions.get(function).filter(|definition| definition.tuple_kinds.is_empty()).map(|definition| {
			match definition.return_kind {
				Kind::Int => Some("int"),
				Kind::Float => Some("float"),
				_ => None, // a Node (a text, a list, an Error) is used as it comes
			}
		})
	};
	let node = guarded_calls(node, &guardable, &wrapped);
	let node = resolved(node, &path, &wrapped);
	let node = forwarded_raises(node, &wrapped);
	with_node_wrappers(node, &wrapped.into_inner())
}

/// A handler of a task on a thread: after every later statement of its block (loop bodies too) it runs once the task's
/// status shows its event; a final check at the block's end waits for the task (a stopped or finished one)
#[derive(Clone)]
pub(super) struct TaskHandler {
	job: Node,
	status: i64,
	body: Node,
	flag: String,
}

impl TaskHandler {
	pub(super) fn check(&self) -> Node {
		use crate::variable_signals::assign;
		let not_handled = key(Node::Empty, Op::Not, Node::Symbol(self.flag.clone()));
		let status = key(
			call(crate::host::TASK_STATUS, vec![self.job.clone()]),
			Op::Eq,
			Node::Number(crate::extensions::numbers::Number::Int(self.status)),
		);
		let condition = key(not_handled, Op::And, status);
		if_then(condition, block(vec![assign(&self.flag, Node::True), self.body.clone()]))
	}

	/// At the end of the block: a finish or stop handler waits for the task, a pause handler does not
	pub(super) fn final_check(&self) -> Option<Node> {
		(self.status != crate::host::TASK_PAUSED)
			.then(|| block(vec![call(crate::host::TASK_JOIN, vec![self.job.clone()]), self.check()]))
	}
}

pub(super) struct TaskHandlers<'a> {
	path: &'a dyn Fn(&str) -> TaskPath,
	count: Counter,
}

impl TaskHandlers<'_> {
	pub(super) fn lower(&self, node: Node, active: &[TaskHandler]) -> Node {
		match node {
			Node::List(items, bracket, separator)
				if crate::variable_signals::is_statement_list(&bracket, &separator) =>
			{
				Node::List(self.statements(items, active), bracket, separator)
			}
			other => other.map_children(|child| self.lower(child, active)),
		}
	}

	/// The checks follow each statement; the last statement keeps the block's value, so the checks of the block (and
	/// the final ones) come before it
	pub(super) fn statements(&self, items: Vec<Node>, outer: &[TaskHandler]) -> Vec<Node> {
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
			let (job, function, event, body) =
				(parts[1].clone(), word(&parts[2]), word(&parts[3]), self.lower(parts[4].clone(), &[]));
			if (self.path)(&function) == TaskPath::Inline {
				// the task finished where it started: a finish handler runs now, one of a stop or pause never
				out.push(match event.as_str() {
					FINISH_EVENT => body,
					_ => never_happens(
						&job,
						&format!(
							"`{event}` of {} never happens: a task finishes where it starts, its handler never runs",
							job.name()
						),
					),
				});
				continue;
			}
			let status = match event.as_str() {
				FINISH_EVENT => crate::host::TASK_FINISHED,
				"pause" | "pauses" | "paused" => crate::host::TASK_PAUSED,
				_ => crate::host::TASK_STOPPED, // stop, cancel
			};
			let flag = format!("{HANDLED_PREFIX}{}", self.count.next_number());
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
pub(super) const WRAPPER_RESULT: &str = "r·node";
/// The one parameter of the wrapper: the list of the arguments, as task_spawn_values carries it (the host passes one
/// Node and needs no parameter types: a browser cannot read them)
pub(super) const WRAPPER_ARGUMENTS: &str = "arguments·node";

/// The program with the wrappers of the started functions (name → parameter count) defined first
pub(crate) fn with_node_wrappers(node: Node, wrapped: &std::collections::BTreeMap<String, usize>) -> Node {
	// a page event's handler has its wrapper already (event_signals.rs), a task may want it too
	let mut defined = std::collections::HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(head, Op::Define, _) = part {
			if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
				defined.extend(items.first().map(word));
			}
		}
	});
	let wrapped: Vec<(&String, &usize)> =
		wrapped.iter().filter(|(function, _)| !defined.contains(&format!("{function}{NODE_WRAPPER_SUFFIX}"))).collect();
	if wrapped.is_empty() {
		return node;
	}
	let items: Vec<Node> = wrapped
		.into_iter()
		.map(|(function, count)| {
			let arguments = symbol(WRAPPER_ARGUMENTS);
			// declared a list: a function of no arguments leaves it unused, which would make it an Int (the host passes a Node)
			let declared = key(arguments.clone(), Op::Colon, symbol("list"));
			let head = call(&format!("{function}{NODE_WRAPPER_SUFFIX}"), vec![declared]);
			let argument = |index: usize| {
				key(
					arguments.clone(),
					Op::Hash,
					Node::Number(crate::extensions::numbers::Number::Int(index as i64 + 1)),
				)
			};
			let call = call(&function.clone(), (0..*count).map(argument).collect());
			let result = symbol(WRAPPER_RESULT);
			let body =
				Node::List(vec![key(result.clone(), Op::Assign, call), result], Bracket::Curly, Separator::Semicolon);
			key(head, Op::Define, body)
		})
		.collect();
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
pub(super) fn forwarded_raises(
	node: Node,
	wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>,
) -> Node {
	let handlers: std::collections::HashSet<String> =
		crate::event_signals::handled_signals(&node).into_iter().map(|(_, function)| function).collect();
	if handlers.is_empty() {
		return node;
	}
	pub(super) fn forward(
		node: Node,
		handlers: &std::collections::HashSet<String>,
		wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>,
	) -> Node {
		match node {
			Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|function| handlers.contains(&word(function)))) =>
			{
				let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
				let function = word(&items[0]);
				wrapped.borrow_mut().insert(function.clone(), items.len() - 1);
				let arguments = Node::List(items[1..].to_vec(), Bracket::Square, Separator::Colon);
				let send = call(
					crate::host::SIGNAL_SEND,
					vec![Node::Text(format!("{function}{NODE_WRAPPER_SUFFIX}")), arguments],
				);
				let template = crate::warp_parser::parse_generated(&format!(
					"if {}() {{ {FORWARD_PLACEHOLDER} }} else {{ {BODY_PLACEHOLDER} }}",
					crate::host::TASK_INSIDE
				));
				let body = crate::library_words::substitute(
					crate::library_words::substitute(template, FORWARD_PLACEHOLDER, &send),
					BODY_PLACEHOLDER,
					&body,
				);
				Node::Key(head, Op::Define, Box::new(body))
			}
			other => other.map_children(|child| forward(child, handlers, wrapped)),
		}
	}
	forward(node, &handlers, wrapped)
}

pub(super) const FORWARD_PLACEHOLDER: &str = "forwarded_raise_placeholder";
pub(super) const BODY_PLACEHOLDER: &str = "handler_body_placeholder";

/// `task·check(task_join(job), task_failure(job)); task_await_value(job)`: a failed task raises its error from wasm, which
/// `try` catches, before the result is read
pub(super) fn checked_await(job: &Node, crossing: Crossing) -> Node {
	use crate::host::{TASK_CHECK, TASK_FAILURE, TASK_JOIN};
	let check = call(TASK_CHECK, vec![call(TASK_JOIN, vec![job.clone()]), call(TASK_FAILURE, vec![job.clone()])]);
	let result = call(crate::host::TASK_AWAIT_VALUE, vec![job.clone()]);
	let result = match crossing {
		Crossing::AsIs => result,
		Crossing::Bool => key(result, Op::Ne, Node::Number(crate::extensions::numbers::Number::Int(0))),
		Crossing::Char => key(result, Op::As, symbol("char")),
	};
	Node::List(vec![check, result], Bracket::None, Separator::Semicolon)
}

/// How the result of a task comes back: as it is, or as the Int a bool or a character crosses as
#[derive(Clone, Copy)]
pub(super) enum Crossing {
	AsIs,
	/// 1/0: comparing it makes it a bool again (card bool-crossing)
	Bool,
	/// its code point: `as char` makes it the character again (card task-char)
	Char,
}

/// How the results of the started functions cross: a bool or a character when every one gives it
/// (analyzer::note_bool_functions)
pub(super) fn crossing(functions: &[Node], path: &dyn Fn(&str) -> TaskPath) -> Crossing {
	let every =
		|gives: &dyn Fn(&str) -> bool| !functions.is_empty() && functions.iter().all(|function| gives(&word(function)));
	if every(&crate::analyzer::is_bool_function) {
		Crossing::Bool
	} else if every(&|function| matches!(path(function), TaskPath::Values(_, crate::type_kinds::Kind::Codepoint))) {
		Crossing::Char
	} else {
		Crossing::AsIs
	}
}

/// `try f(a, b) else Y` of a user function f: the guarded call goes through the host, `guarded_call("f·node", [a, b])`,
/// whose stack overflow is an Error `try` catches (host.rs guarded_call); f's node wrapper is the one tasks use. Any other
/// guarded expression stays as it is
pub(super) fn guarded_calls(
	node: Node,
	guardable: &dyn Fn(&str) -> Option<Option<&'static str>>,
	wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>,
) -> Node {
	match node {
		Node::List(items, bracket, separator)
			if items.len() >= 3 && word(&items[0]) == crate::warp_parser::TRY_MARKER =>
		{
			let items: Vec<Node> = items.into_iter().map(|item| guarded_calls(item, guardable, wrapped)).collect();
			let guarded = match items[1].drop_meta() {
				Node::List(called, Bracket::Round, _) if matches!(called.first().map(Node::drop_meta), Some(Node::Symbol(function)) if guardable(function).is_some()) =>
				{
					let function = word(&called[0]);
					wrapped.borrow_mut().insert(function.clone(), called.len() - 1);
					let arguments = Node::List(called[1..].to_vec(), Bracket::Square, Separator::Colon);
					let guarded = call(
						crate::host::GUARDED_CALL,
						vec![Node::Text(format!("{function}{NODE_WRAPPER_SUFFIX}")), arguments],
					);
					// an Int or Float result is that again (a caught overflow's Error fails the conversion, which `try` catches)
					match guardable(&function).flatten() {
						Some(type_word) => key(guarded, Op::As, symbol(type_word)),
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
pub(super) fn awaited_jobs(list: &Node, crossing: Crossing) -> Node {
	let template = crate::warp_parser::parse_generated(&format!(
		"{TASK_LIST_PLACEHOLDER}.map({AWAITED_JOB} => {AWAITED_PLACEHOLDER})"
	));
	let awaited = checked_await(&symbol(AWAITED_JOB), crossing);
	let template = crate::library_words::substitute(template, TASK_LIST_PLACEHOLDER, list);
	crate::library_words::substitute(template, AWAITED_PLACEHOLDER, &awaited)
}

pub(super) const TASK_LIST_PLACEHOLDER: &str = "awaited_jobs_list";
pub(super) const AWAITED_PLACEHOLDER: &str = "awaited_job_result";

pub(super) fn resolved(
	node: Node,
	path: &dyn Fn(&str) -> TaskPath,
	wrapped: &std::cell::RefCell<std::collections::BTreeMap<String, usize>>,
) -> Node {
	use crate::host::{TASK_CONTROL, TASK_SPAWN_VALUES};
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
						TaskPath::Values(parameters, _) => {
							let float = |argument: Node| key(argument, Op::As, symbol("float"));
							// a character crosses as a one-character text, as a variable holds it
							let arguments = arguments
								.into_iter()
								.zip(parameters.iter().chain(std::iter::repeat(&crate::type_kinds::Kind::Empty)))
								.map(|(argument, kind)| match argument.drop_meta() {
									Node::Char(character) => Node::Text(character.to_string()),
									_ if *kind == crate::type_kinds::Kind::Float => float(argument),
									_ => argument,
								})
								.collect();
							// through the wrapper: whether f takes a list as an array is decided later (list_abi)
							wrapped.borrow_mut().insert(function.clone(), parameters.len());
							let started = format!("{function}{NODE_WRAPPER_SUFFIX}");
							call(
								TASK_SPAWN_VALUES,
								vec![Node::Text(started), Node::List(arguments, Bracket::Square, Separator::Colon)],
							)
						}
						TaskPath::Inline => parens(items[1..].to_vec()),
					}
				}
				TASK_VALUE => {
					let crossing = crossing(&items[2..3], path);
					match path(&word(&items[2])) {
						TaskPath::Values(..) => checked_await(&items[1], crossing),
						TaskPath::Inline => items[1].clone(),
					}
				}
				TASK_LIST | TASK_ELEMENT => {
					let threaded = items[2..].iter().all(|function| path(&word(function)) != TaskPath::Inline);
					let crossing = crossing(&items[2..], path);
					match (threaded, read == TASK_LIST) {
						(false, _) => items[1].clone(),
						(true, false) => checked_await(&items[1], crossing),
						(true, true) => awaited_jobs(&items[1], crossing),
					}
				}
				TASK_CONTROL_MARK => match path(&word(&items[2])) {
					TaskPath::Inline => nothing_to_stop(&items[1], &word(&items[3])),
					_ => call(
						TASK_CONTROL,
						vec![
							items[1].clone(),
							Node::Number(crate::extensions::numbers::Number::Int(control_operation(&word(&items[3])))),
						],
					),
				},
				_ => Node::List(items, bracket, separator),
			}
		}
		other => children_rewritten(other, |child| resolved(child, path, wrapped)),
	}
}

/// A task read as its value without `await` (`job + 1`, `starts = jobs`) on a thread: the program waits there, unsaid.
/// A got-it warning naming `await`; the error a strict program makes of it
pub(super) fn unsaid_wait(read: &str, items: &[Node], path: &dyn Fn(&str) -> TaskPath) -> Option<Node> {
	let functions = match read {
		TASK_VALUE => &items[2..3],
		TASK_LIST | TASK_ELEMENT => &items[2..],
		_ => return None,
	};
	if functions.iter().any(|function| path(&word(function)) == TaskPath::Inline) {
		return None; // it ran where it started: nothing to wait for
	}
	let written = crate::errors::diagnostic::written_text(&items[1]);
	let (all, verb) =
		if read == TASK_LIST { ("all ", "waits for every task in it") } else { ("", "waits for the task") };
	let question = crate::errors::diagnostic::Ask::new(
		IMPLICIT_WAIT_TOPIC,
		format!("{written} is used as its value: the program {verb} here; write `await {all}{written}` to say so"),
		vec![crate::errors::diagnostic::reading("wait here", &format!("await {all}{written}"))],
		crate::errors::diagnostic::Fallback::Warning,
	)
	.written(&written)
	.at_node(&items[1]);
	crate::errors::diagnostic::ask(&question).err()
}

/// `event: body`; `event: x = 7` parses as `(event: x) = 7`
pub(crate) fn handler_parts(handler: &Node) -> Option<(Node, Node)> {
	match handler.drop_meta() {
		Node::Key(event, Op::Colon, body) => Some((event.drop_meta().clone(), body.as_ref().clone())),
		Node::Key(head, op, value) => match head.drop_meta() {
			Node::Key(event, Op::Colon, target) => {
				Some((event.drop_meta().clone(), Node::Key(target.clone(), *op, value.clone())))
			}
			_ => None,
		},
		_ => None,
	}
}

/// A task marker before resolve_tasks: `task·go(f, args)` and the reads and controls naming the started functions
pub(crate) fn is_task_marker(head: &str) -> bool {
	[TASK_GO, TASK_VALUE, TASK_LIST, TASK_ELEMENT, TASK_CONTROL_MARK, TASK_ON].contains(&read_marker(head).0)
}

/// `job` of `job` or of its read `task·value(job, f)`
fn task_variable(node: &Node) -> Node {
	let node = node.drop_meta();
	match node {
		Node::List(items, _, _) => read_job(items).unwrap_or(node).clone(),
		other => other.clone(),
	}
}

/// `job` of the items of its read `task·value(job, f)`; any other list, the empty one too, is none
fn read_job(items: &[Node]) -> Option<&Node> {
	match items {
		[head, job, ..] if word(head) == TASK_VALUE => Some(job),
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
pub(super) fn control_word(node: &Node) -> String {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => items[0].drop_meta().name(),
		other => other.name(),
	}
}

pub(super) fn nothing_to_stop(subject: &Node, control: &str) -> Node {
	never_happens(
		subject,
		&format!("{control} {} has nothing to stop: a task finishes where it starts", subject.name()),
	)
}

/// The task_control operation of a control word
pub(super) fn control_operation(control: &str) -> i64 {
	match control {
		"pause" => crate::host::TASK_PAUSE,
		"resume" => crate::host::TASK_RESUME,
		_ => crate::host::TASK_STOP, // stop, cancel
	}
}

/// A warning about task control that cannot happen on one thread, and the statement's value ø (an error under
/// `use strict`)
pub(super) fn never_happens(located: &Node, message: &str) -> Node {
	match crate::errors::diagnostic::report(&[
		crate::errors::diagnostic::Diagnostic::at(located, message.to_string()).severe()
	]) {
		Ok(()) => Node::Empty,
		Err(error) => error,
	}
}
