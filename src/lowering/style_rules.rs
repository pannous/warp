//! Styles written as CSS inside a style block (card web-styles, notes/web_framework.md step 7): selectors without
//! quotes (`.card { … }`, `ul > li { … }`, `h1, h2 { … }`, `a:hover { … }`, `#main { … }`) become the text keys
//! html.rs renders (`".card": { … }`), and lengths with a unit (`8px`, `1.5em`, `50%`, `-2px`) the texts they are,
//! where warp would read `8px` as `8 * px`. A value after a declaration continues it: `padding: 8px 4px`.
//! A class or id selector with a lone value is its color: `#done = "red"`, `.done: "red"` (card style-selectors).
//! In a style sheet the parser keeps a blank before `.x` or `#x` (`#main .x`, the descendant combinator) as the next item.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const STYLE: &str = "style";
const SELECTOR_VALUE_PROPERTY: &str = "color";
const CLASS_OR_ID: [char; 2] = ['.', '#'];
const CSS_UNITS: [&str; 17] = ["px", "em", "rem", "vh", "vw", "vmin", "vmax", "pt", "ch", "ex", "fr", "deg", "turn", "ms", "s", "cm", "mm"];

pub fn lower(node: Node) -> Node {
	style_block(&node).unwrap_or_else(|| node.map_children(lower))
}

/// `style{ … }` and `style: { … }`: the block with its rules and declarations styled
fn style_block(node: &Node) -> Option<Node> {
	let Node::Key(name, Op::Colon, body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Curly, separator) = body.drop_meta() else { return None };
	matches!(name.drop_meta(), Node::Symbol(word) if word == STYLE)
		.then(|| Node::Key(name.clone(), Op::Colon, Box::new(Node::List(styled(items.clone(), separator), Bracket::Curly, separator.clone()))))
}

/// The items of a style block: rules with their selectors as texts, declarations with their lengths as texts
fn styled(items: Vec<Node>, separator: &Separator) -> Vec<Node> {
	// Separator::Colon is the comma: `h1, h2 { … }`
	let joiner = if *separator == Separator::Colon { ", " } else { " " };
	let mut output: Vec<Node> = vec![];
	let mut pending: Vec<(String, Node)> = vec![];
	for item in items.into_iter().flat_map(split_chained) {
		if let Node::List(group, Bracket::None, group_separator) = item.drop_meta() {
			output.extend(styled(group.clone(), group_separator));
		} else if let Some((selector, body)) = rule_parts(&item) {
			output.push(rule(joined_selector(&mut pending, Some(selector), joiner), body));
		} else if !pending.is_empty() && matches!(item.drop_meta(), Node::List(_, Bracket::Curly, _)) {
			// `.card { … }`: a leading-dot name stays apart from its block
			output.push(rule(joined_selector(&mut pending, None, joiner), &item));
		} else if let Some((selector, value)) = selector_value(&item) {
			output.push(rule(selector, &Node::Key(Box::new(Node::Symbol(SELECTOR_VALUE_PROPERTY.into())), Op::Colon, Box::new(value.clone()))));
		} else if let Some((name, value)) = declaration_value(&item) {
			let value = css_length(value).map(Node::Text).unwrap_or_else(|| value.clone());
			output.push(Node::Key(Box::new(name.clone()), Op::Colon, Box::new(value)));
		} else if let Some(Node::Key(_, Op::Colon, value)) = output.last_mut().filter(|last| declaration_value(last).is_some()) {
			match continued_value(value, &item) {
				Some(joined) => **value = Node::Text(joined),
				None => output.push(item),
			}
		} else if let Some(text) = selector_text(&item) {
			pending.push((text, item));
		} else {
			output.push(item);
		}
	}
	output.extend(pending.into_iter().map(|(_, item)| item));
	output
}

/// `ul li { … }` and `.a { … }` on the next line: the parser chains the leading dot onto the rule before it
/// (`rule.a{…}`), as it does `#main` on the same line (`rule#main{…}`); the rules apart again
fn split_chained(node: Node) -> Vec<Node> {
	let Node::Key(left, op @ (Op::Dot | Op::Hash), right) = node.drop_meta().clone() else { return vec![node] };
	let mut parts = split_chained(*left);
	let last = parts.pop().expect("split_chained gives at least one part");
	if rule_parts(&last).is_some() {
		parts.extend([last, Node::Key(Box::new(Node::Empty), op, right)]);
	} else if parts.is_empty() {
		return vec![node];
	} else {
		parts.push(Node::Key(Box::new(last), op, right));
	}
	parts
}

/// `ul li`, `h1, h2`: the selector words waiting for their block, joined with the last one
fn joined_selector(pending: &mut Vec<(String, Node)>, last: Option<String>, joiner: &str) -> String {
	pending.drain(..).map(|(text, _)| text).chain(last).collect::<Vec<_>>().join(joiner)
}

fn rule(selector: String, body: &Node) -> Node {
	Node::Key(Box::new(Node::Text(selector)), Op::Colon, Box::new(styled_body(body)))
}

fn styled_body(body: &Node) -> Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, separator) => Node::List(styled(items.clone(), separator), Bracket::Curly, separator.clone()),
		single => Node::List(styled(vec![single.clone()], &Separator::None), Bracket::Curly, Separator::None),
	}
}

/// `color: theme`: a property name and a value that is no block
fn declaration_value(node: &Node) -> Option<(&Node, &Node)> {
	let Node::Key(name, Op::Colon, value) = node.drop_meta() else { return None };
	let is_name = matches!(name.drop_meta(), Node::Symbol(_) | Node::Text(_));
	(is_name && !matches!(value.drop_meta(), Node::List(_, Bracket::Curly, _))).then_some((name.as_ref(), value.as_ref()))
}

/// `#done = "red"`, `.done: "red"`: a class or id selector and a value that is no block, nor the one declaration
/// the parser chains onto it (`.done{color:red}` reads `.done:color:red`)
fn selector_value(node: &Node) -> Option<(String, &Node)> {
	let Node::Key(name, Op::Colon | Op::Assign, value) = node.drop_meta() else { return None };
	let selector = selector_text(name).filter(|selector| selector.starts_with(CLASS_OR_ID))?;
	let is_body = matches!(value.drop_meta(), Node::List(_, Bracket::Curly, _) | Node::Key(_, Op::Colon, _));
	(!is_body).then_some((selector, value.as_ref()))
}

/// `padding: 8px` followed by `4px`: the value `8px 4px`, when both parts are constants; a bare number in a value of
/// several words stays as written, as CSS reads it (`flex: 1 1 auto`, `margin: 0 auto`)
fn continued_value(value: &Node, next: &Node) -> Option<String> {
	Some(format!("{} {}", value_piece(value)?, value_piece(next)?))
}

fn value_piece(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Number(number) => Some(number.to_string()),
		Node::Text(text) | Node::Symbol(text) => Some(text.clone()),
		other => css_length(other),
	}
}

/// `8px`, `1.5em`, `50%`, `-2px`: the length as CSS writes it
fn css_length(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Key(amount, Op::Mul, unit) => match (amount.drop_meta(), unit.drop_meta()) {
			(Node::Number(number), Node::Symbol(unit)) if CSS_UNITS.contains(&unit.as_str()) => Some(format!("{number}{unit}")),
			_ => None,
		},
		Node::Key(amount, Op::Mod, nothing) if matches!(nothing.drop_meta(), Node::Empty) => match amount.drop_meta() {
			Node::Number(number) => Some(format!("{number}%")),
			_ => None,
		},
		Node::Key(nothing, Op::Sub | Op::Neg, length) if matches!(nothing.drop_meta(), Node::Empty) => css_length(length).map(|length| format!("-{length}")),
		_ => None,
	}
}

/// How a selector joins the parts the parser read as operators: `ul > li`, `a:hover`, `div.card`, `#main`
fn combinator(op: &Op) -> Option<&'static str> {
	Some(match op {
		Op::Gt => " > ",
		Op::Add => " + ",
		Op::Similar | Op::Rough => " ~ ",
		Op::Dot => ".",
		Op::Colon => ":",
		Op::Hash => "#",
		_ => return None,
	})
}

/// A rule: its selector as CSS text and its block of declarations
fn rule_parts(node: &Node) -> Option<(String, &Node)> {
	let Node::Key(left, op, right) = node.drop_meta() else { return None };
	if *op == Op::Colon && matches!(right.drop_meta(), Node::List(_, Bracket::Curly, _)) {
		return Some((selector_text(left)?, right.as_ref()));
	}
	let (rest, body) = rule_parts(right)?;
	Some((format!("{}{}{rest}", selector_text(left)?, combinator(op)?), body))
}

/// `ul`, `.card`, `#main`, `div.card`, `"@media …"`: the selector a node spells
fn selector_text(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(text) | Node::Text(text) => Some(text.clone()),
		Node::Empty => Some(String::new()),
		Node::Key(left, op, right) => Some(format!("{}{}{}", selector_text(left)?, combinator(op)?, selector_text(right)?)),
		_ => None,
	}
}
