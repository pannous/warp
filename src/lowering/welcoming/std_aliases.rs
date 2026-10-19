//! Other ecosystems' names for the standard library's words (notes/stdlib.md, alias rule): `JSON.parse(t)` (JS) and
//! `json.loads(t)` (Python) are warp's `json.parse(t)` with a got-it note naming it, and bring their module as if the
//! program said `use json`. `json.parse(t)` itself is the internal word parse_json of lib/json.warp; written by hand,
//! `parse_json(t)` gets the note too (card g_ogQg). A program naming the module itself (`re = 3`, `use python "json"`, which foreign_modules
//! already turned into calls) keeps its meaning. Runs before welcome_forms' module_calls: Python's json is also the
//! name of warp's module.

use crate::modules::USE_KEYWORD;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::program_names::ProgramNames;
use std::collections::HashSet;

/// `module.member(…)` of another ecosystem: warp's `word` of `std_module` (none for a prelude word), its arguments the
/// written ones in `order`
struct StdAlias {
	module: &'static str,
	member: &'static str,
	std_module: &'static str,
	word: &'static str,
	order: &'static [usize],
}

const fn alias(
	module: &'static str,
	member: &'static str,
	std_module: &'static str,
	word: &'static str,
	order: &'static [usize],
) -> StdAlias {
	StdAlias { module, member, std_module, word, order }
}

const STD_ALIASES: [StdAlias; 21] = [
	alias("JSON", "parse", "json", "parse_json", &[0]),
	alias("JSON", "stringify", "json", "to_json", &[0]),
	alias("json", "loads", "json", "parse_json", &[0]),
	alias("json", "dumps", "json", "to_json", &[0]),
	alias("re", "findall", "regex", "find_all", &[1, 0]),
	alias("re", "sub", "regex", "replace_all", &[2, 0, 1]),
	alias("fs", "readFileSync", "", "read", &[0]),
	alias("fs", "writeFileSync", "file", "write", &[0, 1]),
	alias("fs", "appendFileSync", "file", "append_file", &[0, 1]),
	alias("fs", "existsSync", "file", "exists", &[0]),
	alias("os", "getenv", "os", "env", &[0]),
	alias("process", "exit", "", "exit", &[0]),
	alias("np", "zeros", "matrix", "zeros", &[0, 1]),
	alias("np", "ones", "matrix", "ones", &[0, 1]),
	alias("np", "eye", "matrix", "identity", &[0]),
	alias("np", "identity", "matrix", "identity", &[0]),
	alias("np", "transpose", "matrix", "transpose", &[0]),
	alias("np", "matmul", "matrix", "matmul", &[0, 1]),
	alias("np", "dot", "matrix", "dot", &[0, 1]),
	alias("String", "from_utf8", "text", "from_utf8", &[0]),
	alias("string", "from_utf8", "text", "from_utf8", &[0]),
];
/// The standard words warp writes qualified by their module: `json.parse(t)` (card g_ogQg), lowered as the aliases are
const WARP_FORMS: [StdAlias; 1] = [alias("json", "parse", "json", "parse_json", &[0])];
/// Standard words called bare, `member(…)`, module "": `json(t)` is json.parse(t) (card json5-parse)
const BARE_FORMS: [StdAlias; 1] = [alias("", "json", "json", "parse_json", &[0])];
/// Standard words read as a member of any value, `value.member`: `x.json` is to_json(x) (card node-json)
const MEMBER_FORMS: [StdAlias; 1] = [alias("", "json", "json", "to_json", &[0])];
/// Old names of standard words, still working, and how warp writes them now: called by hand, a got-it note names it
const OLD_NAMES: [(&str, &str); 1] = [("render_sound", "save sound as: file")];

pub fn lower(program: Node) -> Node {
	let names = crate::program_names::Bindings::of(&program).given();
	let kept: Vec<&str> =
		STD_ALIASES.iter().map(|alias| alias.module).filter(|module| names.contains(*module)).collect();
	// `def parse_json(t) {…}` too: the program's own word gets no note (card hint-own-definition)
	note_internal_words(&program, &names);
	// after `use json`, json's words are called bare too: `parse(t)` is json.parse(t), unless the program defines parse
	// (decision used-module-bare-words, notes/dispatch.md level 3)
	let bare_forms: Vec<&StdAlias> = (BARE_FORMS.iter().filter(|form| !kept.contains(&form.member)))
		.chain(WARP_FORMS.iter().filter(|form| crate::modules::says_use(&program, form.module)))
		.collect();
	let mut used = HashSet::new();
	let names = ProgramNames::of(&program);
	let program = aliased(program, &Aliases { kept, bare_forms }, &names, &mut used);
	with_uses(program, used)
}

/// The aliases a program's calls may mean: the modules it names itself (`JSON.parse` of its own JSON stays its own)
/// and the standard words it may call bare
struct Aliases<'a> {
	kept: Vec<&'a str>,
	bare_forms: Vec<&'static StdAlias>,
}

/// How warp writes a standard word: `json.parse` for parse_json, else the word itself
fn warp_form(word: &str) -> String {
	WARP_FORMS
		.iter()
		.find(|form| form.word == word)
		.map_or(word.to_string(), |form| format!("{}.{}", form.module, form.member))
}

/// `parse_json(t)` written by hand, where the program defines no parse_json of its own: a got-it note naming
/// `json.parse`; a library's own calls are no business of the program
fn note_internal_words(program: &Node, names: &HashSet<String>) {
	let internal = WARP_FORMS.iter().map(|form| (form.word, warp_form(form.word)));
	let old = OLD_NAMES.iter().map(|(word, now)| (*word, now.to_string()));
	for (word, written) in internal.chain(old).filter(|(word, _)| !names.contains(*word)) {
		if let Some(call) = first_call_of(program, word) {
			crate::normalize::set_position_of(call);
			crate::errors::diagnostic::note_alias(word, &written);
		}
	}
}

/// Written in a module's file (`use netbase`), not in the program
fn in_module_file(node: &Node) -> bool {
	node.get_lineinfo().is_some_and(|at| at.file.is_some())
}

fn first_call_of<'a>(node: &'a Node, word: &str) -> Option<&'a Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _)
			if items.first().is_some_and(|first| matches!(first.drop_meta(), Node::Symbol(name) if name == word)) =>
		{
			(!in_module_file(node)).then_some(node)
		}
		Node::List(items, _, _) => items.iter().find_map(|item| first_call_of(item, word)),
		Node::Key(left, _, right) => first_call_of(left, word).or_else(|| first_call_of(right, word)),
		_ => None,
	}
}

/// Every alias call rewritten; a bare form only where the program takes no word of that name and arity in scope
fn aliased(node: Node, aliases: &Aliases, names: &ProgramNames, used: &mut HashSet<&'static str>) -> Node {
	let inner = names.inside(&node);
	let names = inner.as_ref().unwrap_or(names);
	match aliased_call(&node, aliases, names) {
		Some((alias, arguments)) => {
			crate::normalize::set_position_of(&node);
			if !alias.module.is_empty() && !WARP_FORMS.iter().any(|form| std::ptr::eq(form, alias)) {
				crate::errors::diagnostic::note_alias(
					&format!("{}.{}", alias.module, alias.member),
					&warp_form(alias.word),
				);
			}
			if !alias.std_module.is_empty() {
				used.insert(alias.std_module);
			}
			let arguments = alias.order.iter().map(|&index| aliased(arguments[index].clone(), aliases, names, used));
			parens(std::iter::once(symbol(alias.word)).chain(arguments).collect())
		}
		None => node.map_children(|child| aliased(child, aliases, names, used)),
	}
}

/// `module.member(arguments…)` of a known alias with as many arguments as it takes; a shape tuple stands for its items
/// (`np.zeros((2, 3))` is zeros(2, 3))
fn aliased_call<'a>(
	node: &'a Node,
	aliases: &Aliases,
	names: &ProgramNames,
) -> Option<(&'static StdAlias, &'a [Node])> {
	let kept = aliases.kept.as_slice();
	if let Some(bare) = bare_call(node, &aliases.bare_forms, names).or_else(|| member_read(node, kept)) {
		return Some(bare);
	}
	let Node::Key(module, Op::Dot, call) = node.drop_meta() else {
		return None;
	};
	let Node::List(items, _, _) = call.drop_meta() else {
		return None;
	};
	let (member, arguments) = items.split_first()?;
	let (module, member) = (module.drop_meta().name(), member.drop_meta().name());
	let shape_items = |arguments: &'a [Node]| match arguments {
		[tuple] => match tuple.drop_meta() {
			Node::List(items, Bracket::Round, Separator::Comma) => items.as_slice(),
			_ => arguments,
		},
		_ => arguments,
	};
	STD_ALIASES
		.iter()
		.chain(WARP_FORMS.iter())
		.filter(|alias| alias.module == module && alias.member == member && !kept.contains(&alias.module))
		.find_map(|alias| {
			[arguments, shape_items(arguments)]
				.into_iter()
				.find(|given| given.len() == alias.order.len())
				.map(|given| (alias, given))
		})
}

/// `json(t)`, `json t`: a bare form called with as many arguments as it takes, unless the program takes that call
fn bare_call<'a>(
	node: &'a Node,
	forms: &[&'static StdAlias],
	names: &ProgramNames,
) -> Option<(&'static StdAlias, &'a [Node])> {
	let (Node::List(items, Bracket::Round, _) | Node::List(items, Bracket::None, Separator::Space)) = node.drop_meta()
	else {
		return None;
	};
	let (word, arguments) = items.split_first()?;
	let word = word.drop_meta().symbol_name()?;
	(forms.iter().copied())
		.find(|form| form.member == word && form.order.len() == arguments.len())
		.filter(|_| !names.takes_call(word, arguments.len()))
		.map(|form| (form, arguments))
}

/// `x.json`: a member form read off any value, where the program names no such word itself (a field `json` wins)
fn member_read<'a>(node: &'a Node, kept: &[&str]) -> Option<(&'static StdAlias, &'a [Node])> {
	let Node::Key(value, Op::Dot, member) = node.drop_meta() else {
		return None;
	};
	let member = member.drop_meta().symbol_name()?;
	MEMBER_FORMS
		.iter()
		.find(|form| form.member == member && !kept.contains(&form.member))
		.map(|form| (form, std::slice::from_ref(value.as_ref())))
}

/// The program with `use module` first for each module an alias brought
pub(crate) fn with_uses(program: Node, used: HashSet<&'static str>) -> Node {
	if used.is_empty() {
		return program;
	}
	let mut modules: Vec<&str> = used.into_iter().collect();
	modules.sort();
	let uses = modules
		.into_iter()
		.map(|module| Node::List(vec![symbol(USE_KEYWORD), symbol(module)], Bracket::None, Separator::Space));
	crate::lowering::nodes::with_statements_first(program, uses.collect())
}
