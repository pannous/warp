//! Rust macro calls `name!(…)` / `name![…]` read as their warp forms (card rust-macros, decision rust-forms-welcome)

use super::*;

/// The macros that print their formatted text
const PRINT_MACROS: [&str; 4] = ["println", "print", "eprintln", "eprint"];
/// The macros that fail with their formatted text, or with the given default when they have none
const PANIC_MACROS: [(&str, &str); 4] = [
	("panic", "explicit panic"),
	("todo", "not yet implemented"),
	("unimplemented", "not implemented"),
	("unreachable", "entered unreachable code"),
];
/// The macros that assert a comparison of their first two arguments
const COMPARING_ASSERTS: [(&str, Op); 2] = [("assert_eq", Op::Eq), ("assert_ne", Op::Ne)];
const FORMAT_MACRO: &str = "format";
const VEC_MACRO: &str = "vec";
const ASSERT_MACRO: &str = "assert";
const PANIC_WORD: &str = "panic";

impl WarpParser {
	/// `name!(…)`: the `!` glued to a name and to the group after it; `x !(y)` and `x!= y` stay `not` and `!=`
	pub(super) fn try_parse_rust_macro(&mut self, lhs: &Node) -> Option<Node> {
		let Node::Symbol(name) = lhs.drop_meta() else {
			return None;
		};
		let open = self.peek_char(1);
		let glued = self.current_char() == '!' && !self.prev_char().is_whitespace() && matches!(open, '(' | '[');
		if !glued || self.options.data_mode {
			return None;
		}
		self.advance();
		let start = self.pos;
		let arguments = group_items(self.parse_bracketed(open));
		let written: String = self.chars[start + 1..self.pos.saturating_sub(1)].iter().collect();
		Some(rust_macro(name, arguments, &written))
	}
}

fn group_items(group: Node) -> Vec<Node> {
	match group.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	}
}

fn rust_macro(name: &str, arguments: Vec<Node>, written: &str) -> Node {
	if name == FORMAT_MACRO {
		return formatted(arguments);
	}
	if name == VEC_MACRO {
		return Node::List(arguments, Bracket::Square, Separator::Comma);
	}
	if PRINT_MACROS.contains(&name) {
		return call(PRINT_WORD, vec![formatted(arguments)]);
	}
	if let Some((_, default)) = PANIC_MACROS.iter().find(|(word, _)| *word == name) {
		let message = if arguments.is_empty() { Node::Text(default.to_string()) } else { formatted(arguments) };
		return call(PANIC_WORD, vec![message]);
	}
	if name == ASSERT_MACRO && !arguments.is_empty() {
		let mut arguments = arguments.into_iter();
		let condition = arguments.next().expect("not empty");
		return asserted(condition, arguments.collect(), written);
	}
	let comparing = COMPARING_ASSERTS.iter().find(|(word, _)| *word == name).filter(|_| arguments.len() >= 2);
	if let Some((_, op)) = comparing {
		let mut arguments = arguments.into_iter();
		let (left, right) = (arguments.next().expect("two"), arguments.next().expect("two"));
		let condition = Node::Key(Box::new(left), *op, Box::new(right));
		return asserted(condition, arguments.collect(), "");
	}
	call(name, arguments)
}

/// `assert C` with the message formatted from the rest, else naming the condition as written (lower_assert names
/// it when nothing is written)
fn asserted(condition: Node, message: Vec<Node>, written: &str) -> Node {
	let message = if !message.is_empty() {
		formatted(message)
	} else if written.trim().is_empty() {
		Node::Empty
	} else {
		Node::Text(format!("{ASSERTION_FAILED}: {}", written.trim()))
	};
	Node::List(vec![Symbol(ASSERT_MARKER.to_string()), condition, message], Bracket::Round, Separator::None)
}

/// `format!("{} {x} {0:?}", a)`: the template with every placeholder as `{}`, filled in order by lib/text.warp's
/// `format(template, values)`: `{}` takes the next argument, `{0}` the first, `{x}` the variable x
fn formatted(arguments: Vec<Node>) -> Node {
	let mut arguments = arguments.into_iter();
	let Some(template) = arguments.next() else {
		return Node::Text(String::new());
	};
	let Node::Text(template) = template.drop_meta() else {
		return call(
			FORMAT_MACRO,
			vec![template.clone(), Node::List(arguments.collect(), Bracket::Square, Separator::Comma)],
		);
	};
	let positional: Vec<Node> = arguments.collect();
	let (template, values) = placeholders_in_order(template, &positional);
	if values.is_empty() {
		return Node::Text(template);
	}
	call(FORMAT_MACRO, vec![Node::Text(template), Node::List(values, Bracket::Square, Separator::Comma)])
}

fn placeholders_in_order(template: &str, positional: &[Node]) -> (String, Vec<Node>) {
	let mut plain = String::new();
	let mut values = vec![];
	let mut next = 0;
	let mut rest = template;
	while let Some(open) = rest.find('{') {
		let Some(close) = rest[open..].find('}').map(|at| open + at) else {
			break;
		};
		plain.push_str(&rest[..open]);
		plain.push_str("{}");
		let argument = rest[open + 1..close].split(':').next().unwrap_or("").trim();
		values.push(match argument.parse::<usize>() {
			Ok(index) => positional.get(index).cloned().unwrap_or(Node::Empty),
			Err(_) if argument.is_empty() => {
				next += 1;
				positional.get(next - 1).cloned().unwrap_or(Node::Empty)
			}
			Err(_) => Symbol(argument.to_string()),
		});
		rest = &rest[close + 1..];
	}
	plain.push_str(rest);
	(plain, values)
}
