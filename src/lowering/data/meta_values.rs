//! `meta(x)`, the one core reflection builtin (decision reflection-plain): what the compiler knows of a module or a
//! class, as a map. `meta(math)` is `{name: "math", words: […], doc: "…"}`, `meta(P)` of a class
//! `{name: "P", fields: […], methods: […], parent: …}`, its methods the functions themselves. The library builds the
//! typed views on it (`dir`, `fields`, `help`); only it reads the map. A value known only at run time (`f(m) := meta(m)`, `meta(type(o))`, the library's
//! `meta(m)`) is `meta_of(m)`, which the program defines (with_table): a lookup by the value's text (`math`, `class P`)
//! in a table of the modules the program names and its classes. A program's own `meta` wins

use super::nodes::{filled, key};
use crate::class_methods::ClassLayout;
use crate::node::{text, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

const META_WORD: &str = "meta";
/// The program's run-time meta lookup, `meta_of(x) := TABLE[string(x)]`
pub(crate) const META_OF_WORD: &str = "meta_of";
/// `dir(math)`, `methods(math)`, `math.methods` of a module named as written: views of its meta, answered here before
/// any pass reads math as a value (Python's dir, cards dir-introspection, module-methods)
const METHODS_WORD: &str = "methods";
const ASKED_WORDS: [&str; 3] = [META_WORD, crate::words::DIR_WORD, METHODS_WORD];
/// The words that ask meta_of at run time: `meta(x)` as `lower` left it, `dir(x)` through reflection.rs dir_otherwise
const META_ASKING_WORDS: [&str; 2] = [META_OF_WORD, crate::words::DIR_WORD];
/// A class value's text: `string(type(p))` is `class P`
const CLASS_PREFIX: &str = "class ";
/// Not yet `if TABLE has … else error(…)`: a field of that if-expression fails (card field-parenthesized)
const LOOKUP_DEFINITION: &str = "meta_of(subject): map := TABLE[string(subject)]";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&ASKED_WORDS) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let words = ASKED_WORDS.into_iter().filter(|word| !defined.contains(*word)).collect();
	Known::of(&node, words).answered(node)
}

/// The program's `meta_of`, when it or a library word it uses asks meta at run time (program only: a module's own
/// table would lack the program's classes)
pub fn with_table(node: Node) -> Node {
	let defined = crate::library_words::defined_names(&node);
	if !node.mentions_any(&META_ASKING_WORDS) || defined.contains(META_OF_WORD) {
		return node;
	}
	let table = Known::of(&node, vec![]).table();
	super::nodes::with_statements_first(node, vec![filled(LOOKUP_DEFINITION, [("TABLE", table)])])
}

/// The modules and classes a program can ask meta about
struct Known {
	layouts: HashMap<String, ClassLayout>,
	modules: Vec<&'static str>,
	/// the asked words the program doesn't define itself
	words: Vec<&'static str>,
}

impl Known {
	fn of(program: &Node, words: Vec<&'static str>) -> Known {
		let defined = crate::library_words::defined_names(program);
		let modules = crate::modules::std_module_names()
			.filter(|module| !defined.contains(*module))
			.filter(|module| program.mentions(module))
			.collect();
		Known { layouts: crate::class_methods::class_layouts(program), modules, words }
	}

	fn answered(&self, node: Node) -> Node {
		let answer = match asked(&node) {
			Some((word, subject)) if self.words.contains(&word) => self.view(word, subject),
			_ => None,
		};
		answer.unwrap_or_else(|| node.map_children(|child| self.answered(child)))
	}

	fn view(&self, word: &str, subject: &Node) -> Option<Node> {
		let module = || subject.symbol_name().filter(|name| self.modules.contains(name));
		match word {
			META_WORD => {
				let subject = self.answered(subject.clone());
				Some(self.meta(&subject).unwrap_or_else(|| super::nodes::call(META_OF_WORD, vec![subject])))
			}
			METHODS_WORD => module().map(|module| crate::reflection::function_list(&module_words(module))),
			_ => module().map(|module| crate::reflection::text_list(&module_words(module))),
		}
	}

	/// The map of a module or class named as written
	fn meta(&self, subject: &Node) -> Option<Node> {
		let name = subject.symbol_name()?;
		match self.layouts.contains_key(name) {
			true => Some(self.class_meta(name)),
			false => self.modules.contains(&name).then(|| module_meta(name)),
		}
	}

	/// A class's fields as texts, its methods as the functions themselves (decision reflection)
	fn class_meta(&self, class: &str) -> Node {
		let names =
			|own: fn(&ClassLayout) -> &Vec<String>| crate::reflection::inherited_names(&self.layouts, class, own);
		let parent = self.layouts[class].parent.as_deref().map_or(Node::Empty, text);
		map(vec![
			("name", text(class)),
			("fields", crate::reflection::text_list(&names(|layout| &layout.fields))),
			("methods", crate::reflection::word_list(METHODS_WORD, class, &names(|layout| &layout.methods))),
			("parent", parent),
		])
	}

	/// Each module's and class's map by its text
	fn table(&self) -> Node {
		let classes = self.layouts.keys().map(|class| (format!("{CLASS_PREFIX}{class}"), self.class_meta(class)));
		let modules = self.modules.iter().map(|module| (module.to_string(), module_meta(module)));
		let entries = classes.chain(modules).map(|(name, meta)| key(text(&name), Op::Colon, meta)).collect();
		Node::List(entries, Bracket::Curly, Separator::Space)
	}
}

/// `meta(x)`, `dir(x)`, `dir x`, `methods(x)`, `x.methods`: the word and x
fn asked(node: &Node) -> Option<(&str, &Node)> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 => {
			let word = items[0].drop_meta().symbol_name()?;
			ASKED_WORDS.into_iter().find(|asked| *asked == word).map(|word| (word, &items[1]))
		}
		Node::Key(subject, Op::Dot, word) if word.symbol_name() == Some(METHODS_WORD) => Some((METHODS_WORD, subject)),
		_ => None,
	}
}

/// A module's names in the order it defines them
fn module_words(module: &str) -> Vec<String> {
	let names = crate::modules::std_module_definitions(module).unwrap_or_default();
	super::members::type_dir_names(module, names)
}

fn module_meta(module: &str) -> Node {
	let words = module_words(module);
	let doc = crate::modules::std_docs::module_summary(module).unwrap_or_default();
	map(vec![("name", text(module)), ("words", crate::reflection::text_list(&words)), ("doc", text(&doc))])
}

fn map(entries: Vec<(&str, Node)>) -> Node {
	let entries = entries.into_iter().map(|(name, value)| key(Node::Symbol(name.into()), Op::Colon, value)).collect();
	Node::List(entries, Bracket::Curly, Separator::Space)
}
