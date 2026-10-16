//! Foreign lines (decision foreign-signature): `cbrt(x: float): float from 'm'` declares a C library function with its
//! signature in warp, as lib/math.warp declares libm's; the program imports it like `import cbrt from 'm'`, whose
//! signature the library's header gives (src/ffi reads lib/math.warp's lines for the headers that hide theirs)

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const FROM_WORD: &str = "from";

pub fn lower(node: Node) -> Node {
	match foreign_line_of(&node) {
		Some((name, library)) if library == crate::ffi::BUILTIN_LIBRARY => builtin_line(&node, &name),
		Some((name, library)) => crate::warp_parser::parse_generated(&format!("import ({name}) from '{library}'")),
		None => node.map_children(lower),
	}
}

/// `floor(x: number): int from warp`: warp computes it and lib/math.warp says so (library_signatures.rs reads the
/// signature there); a program may repeat that line, never declare another word or signature
fn builtin_line(node: &Node, name: &str) -> Node {
	let line = crate::ffi::math_foreign_lines(crate::ffi::BUILTIN_LIBRARY).find(|line| line.name == name);
	let Some(line) = line else {
		return crate::node::error(&format!("warp computes no {name}: `from warp` declares a word of lib/math.warp"));
	};
	let declared = format!("{name}({}): {}", line.parameters, line.result);
	let parsed = crate::warp_parser::parse_generated(&format!("{declared} from {}", crate::ffi::BUILTIN_LIBRARY));
	if parsed.serialize() == node.serialize() {
		return Node::Empty;
	}
	crate::node::error(&format!("warp's {name} is `{declared}`: a `from warp` line repeats lib/math.warp's"))
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
		Node::Symbol(name) if name == crate::ffi::BUILTIN_LIBRARY => name.clone(),
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
