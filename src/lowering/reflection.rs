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
/// A class's definition as written, in its warp.meta entry: a program importing the module declares the class too
pub const DEFINITION_WORD: &str = "definition";
const DIR_WORD: &str = "dir";
const KEYS_WORD: &str = "keys";
const EXPORTS_WORD: &str = "exports";
/// The run-time choice of a class's names (Objects::dispatched)
const DISPATCH_TEMPLATE: &str = "if RECEIVER is CLASS then NAMES else OTHERWISE";
pub(crate) const PARAMS_WORDS: [&str; 2] = ["params", "parameters"];
const SIGNATURE_WORD: &str = "signature";
/// `f.body`: the body as written, as data (card g_X_3s)
const BODY_WORD: &str = "body";
/// The entries of the module's warp.meta section that name its functions and classes
pub const META_FUNCTIONS: &str = "functions";
pub const META_CLASSES: &str = "classes";

type Functions = std::collections::BTreeMap<String, crate::context::UserFunctionDef>;

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[EFFECTS_WORD, LISTENERS_WORD, SIGNATURE_WORD, BODY_WORD, PARAMS_WORDS[0], PARAMS_WORDS[1]]) {
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
	as_reflection_words(node.clone(), &user_functions(&node), &listened, &defined)
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
/// `f.params`, `f.signature` and `f.body` are constants read off its definition
fn as_reflection_words(node: Node, functions: &Functions, listened: &HashSet<String>, defined: &HashSet<String>) -> Node {
	if let Node::Key(subject, Op::Dot, word) = node.drop_meta() {
		if let (Some(name), Some(word)) = (symbol(subject), symbol(word)) {
			let function = functions.get(name).filter(|_| !defined.contains(word));
			let reflected = match (word, function) {
				(EFFECTS_WORD, Some(_)) => Some(of_phrase(word, name)),
				(LISTENERS_WORD, _) if listened.contains(name) => Some(of_phrase(word, name)),
				(word, Some(function)) if PARAMS_WORDS.contains(&word) => Some(text_list(&function.params.iter().map(|param| param.name.clone()).collect::<Vec<_>>())),
				(SIGNATURE_WORD, Some(function)) => Some(Node::Text(signature(function))),
				(BODY_WORD, Some(function)) => Some(Node::List(vec![Node::Symbol(crate::blocks::DATA_WORD.into()), function.body.as_ref().clone()], Bracket::None, Separator::Space)),
				_ => None,
			};
			if let Some(reflected) = reflected {
				return reflected;
			}
		}
	}
	node.map_children(|child| as_reflection_words(child, functions, listened, defined))
}

/// The warp.meta entries of a program (card reflection-foreign-meta, meta_section.rs): `functions` {f: {params: ["a"]
/// signature: "(a:int) -> int"}}, read off the lowered program (every definition form is one by then, the kinds are
/// inferred), and `classes` {P: {fields: [x] methods: [sum] definition: "class P{…}"}} off the source, inherited
/// fields first; absent when the program defines none. Names are symbols: a one-letter text reads back as a character
pub fn meta_entries(source: &Node, lowered: &Node, code: &str) -> Vec<(&'static str, Node)> {
	let objects = Objects { layouts: crate::class_methods::class_layouts(source), ..Objects::default() };
	let mut classes: Vec<&String> = objects.layouts.keys().collect();
	classes.sort();
	let is_method = |name: &String| objects.layouts.values().any(|layout| layout.methods.contains(name));
	// the program's own functions, not the library's its lowering brought in (html_render, prelude·replace)
	let own = user_functions(source);
	let functions: Vec<(String, Node)> = user_functions(lowered).values().filter(|function| own.contains_key(&function.name) && !is_method(&function.name)).map(|function| {
		let params = function.params.iter().map(|param| param.name.clone()).collect::<Vec<_>>();
		(function.name.clone(), meta_map(vec![(PARAMS_WORDS[0], text_list(&params)), (SIGNATURE_WORD, Node::Text(signature(function)))]))
	}).collect();
	let classes: Vec<(String, Node)> = classes.into_iter().map(|class| {
		(class.clone(), meta_map(vec![(FIELDS_WORDS[0], text_list(&objects.fields(class))), (METHODS_WORD, text_list(&objects.methods(class))),
			(DEFINITION_WORD, class_definition(code, class).unwrap_or(Node::Empty))]))
	}).collect();
	[(META_FUNCTIONS, functions), (META_CLASSES, classes)].into_iter()
		.filter(|(_, entries)| !entries.is_empty())
		.map(|(entry, entries)| (entry, meta_map(entries.iter().map(|(name, value)| (name.as_str(), value.clone())).collect())))
		.collect()
}

/// A class's definition as written in the code: from its keyword (`class P`, `struct P`) to its balanced closing brace
/// (a parsed class does not serialize back to its source)
fn class_definition(code: &str, class: &str) -> Option<Node> {
	let keywords = crate::warp_parser::TYPE_DECLARATION_WORDS.iter().chain(CLASS_WORDS.iter());
	let starts = keywords.flat_map(|keyword| code.match_indices(&format!("{keyword} {class}")).map(|(at, written)| (at, written.len())).collect::<Vec<_>>());
	let (start, _) = starts.filter(|(at, length)| !code[at + length..].starts_with(|next: char| next.is_alphanumeric() || next == '_')).min()?;
	balanced_end(&code[start..]).map(|end| Node::Text(code[start..start + end].to_string()))
}

/// The length of the text up to the brace closing its first `{`, quoted text skipped
fn balanced_end(text: &str) -> Option<usize> {
	let (mut depth, mut quoted, mut escaped) = (0, false, false);
	for (index, character) in text.char_indices() {
		match character {
			_ if escaped => escaped = false,
			'\\' if quoted => escaped = true,
			'"' => quoted = !quoted,
			'{' if !quoted => depth += 1,
			'}' if !quoted => {
				depth -= 1;
				if depth == 0 {
					return Some(index + 1);
				}
			}
			_ => {}
		}
	}
	None
}

fn user_functions(program: &Node) -> Functions {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, program);
	context.user_functions
}

/// `{key: value …}`
fn meta_map(entries: Vec<(&str, Node)>) -> Node {
	Node::List(entries.into_iter().map(|(key, value)| Node::Key(Box::new(Node::Symbol(key.into())), Op::Colon, Box::new(value))).collect(), Bracket::Curly, Separator::Space)
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
/// literals' variables, the exports of the imported core modules by their alias
#[derive(Default)]
struct Objects {
	layouts: HashMap<String, ClassLayout>,
	instances: HashMap<String, String>,
	maps: HashMap<String, Vec<String>>,
	modules: HashMap<String, Vec<String>>,
	/// the `functions` and `classes` entries of an imported module's warp.meta section, by its alias
	module_functions: HashMap<String, Node>,
	module_classes: HashMap<String, Node>,
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
		..Objects::default()
	};
	with_object_words(node, &objects)
}

/// `m.exports`, `dir(m)` of an imported core module m, once modules::resolve has found its file; of a module compiled
/// from warp also `m.f.params`, `m.f.signature` of its functions and `m.P.fields`, `m.P.methods`, `dir(m.P)` of its classes
pub fn lower_module_words(node: Node) -> Node {
	let words: Vec<&str> = PARAMS_WORDS.iter().chain(&FIELDS_WORDS).chain(&[EXPORTS_WORD, DIR_WORD, SIGNATURE_WORD, METHODS_WORD]).copied().collect();
	if !node.mentions_any(&words) {
		return node;
	}
	let objects = module_objects(&node);
	if objects.modules.is_empty() {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	with_object_words(node, &Objects { defined, ..objects })
}

/// `calc.exports`, `dir(calc)` of a component (`use wasm "calc.wasm" as calc`, card reflection-components): its export
/// names, read off the component natively before foreign_modules makes `calc.exports` a call into it; the page reads
/// components only at run time (components.js)
pub fn lower_component_words(node: Node) -> Node {
	if !node.mentions_any(&[EXPORTS_WORD, DIR_WORD]) {
		return node;
	}
	let modules = component_exports(&node);
	if modules.is_empty() {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	with_object_words(node, &Objects { modules, defined, ..Objects::default() })
}

#[cfg(feature = "native")]
fn component_exports(node: &Node) -> HashMap<String, Vec<String>> {
	crate::foreign_modules::component_modules(node).into_iter().filter_map(|(alias, path)| match crate::components::export_names(&path) {
		Ok(names) => Some((alias, names)),
		Err(failure) => {
			eprintln!("[wasm] {failure}");
			None
		}
	}).collect()
}

#[cfg(not(feature = "native"))]
fn component_exports(_node: &Node) -> HashMap<String, Vec<String>> {
	HashMap::new()
}

/// `import lib/fourty_two`: the names fourty_two exports (wasm_modules.rs), sorted; not the setters derived for its globals.
/// A module warp compiled names its own functions in warp.meta: its exports are those (not warp's runtime exports), and
/// that entry is kept by the module's alias for `m.f.params`, its `classes` entry for `m.P.fields`
fn module_objects(node: &Node) -> Objects {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_ffi_imports(&mut context, node);
	let paths: HashSet<&str> = context.ffi_imports.values().map(|import| import.library).filter(|library| crate::wasm_modules::is_module_path(library)).collect();
	let mut objects = Objects::default();
	for path in paths {
		let alias = crate::wasm_modules::module_alias(path);
		let bytes = crate::wasm_modules::module_bytes(path).ok();
		let entry = |key: &str| bytes.as_ref().and_then(|bytes| crate::meta_section::entry(bytes, key));
		let functions = entry(META_FUNCTIONS);
		let is_own = |name: &String| functions.as_ref().is_none_or(|functions| !matches!(functions[name.as_str()].drop_meta(), Node::Empty));
		let mut names: Vec<String> = crate::wasm_modules::exports(path).keys().filter(|name| !name.contains(' ') && is_own(name)).cloned().collect();
		names.sort();
		objects.module_classes.extend(entry(META_CLASSES).map(|classes| (alias.clone(), classes)));
		objects.module_functions.extend(functions.map(|functions| (alias.clone(), functions)));
		objects.modules.insert(alias, names);
	}
	objects
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
	/// `m.f.params`, `m.f.signature` of an imported module's function, `m.P.fields`, `m.P.methods`, `dir(m.P)` of its
	/// class, from the module's warp.meta section; modules.rs has made `m.f` the qualified name of the import
	/// (wasm_modules::qualified)
	fn module_member_word(&self, subject: &Node, word: &str) -> Option<Node> {
		let (module, member) = match subject.drop_meta() {
			Node::Key(module, Op::Dot, member) => (symbol(module)?, symbol(member)?),
			qualified => symbol(qualified)?.split_once('.')?,
		};
		let class = self.module_classes.get(module).map(|classes| &classes[member]).filter(|class| !matches!(class.drop_meta(), Node::Empty));
		if let Some(class) = class {
			return listed_names(entry_names(&class[FIELDS_WORDS[0]]), entry_names(&class[METHODS_WORD]), word).map(|names| text_list(&names));
		}
		let word = if PARAMS_WORDS.contains(&word) { PARAMS_WORDS[0] } else { word };
		match self.module_functions.get(module)?[member][word].drop_meta() {
			Node::Empty => None,
			names if word == PARAMS_WORDS[0] => Some(text_list(&entry_names(names))),
			value => Some(value.clone()),
		}
	}

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
	fn dot_word(&self, subject: &Node, word: &str) -> Option<Node> {
		if self.defined.contains(word) {
			return None;
		}
		let written = || Node::Key(Box::new(subject.clone()), Op::Dot, Box::new(Node::Symbol(word.into())));
		if let Some(reflected) = self.module_member_word(subject, word) {
			return Some(reflected);
		}
		let Some(name) = symbol(subject) else { return self.dispatched(subject, word, written()) };
		if let Some(class) = self.class_of(name) {
			let is_instance = self.instances.contains_key(name);
			return match word {
				_ if self.has_member(class, word) => None,
				_ if CLASS_WORDS.contains(&word) && is_instance => Some(Node::List(vec![Node::Symbol(crate::type_tests::TYPE_WORD.into()), subject.clone()], Bracket::Round, Separator::None)),
				_ => self.listed(class, word).map(|names| text_list(&names)),
			};
		}
		if let Some(exports) = self.modules.get(name) {
			return (word == EXPORTS_WORD).then(|| text_list(exports));
		}
		match self.maps.get(name) {
			Some(keys) => (FIELDS_WORDS.contains(&word) && !keys.iter().any(|key| key == word)).then(|| self.map_keys(subject)),
			None => self.dispatched(subject, word, written()),
		}
	}

	/// `dir(x)`: the fields and methods of an instance or class, a module's exports, the keys of a map
	fn dir(&self, subject: &Node) -> Option<Node> {
		if self.defined.contains(DIR_WORD) {
			return None;
		}
		if let Some(reflected) = self.module_member_word(subject, DIR_WORD) {
			return Some(reflected);
		}
		// no class's instance at run time: a map's keys
		let Some(name) = symbol(subject) else { return self.dispatched(subject, DIR_WORD, self.map_keys(subject)) };
		match (self.class_of(name), self.modules.get(name)) {
			(Some(class), _) => self.listed(class, DIR_WORD).map(|names| text_list(&names)),
			(None, Some(exports)) => Some(text_list(exports)),
			(None, None) if self.maps.contains_key(name) => Some(self.map_keys(subject)),
			(None, None) => self.dispatched(subject, DIR_WORD, self.map_keys(subject)),
		}
	}

	/// The names a reflection word lists for a class: its fields, its methods, or both for `dir`
	fn listed(&self, class: &str, word: &str) -> Option<Vec<String>> {
		listed_names(self.fields(class), self.methods(class), word)
	}

	fn has_member(&self, class: &str, word: &str) -> bool {
		self.fields(class).iter().chain(&self.methods(class)).any(|name| name == word)
	}

	/// A subject whose class is known only at run time (`f(o) := o.fields`): a type test over the program's classes
	/// picks the names, a class's own member of that name is read as written, anything else stays `written`
	fn dispatched(&self, subject: &Node, word: &str, written: Node) -> Option<Node> {
		// a class without a layout lists nothing: whether the word lists names at all
		if self.layouts.is_empty() || self.listed("", word).is_none() {
			return None;
		}
		let mut classes: Vec<&String> = self.layouts.keys().collect();
		classes.sort();
		Some(classes.into_iter().rev().fold(written.clone(), |otherwise, class| {
			let names = match self.has_member(class, word) {
				true => written.clone(),
				false => text_list(&self.listed(class, word).unwrap_or_default()),
			};
			let bindings = [("RECEIVER", subject.clone()), ("CLASS", Node::Symbol(class.clone())), ("NAMES", names), ("OTHERWISE", otherwise)];
			let bindings = bindings.into_iter().map(|(placeholder, node)| (placeholder.to_string(), node)).collect();
			crate::law::substitute(&crate::warp_parser::parse(DISPATCH_TEMPLATE), &bindings).drop_meta().clone()
		}))
	}

	fn map_keys(&self, subject: &Node) -> Node {
		Node::Key(Box::new(subject.clone()), Op::Dot, Box::new(Node::Symbol(KEYS_WORD.into())))
	}
}

fn with_object_words(node: Node, objects: &Objects) -> Node {
	let reflected = match node.drop_meta() {
		Node::Key(subject, Op::Dot, word) => symbol(word).and_then(|word| objects.dot_word(subject, word)),
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 && symbol(&items[0]) == Some(DIR_WORD) => objects.dir(&items[1]),
		_ => None,
	};
	reflected.unwrap_or_else(|| node.map_children(|child| with_object_words(child, objects)))
}

/// Names as a list of texts, as `dir(time)` gives them
/// The names a reflection word lists for a class of these fields and methods: either, or both for `dir`
fn listed_names(fields: Vec<String>, methods: Vec<String>, word: &str) -> Option<Vec<String>> {
	match word {
		DIR_WORD => Some([fields, methods].concat()),
		METHODS_WORD => Some(methods),
		_ if FIELDS_WORDS.contains(&word) => Some(fields),
		_ => None,
	}
}

/// Names as symbols, `[x y]`: a one-letter text would read back as a character
/// The names of a warp.meta list, `["x" "y"]`: a one-letter text reads back as a character
pub(crate) fn entry_names(list: &Node) -> Vec<String> {
	match list.drop_meta() {
		Node::List(items, _, _) => items.iter().map(|item| match item.drop_meta() {
			Node::Char(letter) => letter.to_string(),
			other => other.name(),
		}).collect(),
		_ => vec![],
	}
}

pub(crate) fn text_list(names: &[String]) -> Node {
	Node::List(names.iter().map(|name| Node::Text(name.clone())).collect(), Bracket::Square, Separator::None)
}
