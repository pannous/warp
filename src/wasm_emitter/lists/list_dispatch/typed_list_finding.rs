//! Which list variables can be typed arrays: their sources, appends, aliases and the element type every source agrees on
use super::*;

/// The list and the loop of `list_sum(list, loop)`, what `sum xs` lowers to (library_words.rs)
pub(in crate::wasm_emitter) fn list_sum_parts(node: &Node) -> Option<(&Node, &Node)> {
	match node.drop_meta() {
		Node::List(items, _, _) => list_sum_call(items),
		_ => None,
	}
}

pub(in crate::wasm_emitter) fn list_sum_call(items: &[Node]) -> Option<(&Node, &Node)> {
	match items {
		[call, list, sum_loop] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::library_words::LIST_SUM) => {
			Some((list, sum_loop))
		}
		_ => None,
	}
}

/// The count and zero of `zero_fill(count, zero)`, what `x : 100 int` and `int[100]` lower to
pub(super) fn zero_fill_parts(value: &Node) -> Option<(&Node, &Node)> {
	match value.drop_meta() {
		Node::List(items, _, _) => match items.as_slice() {
			[call, count, zero] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::filled_lists::ZERO_FILL_CALL) => {
				Some((count, zero))
			}
			_ => None,
		},
		_ => None,
	}
}

/// `x = (t = x; …; t)`, an inlined call (inlining.rs) handing x back: the temporary t, which may share x's array; only
/// the inliner's temporaries, which nothing else reads, and only when the block hands back the very variable it got
pub(super) fn moved_through(owner: &str, statements: &[Node], last: &Node) -> Option<String> {
	let Node::Symbol(temporary) = last.drop_meta() else {
		return None;
	};
	if !crate::inlining::is_temporary(temporary) {
		return None;
	}
	let given = statements.iter().any(|statement| matches!(statement.drop_meta(), Node::Key(target, Op::Assign, value)
		if matches!(target.drop_meta(), Node::Symbol(target) if target == temporary) && matches!(value.drop_meta(), Node::Symbol(source) if source == owner)));
	given.then(|| temporary.clone())
}

/// `{a:1}`, `(1, 2)`, `(1 2)`, not the call `(f x)`
pub(super) fn is_tuple_or_object(value: &Node) -> bool {
	use crate::node::Separator;
	match value.drop_meta() {
		Node::List(_, Bracket::Curly, _) | Node::List(_, Bracket::Round, Separator::Comma) => true,
		Node::List(items, Bracket::Round, Separator::Space | Separator::None) => {
			!matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))
		}
		_ => false,
	}
}

/// The list variables a program indexes (`xs#i`, `xs#i = v`) or counts (`#xs`), as a for loop over them does
pub(super) fn indexed_lists(program: &Node) -> HashSet<String> {
	let key_variables = crate::declaration_forms::key_variables(program);
	let mut indexed = HashSet::new();
	program.visit(&mut |part| {
		let Node::Key(list, Op::Hash, index) = part else { return };
		let counted = if list.is_empty_node() { index } else { list };
		let by_key = crate::declaration_forms::looks_up_a_key(index, &key_variables);
		if let (Node::Symbol(name), false) = (counted.drop_meta(), by_key) {
			indexed.insert(name.clone());
		}
	});
	indexed
}

/// `while c do body`, what every loop lowers to
fn is_while_loop(node: &Node) -> bool {
	matches!(node, Node::Key(condition, Op::Do, _) if matches!(condition.drop_meta(), Node::Key(_, Op::While, _)))
}

/// Whether the places holding a list are its last use: no loop runs around one, and no mention of `members` runs after
/// the first (`gaps = Gaps{resumes: resumes}` after the loop filling resumes), so the list converted there changes no more
pub(super) fn held_last(program: &Node, members: &[String], places: &[&Node]) -> bool {
	fn mentions<'n>(node: &'n Node, members: &[String], loops: usize, found: &mut Vec<(&'n Node, usize)>) {
		let node = node.drop_meta();
		match node {
			Node::Symbol(name) if members.contains(name) => found.push((node, loops)),
			_ => {
				let inner = loops + usize::from(is_while_loop(node));
				node.visited_children().into_iter().for_each(|child| mentions(child, members, inner, found));
			}
		}
	}
	let mut found = Vec::new();
	mentions(program, members, 0, &mut found);
	let is_place = |mention: &Node| places.iter().any(|place| std::ptr::eq(*place, mention));
	let Some(first) = found.iter().position(|(mention, _)| is_place(mention)) else {
		return false;
	};
	found[first..].iter().all(|(mention, loops)| *loops == 0 && is_place(mention))
}

/// Whether an array made of a Node list each time `name` is assigned (Source::Converted, one pass over its items) is read
/// by index more often than it is made: some `name#i` or `#name` runs in a loop inside every loop around an assignment
/// of it. `for room in rooms { s += room#i }` makes `room` once a round and reads it once, which the cons cells' cursor
/// serves at once (card room-conversions)
pub(super) fn reads_outrun_conversions(program: &Node, name: &str) -> bool {
	fn walk(
		node: &Node,
		name: &str,
		loops: &mut Vec<usize>,
		count: &mut usize,
		found: &mut (Vec<Vec<usize>>, Vec<Vec<usize>>),
	) {
		let node = node.drop_meta();
		let is_loop = is_while_loop(node);
		if is_loop {
			loops.push(*count);
			*count += 1;
		}
		match node {
			Node::Key(target, Op::Assign | Op::Define, _) if target.drop_meta().is_symbol(name) => {
				found.0.push(loops.clone())
			}
			Node::Key(list, Op::Hash, index) => {
				let counted = if list.is_empty_node() { index } else { list };
				if counted.drop_meta().is_symbol(name) {
					found.1.push(loops.clone());
				}
			}
			_ => {}
		}
		node.visited_children().into_iter().for_each(|child| walk(child, name, loops, count, found));
		if is_loop {
			loops.pop();
		}
	}
	let mut found = (Vec::new(), Vec::new());
	walk(program, name, &mut Vec::new(), &mut 0, &mut found);
	let (assignments, reads) = found;
	reads
		.iter()
		.any(|read| assignments.iter().all(|assigned| read.len() > assigned.len() && read.starts_with(assigned)))
}

/// The items of `xs += [a b]`
pub(super) fn extended_items(value: &Node) -> Option<&[Node]> {
	match value.drop_meta() {
		Node::List(items, Bracket::Square, _) if !items.is_empty() => Some(items),
		_ => None,
	}
}

/// Each list variable with every variable holding the same list: those assigned one another (`ys = xs`), transitively
pub(super) fn alias_groups(sources: &HashMap<String, Vec<Source>>) -> HashMap<String, Vec<String>> {
	let mut neighbours: HashMap<&str, Vec<&str>> = HashMap::new();
	for (name, sources) in sources {
		neighbours.entry(name).or_default();
		for source in sources {
			if let Source::Variable(other) = source {
				neighbours.entry(name).or_default().push(other);
				neighbours.entry(other).or_default().push(name);
			}
		}
	}
	let mut groups = HashMap::new();
	for start in neighbours.keys() {
		if groups.contains_key(*start) {
			continue;
		}
		let (mut group, mut pending) = (vec![start.to_string()], vec![*start]);
		while let Some(name) = pending.pop() {
			for next in &neighbours[name] {
				if !group.iter().any(|member| member == next) {
					group.push(next.to_string());
					pending.push(next);
				}
			}
		}
		for member in &group {
			groups.insert(member.clone(), group.clone());
		}
	}
	groups
}

/// The element type every source agrees on: `Err` when a source is no typed list or they disagree, `Ok(None)` when only
/// variables of yet unknown type feed it
pub(super) fn agreed_element(
	sources: &[Source],
	typed: &HashMap<String, Option<ElementType>>,
	globals: &HashMap<String, (u32, TypedList)>,
) -> Result<Option<ElementType>, ()> {
	let mut agreed = None;
	for source in sources {
		let element = match source {
			Source::Literal(element) | Source::Append(element) => Some(*element),
			Source::Empty => None,
			// an untyped list variable converts like any Node list
			Source::Variable(name) => typed
				.get(name)
				.copied()
				.or_else(|| globals.get(name).map(|(_, list)| Some(list.element)))
				.unwrap_or(Some(ElementType::Node)),
			Source::Converted { .. } => Some(ElementType::Node),
			// parts of other element types are converted like any Node list
			Source::Concat(parts) => agreed_element(parts, typed, globals).unwrap_or(Some(ElementType::Node)),
			Source::Other => return Err(()),
		};
		match (agreed, element) {
			// numbers and Nodes mixed: Nodes
			(Some(a), Some(b)) if a != b && (a == ElementType::Node || b == ElementType::Node) => {
				agreed = Some(ElementType::Node)
			}
			(Some(a), Some(b)) if a != b => return Err(()),
			(None, element) => agreed = element,
			_ => {}
		}
	}
	Ok(agreed)
}

impl Source {
	pub(super) fn is_literal(&self) -> bool {
		matches!(self, Source::Literal(_))
	}
}

impl WasmGcEmitter {
	/// An element provably of the element type: a number literal, a variable, a call of a function returning one, or
	/// arithmetic of numbers. Not by the type alone: it defaults to Int for anything unknown (ø, a key)
	pub(super) fn is_element(&self, item: &Node, element: ElementType) -> bool {
		element == ElementType::Node || (self.get_type(item) == element.kind() && self.is_number_expression(item))
	}

	pub(super) fn is_number_expression(&self, item: &Node) -> bool {
		let is_number = |operand: &Node| {
			matches!(self.get_type(operand), Kind::Int | Kind::Float) && self.is_number_expression(operand)
		};
		match item.drop_meta() {
			Node::Number(_) | Node::Symbol(_) => true,
			// `1.5f` is `1.5 as float`
			Node::Key(value, Op::As, _) => is_number(value),
			Node::Key(left, op, right) if op.is_arithmetic() || op.is_prefix() => {
				(left.is_empty_node() || is_number(left)) && is_number(right)
			}
			Node::List(items, bracket, _) => {
				items.first().is_some_and(|head| self.user_function_head(head, bracket).is_some())
			}
			_ => false,
		}
	}

	/// The element type all `items` share
	pub(super) fn items_element(&self, items: &[Node]) -> Option<ElementType> {
		let first = items.first()?;
		ElementType::ALL.into_iter().find(|element| {
			self.is_element(first, *element) && items.iter().all(|item| self.is_element(item, *element))
		})
	}

	/// The lists `a + b + [4]` concatenates, each a variable or a literal of numbers: none for anything else
	pub(super) fn concatenated_lists<'n>(&self, value: &'n Node) -> Option<Vec<&'n Node>> {
		let Node::Key(left, Op::Add, right) = value.drop_meta() else {
			return None;
		};
		if self.get_type(value) != Kind::List {
			return None;
		}
		let part = |node: &'n Node| match node.drop_meta() {
			Node::Symbol(_) => Some(vec![node]),
			Node::List(items, Bracket::Square, _) if self.items_element(items).is_some() => Some(vec![node]),
			_ => self.concatenated_lists(node),
		};
		Some([part(left)?, part(right)?].concat())
	}

	/// Whether an append to `name` (`xs = xs + [v]`) can run while another variable holds its list: after an assignment
	/// between it and another (`ys = xs`, `xs = ys`) or in a loop around one. A list aliased only once its appends are
	/// done (`d = (out = ø; for … { out = out + [x] }; out)`, every lowered map and filter) grows in place (card map-typed),
	/// also in a loop around all that, which gives `out` a fresh list before its appends each round (card map-inside).
	/// `holds`: the places a holder takes the list as Nodes (`[xs, 5]`, `{r = xs}`), aliases too
	pub(super) fn appends_while_aliased(&self, program: &Node, name: &str, holds: &[&Node]) -> bool {
		let mut walk = EventWalk::default();
		self.list_events(program, name, holds, &mut walk);
		let events = walk.events;
		let of_kind = |kind: ListEventKind| events.iter().filter(move |event| event.kind == kind);
		// a fresh list given in the innermost loop around both, before the append and wherever the append runs
		let fresh_each_round = |append: &ListEvent, ring: usize| {
			of_kind(ListEventKind::Fresh).any(|fresh| {
				!fresh.conditional
					&& fresh.order < append.order
					&& fresh.loops.contains(&ring)
					&& append.loops.starts_with(&fresh.loops)
			})
		};
		let runs_after = |append: &ListEvent, alias: &ListEvent| {
			let innermost_shared = append.loops.iter().rev().find(|ring| alias.loops.contains(ring));
			append.order > alias.order || innermost_shared.is_some_and(|ring| !fresh_each_round(append, *ring))
		};
		of_kind(ListEventKind::Alias)
			.any(|alias| of_kind(ListEventKind::Append).any(|append| runs_after(append, alias)))
	}

	/// The appends to `name` and the assignments between it and another variable, in the order they run (an assignment
	/// after its value), each with the loops around it
	pub(super) fn list_events(&self, node: &Node, name: &str, holds: &[&Node], walk: &mut EventWalk) {
		let node = node.drop_meta();
		let is_loop = is_while_loop(node);
		if is_loop {
			walk.loops.push(walk.count);
			walk.count += 1;
		}
		match node {
			Node::Key(left, op, right) => {
				let conditional = CONDITIONAL_OPS.contains(op);
				walk.conditions += usize::from(conditional);
				[left, right].into_iter().for_each(|part| self.list_events(part, name, holds, walk));
				walk.conditions -= usize::from(conditional);
			}
			Node::List(items, _, _) => items.iter().for_each(|item| self.list_events(item, name, holds, walk)),
			_ => {}
		}
		let kind = match node {
			_ if holds.iter().any(|place| std::ptr::eq(*place, node)) => Some(ListEventKind::Alias),
			Node::Key(target, Op::Assign | Op::Define, value) => match target.drop_meta() {
				Node::Symbol(target) => match self.number_source_of(target, value) {
					Source::Append(_) if target == name => Some(ListEventKind::Append),
					Source::Variable(other) if target == name || other == name => Some(ListEventKind::Alias),
					Source::Literal(_) | Source::Empty | Source::Converted { fresh: true } if target == name => {
						Some(ListEventKind::Fresh)
					}
					_ => None,
				},
				_ => None,
			},
			_ => None,
		};
		if let Some(kind) = kind {
			walk.events.push(ListEvent {
				kind,
				order: walk.count,
				loops: walk.loops.clone(),
				conditional: walk.conditions > 0,
			});
			walk.count += 1;
		}
		if is_loop {
			walk.loops.pop();
		}
	}

	pub(super) fn source_of(&self, target: &str, value: &Node) -> Source {
		match self.number_source_of(target, value) {
			// not a tuple `(1, 2)` or an object `{a:1}`, whose brackets a typed list would lose
			Source::Other if self.get_type(value) == Kind::List && !is_tuple_or_object(value) => Source::Converted {
				// a function giving back the list it got (`f(a) := a`) returns the caller's list, no new one
				fresh: super::super::super::lists::list_abi::called_function(value).is_some_and(|name| {
					!self
						.ctx
						.user_functions
						.get(name)
						.is_some_and(super::super::super::lists::list_sharing::returns_a_parameter)
				}),
			},
			source => source,
		}
	}

	pub(super) fn number_source_of(&self, target: &str, value: &Node) -> Source {
		let literal = |items: &[Node]| self.items_element(items).map_or(Source::Other, Source::Literal);
		match value.drop_meta() {
			// `(a = …; b)`, an inlined call: worth its last value
			Node::List(items, Bracket::Round, crate::node::Separator::Semicolon | crate::node::Separator::Newline)
				if !items.is_empty() =>
			{
				self.source_of(target, &items[items.len() - 1])
			}
			Node::Empty => Source::Empty,
			Node::List(items, Bracket::Square, _) if items.is_empty() => Source::Empty,
			Node::Key(list, Op::Add, appended) if matches!(list.drop_meta(), Node::Symbol(name) if name == target) => {
				match appended.drop_meta() {
					Node::List(items, Bracket::Square, _) if !items.is_empty() => {
						Source::Append(self.items_element(items).unwrap_or(ElementType::Node))
					}
					_ => Source::Other,
				}
			}
			Node::Symbol(name) => Source::Variable(name.clone()),
			_ if self.concatenated_lists(value).is_some() => Source::Concat(
				self.concatenated_lists(value)
					.expect("guarded")
					.into_iter()
					.map(|part| self.number_source_of(target, part))
					.collect(),
			),
			_ if zero_fill_parts(value).is_some() => {
				literal(std::slice::from_ref(zero_fill_parts(value).expect("guarded").1))
			}
			Node::List(items, Bracket::Square, _) => literal(items),
			_ => Source::Other,
		}
	}

	/// The list variables of the current body that can be typed arrays: locals of kind List that are only ever assigned
	/// int (float) literal lists, the empty list, numbers appended to themselves or other such variables, never updated
	/// by another operator (`xs -= …`), not captured by a function, no global, not assigned inside a `try`; a float list
	/// only ever gets floats by index. Lists are shared (P200b): a list some holder keeps as Nodes stays Nodes once it
	/// changes (list_sharing.rs). Any other use reads them as Nodes, so it stays correct.
	pub(in crate::wasm_emitter) fn find_typed_lists(&self, program: &Node) -> HashMap<String, TypedList> {
		// a function defined here has variables of its own: `ocean(x, y) := sun_light .* x` lowers to a loop whose item
		// list holds sun_light, no alias of main's (find_typed_globals already looks at main without them)
		let program = &self.without_definitions(program);
		let mut sources: HashMap<String, Vec<Source>> = HashMap::new();
		let mut excluded: HashSet<String> = HashSet::new();
		let mut updated: HashSet<String> = HashSet::new();
		let mut assigned_kinds: HashMap<String, Vec<Kind>> = HashMap::new();
		// `xs = float[n]`: declared floats, so any number written into it is stored as one
		let mut declared_floats: HashSet<String> = HashSet::new();
		// `row = grid#i`, `ys = p.xs`: a typed list read from a field or item, an array copy of a list others hold
		let mut place_sourced: HashSet<String> = HashSet::new();
		// `xs.pop()`, `xs.clear()`: a Node list changed in place, which a typed array would copy
		let mut changed_in_place: HashSet<String> = HashSet::new();
		program.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			match target.drop_meta() {
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) => {
					if matches!(value.drop_meta(), Node::Key(_, Op::Hash | Op::Dot, _)) {
						place_sourced.insert(name.clone());
					}
					if zero_fill_parts(value).is_some_and(|(_, zero)| {
						self.items_element(std::slice::from_ref(zero)) == Some(ElementType::Float)
					}) {
						declared_floats.insert(name.clone());
					}
					// `xs: floats = [1/3, 2]`: the declared element type, its exact numbers converted (card number-converts)
					let source = match self.source_of(name, value) {
						Source::Literal(ElementType::Int) if declares_float_items(target) => {
							declared_floats.insert(name.clone());
							Source::Literal(ElementType::Float)
						}
						source => source,
					};
					sources.entry(name.clone()).or_default().push(source);
					if super::super::super::lists::list_sharing::changes_in_place(value) {
						updated.insert(name.clone());
						changed_in_place.insert(name.clone());
					}
				}
				Node::Symbol(name) if *op == Op::AddAssign && extended_items(value).is_some() => {
					let items = extended_items(value).expect("guarded");
					sources
						.entry(name.clone())
						.or_default()
						.push(Source::Append(self.items_element(items).unwrap_or(ElementType::Node)));
				}
				Node::Symbol(name) if is_update(op) => {
					excluded.insert(name.clone());
					updated.insert(name.clone());
				}
				Node::Key(list, Op::Hash, index) if matches!(op, Op::Assign | Op::Define) || is_update(op) => {
					let Node::Symbol(name) = list.drop_meta() else { return };
					if self.map_key(index).is_some() || self.dynamic_key(index).is_some() {
						excluded.insert(name.clone());
						return;
					}
					updated.insert(name.clone());
					let assigned = if is_update(op) {
						Node::Key(target.clone(), op.base_op(), value.clone())
					} else {
						value.as_ref().clone()
					};
					assigned_kinds.entry(name.clone()).or_default().push(self.get_type(&assigned));
				}
				_ => {}
			}
		});
		program.visit(&mut |part| {
			// `f(xs) === xs`: a typed array's Node is a fresh copy at every use, so a list compared by identity stays
			// the one Node list (card xs-xs)
			if let Node::Key(left, Op::Identical | Op::NotIdentical, right) = part {
				for side in [left, right] {
					if let Node::Symbol(name) = side.drop_meta() {
						excluded.insert(name.clone());
					}
				}
			}
			if matches!(part, Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == super::super::super::RAN_WITHOUT_ERROR)) {
				part.visit(&mut |inner| if let Node::Key(target, _, _) = inner {
					if let Node::Symbol(name) = target.drop_meta() { excluded.insert(name.clone()); }
				});
			}
		});
		excluded.extend(self.ctx.user_globals.keys().cloned());
		excluded.extend(self.ctx.captures.values().flatten().map(|(name, _)| name.clone()));
		for (name, sources) in &sources {
			if sources.iter().any(|source| matches!(source, Source::Append(_))) {
				updated.insert(name.clone());
			}
		}
		let groups = alias_groups(&sources);
		// `ys = xs; xs.pop()`: the aliases share the one Node list the change is made in (card aliased-list)
		excluded.extend(
			changed_in_place
				.iter()
				.filter_map(|name| groups.get(name))
				.filter(|group| group.len() > 1)
				.flatten()
				.cloned(),
		);
		let group_updated =
			|name: &String| groups.get(name).is_some_and(|group| group.iter().any(|member| updated.contains(member)));
		// a function of an imported warp module may change the Node it gets: the change is written back to it (wasm_modules.rs
		// write_back), not to a typed array
		let changes_arguments = |callee: &str| {
			self.ctx.ffi_imports.get(callee).is_some_and(|import| {
				crate::modules::wasm_modules::is_module_path(import.library)
					&& crate::modules::wasm_modules::exports(import.library)
						.get(import.name)
						.is_some_and(|export| export.node_result.is_some())
			})
		};
		let held = super::super::super::lists::list_sharing::held_elsewhere(
			program,
			&self.ctx.user_functions,
			&changes_arguments,
		);
		// a counter `i++` changes no list
		let is_number =
			|name: &String| self.scope.lookup(name).is_some_and(|local| matches!(local.kind, Kind::Int | Kind::Float));
		let written: HashSet<String> =
			super::super::super::lists::list_sharing::written_roots(program, &self.ctx.user_functions)
				.into_iter()
				.filter(|root| !is_number(root))
				.collect();
		loop {
			let typed = self.typed_list_elements(&sources, &excluded, &declared_floats, &assigned_kinds);
			// such a copy is no snapshot only while no list but the typed arrays is changed in place (shared-lists-typed):
			// `ys = p.xs; p.xs#1 = 7` shares the list instead
			let writes_shared_lists = written.iter().any(|root| !typed.contains_key(root));
			let snapshots: Vec<String> =
				typed.keys().filter(|name| writes_shared_lists && place_sourced.contains(*name)).cloned().collect();
			if !snapshots.is_empty() {
				excluded.extend(snapshots);
				continue;
			}
			// a changing list is no typed array where a holder keeps it as Nodes: an untyped variable or one of another
			// element type holding the same list, a field or a call's argument it came from or goes to
			let element_of = |name: &String| {
				typed
					.get(name)
					.copied()
					.flatten()
					.or_else(|| self.typed_globals.get(name).map(|(_, list)| list.element))
			};
			// a list an item or a field holds once it is complete (`o = {r = 1..n}`, `[xs, 5]`), or as its last use, is
			// converted there, never changed after: its Node copy and the array stay equal
			let held_complete = |name: &String| {
				let places: Vec<&Node> =
					groups[name].iter().filter_map(|member| held.places.get(member)).flatten().copied().collect();
				let last = held_last(program, &groups[name], &places);
				!writes_shared_lists
					&& groups[name].iter().all(|member| {
						(last || !written.contains(member)) && !self.appends_while_aliased(program, member, &places)
					})
			};
			let held_as_nodes =
				|name: &String| held.passed.contains(name) || (held.places.contains_key(name) && !held_complete(name));
			let shared_as_nodes = |name: &String| {
				held_as_nodes(name)
					|| (group_updated(name)
						&& (groups[name]
							.iter()
							.any(|member| element_of(member).is_none() || element_of(member) != element_of(name))
							|| sources[name].iter().any(|source| matches!(source, Source::Converted { fresh: false }))))
			};
			let parted: Vec<String> = typed.keys().filter(|name| shared_as_nodes(name)).cloned().collect();
			if parted.is_empty() {
				// a Node list pays a conversion wherever it is used as a whole: worth it for one that is indexed or counted (more
				// often than it is converted), and for one appended to (map, filter), which as cons cells walks the whole list
				// per append
				let indexed = indexed_lists(program);
				let appended = |name: &String| sources[name].iter().any(|source| matches!(source, Source::Append(_)));
				let converted =
					|name: &String| sources[name].iter().any(|source| matches!(source, Source::Converted { .. }));
				let pays = |name: &String| {
					indexed.contains(name) && (!converted(name) || reads_outrun_conversions(program, name))
				};
				return typed
					.into_iter()
					.filter(|(name, element)| *element != Some(ElementType::Node) || pays(name) || appended(name))
					.filter_map(|(name, element)| {
						let aliased = groups[&name].len() > 1 && self.appends_while_aliased(program, &name, &[]);
						let list = TypedList { element: element?, updated: group_updated(&name), aliased };
						Some((name, list))
					})
					.collect();
			}
			excluded.extend(parted);
		}
	}

	/// The element type of each list variable that can be a typed array, `None` while only variables of yet unknown
	/// type feed it
	pub(super) fn typed_list_elements(
		&self,
		sources: &HashMap<String, Vec<Source>>,
		excluded: &HashSet<String>,
		declared_floats: &HashSet<String>,
		assigned_kinds: &HashMap<String, Vec<Kind>>,
	) -> HashMap<String, Option<ElementType>> {
		// a variable first assigned ø and appended to only in a branch (`out = ø; … if c {out = out + [x]}`) has the kind ø
		let appended = |name: &String| sources[name].iter().any(|source| matches!(source, Source::Append(_)));
		let kind_of = |name: &String| self.scope.lookup(name).map(|local| local.kind);
		// as does one first assigned ø and then a list variable in a loop (`d = ø; for … { d = xs.map(…) }`)
		let given_a_list = |name: &String| {
			sources[name]
				.iter()
				.any(|source| matches!(source, Source::Variable(other) if kind_of(other) == Some(Kind::List)))
		};
		let is_list_local = |name: &String| {
			self.scope.lookup(name).is_some_and(|local| {
				!local.is_param
					&& (local.kind == Kind::List
						|| (local.kind == Kind::Empty && (appended(name) || given_a_list(name))))
			})
		};
		let has_start = |name: &String| sources[name].iter().any(|source| !matches!(source, Source::Append(_)));
		// an index assignment stores what node_with_node_at would: any exact number into an int list, only floats into a
		// float list, any number into a declared float array (converted)
		let takes_assignments = |name: &String, element: ElementType| {
			let takes = |kind: &Kind| match element {
				ElementType::Float => *kind == Kind::Float || (*kind == Kind::Int && declared_floats.contains(name)),
				ElementType::Int => *kind == Kind::Int,
				ElementType::Node => true,
			};
			assigned_kinds.get(name).is_none_or(|kinds| kinds.iter().all(takes))
		};
		let mut typed: HashMap<String, Option<ElementType>> = sources
			.keys()
			.filter(|name| is_list_local(name) && has_start(name) && !excluded.contains(*name))
			.map(|name| (name.clone(), None))
			.collect();
		// greatest fixpoint: drop a variable fed by something untyped, until all that remain agree
		loop {
			let mut changed = false;
			for name in typed.keys().cloned().collect::<Vec<_>>() {
				match agreed_element(&sources[&name], &typed, &self.typed_globals) {
					Ok(Some(element)) if !takes_assignments(&name, element) => {
						typed.remove(&name);
						changed = true;
					}
					Err(()) => {
						typed.remove(&name);
						changed = true;
					}
					// a source that turned into Nodes (its variable left the typed lists) widens an int list to Nodes
					Ok(Some(element)) if typed[&name] != Some(element) => {
						typed.insert(name, Some(element));
						changed = true;
					}
					Ok(_) => {}
				}
			}
			if !changed {
				return typed;
			}
		}
	}

	/// Push what the typed capture global of main's typed list `name` takes: the array, a copy when main changes it later,
	/// so the function keeps what it captured. False for any other capture global.
	pub(in crate::wasm_emitter) fn emit_typed_capture(&mut self, func: &mut Function, name: &str, global: u32) -> bool {
		let Some(&captured) = self.typed_capture_globals.get(&global) else {
			return false;
		};
		let (slot, list) =
			self.typed_list(&Node::Symbol(name.to_string())).expect("a typed capture global takes main's typed list");
		func.instruction(&slot.get());
		if list.updated {
			self.emit_call(func, captured.element.runtime().copy);
		}
		true
	}

	/// A null-initialized mutable global holding a typed list
	pub(in crate::wasm_emitter) fn declare_typed_list_global(&mut self, element: ElementType) -> u32 {
		let list_type = self.typed_list_types(element).1;
		self.declare_global_of(Ref(self.typed_list_ref(element)), ConstExpr::ref_null(HeapType::Concrete(list_type)))
	}

	/// The globals (main's variables that functions read, `global` ones) held as typed arrays: what find_typed_lists finds
	/// for them as main's locals, when no function replaces them, so all their sources are main's. A function may write
	/// their items (`s#i = v`) with values of the element type: `global s = int[n]; def f() { s#i = 1 }` keeps the array
	pub(in crate::wasm_emitter) fn find_typed_globals(
		&mut self,
		program: &Node,
		main: &crate::analyzer::Scope,
	) -> HashMap<String, TypedList> {
		if main.globals.is_empty() {
			return HashMap::new();
		}
		let mut as_locals = main.clone();
		for (name, global) in &main.globals {
			as_locals
				.locals
				.entry(name.clone())
				.or_insert_with(|| crate::compiler::local::Local { is_param: false, ..global.clone() });
		}
		let saved = std::mem::replace(&mut self.scope, as_locals);
		let main_code = self.without_definitions(program); // function bodies are checked below, each in its own scope
		let mut found: HashMap<String, TypedList> =
			self.find_typed_lists(&main_code).into_iter().filter(|(name, _)| main.globals.contains_key(name)).collect();
		self.scope = saved;
		let function_kinds = crate::analyzer::FunctionKinds::new(self.user_function_kinds());
		let functions: Vec<_> =
			self.ctx.user_functions.values().map(|function| (function.params.clone(), function.body.clone())).collect();
		for (params, body) in functions {
			let scope = crate::analyzer::function_body_scope(
				&params,
				&body,
				&function_kinds,
				&main.globals,
				&self.ctx.closure_variable_targets,
			);
			let saved = std::mem::replace(&mut self.scope, scope);
			found.retain(|name, list| {
				!self.replaces(&body, name, list.element) && self.item_writes_fit(&body, name, list)
			});
			self.scope = saved;
		}
		found
	}

	/// The program's statements without its function definitions and imports
	pub(super) fn without_definitions(&self, program: &Node) -> Node {
		match program.drop_meta() {
			Node::List(items, bracket, separator) => Node::List(
				items.iter().filter(|item| !self.is_definition(item)).cloned().collect(),
				bracket.clone(),
				separator.clone(),
			),
			other => other.clone(),
		}
	}

	/// Whether `body` assigns or updates the typed list `name` itself or a field of it (`name.x = v`), other than with a
	/// new array of its element type (`canvas_pixels = int[w * h]`); its items are item_writes_fit's
	pub(super) fn replaces(&self, body: &Node, name: &str, element: ElementType) -> bool {
		let mut found = false;
		body.visit(&mut |part| {
			if let Node::Key(target, op, value) = part {
				let target = match target.drop_meta() {
					Node::Key(owner, Op::Dot, _) => owner.drop_meta(),
					other => other,
				};
				let new_array = *op == Op::Assign
					&& zero_fill_parts(value)
						.is_some_and(|(_, zero)| self.items_element(std::slice::from_ref(zero)) == Some(element));
				found |= (matches!(op, Op::Assign | Op::Define) || is_update(op))
					&& !new_array && matches!(target, Node::Symbol(assigned) if assigned == name);
			}
		});
		found
	}

	/// Whether every item write of `body` to the typed list `name` stores a value of its element type at an index (not a
	/// map key); marks the list updated when there is one
	pub(super) fn item_writes_fit(&self, body: &Node, name: &str, list: &mut TypedList) -> bool {
		let mut fits = true;
		body.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			let Node::Key(owner, Op::Hash, index) = target.drop_meta() else {
				return;
			};
			if !(matches!(op, Op::Assign | Op::Define) || is_update(op))
				|| !matches!(owner.drop_meta(), Node::Symbol(written) if written == name)
			{
				return;
			}
			list.updated = true;
			let assigned = if is_update(op) {
				Node::Key(target.clone(), op.base_op(), value.clone())
			} else {
				value.as_ref().clone()
			};
			let kind = self.get_type(&assigned);
			let stored = match list.element {
				ElementType::Float => kind == Kind::Float,
				ElementType::Int => kind == Kind::Int,
				ElementType::Node => true,
			};
			fits &= stored && self.map_key(index).is_none() && self.dynamic_key(index).is_none();
		});
		fits
	}
}
