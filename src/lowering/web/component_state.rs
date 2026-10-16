//! Components keep their own state (card web-components, notes/web_framework.md step 4): in
//! `def Counter(start) { count = start; div{ button{ on click { count += 1 } "+" } p{ "n " + count } } }` each Counter
//! of the page counts on its own. A variable of a component that one of its element handlers mentions is a list with
//! one entry per instance, `Counter·count`; an instance is the n-th call of the component in a render (as React's
//! hooks: the same call order finds the same state), its first call sets the entry, later renders keep it. The element
//! of a handler carries `data-warp-instance`, which the page passes as `event.instance` (playground.js), so the handler
//! (element_events.rs makes it main-level) changes its own instance's entry. The program's last line becomes the
//! getter `page·markup`, which starts each render at the first instance.
//! `on mount {…}` in a component runs when an instance first renders, `on cleanup {…}` when a render has fewer instances
//! than the one before (the last ones left; card web-cleanup); their state entries go with them.
//! A component whose markup holds a style sheet names itself on its root element (`data-warp-scope: "Card"`), so the
//! sheet styles only its own elements (web/markup.rs, card web-scoped).

use super::nodes::{any_part, call, if_then, key, named_assignment};
use crate::element_events::{element_items, handler_at, has_element_handler, HANDLER_ATTRIBUTE_PREFIX};
use crate::event_signals::{function_with_globals, hole_name, last_statement, text_hole_name};
use crate::law::substitute;
use crate::lowering::function_head;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::ON_WORD;
use std::collections::{HashMap, HashSet};

const JOINER: char = '·';
/// `data-warp-instance`: the attribute naming the instance of a component's element, read by playground.js
const INSTANCE_ATTRIBUTE: &str = "instance";
const INSTANCES: &str = "instances";
const INSTANCE: &str = "instance";
const RENDER: &str = "page·markup";
// the templates' placeholders are upper case: `instance` stays the field of the event
const STATE_DECLARATION: &str = "global STATE = []";
/// In a dev build the state lists live in the dev store, so a reload keeps each instance's state (card web-dev-instances)
const DEV_STATE_DECLARATION: &str = "global STATE = std_io(\"store\", \"declare\", [NAME, [], STORE])";
const DEV_STATE_SAVE: &str = "std_io(\"store\", \"save\", [NAME, STATE, STORE])";
const COUNTER_DECLARATION: &str = "global COUNTER = 0";
const COUNTED: &str = "COUNTER += 1";
const KEYED: &str = "KEY = COUNTER";
const FIRST_SET: &str = "if #STATE < KEY { STATE.push(INITIAL) }";
const READ: &str = "STATE#KEY";
const HANDLER_READ: &str = "STATE#(event.instance)";
const RESET: &str = "COUNTER = 0";
const SEEN_DECLARATION: &str = "global SEEN = 0";
const FIRST_RENDER: &str = "SEEN < KEY";
const CLEANED_FROM: &str = "CLEANED = COUNTER + 1";
const CLEANUP_LOOP: &str = "while CLEANED <= SEEN { CLEANUP(CLEANED); CLEANED += 1 }";
const TRUNCATED: &str = "if #STATE > COUNTER { STATE = STATE[0..COUNTER] }";
const SEEN_SET: &str = "SEEN = COUNTER";
/// `Counter(t, key: t.id)` (card web-instance-keys): the instance's state follows its key, not its position
const KEY_PARAMETER: &str = "key = ø";
const KEY_WORD: &str = "key";
/// A keyed instance moves its key's entries of the render before to its position; a new key takes a fresh slot, the
/// one it finds there moving to the end. FRESH: an instance not shown before, whose state starts anew
const KEYED_PLACEMENT: &str = "FRESH = SEEN < KEY; if key != ø { FOUND = KEYS.index_of(key); FRESH = FOUND == 0; \
	if FRESH { if #KEYS < KEY { KEYS.push(key) } else { KEYS.push(KEYS#KEY); KEYS#KEY = key; STALE_MOVES } } \
	else if FOUND != KEY { SWAPS } } else { if #KEYS < KEY { KEYS.push(ø) } else { KEYS#KEY = ø } }";
const STALE_MOVE: &str = "LIST.push(LIST#KEY); ";
const SWAP: &str = "MOVED = LIST#FOUND; LIST#FOUND = LIST#KEY; LIST#KEY = MOVED; ";
const KEYED_FIRST_SET: &str = "if #STATE < KEY { STATE.push(INITIAL) } else if FRESH { STATE#KEY = INITIAL }";
/// moved keys may stand past the instances of the render before: the cleanup reaches every slot
const KEYED_SEEN: &str = "SEEN = #KEYS";
/// a keyed instance mounts when its key is new
const FRESH_PLACEHOLDER: &str = "FRESH";
const SHOWN: &str = "page·shown";
/// a page with holes keeps its rendering, which they are read from (card web-fine-components)
const SHOWN_DECLARATION: &str = "global SHOWN = ø";
/// renders once before the playground reads the holes (worker.js showHoles)
const PAGE_HOLES: &str = "page·holes";
/// The readers of a rendering's holes: an element by its path, and the text after its n-th child element (lib/markup.warp)
const ELEMENT_AT: &str = "page·element_at";
const TEXT_AT: &str = "page·text_at";
const HOLE_READERS: [(&str, &str); 2] = [
	(ELEMENT_AT, "READER(value, path) := html_element_at(value, path)"),
	(TEXT_AT, "READER(value, path, after) := html_text_at(value, path, after)"),
];
const MOUNT: &str = "mount";
const CLEANUP: &str = "cleanup";
const SEEN: &str = "seen";
const CLEANED: &str = "cleaned";
const KEYS: &str = "keys";
const FRESH: &str = "fresh";
const FOUND: &str = "found";

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else {
		return program;
	};
	let statements: Vec<Node> =
		statements.into_iter().map(|statement| scoped_component(&statement).unwrap_or(statement)).collect();
	if !has_components(&statements) {
		return Node::List(statements, bracket, separator);
	}
	let markups = markup_functions(&statements);
	let keyed = keyed_components(&statements);
	let mut declarations = vec![];
	let mut components = vec![];
	let mut lowered: Vec<Node> = statements
		.into_iter()
		.map(|statement| match stateful_component(&statement) {
			Some((head, body, state)) => {
				let component = Component::new(&head, state, &body, &keyed);
				declarations.extend(component.declarations());
				let definition = key(component.head(head), Op::Define, component.body(body));
				components.push(component);
				definition
			}
			None => statement,
		})
		.collect();
	if components.is_empty() {
		return Node::List(lowered, bracket, separator);
	}
	let state_lists: HashMap<(String, String), String> = components.iter().flat_map(Component::state_lists).collect();
	let own_names: Vec<String> = lowered.iter().filter_map(named_assignment).map(|(name, _)| name).collect();
	lowered = lowered.into_iter().map(|statement| with_state_lists(statement, &state_lists, &own_names)).collect();
	let shown = lowered.pop().expect("a program with a component has a last line");
	let holes = hole_bindings(&shown, &markups);
	if !holes.is_empty() {
		declarations.push(filled(SHOWN_DECLARATION, &[("SHOWN", SHOWN)]));
	}
	let resets = components.iter().map(|component| filled(RESET, &[("COUNTER", &component.counter)]));
	let rendered = crate::variable_signals::assign_variable(SHOWN, shown);
	let after = components.iter().flat_map(Component::after_render);
	let render_body = resets.chain([rendered]).chain(after).chain([symbol(SHOWN)]).collect();
	let render = key(symbol(RENDER), Op::Define, Node::List(render_body, Bracket::Curly, Separator::Semicolon));
	let cleanups = components.iter().filter_map(Component::cleanup_function);
	Node::List(
		declarations
			.into_iter()
			.chain(cleanups)
			.chain(lowered)
			.chain([render])
			.chain(holes)
			.chain([symbol(RENDER)])
			.collect(),
		bracket,
		separator,
	)
}

/// Do the statements define components keeping state, or with a lifecycle: the ones this pass lowers
fn has_components(statements: &[Node]) -> bool {
	statements.iter().any(|statement| has_element_handler(statement) || has_lifecycle(statement))
}

/// The readers of the holes of a page with components (hole_bindings), for a program defining a function that shows
/// markup: defined before modules::resolve, which keeps the definitions of lib/markup.warp they call (page_html.rs
/// use_markup)
pub(crate) fn hole_readers(statements: &[Node]) -> Vec<Node> {
	if markup_functions(statements).is_empty() {
		return vec![];
	}
	HOLE_READERS.iter().map(|(name, template)| super::nodes::filled(template, vec![("READER", symbol(name))])).collect()
}

/// The functions showing markup, by name with that markup as written: what a call of one shows
fn markup_functions(statements: &[Node]) -> HashMap<String, Node> {
	statements
		.iter()
		.filter_map(function_definition)
		.filter_map(|(name, _, _, body)| {
			let markup = last_statement(&body).into_owned();
			crate::web::markup::is_markup(&markup).then_some((name, markup))
		})
		.collect()
}

/// The holes of the shown markup, looking through the components it calls (card web-fine-components): `page·holes`
/// renders once, each hole is its part of that rendering, the instance's own state shown. Only a program rendering
/// itself has them: the playground reads them
fn hole_bindings(shown: &Node, markups: &HashMap<String, Node>) -> Vec<Node> {
	if !crate::pipeline::compile_mode().renders_itself() {
		return vec![];
	}
	let (elements, texts) = crate::web::markup::holes_through(shown, markups);
	let globals = HashSet::from([SHOWN.to_string()]);
	let bound = |name: String, part: Node| function_with_globals(&name, false, &[part], &globals);
	let path_list = |path: &[usize]| Node::list(path.iter().map(|index| Node::int(*index as i64)).collect());
	let element_holes = elements
		.into_iter()
		.map(|(path, _)| bound(hole_name(&path), call(ELEMENT_AT, vec![symbol(SHOWN), path_list(&path)])));
	let text_holes = texts.into_iter().map(|hole| {
		let part = call(TEXT_AT, vec![symbol(SHOWN), path_list(&hole.path), Node::int(hole.after as i64)]);
		bound(text_hole_name(&hole.path, hole.after), part)
	});
	let holes: Vec<Node> = element_holes.chain(text_holes).collect();
	if holes.is_empty() {
		return holes;
	}
	let render = function_with_globals(PAGE_HOLES, false, &[symbol(RENDER)], &HashSet::new());
	[render].into_iter().chain(holes).collect()
}

/// `def F(p) {…}`, `F(p) := …`: its name, its head `F(p)`, its operator and body
fn function_definition(statement: &Node) -> Option<(String, Node, Op, Node)> {
	let lowered = crate::declarations::lower_c_functions(statement.clone());
	let definition = function_head(&lowered)?;
	let op = definition.plain_operator()?;
	Some((definition.name.to_string(), definition.head.clone(), op, definition.body.clone()))
}

/// Outside its component (card web-outside-state): `counters`, Counter's lower-case plural, is its instances in page
/// order: `counters#1.count` the first one's count, `counters.count` every one's. `Counter.instances` is the long form,
/// `Counter.count` short for `counters.count`. A program's own variable `counters` stays its own.
fn with_state_lists(node: Node, state_lists: &HashMap<(String, String), String>, own_names: &[String]) -> Node {
	let rewritten = match node.drop_meta() {
		Node::Key(target, Op::Dot, field) => match target.drop_meta() {
			Node::Key(instances, Op::Hash, index) => state_list(instances, field, state_lists, own_names).map(|list| {
				key(Node::Symbol(list), Op::Hash, with_state_lists(index.as_ref().clone(), state_lists, own_names))
			}),
			_ => state_list(target, field, state_lists, own_names).map(Node::Symbol),
		},
		_ => None,
	};
	rewritten.unwrap_or_else(|| node.map_children(|child| with_state_lists(child, state_lists, own_names)))
}

/// The list of the state variable `field` read through `instances`: `counters`, `Counter.instances` or `Counter`
fn state_list(
	instances: &Node,
	field: &Node,
	state_lists: &HashMap<(String, String), String>,
	own_names: &[String],
) -> Option<String> {
	let field = field.symbol_name()?;
	let component = state_lists
		.keys()
		.map(|(component, _)| component)
		.find(|component| names_instances(instances, component, own_names))?;
	state_lists.get(&(component.clone(), field.to_string())).cloned()
}

fn names_instances(instances: &Node, component: &str, own_names: &[String]) -> bool {
	match instances.drop_meta() {
		Node::Symbol(word) => !own_names.contains(word) && (word == component || *word == instances_name(component)),
		Node::Key(name, Op::Dot, word) if name.is_symbol(component) && word.is_symbol(INSTANCES) => {
			crate::normalize::hint(
				&format!("{component}.{INSTANCES}"),
				&instances_name(component),
				"a component's instances are its lower-case plural",
			);
			true
		}
		_ => false,
	}
}

/// `Counter` → `counters`, `Box` → `boxes`, `Story` → `stories`
fn instances_name(component: &str) -> String {
	let mut lower = component.to_string();
	if let Some(first) = lower.get(..1).map(str::to_lowercase) {
		lower.replace_range(..1, &first);
	}
	let consonant_y = lower.ends_with('y') && !lower[..lower.len() - 1].ends_with(['a', 'e', 'i', 'o', 'u']);
	match () {
		_ if consonant_y => format!("{}ies", &lower[..lower.len() - 1]),
		_ if ["s", "x", "z", "ch", "sh"].iter().any(|ending| lower.ends_with(ending)) => format!("{lower}es"),
		_ => format!("{lower}s"),
	}
}

/// The functions called with `key:`, `Counter(t, key: t.id)`
fn keyed_components(statements: &[Node]) -> Vec<String> {
	let mut keyed = vec![];
	for statement in statements {
		statement.visit(&mut |part| {
			let Node::List(items, _, _) = part.drop_meta() else {
				return;
			};
			let has_key = items.iter().skip(1).any(
				|argument| matches!(argument.drop_meta(), Node::Key(word, Op::Colon, _) if word.is_symbol(KEY_WORD)),
			);
			keyed.extend(items.first().and_then(Node::symbol_name).filter(|_| has_key).map(str::to_string));
		});
	}
	keyed
}

/// `on mount {…}` or `on cleanup {…}` among a component's statements: the word and the body
fn lifecycle(statement: &Node) -> Option<(String, Node)> {
	let Node::List(words, _, Separator::Space | Separator::None) = statement.drop_meta() else {
		return None;
	};
	if words.first().and_then(Node::symbol_name).is_none_or(|word| word != ON_WORD) {
		return None;
	}
	let (word, body) = match &words[1..] {
		[word, body] => (word.drop_meta().name(), body.clone()),
		[pair] => match pair.drop_meta() {
			Node::Key(word, Op::Colon, body) => (word.drop_meta().name(), body.as_ref().clone()),
			_ => return None,
		},
		_ => return None,
	};
	[MOUNT, CLEANUP].contains(&word.as_str()).then_some((word, body))
}

fn has_lifecycle(statement: &Node) -> bool {
	any_part(statement, |part| lifecycle(part).is_some())
}

/// A component's head, its statements and its state: each variable its handlers mention, with its first value
type StatefulComponent = (Node, Vec<Node>, Vec<(String, Node)>);

/// `def F(p) { x = … markup with handlers }`
fn stateful_component(statement: &Node) -> Option<StatefulComponent> {
	if !has_element_handler(statement) && !has_lifecycle(statement) {
		return None;
	}
	let (_, head, _, body) = function_definition(statement)?;
	let statements = match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.clone(),
		single => vec![single.clone()],
	};
	// what its handlers and its cleanup read is kept per instance
	let cleanups = statements
		.iter()
		.filter_map(lifecycle)
		.filter(|(word, _)| word == CLEANUP)
		.flat_map(|(_, body)| crate::variable_signals::symbols(&body));
	let mentioned: Vec<String> = handler_symbols(&statements).into_iter().chain(cleanups).collect();
	let state: Vec<(String, Node)> =
		statements.iter().filter_map(named_assignment).filter(|(name, _)| mentioned.contains(name)).collect();
	(!state.is_empty() || statements.iter().any(|statement| lifecycle(statement).is_some()))
		.then_some((head, statements, state))
}

/// `def Card(t) = div{ style{ "h2": {…} } h2{t} }`: the definition with its root element naming the component
fn scoped_component(statement: &Node) -> Option<Node> {
	let mut has_sheet = false;
	statement.visit(&mut |part| has_sheet |= crate::web::markup::is_style_sheet(part));
	if !has_sheet {
		return None;
	}
	let (name, head, op, body) = function_definition(statement)?;
	let attribute = key(symbol(crate::web::markup::SCOPE_ATTRIBUTE), Op::Colon, Node::Text(name));
	let body = match body {
		Node::List(mut items, Bracket::Curly, separator) => {
			let root = with_attribute(items.pop()?, attribute)?;
			items.push(root);
			Node::List(items, Bracket::Curly, separator)
		}
		root => with_attribute(root, attribute)?,
	};
	Some(key(head, op, body))
}

/// The element with the attribute first among its items
fn with_attribute(element: Node, attribute: Node) -> Option<Node> {
	let Node::Key(tag, op, content) = element.drop_meta().clone() else {
		return None;
	};
	if !crate::web::markup::is_element_tag(&tag.drop_meta().name()) {
		return None;
	}
	let items = match *content {
		Node::List(items, Bracket::Curly, _) => items,
		single => vec![single],
	};
	Some(Node::Key(
		tag,
		op,
		Box::new(Node::List(std::iter::once(attribute).chain(items).collect(), Bracket::Curly, Separator::Space)),
	))
}

/// The names the element handlers in the statements mention
fn handler_symbols(statements: &[Node]) -> Vec<String> {
	let mut names = vec![];
	for statement in statements {
		statement.visit(&mut |part| {
			if let Some(items) = element_items(part) {
				names.extend(
					(0..items.len())
						.filter_map(|index| handler_at(items, index))
						.flat_map(|(_, body, _)| crate::variable_signals::symbols(&body)),
				);
			}
		});
	}
	names
}

/// A template with its placeholders filled by names
fn filled(template: &str, names: &[(&str, &str)]) -> Node {
	super::nodes::filled(template, names.iter().map(|(placeholder, name)| (*placeholder, symbol(name))))
}

/// `template` for the state list `list`, named in the dev store by its own name
fn kept_in_dev(template: &str, list: &str) -> Node {
	super::nodes::filled(
		template,
		vec![
			("STATE", symbol(list)),
			("NAME", Node::Text(list.to_string())),
			("STORE", Node::Text(crate::stored_values::DEV_STORE.to_string())),
		],
	)
}

struct Component {
	/// `Counter·instances`, `Counter·instance`
	counter: String,
	key: String,
	/// each state variable and its first value, with its list `Counter·count`
	state: Vec<(String, Node, String)>,
	/// `Counter·seen`: the instances of the render before; `Counter·cleanup`, `Counter·cleaned`
	seen: String,
	cleanup: String,
	cleaned: String,
	/// the bodies of `on cleanup {…}`
	cleanups: Vec<Node>,
	/// a component called with `key:`: `Counter·keys`, each instance's key, and the placement's `Counter·fresh`, `Counter·found`
	keys: Option<String>,
	fresh: String,
	found: String,
}

impl Component {
	fn new(head: &Node, state: Vec<(String, Node)>, statements: &[Node], keyed: &[String]) -> Self {
		let Node::List(parts, _, _) = head.drop_meta() else { unreachable!("a function head") };
		let name = parts[0].name();
		let named = |part: &str| format!("{name}{JOINER}{part}");
		let state = state
			.into_iter()
			.map(|(variable, initial)| {
				let list = named(&variable);
				(variable, initial, list)
			})
			.collect();
		let cleanups =
			statements.iter().filter_map(lifecycle).filter(|(word, _)| word == CLEANUP).map(|(_, body)| body).collect();
		let keys = keyed.contains(&name).then(|| named(KEYS));
		Component {
			counter: named(INSTANCES),
			key: named(INSTANCE),
			state,
			seen: named(SEEN),
			cleanup: named(CLEANUP),
			cleaned: named(CLEANED),
			cleanups,
			keys,
			fresh: named(FRESH),
			found: named(FOUND),
		}
	}

	/// The head, with the parameter `key = ø` for a keyed component
	fn head(&self, head: Node) -> Node {
		let Node::List(mut parts, bracket, separator) = head.drop_meta().clone() else {
			return head;
		};
		if self.keys.is_some() && !parts.iter().skip(1).any(|part| part.drop_meta().name() == KEY_WORD) {
			parts.push(crate::warp_parser::parse_generated(KEY_PARAMETER).drop_meta().clone());
		}
		Node::List(parts, bracket, separator)
	}

	/// The lists kept per instance: each state variable's and, keyed, the keys
	fn lists(&self) -> Vec<&String> {
		self.keys.iter().chain(self.state.iter().map(|(_, _, list)| list)).collect()
	}

	/// The statements putting a keyed instance's entries at its position
	fn placement(&self, keys: &str) -> Vec<Node> {
		// placeholders LIST0, LIST1, … (the keys first) and MOVED0, …: a list's name `Counter·count` reads as a product
		let lists = self.lists();
		let placeholder = |word: &str, index: usize| format!("{word}{index}");
		let numbered = |template: &str, index: usize| {
			template.replace("LIST", &placeholder("LIST", index)).replace("MOVED", &placeholder("MOVED", index))
		};
		let stale_moves: String = (1..lists.len()).map(|index| numbered(STALE_MOVE, index)).collect();
		let swaps: String = (0..lists.len()).map(|index| numbered(SWAP, index)).collect();
		let template = KEYED_PLACEMENT.replace("STALE_MOVES", &stale_moves).replace("SWAPS", &swaps);
		let numbered_names: Vec<(String, String)> = lists
			.iter()
			.enumerate()
			.flat_map(|(index, list)| {
				[
					(placeholder("LIST", index), list.to_string()),
					(placeholder("MOVED", index), format!("{}{index}", self.found)),
				]
			})
			.collect();
		let names: Vec<(&str, &str)> = [
			("FRESH", self.fresh.as_str()),
			("SEEN", &self.seen),
			("KEYS", keys),
			("KEY", &self.key),
			("FOUND", &self.found),
		]
		.into_iter()
		.chain(numbered_names.iter().map(|(placeholder, name)| (placeholder.as_str(), name.as_str())))
		.collect();
		match filled(&template, &names) {
			Node::List(statements, _, Separator::Semicolon) => statements,
			statement => vec![statement],
		}
	}

	/// (`Counter`, `count`) → `Counter·count` for each state variable
	fn state_lists(&self) -> Vec<((String, String), String)> {
		let component = self.counter.split(JOINER).next().unwrap_or_default().to_string();
		self.state.iter().map(|(variable, _, list)| ((component.clone(), variable.clone()), list.clone())).collect()
	}

	/// The main-level lists of the state, the instance counter and the instances of the render before
	fn declarations(&self) -> Vec<Node> {
		let template = if crate::pipeline::compile_mode().for_dev { DEV_STATE_DECLARATION } else { STATE_DECLARATION };
		let lists = self.lists().into_iter().map(|list| kept_in_dev(template, list));
		[filled(COUNTER_DECLARATION, &[("COUNTER", &self.counter)]), filled(SEEN_DECLARATION, &[("SEEN", &self.seen)])]
			.into_iter()
			.chain(lists)
			.collect()
	}

	/// `Counter·cleanup(Counter·instance) := { the cleanup bodies }`, reading that instance's state
	fn cleanup_function(&self) -> Option<Node> {
		if self.cleanups.is_empty() {
			return None;
		}
		let head = call(&self.cleanup.clone(), vec![Node::Symbol(self.key.clone())]);
		let body = self.cleanups.iter().cloned().map(|body| self.read(body)).collect();
		Some(key(head, Op::Define, Node::List(body, Bracket::Curly, Separator::Semicolon)))
	}

	/// After a render: the instances it left are cleaned up, their state dropped, its count kept for the next
	fn after_render(&self) -> Vec<Node> {
		let names = [
			("COUNTER", self.counter.as_str()),
			("SEEN", &self.seen),
			("CLEANED", &self.cleaned),
			("CLEANUP", &self.cleanup),
		];
		let cleaning = match self.cleanups.is_empty() {
			true => vec![],
			false => vec![filled(CLEANED_FROM, &names), filled(CLEANUP_LOOP, &names)],
		};
		let truncations =
			self.lists().into_iter().map(|list| filled(TRUNCATED, &[("STATE", list), ("COUNTER", &self.counter)]));
		let all_seen = self.keys.iter().map(|keys| filled(KEYED_SEEN, &[("SEEN", &self.seen), ("KEYS", keys)]));
		let dev_saves = self
			.lists()
			.into_iter()
			.filter(|_| crate::pipeline::compile_mode().for_dev)
			.map(|list| kept_in_dev(DEV_STATE_SAVE, list));
		all_seen.chain(cleaning).chain(truncations).chain(dev_saves).chain([filled(SEEN_SET, &names)]).collect()
	}

	/// The body counting its instance, setting the state at the first render, reading the instance's entries
	fn body(&self, statements: Vec<Node>) -> Node {
		let mut lowered = vec![
			filled(COUNTED, &[("COUNTER", &self.counter)]),
			filled(KEYED, &[("KEY", &self.key), ("COUNTER", &self.counter)]),
		];
		lowered.extend(self.keys.iter().flat_map(|keys| self.placement(keys)));
		let (mount_condition, state_start) = match self.keys {
			Some(_) => (FRESH_PLACEHOLDER, KEYED_FIRST_SET),
			None => (FIRST_RENDER, FIRST_SET),
		};
		for statement in statements {
			if let Some((word, body)) = lifecycle(&statement) {
				if word == MOUNT {
					let condition =
						filled(mount_condition, &[("SEEN", &self.seen), ("KEY", &self.key), ("FRESH", &self.fresh)]);
					lowered.push(if_then(condition, self.read(body)));
				}
				continue;
			}
			let first_set = named_assignment(&statement).and_then(|(name, initial)| {
				self.state.iter().find(|(variable, _, _)| *variable == name).map(|(_, _, list)| (list.clone(), initial))
			});
			lowered.push(match first_set {
				Some((list, initial)) => super::nodes::filled(
					state_start,
					vec![
						("STATE", Node::Symbol(list)),
						("KEY", Node::Symbol(self.key.clone())),
						("FRESH", Node::Symbol(self.fresh.clone())),
						("INITIAL", self.read(initial)),
					],
				),
				None => self.read(statement),
			});
		}
		Node::List(lowered, Bracket::Curly, Separator::Semicolon)
	}

	/// Each state variable as the `read` code reading it from its list (STATE) at the instance (KEY)
	fn state_reads(&self, read: &str) -> HashMap<String, Node> {
		let key = Node::Symbol(self.key.clone());
		let filled =
			|list: &String| super::nodes::filled(read, [("STATE", Node::Symbol(list.clone())), ("KEY", key.clone())]);
		self.state.iter().map(|(variable, _, list)| (variable.clone(), filled(list))).collect()
	}

	/// The node reading the instance's entries: in a handler the event's instance, elsewhere the render's
	fn read(&self, node: Node) -> Node {
		if element_items(&node).is_some() {
			return self.element(node);
		}
		let reads = self.state_reads(READ);
		match node {
			Node::Symbol(ref name) => reads.get(name).cloned().unwrap_or(node),
			other => other.map_children(|child| self.read(child)),
		}
	}

	/// An element: its handlers read the event's instance, the element names its instance
	fn element(&self, node: Node) -> Node {
		let Node::Key(tag, op, content) = node else { unreachable!("an element") };
		let Node::List(items, bracket, separator) = content.drop_meta().clone() else {
			unreachable!("an element's block")
		};
		let has_handler = (0..items.len()).any(|index| handler_at(&items, index).is_some());
		let mut kept = vec![];
		let mut index = 0;
		while index < items.len() {
			match handler_at(&items, index) {
				Some((_, body, length)) => {
					kept.extend(items[index..index + length - 1].iter().cloned());
					let handled = substitute(&body, &self.state_reads(HANDLER_READ));
					// `on click {…}` as a block reads it is `on`, `click: {…}`
					match items[index + length - 1].drop_meta() {
						Node::Key(event, Op::Colon, _) if length == 2 => {
							kept.push(Node::Key(event.clone(), Op::Colon, Box::new(handled)))
						}
						_ => kept.push(handled),
					}
					index += length;
				}
				None => {
					kept.push(self.read(items[index].clone()));
					index += 1;
				}
			}
		}
		if has_handler {
			let attribute = Node::Symbol(format!("{HANDLER_ATTRIBUTE_PREFIX}{INSTANCE_ATTRIBUTE}"));
			kept.insert(0, key(attribute, Op::Colon, Node::Symbol(self.key.clone())));
		}
		Node::Key(tag, op, Box::new(Node::List(kept, bracket, separator)))
	}
}
