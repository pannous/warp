use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
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
	if input.ends_with(".wasp") {
		return parse_file(input);
	}
	WaspParser::parse(input)
}

pub struct WaspParser {
	input: String,
	chars: Vec<char>,
	pos: usize,
	line_nr: usize,
	column: usize,
	char: char,
	pub current_line: String,
	base_indent: usize,
}

impl WaspParser {
	pub fn new(input: String) -> Self {
		let current_line = input.lines().next().unwrap_or("").to_string();
		let chars: Vec<char> = input.chars().collect();
		WaspParser {
			input,
			chars,
			pos: 0,
			line_nr: 1,
			column: 1,
			char: '\0',
			current_line,
			base_indent: 0,
		}
	}

	pub fn parse(input: &str) -> Node {
		let mut parser = WaspParser::new(input.to_string());
		parser.parse_list_with_separators(None, Bracket::None)
	}

	fn end_of_input(&self) -> bool {
		self.pos >= self.chars.len()
	}

	fn current_char(&self) -> char {
		*self.chars.get(self.pos).unwrap_or(&'\0')
	}

	fn peek_char(&self, offset: usize) -> char {
		*self.chars.get(self.pos + offset).unwrap_or(&'\0')
	}

	fn advance(&mut self) {
		let ch = self.current_char();
		self.char = ch; // debug
		if ch == '\n' {
			self.line_nr += 1;
			self.column = 1;
			// Update current_line to the new line
			let lines: Vec<&str> = self.input.lines().collect();
			if self.line_nr > 0 && self.line_nr <= lines.len() {
				self.current_line = lines[self.line_nr - 1].to_string();
			} else {
				self.current_line = String::new();
			}
		} else {
			self.column += 1;
		}
		self.pos += 1;
	}

	fn get_position(&self) -> (usize, usize) {
		(self.line_nr, self.column)
	}

	fn prev_char(&self) -> char {
		self.chars.get(self.pos - 1).unwrap_or(&'\0').clone()
	}

	fn is_at_line_start(&self) -> bool {
		// Check if we're at the very beginning or right after whitespace/newline
		self.pos == 0 || self.prev_char().is_whitespace()
	}

	/// Skip whitespace, return (had_newline, current_line_indent)
	/// Only tabs count as semantic indent (not spaces)
	fn skip_whitespace(&mut self) -> (bool, usize) {
		let mut had_newline = false;
		let mut line_indent = 0;
		loop {
			let ch = self.current_char();
			if !ch.is_whitespace() { break; }
			if ch == '\n' {
				had_newline = true;
				line_indent = 0; // reset for new line
			} else if ch == '\t' {
				line_indent += 1; // only tabs count as indent
			}
			self.advance();
		}
		(had_newline, line_indent)
	}

	fn skip_spaces(&mut self) {
		while self.current_char() == ' ' || self.current_char() == '\t' {
			self.advance();
		}
	}

	fn consume_rest_of_line(&mut self) -> String {
		let mut line_comment = String::new();
		loop {
			let ch = self.current_char();
			if ch == '\0' { break; }
			if ch == '\n' { self.advance(); break; }
			line_comment.push(ch);
			self.advance();
		}
		line_comment.trim().to_string()
	}

	fn skip_whitespace_and_comments(&mut self) -> (bool, usize, Option<String>) {
		let mut had_newline = false;
		let mut line_indent = 0;
		let mut comments = Vec::new();
		loop {
			let (newline, indent) = self.skip_whitespace();
			had_newline = newline || had_newline;
			if newline { line_indent = indent; }
			// Check for # line comment (shell-style, shebang)
			// Only treat # as comment if at line start (to allow list#index later)
			if self.current_char() == '#' && self.is_at_line_start() {
				self.advance(); // skip #
				let text = self.consume_rest_of_line();
				if !text.is_empty() {
					comments.push(text);
				}
				had_newline = true;
				continue;
			}

			// Check for // line comment
			if self.current_char() == '/' && self.peek_char(1) == '/' {
				self.advance(); // skip first /
				self.advance(); // skip second /
				let text = self.consume_rest_of_line();
				if !text.is_empty() {
					comments.push(text);
				}
				had_newline = true;
				continue;
			}

			if self.pos >= self.input.len() {
				let comment = if comments.is_empty() { None } else { Some(comments.join("\n")) };
				return (had_newline, line_indent, comment);
			}

			// Check for /* block comment */
			if self.current_char() == '/' && self.peek_char(1) == '*' {
				self.advance(); // skip /
				self.advance(); // skip *
				let mut block_comment = String::new();
				loop {
					let ch = self.current_char();
					if ch == '\0' { break; } // unterminated comment
					if ch == '*' && self.peek_char(1) == '/' {
						self.advance(); // skip *
						self.advance(); // skip /
						break;
					}
					if ch == '\n' { had_newline = true; }
					block_comment.push(ch);
					self.advance();
				}
				let trimmed = block_comment.trim();
				if !trimmed.is_empty() {
					comments.push(trimmed.to_string());
				}
				continue;
			}

			break;
		}
		let comment = if comments.is_empty() { None } else { Some(comments.join("\n")) };
		(had_newline, line_indent, comment)
	}

	fn is_at_line_end(&self) -> bool {
		self.column == 0 && self.current_char() == '\n' || self.pos >= self.input.len()
	}

	fn parse_value(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();

		// Capture position before parsing
		let (line_nr, column) = self.get_position();
		if self.is_at_line_end() {
			return Empty;
		};
		let node = match self.current_char() {
			ch => {
				match ch {
					'"' | '\'' | '«' => self.parse_string(),
					'(' => self.parse_bracketed('(', ')', Bracket::Round),
					'[' => self.parse_bracketed('[', ']', Bracket::Square),
					'{' => self.parse_bracketed('{', '}', Bracket::Curly),
					'<' => self.parse_bracketed('<', '>', Bracket::Round), // Generics as groups
					';' => Empty,                                          // Semicolons handled by main parse loop
					'>' => Empty, // Closing angle bracket, handled by parse_bracketed
					'-' if self.peek_char(1) == '>' => {
						// Arrow operator: ->
						self.advance(); // skip -
						self.advance(); // skip >
						Node::Symbol("->".to_string())
					}
					_ if ch.is_numeric() || ch == '-' => self.parse_number(), // '三'.is_numeric()!
					_ if ch.is_alphabetic() || ch == '_' => self.parse_symbol_or_named_block(),
					// _ if ch.is_ascii_graphic() // is_ascii minus  is_control
					// _ if ch.is_control() => self.
					_ => {
						// todo implement the rest like operators, etc.
						warn!(
							"Unexpected character '{}' at line {}, column {}",
							ch, line_nr, column
						);
						warn!("Current line: {}", self.current_line); // todo ---^ 'here' via offset
						self.advance();
						Error(format!("Unexpected character '{}'", ch))
					}
				}
			}
		};
		if node == Empty {
			return node;
		}
		let node = node.with_meta_data(LineInfo {
			line_nr,
			column,
			line: self.current_line.s(),
		});
		// Attach preceding comment as metadata
		if let Some(comment_text) = comment {
			node.with_comment(comment_text)
		} else {
			node
		}
	}

	fn parse_string(&mut self) -> Node {
		let quote = self.current_char();
		self.advance(); // skip opening quote

		let mut s = String::new();
		loop {
			let ch = self.current_char();
			if ch == '\0' { return Error("Unterminated string".to_string()); }
			if ch == quote {
				self.advance(); // skip closing quote
				// quotes with exactly one character become Codepoint
				let mut chars = s.chars();
				if let Some(c) = chars.next() {
					if chars.next().is_none() {
						return Node::codepoint(c);
					}
				}
				return Node::text(&s);
			}
			if ch == '\\' {
				self.advance();
				match self.current_char() {
					'n' => s.push('\n'),
					't' => s.push('\t'),
					'r' => s.push('\r'),
					c => s.push(c),
				}
				self.advance();
			} else {
				s.push(ch);
				self.advance();
			}
		}
	}

	fn parse_number(&mut self) -> Node {
		let mut num_str = String::new();
		let mut has_dot = false;

		// todo edge case: leading plus
		if self.current_char() == '-' {
			num_str.push('-');
			self.advance();
		}

		// Check for hexadecimal: 0x or 0X
		if self.current_char() == '0' {
			let next_ch = self.peek_char(1);
			if next_ch == 'x' || next_ch == 'X' {
				// Parse hexadecimal
				self.advance(); // skip '0'
				self.advance(); // skip 'x'
				let mut hex_str = String::new();
				loop {
					let ch = self.current_char();
					if !ch.is_ascii_hexdigit() { break; }
					hex_str.push(ch);
					self.advance();
				}
				return i64::from_str_radix(&hex_str, 16)
					.map(Node::int)
					.unwrap_or_else(|_| Error(format!("Invalid hex: 0x{}", hex_str)));
			}
		}

		loop {
			let ch = self.current_char();
			// '三'.is_numeric() is true but not ASCII
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
		loop {
			let ch = self.current_char();
			if ch.is_alphanumeric() || ch == '_' || ch == '-' {
				symbol.push(ch);
				self.advance();
			} else {
				break;
			}
		}
		if symbol.is_empty() { Err("Empty symbol".to_string()) } else { Ok(symbol) }
	}

	fn parse_symbol_or_named_block(&mut self) -> Node {
		let symbol = match self.parse_symbol() {
			Ok(s) => s,
			Err(e) => return Error(e),
		};
		self.skip_spaces(); // Only spaces, not newlines

		// Check for named block: name{...} or name(...) or name<...>
		match self.current_char() {
			'{' => {
				let block = self.parse_bracketed('{', '}', Bracket::Curly);
				// Create Tag for named blocks: html{...} -> Tag("html", None, body)
				Node::tag(&symbol, block)
			}
			'<' => {
				// Generic type: option<string> -> Tag("option", <string>)
				let generic = self.parse_bracketed('<', '>', Bracket::Round);
				Node::tag(&symbol, generic)
			}
			'(' => {
				// Function-like: def name(params){body} or name(params):value
				let params = match self.parse_parenthesized() {
					Ok(p) => p,
					Err(e) => return Error(e),
				};
				let _ = self.skip_whitespace();

				if self.current_char() == '{' {
					let body = self.parse_bracketed('{', '}', Bracket::Curly);
					// Use Pair for function syntax: name(params) : body
					let signature = Node::text(&format!("{}{}", symbol, params));
					Node::pair(signature, body)
				} else if self.current_char() == ':' || self.current_char() == '=' {
					// name(params):value or name(params)=value
					self.advance();
					let _ = self.skip_whitespace();
					let value = self.parse_value();
					let key = format!("{}{}", symbol, params);
					Node::key(&key, value)
				} else {
					// Just a call: name(params)
					Node::text(&format!("{}{}", symbol, params))
				}
			}
			':' | '=' => {
				// Key-value pair (both a:b and a=b are supported)
				self.advance();
				let _ = self.skip_whitespace();
				let value = self.parse_value();
				Node::key(&symbol, value)
			}
			_ => Node::symbol(&symbol),
		}
	}

	fn parse_parenthesized(&mut self) -> Result<String, String> {
		let mut result = String::new();
		result.push('(');
		self.advance(); // skip '('

		let mut depth = 1;
		while !self.end_of_input() {
			let ch = self.current_char();
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

	fn parse_bracketed(&mut self, _open: char, close: char, bracket: Bracket) -> Node {
		self.advance(); // skip opening bracket
		self.parse_list_with_separators(Some(close), bracket)
	}

	fn parse_list_with_separators(&mut self, close: Option<char>, bracket: Bracket) -> Node {
		// Collect all items with their following separators
		let mut items_with_seps: Vec<(Node, Separator)> = Vec::new();

		loop {
			// Skip whitespace but track indent for dedent detection
			let (had_newline, line_indent) = self.skip_whitespace();

			// Check for dedent - exit this block if we're back to lower indent
			if had_newline && line_indent < self.base_indent && bracket == Bracket::None {
				break;
			}

			// Check for end condition
			let at_end = match close {
				Some(c) => self.current_char() == c,
				None => self.end_of_input(),
			};
			if at_end {
				if close.is_some() {
					self.advance(); // consume closing bracket
				}
				break;
			}

			let pos_before = self.pos;
			let item = self.parse_value();

			if item == Empty {
				if self.pos == pos_before {
					self.advance();
				}
				continue;
			}

			let (had_newline, line_indent, _) = self.skip_whitespace_and_comments();

			// Handle indentation-based blocks
			let item = if had_newline && line_indent > self.base_indent && bracket == Bracket::None {
				// Indented block follows - parse it as body of current item
				let old_indent = self.base_indent;
				self.base_indent = line_indent;
				let body = self.parse_list_with_separators(None, Bracket::None);
				self.base_indent = old_indent;
				// Combine item with indented body as Tag
				Node::Tag {
					title: item.name(),
					params: Box::new(Empty),
					body: Box::new(body),
				}
			} else if had_newline && line_indent < self.base_indent && bracket == Bracket::None {
				// Dedent - push item and exit this level
				items_with_seps.push((item, Separator::None));
				break;
			} else {
				item
			};

			// Determine separator after this item
			let ch = self.current_char();
			let at_end = match close {
				Some(c) => ch == c || ch == '\0',
				None => self.end_of_input(),
			};
			let sep = if at_end {
				Separator::None
			} else if ch == ',' {
				self.advance();
				Separator::Comma
			} else if ch == ';' {
				self.advance();
				Separator::Semicolon
			} else if had_newline {
				Separator::Newline
			} else {
				Separator::Space
			};

			items_with_seps.push((item, sep));

			if self.pos == pos_before {
				self.advance();
			}
		}

		// Use recursive grouping
		self.group_by_separators(items_with_seps, bracket)
	}
	fn group_by_separators(
		&self,
		items_with_seps: Vec<(Node, Separator)>,
		bracket: Bracket,
	) -> Node {
		if items_with_seps.is_empty() {
			return Empty;
		}

		if items_with_seps.len() == 1 && bracket == Bracket::None {
			// Only unwrap single items for implicit groupings
			return items_with_seps[0].0.clone();
		}

		// Collect all unique separator precedences (excluding None)
		let mut precedences: Vec<u8> = items_with_seps
			.iter()
			.map(|(_, sep)| sep.precedence())
			.filter(|&p| p < 255)
			.collect();
		precedences.sort();
		precedences.dedup();

		if precedences.is_empty() {
			// All items have None separator - return as space-separated list
			let items: Vec<Node> = items_with_seps.into_iter().map(|(node, _)| node).collect();
			if items.len() == 1 && bracket == Bracket::None {
				// Only unwrap single items for implicit groupings
				return items[0].clone();
			}
			return Node::List(items, bracket, Separator::Space);
		}

		// Start with the loosest (highest precedence value) separator
		let max_prec = *precedences.last().unwrap();
		let split_sep = items_with_seps
			.iter()
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
		let grouped_nodes: Vec<Node> = groups
			.into_iter()
			.map(|group| {
				if group.len() == 1 && group[0].1 == Separator::None {
					// Single item with no further separators
					group[0].0.clone()
				} else {
					// Has multiple items or tighter separators - recurse
					// Inner groups use Bracket::None to avoid extra braces in serialization
					self.group_by_separators(group, Bracket::None)
				}
			})
			.collect();

		// Return result
		if grouped_nodes.len() == 1 && bracket == Bracket::None {
			// Only unwrap single items for implicit groupings (Bracket::None)
			// Explicit brackets like {x} or [x] should preserve the wrapper
			grouped_nodes[0].clone()
		} else {
			Node::List(grouped_nodes, bracket, split_sep)
		}
	}
}

// Tests moved to tests/test_parser.rs
