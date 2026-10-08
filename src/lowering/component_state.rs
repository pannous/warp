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
//! sheet styles only its own elements (html.rs, card web-scoped).

use crate::element_events::{element_items, handler_at, has_element_handler, HANDLER_ATTRIBUTE_PREFIX};
use crate::law::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;
use std::collections::HashMap;

const JOINER: char = '·';
/// `data-warp-instance`: the attribute naming the instance of a component's element, read by playground.js
const INSTANCE_ATTRIBUTE: &str = "instance";
const INSTANCES: &str = "instances";
const INSTANCE: &str = "instance";
const RENDER: &str = "page·markup";
// the templates' placeholders are upper case: `instance` stays the field of the event
const STATE_DECLARATION: &str = "global STATE = []";
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
const SHOWN: &str = "page·shown";
const ON_WORD: &str = "on";
const MOUNT: &str = "mount";
const CLEANUP: &str = "cleanup";
const SEEN: &str = "seen";
const CLEANED: &str = "cleaned";

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else { return program };
	let statements: Vec<Node> = statements.into_iter().map(|statement| scoped_component(&statement).unwrap_or(statement)).collect();
	if !statements.iter().any(|statement| has_element_handler(statement) || has_lifecycle(statement)) {
		return Node::List(statements, bracket, separator);
	}
	let mut declarations = vec![];
	let mut components = vec![];
	let mut lowered: Vec<Node> = statements.into_iter().map(|statement| match stateful_component(&statement) {
		Some((head, body, state)) => {
			let component = Component::new(&head, state, &body);
			declarations.extend(component.declarations());
			let definition = Node::Key(Box::new(head), Op::Define, Box::new(component.body(body)));
			components.push(component);
			definition
		}
		None => statement,
	}).collect();
	if components.is_empty() {
		return Node::List(lowered, bracket, separator);
	}
	let shown = lowered.pop().expect("a program with a component has a last line");
	let resets = components.iter().map(|component| filled(RESET, &[("COUNTER", &component.counter)]));
	let rendered = crate::variable_signals::assign(SHOWN, shown);
	let after = components.iter().flat_map(Component::after_render);
	let render_body = resets.chain([rendered]).chain(after).chain([Node::Symbol(SHOWN.into())]).collect();
	let render = Node::Key(Box::new(Node::Symbol(RENDER.into())), Op::Define, Box::new(Node::List(render_body, Bracket::Curly, Separator::Semicolon)));
	let cleanups = components.iter().filter_map(Component::cleanup_function);
	Node::List(declarations.into_iter().chain(cleanups).chain(lowered).chain([render, Node::Symbol(RENDER.into())]).collect(), bracket, separator)
}

/// `on mount {…}` or `on cleanup {…}` among a component's statements: the word and the body
fn lifecycle(statement: &Node) -> Option<(String, Node)> {
	let Node::List(words, _, Separator::Space | Separator::None) = statement.drop_meta() else { return None };
	if !matches!(words.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == ON_WORD) {
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
	let mut found = false;
	statement.visit(&mut |part| found |= lifecycle(part).is_some());
	found
}

/// A component's head, its statements and its state: each variable its handlers mention, with its first value
type StatefulComponent = (Node, Vec<Node>, Vec<(String, Node)>);

/// `def F(p) { x = … markup with handlers }`
fn stateful_component(statement: &Node) -> Option<StatefulComponent> {
	if !has_element_handler(statement) && !has_lifecycle(statement) {
		return None;
	}
	let Node::Key(head, Op::Define | Op::Assign, body) = crate::declarations::lower_c_functions(statement.clone()).drop_meta().clone() else { return None };
	let Node::List(parts, Bracket::Round, _) = head.drop_meta() else { return None };
	if !matches!(parts.first().map(Node::drop_meta), Some(Node::Symbol(_))) {
		return None;
	}
	let statements = match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.clone(),
		single => vec![single.clone()],
	};
	// what its handlers and its cleanup read is kept per instance
	let cleanups = statements.iter().filter_map(lifecycle).filter(|(word, _)| word == CLEANUP).flat_map(|(_, body)| crate::variable_signals::symbols(&body));
	let mentioned: Vec<String> = handler_symbols(&statements).into_iter().chain(cleanups).collect();
	let state: Vec<(String, Node)> = statements.iter().filter_map(assignment).filter(|(name, _)| mentioned.contains(name)).collect();
	(!state.is_empty() || statements.iter().any(|statement| lifecycle(statement).is_some())).then_some((*head, statements, state))
}

/// `def Card(t) = div{ style{ "h2": {…} } h2{t} }`: the definition with its root element naming the component
fn scoped_component(statement: &Node) -> Option<Node> {
	let mut has_sheet = false;
	statement.visit(&mut |part| has_sheet |= crate::markup::is_style_sheet(part));
	if !has_sheet {
		return None;
	}
	let Node::Key(head, op @ (Op::Define | Op::Assign), body) = crate::declarations::lower_c_functions(statement.clone()).drop_meta().clone() else { return None };
	let Node::List(parts, Bracket::Round, _) = head.drop_meta() else { return None };
	let name = parts.first()?.name();
	let attribute = Node::Key(Box::new(Node::Symbol(crate::markup::SCOPE_ATTRIBUTE.into())), Op::Colon, Box::new(Node::Text(name)));
	let body = match *body {
		Node::List(mut items, Bracket::Curly, separator) => {
			let root = with_attribute(items.pop()?, attribute)?;
			items.push(root);
			Node::List(items, Bracket::Curly, separator)
		}
		root => with_attribute(root, attribute)?,
	};
	Some(Node::Key(head, op, Box::new(body)))
}

/// The element with the attribute first among its items
fn with_attribute(element: Node, attribute: Node) -> Option<Node> {
	let Node::Key(tag, op, content) = element.drop_meta().clone() else { return None };
	if !crate::markup::is_element_tag(&tag.drop_meta().name()) {
		return None;
	}
	let items = match *content {
		Node::List(items, Bracket::Curly, _) => items,
		single => vec![single],
	};
	Some(Node::Key(tag, op, Box::new(Node::List(std::iter::once(attribute).chain(items).collect(), Bracket::Curly, Separator::Space))))
}

/// `x = value`: the name and the value
fn assignment(statement: &Node) -> Option<(String, Node)> {
	match statement.drop_meta() {
		Node::Key(name, Op::Assign, value) => match name.drop_meta() {
			Node::Symbol(name) => Some((name.clone(), value.as_ref().clone())),
			_ => None,
		},
		_ => None,
	}
}

/// The names the element handlers in the statements mention
fn handler_symbols(statements: &[Node]) -> Vec<String> {
	let mut names = vec![];
	for statement in statements {
		statement.visit(&mut |part| if let Some(items) = element_items(part) {
			names.extend((0..items.len()).filter_map(|index| handler_at(items, index)).flat_map(|(_, body, _)| crate::variable_signals::symbols(&body)));
		});
	}
	names
}

/// A template with its placeholders filled by names
fn filled(template: &str, names: &[(&str, &str)]) -> Node {
	with(template, names.iter().map(|(placeholder, name)| (*placeholder, Node::Symbol(name.to_string()))).collect())
}

fn with(template: &str, bindings: Vec<(&str, Node)>) -> Node {
	let parsed = parse(template);
	let bindings: HashMap<String, Node> = bindings.into_iter().map(|(placeholder, node)| (placeholder.to_string(), node)).collect();
	substitute(parsed.drop_meta(), &bindings)
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
}

impl Component {
	fn new(head: &Node, state: Vec<(String, Node)>, statements: &[Node]) -> Self {
		let Node::List(parts, _, _) = head.drop_meta() else { unreachable!("a function head") };
		let name = parts[0].name();
		let named = |part: &str| format!("{name}{JOINER}{part}");
		let state = state.into_iter().map(|(variable, initial)| { let list = named(&variable); (variable, initial, list) }).collect();
		let cleanups = statements.iter().filter_map(lifecycle).filter(|(word, _)| word == CLEANUP).map(|(_, body)| body).collect();
		Component { counter: named(INSTANCES), key: named(INSTANCE), state, seen: named(SEEN), cleanup: named(CLEANUP), cleaned: named(CLEANED), cleanups }
	}

	/// The main-level lists of the state, the instance counter and the instances of the render before
	fn declarations(&self) -> Vec<Node> {
		let lists = self.state.iter().map(|(_, _, list)| filled(STATE_DECLARATION, &[("STATE", list)]));
		[filled(COUNTER_DECLARATION, &[("COUNTER", &self.counter)]), filled(SEEN_DECLARATION, &[("SEEN", &self.seen)])].into_iter().chain(lists).collect()
	}

	/// `Counter·cleanup(Counter·instance) := { the cleanup bodies }`, reading that instance's state
	fn cleanup_function(&self) -> Option<Node> {
		if self.cleanups.is_empty() {
			return None;
		}
		let head = Node::List(vec![Node::Symbol(self.cleanup.clone()), Node::Symbol(self.key.clone())], Bracket::Round, Separator::None);
		let body = self.cleanups.iter().cloned().map(|body| self.read(body)).collect();
		Some(Node::Key(Box::new(head), Op::Define, Box::new(Node::List(body, Bracket::Curly, Separator::Semicolon))))
	}

	/// After a render: the instances it left are cleaned up, their state dropped, its count kept for the next
	fn after_render(&self) -> Vec<Node> {
		let names = [("COUNTER", self.counter.as_str()), ("SEEN", &self.seen), ("CLEANED", &self.cleaned), ("CLEANUP", &self.cleanup)];
		let cleaning = match self.cleanups.is_empty() {
			true => vec![],
			false => vec![filled(CLEANED_FROM, &names), filled(CLEANUP_LOOP, &names)],
		};
		let truncations = self.state.iter().map(|(_, _, list)| filled(TRUNCATED, &[("STATE", list), ("COUNTER", &self.counter)]));
		cleaning.into_iter().chain(truncations).chain([filled(SEEN_SET, &names)]).collect()
	}

	/// The body counting its instance, setting the state at the first render, reading the instance's entries
	fn body(&self, statements: Vec<Node>) -> Node {
		let mut lowered = vec![filled(COUNTED, &[("COUNTER", &self.counter)]), filled(KEYED, &[("KEY", &self.key), ("COUNTER", &self.counter)])];
		for statement in statements {
			if let Some((word, body)) = lifecycle(&statement) {
				if word == MOUNT {
					let condition = filled(FIRST_RENDER, &[("SEEN", &self.seen), ("KEY", &self.key)]);
					lowered.push(crate::variable_signals::if_then(condition, self.read(body)));
				}
				continue;
			}
			let first_set = assignment(&statement).and_then(|(name, initial)| self.state.iter().find(|(variable, _, _)| *variable == name).map(|(_, _, list)| (list.clone(), initial)));
			lowered.push(match first_set {
				Some((list, initial)) => with(FIRST_SET, vec![("STATE", Node::Symbol(list)), ("KEY", Node::Symbol(self.key.clone())), ("INITIAL", self.read(initial))]),
				None => self.read(statement),
			});
		}
		Node::List(lowered, Bracket::Curly, Separator::Semicolon)
	}

	/// The node reading the instance's entries: in a handler the event's instance, elsewhere the render's
	fn read(&self, node: Node) -> Node {
		if element_items(&node).is_some() {
			return self.element(node);
		}
		let reads: HashMap<String, Node> = self.state.iter().map(|(variable, _, list)| (variable.clone(), with(READ, vec![("STATE", Node::Symbol(list.clone())), ("KEY", Node::Symbol(self.key.clone()))]))).collect();
		match node {
			Node::Symbol(ref name) => reads.get(name).cloned().unwrap_or(node),
			other => other.map_children(|child| self.read(child)),
		}
	}

	/// An element: its handlers read the event's instance, the element names its instance
	fn element(&self, node: Node) -> Node {
		let Node::Key(tag, op, content) = node else { unreachable!("an element") };
		let Node::List(items, bracket, separator) = content.drop_meta().clone() else { unreachable!("an element's block") };
		let has_handler = (0..items.len()).any(|index| handler_at(&items, index).is_some());
		let mut kept = vec![];
		let mut index = 0;
		while index < items.len() {
			match handler_at(&items, index) {
				Some((_, body, length)) => {
					kept.extend(items[index..index + length - 1].iter().cloned());
					let handled = substitute(&body, &self.state.iter().map(|(variable, _, list)| (variable.clone(), with(HANDLER_READ, vec![("STATE", Node::Symbol(list.clone()))]))).collect());
					// `on click {…}` as a block reads it is `on`, `click: {…}`
					match items[index + length - 1].drop_meta() {
						Node::Key(event, Op::Colon, _) if length == 2 => kept.push(Node::Key(event.clone(), Op::Colon, Box::new(handled))),
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
			kept.insert(0, Node::Key(Box::new(attribute), Op::Colon, Box::new(Node::Symbol(self.key.clone()))));
		}
		Node::Key(tag, op, Box::new(Node::List(kept, bracket, separator)))
	}
}
