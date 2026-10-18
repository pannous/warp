//! `help(sound)`, `help(midi)`, `function(name).dump` of the standard modules' words (cards help-undefined,
//! joinable-error, samples/sound/sound_features.warp). A module's help is its index (`warp help sound`, std_docs.rs), a word's
//! its head with the comment above it, a word's dump its definitions as written. A name known only at run time
//! (`for trick in dir(sound): print help(trick)`) is looked up in a table of the words the program knows: those of the
//! modules it uses and its own functions. `function trick` is the function named trick (P82), so a trick that is no
//! function but a variable holding a name is read as that name. A program's own `help` wins. `help(x)` gives the
//! text, a statement prints nothing: `print help(x)` (decision reflection-plain)

use std::collections::{BTreeMap, HashSet};

use super::nodes::{filled, key, word_applied};
use crate::node::{text, Bracket, Node, Separator};
use crate::operators::Op;

const HELP_WORD: &str = "help";
const FUNCTION_WORD: &str = "function";
const DUMP_WORD: &str = "dump";
/// The lookup of a name known at run time in a table of the words' texts
const LOOKUP_TEMPLATE: &str = "if TABLE has NAME then TABLE[NAME] else error(\"no word named \" + NAME + \": dir(<module>) lists a module's words\")";

/// `help(sound)`, `help(midi)`: a standard module and a standard module word, read before reflection.rs answers
/// `help(sound)` with the signature of the word sound
pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[HELP_WORD]) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	if defined.contains(HELP_WORD) {
		return node;
	}
	answered_statically(node, &defined)
}

fn answered_statically(node: Node, defined: &HashSet<String>) -> Node {
	if let Some(name) = help_subject(&node).and_then(Node::symbol_name) {
		if let Some(help) = (!defined.contains(name)).then(|| std_help(name)).flatten() {
			return Node::Text(help);
		}
	}
	node.map_children(|child| answered_statically(child, defined))
}

/// The help of a standard module, else of a standard module word (a prelude word's help is its head,
/// reflection.rs)
fn std_help(name: &str) -> Option<String> {
	crate::modules::std_docs::module_help(name).or_else(|| {
		let module =
			crate::modules::std_module_defining(name).filter(|module| *module != crate::modules::PRELUDE_MODULE)?;
		crate::modules::std_docs::word_help(module, name)
	})
}

/// `help(x)` and `function(x).dump` no earlier pass answered, after the used modules joined the program: the text of
/// the word x names, from a table when x is known only at run time
pub fn lower_named_words(node: Node) -> Node {
	if !node.mentions_any(&[HELP_WORD, DUMP_WORD]) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let words = Words::of(&node, &defined);
	words.answered(node)
}

/// What help and dump show of each word the program knows, by name
struct Words {
	help: BTreeMap<String, String>,
	dump: BTreeMap<String, String>,
	/// a program's own `help` wins
	own_help: bool,
}

impl Words {
	fn of(program: &Node, defined: &HashSet<String>) -> Words {
		let module_words = crate::compiler::compile_context::read(|context| context.module_definitions.clone());
		let mut words = Words { help: BTreeMap::new(), dump: BTreeMap::new(), own_help: defined.contains(HELP_WORD) };
		let used_modules = crate::modules::std_module_names().filter(|module| {
			*module != crate::modules::PRELUDE_MODULE
				&& crate::modules::std_module_definitions(module)
					.is_some_and(|names| names.iter().any(|name| module_words.contains(name)))
		});
		for module in used_modules {
			for name in crate::modules::std_module_definitions(module).unwrap_or_default() {
				let docs = crate::modules::std_docs::word_help(module, name);
				words.help.insert(name.clone(), docs.unwrap_or_else(|| name.clone()));
				let definition = crate::modules::std_docs::word_definition(module, name);
				words.dump.extend(definition.map(|definition| (name.clone(), definition)));
			}
		}
		for (name, function) in super::reflection::user_functions(program) {
			if module_words.contains(&name) || words.dump.contains_key(&name) {
				continue;
			}
			words.help.insert(name.clone(), super::reflection::help_text(&function));
			words.dump.insert(name, super::reflection::definition(&function));
		}
		words
	}

	fn answered(&self, node: Node) -> Node {
		if !self.own_help {
			if let Some(subject) = help_subject(&node) {
				if let Some(answer) = self.lookup(&self.help, self.name_of(subject)) {
					return answer;
				}
			}
		}
		if let Node::Key(subject, Op::Dot, word) = node.drop_meta() {
			if word.symbol_name() == Some(DUMP_WORD) {
				if let Some(answer) = self.function_named(subject).and_then(|name| self.lookup(&self.dump, name)) {
					return answer;
				}
			}
		}
		node.map_children(|child| self.answered(child))
	}

	/// `function beep`, `function("beep")`: the name; `function trick` of a variable trick, `function(trick)`: the
	/// variable, whose value is the name
	fn function_named(&self, subject: &Node) -> Option<Node> {
		let subject = ungrouped(subject);
		if let Some(name) = crate::closures::referenced_function(subject) {
			return Some(self.name_of(&Node::Symbol(name)));
		}
		Some(self.name_of(word_applied(subject, FUNCTION_WORD)?))
	}

	/// A word's own name as a text; anything else is an expression that gives the name at run time
	fn name_of(&self, subject: &Node) -> Node {
		let subject = ungrouped(subject);
		match subject.drop_meta() {
			Node::Symbol(name) if self.help.contains_key(name) => Node::Text(name.clone()),
			_ => subject.clone(),
		}
	}

	/// The text of a name known now, else the lookup of the name at run time; none for a symbol no word is named
	/// (an instance's `help(p)` reflection.rs answered, a variable's `.dump` that is a field)
	fn lookup(&self, table: &BTreeMap<String, String>, name: Node) -> Option<Node> {
		if let Node::Text(name) = &name {
			return Some(match table.get(name) {
				Some(answer) => Node::Text(answer.clone()),
				None => crate::node::error(&format!("no word named {name}: dir(<module>) lists a module's words")),
			});
		}
		if table.is_empty() {
			return None;
		}
		let entries = table.iter().map(|(name, answer)| key(text(name), Op::Colon, text(answer))).collect();
		let table = Node::List(entries, Bracket::Curly, Separator::Space);
		Some(filled(LOOKUP_TEMPLATE, [("TABLE", table), ("NAME", name)]))
	}
}

/// `help(x)`, `help x`: x
fn help_subject(node: &Node) -> Option<&Node> {
	word_applied(node, HELP_WORD).map(ungrouped)
}

/// `(x)` → x, a node outside parentheses keeps its meta (its position)
fn ungrouped(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => ungrouped(&items[0]),
		_ => node,
	}
}
