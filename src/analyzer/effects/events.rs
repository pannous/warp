//! The effects of the events a program emits and listens to
use super::*;

/// The events a body emits itself and the calls it makes
#[derive(Default)]
struct EventUses {
	own: BTreeSet<String>,
	calls: Vec<String>,
}

impl EventUses {
	fn of(body: &Node, functions: &BTreeSet<String>, verbs: &[String]) -> Self {
		let mut uses = EventUses::default();
		uses.collect(body, functions, verbs);
		uses
	}

	fn collect(&mut self, node: &Node, functions: &BTreeSet<String>, verbs: &[String]) {
		if node.defined_name().is_some_and(|name| functions.contains(&name)) {
			return; // a definition has its own events
		}
		if let Some((event, data)) = crate::event_signals::emitted(node, verbs) {
			self.own.insert(event);
			return self.collect(&data, functions, verbs);
		}
		match node.drop_meta() {
			Node::Symbol(name) if functions.contains(name) => self.calls.push(name.clone()),
			Node::List(items, _, _) => items.iter().for_each(|item| self.collect(item, functions, verbs)),
			Node::Key(left, _, right) => {
				self.collect(left, functions, verbs);
				self.collect(right, functions, verbs);
			}
			_ => {}
		}
	}
}

/// Each function's events, own and with its callees' (main for the top level): an `emit ask` adds ask, a call inside
/// `on ask {…} in {…}` passes on its callee's events but ask (card effect-handlers step 2)
pub fn event_effects(program: &Node) -> BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> {
	let context = function_definitions(program);
	let functions: BTreeSet<String> = context.user_functions.keys().cloned().collect();
	let verbs = crate::event_signals::emit_verbs(program);
	let mut uses: BTreeMap<String, EventUses> = context
		.user_functions
		.iter()
		.map(|(name, definition)| (name.clone(), EventUses::of(&definition.body, &functions, &verbs)))
		.collect();
	uses.insert(ENTRY.into(), EventUses::of(program, &functions, &verbs));
	let mut events: BTreeMap<String, BTreeSet<String>> =
		uses.iter().map(|(name, uses)| (name.clone(), uses.own.clone())).collect();
	loop {
		let mut changed = false;
		for (name, function) in &uses {
			let reached: BTreeSet<String> = function
				.calls
				.iter()
				.flat_map(|callee| events.get(callee).into_iter().flatten().cloned().collect::<Vec<_>>())
				.collect();
			let all = events.get_mut(name).expect("every function has its events");
			let before = all.len();
			all.extend(reached);
			changed |= all.len() != before;
		}
		if !changed {
			return uses
				.into_iter()
				.map(|(name, function)| {
					let all = events.remove(&name).unwrap_or_default();
					(name, (function.own, all))
				})
				.collect();
		}
	}
}

/// A list `[a b]` or a record `{name: a}` built where it is written; not a block `{global y; y*y}`
pub(super) fn constructs(bracket: &Bracket, items: &[Node]) -> bool {
	let is_field = |item: &Node| matches!(item.drop_meta(), Node::Key(key, Op::Colon, _) if key.drop_meta().name() != STATE_KEYWORD);
	match bracket {
		Bracket::Square => true,
		Bracket::Curly => !items.is_empty() && items.iter().all(is_field),
		_ => false,
	}
}
