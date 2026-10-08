//! Other ecosystems' names for the standard library's words (notes/stdlib.md, alias rule): `JSON.parse(t)` (JS) and
//! `json.loads(t)` (Python) are `parse_json(t)` with a got-it note naming warp's word, and bring their module as if the
//! program said `use json`. A program naming the module itself (`re = 3`, `use python "json"`, which foreign_modules
//! already turned into calls) keeps its meaning. Runs before welcome_forms' module_calls: Python's json is also the
//! name of warp's module.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
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

const fn alias(module: &'static str, member: &'static str, std_module: &'static str, word: &'static str, order: &'static [usize]) -> StdAlias {
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
const USE_WORD: &str = "use";

pub fn lower(program: Node) -> Node {
	let kept: Vec<&str> = STD_ALIASES.iter().map(|alias| alias.module).filter(|module| crate::soft_keywords::program_names(&program, module)).collect();
	let mut used = HashSet::new();
	let program = aliased(program, &kept, &mut used);
	with_uses(program, used)
}

fn aliased(node: Node, kept: &[&str], used: &mut HashSet<&'static str>) -> Node {
	match aliased_call(&node, kept) {
		Some((alias, arguments)) => {
			crate::normalize::set_position_of(&node);
			crate::diagnostic::note_alias(&format!("{}.{}", alias.module, alias.member), alias.word);
			if !alias.std_module.is_empty() {
				used.insert(alias.std_module);
			}
			let arguments = alias.order.iter().map(|&index| aliased(arguments[index].clone(), kept, used));
			Node::List(std::iter::once(Node::Symbol(alias.word.to_string())).chain(arguments).collect(), Bracket::Round, Separator::None)
		}
		None => node.map_children(|child| aliased(child, kept, used)),
	}
}

/// `module.member(arguments…)` of a known alias with as many arguments as it takes; a shape tuple stands for its items
/// (`np.zeros((2, 3))` is zeros(2, 3))
fn aliased_call<'a>(node: &'a Node, kept: &[&str]) -> Option<(&'static StdAlias, &'a [Node])> {
	let Node::Key(module, Op::Dot, call) = node.drop_meta() else { return None };
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let (member, arguments) = items.split_first()?;
	let (module, member) = (module.drop_meta().name(), member.drop_meta().name());
	let shape_items = |arguments: &'a [Node]| match arguments {
		[tuple] => match tuple.drop_meta() {
			Node::List(items, Bracket::Round, Separator::Colon) => items.as_slice(),
			_ => arguments,
		},
		_ => arguments,
	};
	STD_ALIASES.iter()
		.filter(|alias| alias.module == module && alias.member == member && !kept.contains(&alias.module))
		.find_map(|alias| [arguments, shape_items(arguments)].into_iter().find(|given| given.len() == alias.order.len()).map(|given| (alias, given)))
}

/// The program with `use module` first for each module an alias brought
fn with_uses(program: Node, used: HashSet<&'static str>) -> Node {
	if used.is_empty() {
		return program;
	}
	let mut modules: Vec<&str> = used.into_iter().collect();
	modules.sort();
	let uses = modules.into_iter().map(|module| Node::List(vec![Node::Symbol(USE_WORD.to_string()), Node::Symbol(module.to_string())], Bracket::None, Separator::Space));
	match program {
		Node::List(statements, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => Node::List(uses.chain(statements).collect(), Bracket::None, separator),
		single => Node::List(uses.chain(std::iter::once(single)).collect(), Bracket::None, Separator::Semicolon),
	}
}
