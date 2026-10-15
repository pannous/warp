//! `$xyz.value`: a member of the page's element of id xyz, `document.getElementById("xyz").value`; `$document.title`,
//! `$window`, `$Math`, any global JavaScript or WebIDL declares: that global (card browser-interaction, user: `$xyz` is
//! the element, not its value). Only as a member's receiver: a bare `$main` stays the symbol, data as WAT names are. Each brings its `use js` along, which foreign_modules then
//! lowers, so `$xyz.vlaue` is a compile-time error as `document.getElementById("xyz").vlaue` is. The other `$`s stay:
//! `$0` (a closure's first parameter), `$a` naming an enclosing node of a data literal (references.rs); a shader's
//! holes, a route's form fields and a code pattern's holes are taken before this pass or sealed.

use crate::lowering::blocks::DATA_WORD;
use crate::modules::needed_definitions::with_statements_first;
use crate::node::Node;
use crate::operators::Op;
use std::collections::BTreeSet;

const SIGIL: char = '$';
/// The global the elements are found in, and how
const DOCUMENT: &str = "document";
/// JavaScript's own globals, which WebIDL (the browser's) does not declare
const LANGUAGE_GLOBALS: [&str; 14] = [
	"Math", "JSON", "Date", "Intl", "Reflect", "Object", "Array", "Map", "Set", "Number", "String", "Promise",
	"RegExp", "BigInt",
];

pub fn lower(program: Node) -> Node {
	let mut globals = BTreeSet::new();
	let program = rewritten(program, &mut Vec::new(), &mut globals);
	let uses = globals.iter().map(|global| crate::warp_parser::parse_generated(&format!("use js {global}"))).collect();
	with_statements_first(program, uses)
}

/// `enclosing`: the names of the data literal's nodes around the node (`a{ c{ parent=$a } }`)
fn rewritten(node: Node, enclosing: &mut Vec<String>, globals: &mut BTreeSet<String>) -> Node {
	match node {
		// a code pattern's holes (`match c { data($x.warp) => x }`, grouped `(data $x.warp)` there), not always sealed yet
		quoted if is_quoted(&quoted) => quoted,
		Node::Key(receiver, Op::Dot, member) => {
			let receiver = match receiver.drop_meta() {
				Node::Symbol(name) => browser_name(name)
					.filter(|name| !enclosing.iter().any(|node| node == name))
					.map(|name| browser_value(name, globals)),
				_ => None,
			}
			.unwrap_or_else(|| rewritten(*receiver, enclosing, globals));
			Node::Key(Box::new(receiver), Op::Dot, Box::new(rewritten(*member, enclosing, globals)))
		}
		other => {
			let named = crate::lowering::references::named_body(&other).map(|(name, _)| name);
			enclosing.extend(named.clone());
			let other = other.map_children(|child| rewritten(child, enclosing, globals));
			if named.is_some() {
				enclosing.pop();
			}
			other
		}
	}
}

/// `data …`, also grouped as a code pattern is (`(data $x.warp)`)
fn is_quoted(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.drop_meta().is_symbol(DATA_WORD)))
}

/// The global of that name, else the element of that id, its `use js` noted
fn browser_value(name: &str, globals: &mut BTreeSet<String>) -> Node {
	if is_global(name) {
		globals.insert(name.to_string());
		return Node::Symbol(name.to_string());
	}
	globals.insert(DOCUMENT.to_string());
	element_of_id(name)
}

/// `$xyz` → xyz; none for `$0` or a name without the sigil
fn browser_name(name: &str) -> Option<&str> {
	name.strip_prefix(SIGIL).filter(|rest| rest.starts_with(|first: char| first.is_alphabetic() || first == '_'))
}

/// A global of JavaScript or of the browser (WebIDL), also one only a page has (document)
fn is_global(name: &str) -> bool {
	LANGUAGE_GLOBALS.contains(&name) || !matches!(crate::formats::web_idl::global_interface(name), Ok(None))
}

/// `document.getElementById("xyz")`
fn element_of_id(id: &str) -> Node {
	crate::warp_parser::parse_generated(&format!(
		"{DOCUMENT}.getElementById({})",
		Node::Text(id.to_string()).serialize()
	))
}
