//! Reflection words as dot forms (card reflection, notes/reflection.md): `f.effects` is `effects of f` (effects.rs),
//! `e.listeners` is `listeners of e` (event_signals.rs, signal_values.rs), so each word has one implementation.
//! A real field comes first: only a function's `.effects` and a listened name's `.listeners` are rewritten, a map's
//! own field `listeners` stays its field. `listeners of x` for a name nothing listens to is loud (card listeners-tick).
//! Objects known at compile time (step 2): an instance's `p.class` and `p.type` are `type(p)`; `p.fields` (aliases
//! `attributes`, `members`), `p.methods` and `dir(p)` of an instance or a class are the names of its class layout,
//! inherited fields first; of a map literal's variable they are its keys, read at run time (`m.keys`).

use std::collections::{HashMap, HashSet};

use crate::class_methods::ClassLayout;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const OF_WORD: &str = "of";
const EFFECTS_WORD: &str = "effects";
const LISTENERS_WORD: &str = "listeners";
/// `on alarm {…}`, `once tick {…}`, `on set x {…}`: the words before a listened name
const LISTENING_WORDS: [&str; 2] = ["on", "once"];
const VARIABLE_EVENTS: [&str; 2] = ["set", "change"];
const CLASS_WORDS: [&str; 2] = ["class", crate::type_tests::TYPE_WORD];
const FIELDS_WORDS: [&str; 3] = ["fields", "attributes", "members"];
const METHODS_WORD: &str = "methods";
const DIR_WORD: &str = "dir";
const KEYS_WORD: &str = "keys";
const PARAMS_WORDS: [&str; 2] = ["params", "parameters"];
const SIGNATURE_WORD: &str = "signature";

type Functions = std::collections::BTreeMap<String, crate::context::UserFunctionDef>;

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[EFFECTS_WORD, LISTENERS_WORD, SIGNATURE_WORD, PARAMS_WORDS[0], PARAMS_WORDS[1]]) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let mut listened = HashSet::new();
	node.visit(&mut |child| if let Some(name) = listened_name(child) {
		listened.insert(name);
	});
	if let Some(name) = unlistened_reflection(&node, &listened, &defined) {
		return crate::node::error(&format!("nothing listens to {name}: `listeners of {name}` needs an `on {name} {{…}}` handler or a variable {name}"));
	}
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, &node);
	as_reflection_words(node, &context.user_functions, &listened, &defined)
}

/// The name an `on` or `once` statement listens to
fn listened_name(node: &Node) -> Option<String> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	// `h = on alarm {…}` arrives grouped `on (alarm {…})`
	let words: Vec<&str> = items.iter().flat_map(|item| match item.drop_meta() {
		Node::List(parts, Bracket::None, _) => parts.iter().collect(),
		_ => vec![item],
	}).map(|item| symbol(item).unwrap_or("")).collect();
	match words.as_slice() {
		[listening, event, variable, ..] if LISTENING_WORDS.contains(listening) && VARIABLE_EVENTS.contains(event) && !variable.is_empty() => Some(variable.to_string()),
		[listening, event, ..] if LISTENING_WORDS.contains(listening) && !event.is_empty() => Some(event.to_string()),
		_ => None,
	}
}

fn symbol(node: &Node) -> Option<&str> {
	match node.drop_meta() {
		Node::Symbol(word) => Some(word),
		_ => None,
	}
}

/// A main-level `listeners of x` whose x is neither listened to nor a variable (a function's parameter may be)
fn unlistened_reflection(node: &Node, listened: &HashSet<String>, defined: &HashSet<String>) -> Option<String> {
	match node.drop_meta() {
		Node::Key(_, Op::Define, _) => None,
		Node::List(items, _, _) => items.windows(3).find_map(|window| match [symbol(&window[0]), symbol(&window[1]), symbol(&window[2])] {
			[Some(LISTENERS_WORD), Some(OF_WORD), Some(name)] if !listened.contains(name) && !defined.contains(name) => Some(name.to_string()),
			_ => None,
		}).or_else(|| items.iter().find_map(|item| unlistened_reflection(item, listened, defined))),
		Node::Key(left, _, right) => unlistened_reflection(left, listened, defined).or_else(|| unlistened_reflection(right, listened, defined)),
		_ => None,
	}
}

/// `f.effects` → `effects of f` for a function f, `e.listeners` → `listeners of e` for a listened e; a function's
/// `f.params` and `f.signature` are constants read off its definition
fn as_reflection_words(node: Node, functions: &Functions, listened: &HashSet<String>, defined: &HashSet<String>) -> Node {
	if let Node::Key(subject, Op::Dot, word) = node.drop_meta() {
		if let (Some(name), Some(word)) = (symbol(subject), symbol(word)) {
			let function = functions.get(name).filter(|_| !defined.contains(word));
			let reflected = match (word, function) {
				(EFFECTS_WORD, Some(_)) => Some(of_phrase(word, name)),
				(LISTENERS_WORD, _) if listened.contains(name) => Some(of_phrase(word, name)),
				(word, Some(function)) if PARAMS_WORDS.contains(&word) => Some(text_list(&function.params.iter().map(|param| param.name.clone()).collect::<Vec<_>>())),
				(SIGNATURE_WORD, Some(function)) => Some(Node::Text(signature(function))),
				_ => None,
			};
			if let Some(reflected) = reflected {
				return reflected;
			}
		}
	}
	node.map_children(|child| as_reflection_words(child, functions, listened, defined))
}

/// `effects of f`
fn of_phrase(word: &str, name: &str) -> Node {
	Node::List(vec![Node::Symbol(word.into()), Node::Symbol(OF_WORD.into()), Node::Symbol(name.into())], Bracket::None, Separator::Space)
}

/// `(a:int, b:int) -> int`: each parameter with its declared or demanded type, the result's kind when known
fn signature(function: &crate::context::UserFunctionDef) -> String {
	let params: Vec<String> = function.params.iter().map(|param| {
		let type_name = param.annotation.as_ref().map(|annotation| annotation.drop_meta().name()).or_else(|| param.used_as.map(|kind| kind.to_string()));
		type_name.map_or_else(|| param.name.clone(), |type_name| format!("{}:{type_name}", param.name))
	}).collect();
	let result = match function.return_kind {
		crate::type_kinds::Kind::Empty => String::new(),
		kind => format!(" -> {kind}"),
	};
	format!("({}){result}", params.join(", "))
}

/// What the compile-time object words know: each class's layout, the classes of instance variables, the keys of map
/// literals' variables
struct Objects {
	layouts: HashMap<String, ClassLayout>,
	instances: HashMap<String, String>,
	maps: HashMap<String, Vec<String>>,
	defined: HashSet<String>,
}

pub fn lower_objects(node: Node) -> Node {
	let words: Vec<&str> = CLASS_WORDS.iter().chain(&FIELDS_WORDS).chain(&[METHODS_WORD, DIR_WORD]).copied().collect();
	if !node.mentions_any(&words) {
		return node;
	}
	let layouts = crate::class_methods::class_layouts(&node);
	let classes: Vec<String> = layouts.keys().cloned().collect();
	let objects = Objects {
		instances: crate::class_methods::instance_classes(&node, &classes),
		maps: map_keys(&node),
		defined: crate::library_words::defined_names(&node),
		layouts,
	};
	with_object_words(node, &objects)
}

/// `m = {a:1, b:2}`: m's keys as written
fn map_keys(node: &Node) -> HashMap<String, Vec<String>> {
	let mut maps = HashMap::new();
	node.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define, value) = part {
		if let (Some(name), Node::List(items, Bracket::Curly, _)) = (symbol(target), value.drop_meta()) {
			let keys: Option<Vec<String>> = items.iter().map(|item| match item.drop_meta() {
				Node::Key(key, Op::Colon, _) => symbol(key).map(String::from),
				_ => None,
			}).collect();
			if let Some(keys) = keys.filter(|keys| !keys.is_empty()) {
				maps.insert(name.to_string(), keys);
			}
		}
	});
	maps
}

impl Objects {
	/// The class an instance variable holds, or the class a name is
	fn class_of<'a>(&'a self, name: &'a str) -> Option<&'a str> {
		self.instances.get(name).map(String::as_str).or_else(|| self.layouts.contains_key(name).then_some(name))
	}

	/// Inherited names first, a name the class redeclares kept in its parent's place
	fn names(&self, class: &str, own: fn(&ClassLayout) -> &Vec<String>) -> Vec<String> {
		let Some(layout) = self.layouts.get(class) else { return vec![] };
		let mut names = layout.parent.as_deref().map(|parent| self.names(parent, own)).unwrap_or_default();
		names.extend(own(layout).iter().filter(|name| !names.contains(name)).cloned().collect::<Vec<_>>());
		names
	}

	fn fields(&self, class: &str) -> Vec<String> {
		self.names(class, |layout| &layout.fields)
	}

	fn methods(&self, class: &str) -> Vec<String> {
		self.names(class, |layout| &layout.methods)
	}

	/// `subject.word` as a reflection word, unless subject has a real field or key of that name
	fn dot_word(&self, subject: &str, word: &str) -> Option<Node> {
		if self.defined.contains(word) {
			return None;
		}
		if let Some(class) = self.class_of(subject) {
			let is_instance = self.instances.contains_key(subject);
			return match word {
				_ if self.fields(class).iter().chain(&self.methods(class)).any(|name| name == word) => None,
				_ if CLASS_WORDS.contains(&word) && is_instance => Some(Node::List(vec![Node::Symbol(crate::type_tests::TYPE_WORD.into()), Node::Symbol(subject.into())], Bracket::Round, Separator::None)),
				_ if FIELDS_WORDS.contains(&word) => Some(text_list(&self.fields(class))),
				METHODS_WORD => Some(text_list(&self.methods(class))),
				_ => None,
			};
		}
		let keys = self.maps.get(subject)?;
		(FIELDS_WORDS.contains(&word) && !keys.iter().any(|key| key == word)).then(|| self.map_keys(subject))
	}

	/// `dir(x)`: the fields and methods of an instance or class, the keys of a map
	fn dir(&self, subject: &str) -> Option<Node> {
		if self.defined.contains(DIR_WORD) {
			return None;
		}
		match self.class_of(subject) {
			Some(class) => Some(text_list(&[self.fields(class), self.methods(class)].concat())),
			None => self.maps.contains_key(subject).then(|| self.map_keys(subject)),
		}
	}

	fn map_keys(&self, subject: &str) -> Node {
		Node::Key(Box::new(Node::Symbol(subject.into())), Op::Dot, Box::new(Node::Symbol(KEYS_WORD.into())))
	}
}

fn with_object_words(node: Node, objects: &Objects) -> Node {
	let reflected = match node.drop_meta() {
		Node::Key(subject, Op::Dot, word) => symbol(subject).zip(symbol(word)).and_then(|(subject, word)| objects.dot_word(subject, word)),
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 && symbol(&items[0]) == Some(DIR_WORD) => symbol(&items[1]).and_then(|subject| objects.dir(subject)),
		_ => None,
	};
	reflected.unwrap_or_else(|| node.map_children(|child| with_object_words(child, objects)))
}

/// Names as a list of texts, as `dir(time)` gives them
pub(crate) fn text_list(names: &[String]) -> Node {
	Node::List(names.iter().map(|name| Node::Text(name.clone())).collect(), Bracket::Square, Separator::None)
}
