//! Where a diagnostic points: positions in messages, the source excerpt under them, the written statement
use super::*;

/// The position a diagnostic without a recorded one names (Diagnostic::at)
const UNKNOWN_POSITION: &str = " at 0:0";
/// Between a diagnostic's message and its fix
const FIX_SEPARATOR: &str = "; fix: ";

/// `message` at `position`, before its fix, unless it names a position already
pub fn located_at(message: String, position: &crate::compiler::meta::LineInfo) -> String {
	let at = format!(" at {}", position.place());
	if message.contains(UNKNOWN_POSITION) {
		return message.replacen(UNKNOWN_POSITION, &at, 1);
	}
	if names_position(&message) {
		return message;
	}
	match message.split_once(FIX_SEPARATOR) {
		Some((what, fix)) => format!("{what}{at}{FIX_SEPARATOR}{fix}"),
		None => format!("{message}{at}"),
	}
}

/// The first Error in the tree, its message located at the nearest position around it: a parser error is wrapped in
/// the position where it was read
pub fn first_located_error(node: &Node) -> Option<Node> {
	located_error_below(node, None)
}

fn located_error_below(node: &Node, around: Option<&crate::compiler::meta::LineInfo>) -> Option<Node> {
	let own = node.get_lineinfo();
	let around = own.as_ref().or(around);
	let below = |part: &Node| located_error_below(part, around);
	match node {
		Node::Error(message) => Some(match (message.drop_meta(), around) {
			(Node::Text(text), Some(position)) => crate::node::error(&located_at(text.clone(), position)),
			_ => node.clone(),
		}),
		Node::Key(left, _, right) => below(left).or_else(|| below(right)),
		Node::List(items, _, _) => items.iter().find_map(below),
		Node::Meta { node, .. } => below(node),
		Node::Type { name, body } => below(name).or_else(|| below(body)),
		_ => None,
	}
}

/// Does a message end its description with a position ` at line:column`
pub fn names_position(message: &str) -> bool {
	message_position(message).is_some() || message_file_position(message).is_some()
}

/// The position ` at line:column` that ends a message's description
pub fn message_position(message: &str) -> Option<(usize, usize)> {
	let (line, column) = described_place(message)?.split_once(':')?;
	Some((position_number(line)?, position_number(column)?))
}

/// The last position ` at file:line:column` in the description of a module's failure (card web-dev-modules)
pub fn message_file_position(message: &str) -> Option<(&str, usize, usize)> {
	places(message).find_map(|(_, place)| {
		let mut parts = place.rsplitn(3, ':');
		let (column, line, file) = (parts.next()?, parts.next()?, parts.next()?);
		Some((file, position_number(line)?, position_number(column)?))
	})
}

/// A module's failure: its last position ` at line:column` becomes ` at file:line:column`, so it is not read as the
/// program's
pub fn located_in_file(failure: &Node, file: &std::path::Path) -> Node {
	let Node::Error(message) = failure.drop_meta() else {
		return failure.clone();
	};
	let Node::Text(text) = message.drop_meta() else {
		return failure.clone();
	};
	let is_position = |place: &str| {
		place
			.split_once(':')
			.is_some_and(|(line, column)| position_number(line).is_some() && position_number(column).is_some())
	};
	let Some((at, _)) = places(text).find(|(_, place)| is_position(place)) else {
		return failure.clone();
	};
	Node::Error(Box::new(Node::Text(format!("{}{}:{}", &text[..at], file.display(), &text[at..]))))
}

/// The words after each ` at ` of a message's description, the last first, with where each starts
fn places(message: &str) -> impl Iterator<Item = (usize, &str)> {
	let description = described(message);
	description.rmatch_indices(" at ").map(move |(at, word)| {
		let start = at + word.len();
		(start, description[start..].split_whitespace().next().unwrap_or_default())
	})
}

fn described(message: &str) -> &str {
	message.split("; fix: ").next().unwrap_or(message)
}

fn described_place(message: &str) -> Option<&str> {
	described(message).rsplit_once(" at ").map(|(_, place)| place)
}

fn position_number(digits: &str) -> Option<usize> {
	(!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| digits.parse().ok()).flatten()
}

/// Where a failed program's error is: the position its message names (also for an error no diagnostic made)
pub fn error_position(result: &Node) -> Option<(usize, usize)> {
	let Node::Error(message) = result.drop_meta() else {
		return None;
	};
	let Node::Text(message) = message.drop_meta() else {
		return None;
	};
	message_position(message)
}

/// The source line at a position with `^^` under the word there (one `^` under any other character); tabs before it
/// stay tabs so the marks line up
pub fn excerpt(source: &str, line: usize, column: usize) -> Option<String> {
	let text = source.lines().nth(line.checked_sub(1)?)?;
	let before: Vec<char> = text.chars().take(column.checked_sub(1)?).collect();
	if before.len() + 1 != column {
		return None;
	}
	let is_word = |c: &char| c.is_alphanumeric() || *c == '_';
	let word = text.chars().skip(before.len()).take_while(is_word).count().max(1);
	let indent: String = before.iter().map(|&c| if c == '\t' { '\t' } else { ' ' }).collect();
	let gutter = " ".repeat(line.to_string().len());
	Some(format!("  {line} | {text}\n  {gutter} | {indent}{}", "^".repeat(word)))
}

/// Print the line a later warning or error names, from this program (the CLI's programs; a host shows them itself)
pub fn show_lines_of(source: &str) {
	*SHOWN_SOURCE.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(source.to_string());
}

/// The line of the shown program at a position, with its marks
pub fn shown_excerpt(line: usize, column: usize) -> Option<String> {
	SHOWN_SOURCE
		.lock()
		.unwrap_or_else(|poisoned| poisoned.into_inner())
		.as_deref()
		.and_then(|source| excerpt(source, line, column))
}

/// The text of a node for a message: its serialization, or the source from its position to the end of the statement
/// when a lowering pass rebuilt it with compiler temporaries (`range_value·item`), which the program never wrote
pub fn written_text(node: &Node) -> String {
	let serialized = node.serialize().trim().to_string();
	if !serialized.contains(crate::analyzer::TEMPORARY_SEPARATOR) {
		return serialized;
	}
	written_statement(node).unwrap_or(serialized)
}

/// The source of the program from `node`'s position to the end of its statement, as the program wrote it
pub fn written_statement(node: &Node) -> Option<String> {
	let source_line = |(line, column): (usize, usize)| {
		compile_context::read(|context| {
			let rest: String =
				context.source.lines().nth(line.checked_sub(1)?)?.chars().skip(column.saturating_sub(1)).collect();
			Some(statement_prefix(&rest).trim().to_string())
		})
	};
	position(node).and_then(source_line).filter(|text| !text.is_empty())
}

/// The start of `text` up to the end of its statement: a `;` or a closing bracket it did not open (`[cube 1..n]`)
fn statement_prefix(text: &str) -> &str {
	let mut depth = 0usize;
	for (offset, character) in text.char_indices() {
		match character {
			'(' | '[' | '{' => depth += 1,
			')' | ']' | '}' if depth == 0 => return &text[..offset],
			')' | ']' | '}' => depth -= 1,
			';' if depth == 0 => return &text[..offset],
			_ => {}
		}
	}
	text
}

/// The line and column the parser recorded for `node`, or for the first of its parts that has them: a node a lowering
/// pass rebuilt keeps the positions of the parts it was built from
pub fn position(node: &Node) -> Option<(usize, usize)> {
	line_info(node).map(|info| (info.line_nr, info.column))
}

/// The position `position` finds, with the module file it is in
pub fn line_info(node: &Node) -> Option<crate::compiler::meta::LineInfo> {
	if let Some(info) = node.get_lineinfo() {
		return Some(info);
	}
	match node {
		Node::Meta { node, .. } => line_info(node),
		Node::Key(left, _, right) => line_info(left).or_else(|| line_info(right)),
		Node::List(items, _, _) => items.iter().find_map(line_info),
		_ => None,
	}
}
