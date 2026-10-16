//! Foreign lines (decision foreign-signature): `cbrt(x: float): float from 'm'` declares a C library function with its
//! signature in warp, as lib/math.warp declares libm's; the program imports it like `import cbrt from 'm'`, whose
//! signature the library's header gives (src/ffi reads lib/math.warp's lines for the headers that hide theirs)

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const FROM_WORD: &str = "from";

pub fn lower(node: Node) -> Node {
	match foreign_line_of(&node) {
		Some((name, library)) => crate::warp_parser::parse(&format!("import ({name}) from '{library}'")),
		None => node.map_children(lower),
	}
}

fn foreign_line_of(node: &Node) -> Option<(String, String)> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) => foreign_line(items),
		// `: float from 'm'` as the parser may group it, the result type with the rest
		Node::Key(head, Op::Colon, rest) => match rest.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space) if items.len() == 3 => {
				foreign_line(&[(**head).clone(), items[1].clone(), items[2].clone()])
			}
			_ => None,
		},
		_ => None,
	}
}

/// `cbrt(x: float): float from 'm'`: the function's name and its library
fn foreign_line(items: &[Node]) -> Option<(String, String)> {
	let [signature, from, library] = items else {
		return None;
	};
	if from.drop_meta().symbol_name() != Some(FROM_WORD) {
		return None;
	}
	// `'m'`, one letter in single quotes, reads as a character
	let library = match library.drop_meta() {
		Node::Text(_) | Node::Char(_) => library.drop_meta().text_or_name(),
		_ => return None,
	};
	let head = match signature.drop_meta() {
		Node::Key(head, Op::Colon, _) => head.drop_meta(),
		head => head,
	};
	Some((function_name(head)?, library))
}

/// `cbrt(x: float)`, or `sqrt(x: float)` as the parser reads it: the prefix operator √ applied
fn function_name(call: &Node) -> Option<String> {
	match call {
		Node::List(items, Bracket::Round, _) => Some(items.first()?.drop_meta().symbol_name()?.to_string()),
		Node::Key(_, op, _) => {
			let (word, _) = crate::warp_parser::PREFIX_OPERATOR_WORDS.into_iter().find(|(_, known)| known == op)?;
			Some(word.to_string())
		}
		_ => None,
	}
}
