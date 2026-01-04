use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use crate::meta::LineInfo;
use crate::node::Node::{Empty, Error, Symbol};
use crate::node::{Bracket, Node, Op, Separator};
use log::warn;
use std::fs;

/// Parser options for handling different file formats
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParserOptions {
	/// XML mode: treat <tag> as XML tags, not C++ generics
	pub xml_mode: bool,
	// Future: other format-specific options can be added here
}

impl Default for ParserOptions {
	fn default() -> Self {
		ParserOptions { xml_mode: false }
	}
}

impl ParserOptions {
	pub fn xml() -> Self {
		ParserOptions { xml_mode: true }
	}
}

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

pub fn parse_xml(input: &str) -> Node {
	WaspParser::parse_with_options(input, ParserOptions::xml())
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
	options: ParserOptions,
}

impl WaspParser {
	pub fn new(input: String) -> Self {
		Self::new_with_options(input, ParserOptions::default())
	}

	pub fn new_with_options(input: String, options: ParserOptions) -> Self {
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
			options,
		}
	}

	pub fn parse(input: &str) -> Node {
		Self::parse_with_options(input, ParserOptions::default())
	}

	pub fn parse_with_options(input: &str, options: ParserOptions) -> Node {
		let mut parser = WaspParser::new_with_options(input.to_string(), options);
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

	/// Skip characters until the target character is found
	fn skip_until(&mut self, target: char) {
		while !self.end_of_input() && self.current_char() != target {
			self.advance();
		}
	}

	/// Parse XML text content (everything until '<' or end of input)
	fn parse_xml_text_content(&mut self) -> String {
		let mut text = String::new();
		while !self.end_of_input() && self.current_char() != '<' {
			text.push(self.current_char());
			self.advance();
		}
		text.trim().to_string()
	}

	/// Skip XML processing instruction: <?xml ... ?> or <?target ... ?>
	/// Returns Empty since we don't preserve processing instructions
	fn skip_processing_instruction(&mut self) -> Node {
		self.advance(); // skip '?'

		// Skip everything until '?>'
		while !self.end_of_input() {
			if self.current_char() == '?' && self.peek_char(1) == '>' {
				self.advance(); // skip '?'
				self.advance(); // skip '>'
				return Empty; // Processing instructions are skipped
			}
			self.advance();
		}

		Empty
	}

	/// Skip XML comment: <!--...-->
	/// Returns Empty since comments are handled elsewhere
	fn skip_xml_comment(&mut self) -> Node {
		self.advance(); // skip first '-'
		self.advance(); // skip second '-'

		// Skip everything until '-->'
		while !self.end_of_input() {
			if self.current_char() == '-' && self.peek_char(1) == '-' && self.peek_char(2) == '>' {
				self.advance(); // skip first '-'
				self.advance(); // skip second '-'
				self.advance(); // skip '>'
				return Empty; // Comments are skipped in XML mode
			}
			self.advance();
		}

		Empty
	}

	/// Skip DOCTYPE declaration: <!DOCTYPE ...>
	/// Handles both simple and complex DOCTYPE with internal subset
	fn skip_doctype(&mut self) -> Node {
		// Skip until '>', handling nested brackets in internal subset
		let mut bracket_depth = 0;

		while !self.end_of_input() {
			let ch = self.current_char();

			if ch == '[' {
				bracket_depth += 1;
			} else if ch == ']' {
				bracket_depth -= 1;
			} else if ch == '>' && bracket_depth == 0 {
				self.advance(); // skip '>'
				return Empty; // DOCTYPE declarations are skipped
			}

			self.advance();
		}

		Empty
	}

	/// Parse CDATA section: <![CDATA[...]]>
	/// Returns the content as a Text node
	fn parse_cdata(&mut self) -> Node {
		// Expect: [CDATA[
		if self.current_char() != '[' {
			return Empty;
		}
		self.advance(); // skip '['

		// Check for "CDATA["
		let expected = "CDATA[";
		for expected_char in expected.chars() {
			if self.current_char() != expected_char {
				return Empty;
			}
			self.advance();
		}

		// Collect content until ]]>
		let mut content = String::new();
		while !self.end_of_input() {
			if self.current_char() == ']' && self.peek_char(1) == ']' && self.peek_char(2) == '>' {
				self.advance(); // skip first ']'
				self.advance(); // skip second ']'
				self.advance(); // skip '>'
				return Node::Text(content);
			}
			content.push(self.current_char());
			self.advance();
		}

		Node::Text(content) // Return what we have even if unterminated
	}

	fn is_at_line_end(&self) -> bool {
		self.column == 0 && self.current_char() == '\n' || self.pos >= self.input.len()
	}

	/// Peek ahead for an infix operator, returns (Op, chars_to_consume) if found
	/// Checks longer operators first (greedy matching)
	fn peek_operator(&self) -> Option<(Op, usize)> {
		let c1 = self.current_char();
		let c2 = self.peek_char(1);

		// Check 2-char operators first
		match (c1, c2) {
			(':', '=') => return Some((Op::Define, 2)),
			(':', ':') => return Some((Op::Scope, 2)),
			('-', '>') => return Some((Op::Arrow, 2)),
			('=', '>') => return Some((Op::FatArrow, 2)),
			_ => {}
		}

		// Check 1-char operators
		match c1 {
			':' => Some((Op::Colon, 1)),
			'=' => Some((Op::Assign, 1)),
			'.' => Some((Op::Dot, 1)),
			_ => None,
		}
	}

	/// Parse an atomic expression (no infix operators)
	/// Handles: numbers, strings, brackets, symbols with named blocks
	fn parse_atom(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();
		let (line_nr, column) = self.get_position();

		if self.is_at_line_end() {
			return Empty;
		}

		let node = match self.current_char() {
			'"' | '\'' | '«' => self.parse_string(),
			'(' | '[' | '{' => self.parse_bracketed(self.current_char()),
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			'<' => self.parse_bracketed('<'),
			';' | '>' => Empty,
			ch if ch.is_numeric() || (ch == '-' && self.peek_char(1).is_numeric()) => self.parse_number(),
			ch if ch.is_alphabetic() || ch == '_' => self.parse_symbol_with_suffix(),
			ch => {
				warn!("Unexpected character '{}' at line {}, column {}", ch, line_nr, column);
				self.advance();
				Error(format!("Unexpected character '{}'", ch))
			}
		};

		if node == Empty {
			return node;
		}

		// Attach metadata
		let node = node.with_meta_data(LineInfo { line_nr, column, line: self.current_line.clone() });
		if let Some(c) = comment {
			node.with_comment(c)
		} else {
			node
		}
	}

	/// Parse symbol with optional suffix: name{...}, name<...>, name(...)
	/// Does NOT handle infix operators like : or = (those are handled by parse_expr)
	fn parse_symbol_with_suffix(&mut self) -> Node {
		let symbol = match self.parse_symbol() {
			Ok(s) => s,
			Err(e) => return Error(e),
		};
		self.skip_spaces();

		// Check for suffix blocks (these bind tighter than any infix operator)
		match self.current_char() {
			'{' => {
				let block = self.parse_bracketed('{');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(block))
			}
			'<' if !self.options.xml_mode => {
				let generic = self.parse_bracketed('<');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(generic))
			}
			'(' => {
				let params = match self.parse_parenthesized() {
					Ok(p) => p,
					Err(e) => return Error(e),
				};
				self.skip_whitespace();

				if self.current_char() == '{' {
					let body = self.parse_bracketed('{');
					let signature = Node::text(&format!("{}{}", symbol, params));
					Node::List(vec![signature, body], Bracket::Round, Separator::None)
				} else {
					// Just a call: name(params) - operators handled by parse_expr
					Node::text(&format!("{}{}", symbol, params))
				}
			}
			_ => Node::symbol(&symbol),
		}
	}

	/// Pratt parser: parse expression with given minimum binding power
	fn parse_expr(&mut self, min_bp: u8) -> Node {
		let mut lhs = self.parse_atom();

		loop {
			self.skip_spaces(); // Only spaces, not newlines (newlines are separators)

			// Check for infix operator
			let (op, chars) = match self.peek_operator() {
				Some(pair) => pair,
				None => break,
			};

			let (l_bp, r_bp) = op.binding_power();

			// Stop if operator binds less tightly than our minimum
			if l_bp < min_bp {
				break;
			}

			// Consume the operator
			for _ in 0..chars {
				self.advance();
			}
			self.skip_whitespace();

			// Parse right-hand side with appropriate binding power
			let rhs = self.parse_expr(r_bp);

			// Build the Key node
			lhs = Node::Key(Box::new(lhs), op, Box::new(rhs));
		}

		lhs
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
					'(' | '[' | '{' => self.parse_bracketed(ch),
					'<' if self.options.xml_mode => self.parse_xml_tag(),
					'<' => self.parse_bracketed(ch), // C++ generics as groups
					';' => Empty,                    // Semicolons handled by main parse loop
					'>' => Empty, // Closing angle bracket, handled by parse_bracketed
					'-' if self.peek_char(1) == '>' => {
						// Arrow operator: ->
						self.advance(); // skip -
						self.advance(); // skip >
						Node::Symbol("->".to_string())
					}
					_ if ch.is_numeric() || ch == '-' => self.parse_number(), // '三'.is_numeric()!
					_ if ch.is_alphabetic() || ch == '_' => self.parse_expr(0),
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

	fn parse_bracketed(&mut self, open: char) -> Node {
		let (close, bracket_type) = match open {
			'(' => (')', Bracket::Round),
			'[' => (']', Bracket::Square),
			'{' => ('}', Bracket::Curly),
			'<' => ('>', Bracket::Round),
			_ => panic!("Invalid bracket: {}", open),
		};
		self.advance(); // skip opening bracket
		self.parse_list_with_separators(Some(close), bracket_type)
	}

	/// Parse XML tag: <tag attr="value">content</tag> or <tag />
	fn parse_xml_tag(&mut self) -> Node {
		self.advance(); // skip '<'

		// Handle XML directives and special constructs
		if self.current_char() == '?' {
			// Processing instruction: <?xml ... ?> or <?xml-stylesheet ... ?>
			return self.skip_processing_instruction();
		}

		if self.current_char() == '!' {
			// Could be: <!--comment-->, <!DOCTYPE...>, or <![CDATA[...]]>
			self.advance(); // skip '!'

			if self.current_char() == '-' && self.peek_char(1) == '-' {
				// Comment: <!--...-->
				return self.skip_xml_comment();
			}

			if self.current_char() == '[' {
				// CDATA: <![CDATA[...]]>
				return self.parse_cdata();
			}

			// DOCTYPE or other declaration: <!DOCTYPE...>
			return self.skip_doctype();
		}

		// Check for closing tag </tag>
		if self.current_char() == '/' {
			// This is a closing tag, should be handled by parent
			// Return error for unmatched closing tag
			self.advance(); // skip '/'
			let tag_name = self.parse_symbol().unwrap_or_default();
			self.skip_until('>');
			self.advance(); // skip '>'
			return Error(format!("Unmatched closing tag </{}>", tag_name));
		}

		// Parse tag name
		let tag_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(e) => return Error(e),
		};

		// Parse attributes
		let mut attributes = Vec::new();
		self.skip_whitespace_and_comments();

		while self.current_char() != '>' && self.current_char() != '/' && !self.end_of_input() {
			let attr_name = match self.parse_symbol() {
				Ok(name) => name,
				Err(_) => break,
			};

			self.skip_whitespace_and_comments();

			// Check for = sign
			if self.current_char() == '=' {
				self.advance(); // skip '='
				self.skip_whitespace_and_comments();

				// Parse attribute value (must be quoted)
				let attr_value = if self.current_char() == '"' || self.current_char() == '\'' {
					self.parse_string()
				} else {
					// Try to parse unquoted value
					match self.parse_symbol() {
						Ok(val) => Node::Text(val),
						Err(_) => Node::Empty,
					}
				};

				// Store attribute as dotted key
				attributes.push(Node::Key(Box::new(Symbol(format!(".{}", attr_name))), Op::Assign, Box::new(attr_value)));
			} else {
				// Boolean attribute (no value)
				attributes.push(Node::Key(
					Box::new(Symbol(format!(".{}", attr_name))),
					Op::Assign,
					Box::new(Node::True),
				));
			}

			self.skip_whitespace_and_comments();
		}

		// Check for self-closing tag
		if self.current_char() == '/' {
			self.advance(); // skip '/'
			self.skip_whitespace_and_comments();
			if self.current_char() == '>' {
				self.advance(); // skip '>'
			}
			// Return self-closing tag with only attributes
			return if attributes.is_empty() {
				Node::Key(Box::new(Symbol(tag_name)), Op::Colon, Box::new(Node::Empty))
			} else {
				Node::Key(
					Box::new(Symbol(tag_name)),
					Op::Colon,
					Box::new(Node::List(attributes, Bracket::Curly, Separator::None)),
				)
			};
		}

		// Skip closing '>' of opening tag
		if self.current_char() == '>' {
			self.advance();
		}

		// Parse content until closing tag
		let mut content_items = Vec::new();

		while !self.end_of_input() {
			// Check for closing tag (before skipping whitespace)
			if self.current_char() == '<' && self.peek_char(1) == '/' {
				self.advance(); // skip '<'
				self.advance(); // skip '/'
				let closing_name = self.parse_symbol().unwrap_or_default();
				self.skip_until('>');
				self.advance(); // skip '>'

				if closing_name != tag_name {
					return Error(format!(
						"Mismatched tags: <{}> closed with </{}>",
						tag_name, closing_name
					));
				}
				break; // Successfully closed
			}

			// Check for nested tag
			if self.current_char() == '<' && self.peek_char(1) != '/' {
				let nested = self.parse_xml_tag();
				if nested != Empty {
					content_items.push(nested);
				}
				continue;
			}

			// Parse text content until next tag
			let text = self.parse_xml_text_content();
			if !text.is_empty() {
				content_items.push(Node::Text(text));
			}
		}

		// Combine attributes and content
		let mut body_items = attributes;
		body_items.extend(content_items);

		if body_items.is_empty() {
			Node::Key(Box::new(Symbol(tag_name.clone())), Op::Colon, Box::new(Node::Empty))
		} else if body_items.len() == 1 {
			Node::Key(Box::new(Symbol(tag_name)), Op::Colon, Box::new(body_items.into_iter().next().unwrap()))
		} else {
			Node::Key(
				Box::new(Symbol(tag_name)),
				Op::Colon,
				Box::new(Node::List(body_items, Bracket::Curly, Separator::None)),
			)
		}
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
				// Combine item with indented body as Key
				Node::Key(Box::new(Symbol(item.name())), Op::Colon, Box::new(body))
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
				Separator::Colon
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
