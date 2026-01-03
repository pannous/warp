use crate::extensions::numbers::Number;
use crate::meta::LineInfo;
use crate::node::Node::{Empty, Error};
use crate::node::{Bracket, Node, Separator};
use log::warn;
use std::fs;

/// Read and parse a WASP file
pub fn parse_file(path: &str) -> Node {
	match fs::read_to_string(path) {
		Ok(content) => WaspParser::parse(&content),
		_ => Error(format!("Failed to read {}", path)),
	}
}

pub fn parse(input: &str) -> Node {
	if input.ends_with(".wasp") { return parse_file(input); }
	WaspParser::parse(input)
}

pub struct WaspParser {
	input: String,
	pos: usize,
	line: usize,
	column: usize,
	char: char,
	pub current_line: String,
}

impl WaspParser {
	pub fn new(input: String) -> Self {
		let current_line = input.lines().next().unwrap_or("").to_string();
		WaspParser {
			input,
			pos: 0,
			line: 1,
			column: 1,
			char: '\0',
			current_line,
		}
	}

	pub fn parse(input: &str) -> Node {
		let mut parser = WaspParser::new(input.to_string());

		// Use the separator-aware parsing logic
		let result = parser.parse_top_level();

		result
	}


	fn parse_top_level(&mut self) -> Node {
		let mut actual = Node::List(vec![], Bracket::Square, Separator::None);

		// Handle initial comments
		let (_, comment) = self.skip_whitespace_and_comments();
		if let Some(text) = comment {
			if let Node::List(ref mut items, _, _) = actual {
				items.push(Node::key("comment", Node::text(&text)));
			}
		}

		while self.pos < self.input.chars().count() {
			let pos_before = self.pos;
			let item = self.parse_value();

			if item == Empty {
				if self.pos == pos_before {
					if self.current_char().is_some() {
						self.advance();
					} else {
						break;
					}
				}
				continue;
			}

			// Add item FIRST, before checking separators
			if let Node::List(ref mut items, _, _) = actual {
				items.push(item);
			}

			let (had_newline, comment) = self.skip_whitespace_and_comments();
			if let Some(text) = comment {
				if let Node::List(ref mut items, _, _) = actual {
					items.push(Node::key("comment", Node::text(&text)));
				}
			}

			// Determine separator after this item
			let sep = if let Some(ch) = self.current_char() {
				if ch == ',' {
					self.advance();
					Separator::Comma
				} else if ch == ';' {
					self.advance();
					Separator::Semicolon
				} else if had_newline && self.pos < self.input.chars().count() {
					Separator::Newline
				} else {
					Separator::Space
				}
			} else if had_newline {
				Separator::Newline
			} else {
				Separator::None
			};

			// C++ logic: if separator changes, wrap existing content and start new group
			let current_sep = match &actual {
				Node::List(_, _, s) => s.clone(),
				_ => Separator::None,
			};

			if current_sep == Separator::None && sep != Separator::None {
				// First separator - just set it
				if let Node::List(_, _, ref mut s) = actual {
					*s = sep.clone();
				}
			} else if current_sep != Separator::None && sep != current_sep && sep != Separator::None {
				// Only wrap when moving to a LOOSER separator (higher precedence value)
				if sep.precedence() > current_sep.precedence() {
					// Separator changed to looser - wrap existing content and start new group
					let old_actual = std::mem::replace(&mut actual, Node::List(vec![], Bracket::Square, sep.clone()));
					if let Node::List(items, b, _) = old_actual {
						if items.len() > 1 {
							// Multiple items - wrap them
							let wrapped = Node::List(items, b, current_sep);
							actual = Node::List(vec![wrapped], Bracket::Square, sep);
						} else if items.len() == 1 {
							// Single item - just change separator
							actual = Node::List(items, b, sep);
						}
					}
				} else {
					// Moving to tighter separator - just update
					if let Node::List(_, _, ref mut s) = actual {
						*s = sep.clone();
					}
				}
			}

			// Safety check
			if self.pos == pos_before {
				if self.current_char().is_some() {
					self.advance();
				} else {
					break;
				}
			}
		}

		// Unwrap if only single item
		if let Node::List(mut items, _, sep) = actual {
		if items.is_empty() {
			return Empty;
		}
			if items.len() == 1 && sep == Separator::None {
				return items.remove(0);
			}
			actual = Node::List(items, Bracket::Square, sep);
		}

		actual
	}

	fn current_char(&self) -> Option<char> {
		self.input.chars().nth(self.pos)
	}

	fn peek_char(&self, offset: usize) -> Option<char> {
		self.input.chars().nth(self.pos + offset)
	}

	fn advance(&mut self) {
		if let Some(ch) = self.current_char() {
			self.char = ch; // debug
			if ch == '\n' {
				self.line += 1;
				self.column = 1;
				// Update current_line to the new line
				let lines: Vec<&str> = self.input.lines().collect();
				if self.line > 0 && self.line <= lines.len() {
					self.current_line = lines[self.line - 1].to_string();
				} else {
					self.current_line = String::new();
				}
			} else {
				self.column += 1;
			}
		}
		self.pos += 1;
	}

	fn get_position(&self) -> (usize, usize) {
		(self.line, self.column)
	}

	fn prev_char(&self) -> Option<char> {
		if self.pos > 0 {
			self.input.chars().nth(self.pos - 1)
		} else {
			None
		}
	}

	fn is_at_line_start(&self) -> bool {
		// Check if we're at the very beginning or right after whitespace/newline
		self.pos == 0 || self.prev_char().map_or(false, |ch| ch.is_whitespace())
	}

	fn skip_whitespace(&mut self) -> bool {
		let mut had_newline = false;
		while let Some(ch) = self.current_char() {
			if ch.is_whitespace() {
				if ch == '\n' {
					had_newline = true;
				}
				self.advance();
			} else {
				break;
			}
		}
		had_newline
	}

	fn consume_rest_of_line(&mut self) -> String {
		let mut line_comment = String::new();
		while let Some(ch) = self.current_char() {
			if ch == '\n' {
				self.advance();
				break;
			}
			line_comment.push(ch);
			self.advance();
		}
		line_comment.trim().to_string()
	}

	fn skip_whitespace_and_comments(&mut self) -> (bool, Option<String>) {
		let mut had_newline = false;
		let mut comment = None;
		loop {
			had_newline = self.skip_whitespace() || had_newline;

			// Check for # line comment (shell-style, shebang)
			if self.current_char() == Some('#') && self.is_at_line_start() {
				self.advance();
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comment = Some(text); }
				had_newline = true;
				continue;
			}

			// Check for // line comment
			if self.current_char() == Some('/') && self.peek_char(1) == Some('/') {
				self.advance();
				self.advance();
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comment = Some(text); }
				had_newline = true;
				continue;
			}

			if self.pos >= self.input.len() {
				return (had_newline, comment);
			}

			// Check for /* block comment */
			if self.current_char() == Some('/') && self.peek_char(1) == Some('*') {
				self.advance();
				self.advance();
				let mut text = String::new();
				while let Some(ch) = self.current_char() {
					if ch == '*' && self.peek_char(1) == Some('/') {
						self.advance();
						self.advance();
						break;
					}
					if ch == '\n' { had_newline = true; }
					text.push(ch);
					self.advance();
				}
				let text = text.trim().to_string();
				if !text.is_empty() { comment = Some(text); }
				continue;
			}

			break;
		}
		(had_newline, comment)
	}

	fn is_at_line_end(&self) -> bool {
		self.column == 0 && self.current_char() == Some('\n') || self.pos >= self.input.len()
	}

	fn parse_value(&mut self) -> Node {
		let _ = self.skip_whitespace_and_comments();

		// Capture position before parsing
		let (line, column) = self.get_position();
		if self.is_at_line_end() {
			return Empty;
		};
		let mut node = match self.current_char() {
			None => {
				self.advance();
				Empty // end of input ≠ ø !
			}
			Some(ch) => {
				match ch {
					'"' | '\'' | '«' => self.parse_string(),
					'(' => self.parse_bracketed('(', ')', Bracket::Round),
					'[' => self.parse_bracketed('[', ']', Bracket::Square),
					'{' => self.parse_bracketed('{', '}', Bracket::Curly),
					'<' => self.parse_bracketed('<', '>', Bracket::Round), // Generics as groups
					';' => Empty, // Semicolons handled by main parse loop
					'>' => Empty, // Closing angle bracket, handled by parse_bracketed
					// assert!(!'三'.is_numeric());
					'-' if self.peek_char(1) == Some('>') => {
						// Arrow operator: ->
						self.advance(); // skip -
						self.advance(); // skip >
						Node::Symbol("->".to_string())
					}
					_ if ch.is_numeric() || ch == '-' => self.parse_number(),
					_ if ch.is_alphabetic() || ch == '_' => self.parse_symbol_or_named_block(),
					// _ if ch.is_ascii_graphic()
					// _ if ch.is_control() => self.
					_ => {
						// todo implement the rest like operators, etc.
						warn!(
							"Unexpected character '{}' at line {}, column {}",
							ch, line, column
						);
						self.advance();
						Error(format!("Unexpected character '{}'", ch))
					}
				}
			}
		};
		if node == Empty {
			return node;
		}
		// Attach metadata with position
		node = node.with_meta(LineInfo::with_position(line, column));
		// TODO: Re-enable comment tracking after separator implementation
		// if let Some(c) = comment {
		// 	node = Node::meta(node,Node::key("comment", Node::text(&c)))
		// }
		node
	}

	fn parse_string(&mut self) -> Node {
		let quote = self.current_char().unwrap_or('"');
		self.advance(); // skip opening quote

		let mut s = String::new();
		while let Some(ch) = self.current_char() {
			if ch == quote {
				self.advance(); // skip closing quote

				// Single quotes with exactly one character become Codepoint
				if quote == '\'' {
					let mut chars = s.chars();
					if let Some(c) = chars.next() {
						if chars.next().is_none() {
							// Exactly one character
							return Node::codepoint(c);
						}
					}
				}

				return Node::text(&s);
			}
			if ch == '\\' {
				self.advance();
				if let Some(escaped) = self.current_char() {
					match escaped {
						'n' => s.push('\n'),
						't' => s.push('\t'),
						'r' => s.push('\r'),
						_ => s.push(escaped),
					}
					self.advance();
				}
			} else {
				s.push(ch);
				self.advance();
			}
		}
		Error("Unterminated string".to_string())
	}

	fn parse_number(&mut self) -> Node {
		let mut num_str = String::new();
		let mut has_dot = false;

		// todo edge case: leading plus
		if self.current_char() == Some('-') {
			num_str.push('-');
			self.advance();
		}

		// Check for hexadecimal: 0x or 0X
		if self.current_char() == Some('0') {
			if let Some(next_ch) = self.peek_char(1) {
				if next_ch == 'x' || next_ch == 'X' {
					// Parse hexadecimal
					self.advance(); // skip '0'
					self.advance(); // skip 'x'
					let mut hex_str = String::new();
					while let Some(ch) = self.current_char() {
						if ch.is_ascii_hexdigit() {
							hex_str.push(ch);
							self.advance();
						} else {
							break;
						}
					}
					return i64::from_str_radix(&hex_str, 16)
						.map(Node::int)
						.unwrap_or_else(|_| Error(format!("Invalid hex: 0x{}", hex_str)));
				}
			}
		}

		while let Some(ch) = self.current_char() {
			// assert!(!'三'.is_numeric());
			if ch.is_numeric() {
				num_str.push(ch);
				self.advance();
			} else if ch == '.' && !has_dot {
				has_dot = true;
				num_str.push(ch);
				self.advance();
			} else {
				break;
			}
		}

		if has_dot {
			num_str
				.parse::<f64>()
				.map(Node::float)
				.unwrap_or_else(|_| Error(format!("Invalid float: {}", num_str)))
		} else {
			num_str
				.parse::<i64>()
				.map(Node::int)
				.unwrap_or_else(|_| Error(format!("Invalid int: {}", num_str)))
		}
	}

	fn parse_symbol(&mut self) -> Result<String, String> {
		let mut symbol = String::new();

		while let Some(ch) = self.current_char() {
			if ch.is_alphanumeric() || ch == '_' || ch == '-' {
				symbol.push(ch);
				self.advance();
			} else {
				break;
			}
		}

		if symbol.is_empty() {
			Err("Empty symbol".to_string())
		} else {
			Ok(symbol)
		}
	}

	fn parse_symbol_or_named_block(&mut self) -> Node {
		let symbol = match self.parse_symbol() {
			Ok(s) => s,
			Err(e) => return Error(e),
		};
		self.skip_whitespace();

		// Check for named block: name{...} or name(...) or name<...>
		match self.current_char() {
			Some('{') => {
				let block = self.parse_bracketed('{', '}', Bracket::Curly);
				// Create Tag for named blocks: html{...} -> Tag("html", None, body)
				Node::tag(&symbol, block)
			}
			Some('<') => {
				// Generic type: option<string> -> Tag("option", <string>)
				let generic = self.parse_bracketed('<', '>', Bracket::Round);
				Node::tag(&symbol, generic)
			}
			Some('(') => {
				// Function-like: def name(params){body} or name(params):value
				let params = match self.parse_parenthesized() {
					Ok(p) => p,
					Err(e) => return Error(e),
				};
				self.skip_whitespace();

				if self.current_char() == Some('{') {
					let body = self.parse_bracketed('{', '}', Bracket::Curly);
					// Use Pair for function syntax: name(params) : body
					let signature = Node::text(&format!("{}{}", symbol, params));
					Node::pair(signature, body)
				} else if self.current_char() == Some(':') || self.current_char() == Some('=') {
					// name(params):value or name(params)=value
					self.advance();
					self.skip_whitespace();
					let value = self.parse_value();
					let key = format!("{}{}", symbol, params);
					Node::key(&key, value)
				} else {
					// Just a call: name(params)
					Node::text(&format!("{}{}", symbol, params))
				}
			}
			Some(':') | Some('=') => {
				// Key-value pair (both : and = are supported)
				self.advance();
				self.skip_whitespace();
				let value = self.parse_value();
				Node::key(&symbol, value)
			}
			_ => {
				// Just a symbol
				Node::symbol(&symbol)
			}
		}
	}

	fn parse_parenthesized(&mut self) -> Result<String, String> {
		let mut result = String::new();
		result.push('(');
		self.advance(); // skip '('

		let mut depth = 1;
		while let Some(ch) = self.current_char() {
			result.push(ch);
			if ch == '(' {
				depth += 1;
			} else if ch == ')' {
				depth -= 1;
				self.advance();
				if depth == 0 {
					return Ok(result);
				}
				continue;
			}
			self.advance();
		}

		Err("Unterminated parentheses".to_string())
	}

	fn parse_bracketed(&mut self, open: char, close: char, bracket: Bracket) -> Node {
		self.advance(); // skip opening bracket
		self.parse_list_with_separators(close, bracket)
	}

	fn parse_list_with_separators(&mut self, close: char, bracket: Bracket) -> Node {
		// Build nested structure dynamically like C++ parseListSeparator
		let bracket = bracket; // Ensure we own it
		let mut actual = Node::List(vec![], bracket.clone(), Separator::None);

		loop {
			let (_, comment) = self.skip_whitespace_and_comments();
			if let Some(text) = comment {
				if let Node::List(ref mut items, _, _) = actual {
					items.push(Node::key("comment", Node::text(&text)));
				}
			}

			if self.current_char() == Some(close) {
				self.advance();
				break;
			}
			if self.current_char().is_none() {
				return Error(format!("Unterminated (expected '{}')", close));
			}

			let pos_before = self.pos;
			let item = self.parse_value();

			if item == Empty {
				if self.pos == pos_before && self.current_char() != Some(close) {
					if self.current_char().is_some() {
						self.advance();
					}
				}
				continue;
			}

			// Add item FIRST, before checking separators
			if let Node::List(ref mut items, _, _) = actual {
				items.push(item);
			}

			let (had_newline, comment) = self.skip_whitespace_and_comments();
			if let Some(text) = comment {
				if let Node::List(ref mut items, _, _) = actual {
					items.push(Node::key("comment", Node::text(&text)));
				}
			}

			// Determine separator after this item
			let sep = if let Some(ch) = self.current_char() {
				if ch == ',' {
					self.advance();
					Separator::Comma
				} else if ch == ';' {
					self.advance();
					Separator::Semicolon
				} else if ch == close {
					Separator::None // Last item
				} else if had_newline {
					Separator::Newline
				} else {
					Separator::Space
				}
			} else if had_newline {
				Separator::Newline
			} else {
				Separator::None
			};

			// C++ logic: if separator changes, wrap existing content and start new group
			let current_sep = match &actual {
				Node::List(_, _, s) => s.clone(),
				_ => Separator::None,
			};

			if current_sep == Separator::None && sep != Separator::None {
				// First separator - just set it
				if let Node::List(_, _, ref mut s) = actual {
					*s = sep.clone();
				}
			} else if current_sep != Separator::None && sep != current_sep && sep != Separator::None {
				// Only wrap when moving to a LOOSER separator (higher precedence value)
				if sep.precedence() > current_sep.precedence() {
					// Separator changed to looser - wrap existing content and start new group
					let old_actual = std::mem::replace(&mut actual, Node::List(vec![], bracket.clone(), sep.clone()));
					if let Node::List(items, b, _) = old_actual {
						if items.len() > 1 {
							// Multiple items - wrap them
							let wrapped = Node::List(items, b, current_sep);
							actual = Node::List(vec![wrapped], bracket.clone(), sep);
						} else if items.len() == 1 {
							// Single item - just change separator
							actual = Node::List(items, b, sep);
						}
					}
				} else {
					// Moving to tighter separator - just update
					if let Node::List(_, _, ref mut s) = actual {
						*s = sep.clone();
					}
				}
			}

			// Safety check
			if self.pos == pos_before && self.current_char() != Some(close) {
				if self.current_char().is_some() {
					self.advance();
				}
			}
		}

		// Unwrap if only single item
		if let Node::List(mut items, _, sep) = actual {
			if items.is_empty() {
				return Empty;
			}
			if items.len() == 1 && sep == Separator::None && bracket != Bracket::Curly {
				return items.remove(0);
			}
			actual = Node::List(items, bracket, sep);
		}

		actual

	}
	fn group_by_separators(&self, items_with_seps: Vec<(Node, Separator)>, bracket: Bracket) -> Node {
		if items_with_seps.is_empty() {
			return Empty;
		}

		if items_with_seps.len() == 1 {
			return items_with_seps[0].0.clone();
		}

		// Collect all unique separator precedences (excluding None)
		let mut precedences: Vec<u8> = items_with_seps.iter()
			.map(|(_, sep)| sep.precedence())
			.filter(|&p| p < 255)
			.collect();
		precedences.sort();
		precedences.dedup();

		if precedences.is_empty() {
			// All items have None separator - return as space-separated list
			let items: Vec<Node> = items_with_seps.into_iter().map(|(node, _)| node).collect();
			if items.len() == 1 {
				return items[0].clone();
			}
			return Node::List(items, bracket, Separator::Space);
		}

		// Start with the loosest (highest precedence value) separator
		let max_prec = *precedences.last().unwrap();
		let split_sep = items_with_seps.iter()
			.find(|(_, sep)| sep.precedence() == max_prec)
			.map(|(_, sep)| sep.clone())
			.unwrap_or(Separator::Space);

		// Split items into groups by this separator
		let mut groups: Vec<Vec<(Node, Separator)>> = Vec::new();
		let mut current_group = Vec::new();

		for (item, sep) in items_with_seps {
			if sep.precedence() == max_prec {
				// Found a split point - add item and close group
				current_group.push((item, Separator::None));
				if !current_group.is_empty() {
					groups.push(current_group);
					current_group = Vec::new();
				}
			} else {
				// Keep this separator for processing in sub-groups
				current_group.push((item, sep));
			}
		}

		// Add final group
		if !current_group.is_empty() {
			groups.push(current_group);
		}

		// Filter empty groups
		groups.retain(|g| !g.is_empty());

		if groups.is_empty() {
			return Empty;
		}

		// Recursively process each group for tighter separators
		let grouped_nodes: Vec<Node> = groups.into_iter()
			.map(|group| {
				if group.len() == 1 && group[0].1 == Separator::None {
					// Single item with no further separators
					group[0].0.clone()
				} else {
					// Has multiple items or tighter separators - recurse
					self.group_by_separators(group, bracket.clone())
				}
			})
			.collect();

		// Return result
		if grouped_nodes.len() == 1 {
			grouped_nodes[0].clone()
		} else {
			Node::List(grouped_nodes, bracket, split_sep)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::eq;

	#[test]
	fn test_parse_number() {
		let node = WaspParser::parse("42");
		eq!(node, 42);
	}

	#[test]
	fn test_parse_string() {
		let node = WaspParser::parse(r#""hello""#);
		eq!(node, "hello");
	}

	#[test]
	fn test_parse_symbol() {
		let node = WaspParser::parse("red");
		if let Node::Symbol(s) = node {
			eq!(s, "red");
		}
	}

	#[test]
	fn test_parse_list() {
		let node = WaspParser::parse("[1, 2, 3]");
		if let Node::List(items, _, _) = node {
			eq!(items.len(), 3);
			eq!(items[0], 1);
		}
	}

	#[test]
	fn test_parse_key_value() {
		let node = WaspParser::parse(r#"name: "Alice""#);
		eq!(node.get_key(), "name");
	}

	#[test]
	fn test_parse_named_block() {
		let node = WaspParser::parse("html{ }");
		// Named blocks become Tags
		if let Node::Tag { title, .. } = node.unwrap_meta() {
			eq!(title, "html");
		} else {
			panic!("Expected Tag node");
		}
	}

	#[test]
	fn test_parse_complex() {
		let input = r#"html{
            ul{ li:"hi" li:"ok" }
            colors=[red, green, blue]
        }"#;
		let node = WaspParser::parse(input);
		println!("{:?}", node);
		if let Node::Tag { title, .. } = node.unwrap_meta() {
			eq!(title, "html");
		} else {
			panic!("Expected Tag node");
		}
	}

	#[test]
	fn test_parse_function() {
		let input = "def myfun(a, b){ return a + b }";
		let node = WaspParser::parse(input);
		println!("{:?}", node);
		// Should be Pair(signature, body)
		if let Node::Pair(sig, body) = node {
			println!("Signature: {:?}, Body: {:?}", sig, body);
		}
	}

	#[test]
	fn test_parse_multiple_values() {
		// Multiple numbers
		let node = WaspParser::parse("1 2 3");
		if let Node::List(items, _, _) = node {
			eq!(items.len(), 3);
			eq!(items[0], 1);
			eq!(items[1], 2);
			eq!(items[2], 3);
		} else {
			panic!("Expected List node, got {:?}", node);
		}

		// Multiple symbols
		let node = WaspParser::parse("hello world");
		if let Node::List(items, _, _) = node {
			eq!(items.len(), 2);
			if let Node::Symbol(s) = &items[0].unwrap_meta() {
				eq!(s, "hello");
			}
			if let Node::Symbol(s) = &items[1].unwrap_meta() {
				eq!(s, "world");
			}
		} else {
			panic!("Expected List node, got {:?}", node);
		}

		// Single value should not be wrapped in List
		let node = WaspParser::parse("42");
		eq!(node, 42);
	}

	#[test]
	fn test_current_line() {
		let input = "line1\nline2\nline3";
		let parser = WaspParser::new(input.to_string());
		eq!(parser.current_line, "line1");

		let mut parser = WaspParser::new(input.to_string());
		// Advance to second line
		while parser.line == 1 && parser.current_char().is_some() {
			parser.advance();
		}
		eq!(parser.current_line, "line2");
	}
}
