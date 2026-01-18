use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use crate::meta::LineInfo;
use crate::node::Node::{Empty, Symbol};
use crate::node::{error, float, key_ops, Bracket, Node, Separator};
use crate::operators::Op;
use crate::normalize::hints as norm;
use crate::*;
use log::warn;
use std::fs::read_to_string;

/// Parser options for handling different file formats
#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(Default)]
pub struct ParserOptions {
	/// XML mode: treat <tag> as XML tags, not C++ generics
	pub xml_mode: bool,
	// Future: other format-specific options can be added here
}


impl ParserOptions {
	pub fn xml() -> Self {
		ParserOptions { xml_mode: true }
	}
}

/// Read and parse a WASP file
pub fn parse_file(path: &str) -> Node {
	match read_to_string(path) {
		Ok(content) => WaspParser::parse(&content),
		_ => error(&format!("Failed to read {}", path)),
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
		if self.pos == 0 { '\0' } else { *self.chars.get(self.pos - 1).unwrap_or(&'\0') }
	}

	/// Check if input at current position matches a keyword (followed by non-alphanumeric)
	fn matches_keyword(&self, keyword: &str) -> bool {
		keyword.chars().enumerate().all(|(i, c)| self.peek_char(i) == c)
			&& !self.peek_char(keyword.len()).is_alphanumeric()
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
			if !ch.is_whitespace() {
				break;
			}
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
			if ch == '\0' {
				break;
			}
			if ch == '\n' {
				self.advance();
				break;
			}
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
			had_newline |= newline;
			if newline { line_indent = indent; }

			let (c1, c2) = (self.current_char(), self.peek_char(1));

			// # line comment (shell-style) - only at line start
			if c1 == '#' && self.is_at_line_start() {
				self.advance();
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comments.push(text); }
				had_newline = true;
				continue;
			}
			// // line comment (but not :// URL scheme)
			if c1 == '/' && c2 == '/' && self.prev_char() != ':' {
				self.advance_by(2);
				let text = self.consume_rest_of_line();
				if !text.is_empty() { comments.push(text); }
				had_newline = true;
				continue;
			}
			// /* block comment */
			if c1 == '/' && c2 == '*' {
				self.advance_by(2);
				let mut block = String::new();
				while self.current_char() != '\0' {
					if self.current_char() == '*' && self.peek_char(1) == '/' {
						self.advance_by(2);
						break;
					}
					if self.current_char() == '\n' { had_newline = true; }
					block.push(self.current_char());
					self.advance();
				}
				let trimmed = block.trim();
				if !trimmed.is_empty() { comments.push(trimmed.to_string()); }
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

	/// Skip XML processing instruction: <?xml ... ?>
	fn skip_processing_instruction(&mut self) -> Node {
		self.advance(); // skip '?'
		while !self.end_of_input() {
			if self.current_char() == '?' && self.peek_char(1) == '>' {
				self.advance_by(2);
				return Empty;
			}
			self.advance();
		}
		Empty
	}

	/// Skip XML comment: <!--...-->
	fn skip_xml_comment(&mut self) -> Node {
		self.advance_by(2); // skip '--'
		while !self.end_of_input() {
			if self.current_char() == '-' && self.peek_char(1) == '-' && self.peek_char(2) == '>' {
				self.advance_by(3);
				return Empty;
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
	fn parse_cdata(&mut self) -> Node {
		// Expect: [CDATA[
		let marker = "[CDATA[";
		if !marker.chars().enumerate().all(|(i, c)| self.peek_char(i) == c) {
			return Empty;
		}
		self.advance_by(marker.len());

		let mut content = String::new();
		while !self.end_of_input() {
			if self.current_char() == ']' && self.peek_char(1) == ']' && self.peek_char(2) == '>' {
				self.advance_by(3);
				return Node::Text(content);
			}
			content.push(self.current_char());
			self.advance();
		}
		Node::Text(content)
	}

	fn is_at_line_end(&self) -> bool {
		self.column == 0 && self.current_char() == '\n' || self.pos >= self.input.len()
	}

	/// Check if current character can start an atom (for implicit application)
	fn can_start_atom(&self) -> bool {
		let ch = self.current_char();
		ch.is_alphanumeric() || ch == '_' || ch == '"' || ch == '\'' || ch == '(' || ch == '[' || ch == '{'
	}

	/// Check if character terminates a URL
	fn is_url_terminator(&self, ch: char) -> bool {
		ch == '\0' || ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r'
			|| ch == ';' || ch == ')' || ch == ']' || ch == '}' || ch == '>'
			|| ch == '"' || ch == '\'' || ch == ',' || ch == '«'
	}

	/// Peek ahead for an infix operator, returns (Op, chars_to_consume) if found
	/// Checks longer operators first (greedy matching)
	fn peek_operator(&self) -> Option<(Op, usize)> {
		let (c1, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));

		// Keywords (4-char)
		if self.matches_keyword("then") { return Some((Op::Then, 4)); }
		if self.matches_keyword("else") { return Some((Op::Else, 4)); }

		// 3-char operators
		match (c1, c2, c3) {
			('.', '.', '.') => return Some((Op::To, 3)),
			('.', '.', '<') => return Some((Op::Range, 3)), // Swift-style exclusive range
			('&', '&', '=') => return Some((Op::AndAssign, 3)),
			('|', '|', '=') => return Some((Op::OrAssign, 3)),
			('^', '^', '=') => return Some((Op::XorAssign, 3)),
			('*', '*', '=') => return Some((Op::PowAssign, 3)),
			_ => {}
		}
		// Keywords (3-char)
		if self.matches_keyword("and") { return Some((Op::And, 3)); }
		if self.matches_keyword("xor") { return Some((Op::Xor, 3)); }
		if self.matches_keyword("not") { return Some((Op::Not, 3)); }

		// 2-char operators
		match (c1, c2) {
			('a', 's') if !c3.is_alphanumeric() => return Some((Op::As, 2)),
			(':', '=') => return Some((Op::Define, 2)),
			(':', ':') => return Some((Op::Scope, 2)),
			('-', '>') => return Some((Op::Arrow, 2)),
			('=', '>') => return Some((Op::FatArrow, 2)),
			('*', '*') => { norm::power_operator("**"); return Some((Op::Pow, 2)); }
			('+', '=') => return Some((Op::AddAssign, 2)),
			('-', '=') => return Some((Op::SubAssign, 2)),
			('*', '=') => return Some((Op::MulAssign, 2)),
			('/', '=') => return Some((Op::DivAssign, 2)),
			('%', '=') => return Some((Op::ModAssign, 2)),
			('^', '=') => return Some((Op::PowAssign, 2)),
			('<', '=') => return Some((Op::Le, 2)),
			('>', '=') => return Some((Op::Ge, 2)),
			('=', '=') => return Some((Op::Eq, 2)),
			('!', '=') => return Some((Op::Ne, 2)),
			('+', '+') => return Some((Op::Inc, 2)),
			('-', '-') => return Some((Op::Dec, 2)),
			('.', '.') => return Some((Op::Range, 2)),
			('&', '&') => { norm::and_operator("&&"); return Some((Op::And, 2)); }
			('|', '|') => { norm::or_operator("||"); return Some((Op::Or, 2)); }
			_ => {}
		}
		// Keywords (2-char)
		if self.matches_keyword("or") { return Some((Op::Or, 2)); }
		if self.matches_keyword("if") { return Some((Op::If, 2)); }
		if self.matches_keyword("do") { return Some((Op::Do, 2)); }
		if self.matches_keyword("to") { return Some((Op::To, 2)); }

		// 1-char operators
		match c1 {
			':' => Some((Op::Colon, 1)),
			'=' => Some((Op::Assign, 1)),
			'.' => Some((Op::Dot, 1)),
			'+' => Some((Op::Add, 1)),
			'-' => Some((Op::Sub, 1)),
			'*' => Some((Op::Mul, 1)),
			'/' => Some((Op::Div, 1)),
			'%' => Some((Op::Mod, 1)),
			'^' => Some((Op::Pow, 1)),
			'×' | '⋅' => Some((Op::Mul, 1)),
			'÷' => Some((Op::Div, 1)),
			'<' => Some((Op::Lt, 1)),
			'>' => Some((Op::Gt, 1)),
			'≤' => Some((Op::Le, 1)),
			'≥' => Some((Op::Ge, 1)),
			'≠' => Some((Op::Ne, 1)),
			'!' => { norm::not_operator("!"); Some((Op::Not, 1)) }
			'¬' => Some((Op::Not, 1)),
			'&' => { norm::and_operator("&"); Some((Op::And, 1)) }
			'|' => { norm::or_operator("|"); Some((Op::Or, 1)) }
			'∧' => Some((Op::And, 1)),
			'⋁' => Some((Op::Or, 1)),
			'⊻' => Some((Op::Xor, 1)),
			'#' => Some((Op::Hash, 1)),
			'?' => { norm::conditional(true); Some((Op::Question, 1)) }
			'…' => Some((Op::To, 1)),
			_ => None,
		}
	}

	/// Peek for prefix operators (unary operators that bind to right operand)
	fn peek_prefix_operator(&self) -> Option<(Op, usize)> {
		if self.matches_keyword("while") { return Some((Op::While, 5)); }
		if self.matches_keyword("sqrt") { return Some((Op::Sqrt, 4)); }
		if self.matches_keyword("not") { return Some((Op::Not, 3)); }
		if self.matches_keyword("abs") { return Some((Op::Abs, 3)); }
		if self.matches_keyword("if") { return Some((Op::If, 2)); }

		let (c1, c2) = (self.current_char(), self.peek_char(1));
		match c1 {
			'-' if !c2.is_ascii_digit() => Some((Op::Neg, 1)),
			'!' | '¬' => Some((Op::Not, 1)),
			'√' => Some((Op::Sqrt, 1)),
			'‖' => Some((Op::Abs, 1)),
			'#' => Some((Op::Hash, 1)), // prefix # means count/length
			_ => None,
		}
	}

	/// Peek for suffix operators (unary operators that bind to left operand)
	fn peek_suffix_operator(&self) -> Option<(Op, usize)> {
		match (self.current_char(), self.peek_char(1)) {
			('+', '+') => Some((Op::Inc, 2)),
			('-', '-') => Some((Op::Dec, 2)),
			('²', _) => Some((Op::Square, 1)),
			('³', _) => Some((Op::Cube, 1)),
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
			'ø' => return Empty,
			// $n parameter reference (e.g., $0 = first param)
			'$' if self.peek_char(1).is_numeric() => {
				self.advance(); // skip '$'
				let mut num_str = String::new();
				while self.current_char().is_numeric() {
					num_str.push(self.current_char());
					self.advance();
				}
				Node::Symbol(format!("${}", num_str))
			}
			ch if ch.is_numeric() || (ch == '-' && self.peek_char(1).is_numeric()) => {
				self.parse_number()
			}
			ch if ch.is_alphabetic() || ch == '_' => self.parse_symbol_with_suffix(),
			ch => {
				warn!(
					"Unexpected character '{}' at line {}, column {}",
					ch, line_nr, column
				);
				self.advance();
				error(&format!("Unexpected character '{}'", ch))
			}
		};

		if node == Empty {
			return node;
		}

		// Attach metadata
		let node = node.with_meta_data(LineInfo {
			line_nr,
			column,
			line: self.current_line.clone(),
		});
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
			Err(e) => return error(&e),
		};

		// Check for URL pattern: scheme://...
		// Common schemes: http, https, ftp, file, data, ws, wss
		if matches!(symbol.as_str(), "http" | "https" | "ftp" | "file" | "data" | "ws" | "wss")
			&& self.current_char() == ':'
			&& self.peek_char(1) == '/'
			&& self.peek_char(2) == '/'
		{
			// Parse entire URL as a single token
			let mut url = symbol;
			// Consume :// and the rest of the URL
			while !self.is_url_terminator(self.current_char()) {
				url.push(self.current_char());
				self.advance();
			}
			return Node::Text(url);
		}

		if let Some(constant) = check_constants(&symbol) {
			return constant; // if true {} fall through :?
		}

		// Handle "global" keyword: global name = value
		if symbol == "global" {
			self.skip_whitespace();
			// Parse the rest as an expression (should be name=value or name:=value)
			let decl = self.parse_expr(0);
			return Node::Key(Box::new(Symbol("global".to_string())), Op::Colon, Box::new(decl));
		}

		// Handle "class"/"struct"/"type" keyword: class Name { fields }
		// But NOT type(x) which is a function call for type introspection
		if symbol == "class" || symbol == "struct" || (symbol == "type" && self.current_char() != '(') {
			self.skip_whitespace();
			let type_name = match self.parse_symbol() {
				Ok(s) => s,
				Err(e) => return error(&e),
			};
			self.skip_whitespace();
			let body = if self.current_char() == '{' {
				let block = self.parse_bracketed('{');
				// Transform field values from Symbol to Type nodes
				Self::transform_fields_to_types(block)
			} else {
				Empty
			};
			return Node::Type {
				name: Box::new(Symbol(type_name)),
				body: Box::new(body),
			};
		}

		// Check for IMMEDIATE suffix blocks (no space allowed)
		// This distinguishes List<int> (generic) from x < y (comparison)
		let ch = self.current_char();
		match ch {
			'{' => {
				let block = self.parse_bracketed('{');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(block))
			}
			'<' if !self.options.xml_mode && !self.peek_char(1).is_numeric() => {
				// Only treat as generic if immediately after symbol (no space)
				// and NOT followed by a number (that would be comparison: i<9)
				let generic = self.parse_bracketed('<');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(generic))
			}
			'(' => {
				// Parse arguments as a proper Node
				let args_node = self.parse_bracketed('(');
				self.skip_spaces(); // Only spaces, preserve newlines as statement separators

				if self.current_char() == '{' {
					// Function with body: name(params) { body }
					let body = self.parse_bracketed('{');
					let signature = Node::List(
						vec![Symbol(symbol), args_node],
						Bracket::Round,
						Separator::None,
					);
					Node::List(vec![signature, body], Bracket::Round, Separator::None)
				} else {
					// Function call: name(params) -> List([symbol, args...])
					let mut items = vec![Symbol(symbol)];
					match args_node {
						Node::List(args, _, _) => items.extend(args),
						Node::Empty => {}
						other => items.push(other),
					}
					Node::List(items, Bracket::Round, Separator::None)
				}
			}
			_ => Node::symbol(&symbol),
		}
	}

	/// Helper to advance by N characters
	fn advance_by(&mut self, n: usize) {
		for _ in 0..n {
			self.advance();
		}
	}

	/// Pratt parser: parse expression with given minimum binding power
	/// Handles prefix, infix, and suffix operators
	fn parse_expr(&mut self, min_bp: u8) -> Node {
		self.skip_spaces();

		// Step 1: Handle prefix operators
		let mut lhs = if let Some((op, chars)) = self.peek_prefix_operator() {
			// Check if this is really a prefix (not infix like x - y)
			// Prefix operators should be at start or after another operator
			self.advance_by(chars);
			self.skip_spaces();
			let (_, r_bp) = op.binding_power();
			let rhs = self.parse_expr(r_bp);

			// Special handling for `if condition { block } [else { block }]`
			if op == Op::If {
				self.skip_spaces();
				if self.current_char() == '{' {
					let then_block = self.parse_atom(); // parse { block }
					self.skip_spaces();
					if self.matches_keyword("else") {
						self.advance_by(4); // skip "else"
						self.skip_spaces();
						let else_block = self.parse_atom(); // parse { block }
						// Structure: ((if condition) then then_block) else else_block
						let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(rhs));
						let if_then =
							Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
						Node::Key(Box::new(if_then), Op::Else, Box::new(else_block))
					} else {
						// Just if condition { block } - no else
						let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(rhs));
						Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block))
					}
				} else if let Node::Key(cond, Op::Colon, then_expr) = &rhs {
					// if cond:then - convert colon to then structure
					self.skip_spaces();
					if self.matches_keyword("else") {
						self.advance_by(4); // skip "else"
						self.skip_spaces();
						let else_expr = self.parse_expr(0);
						// Structure: ((if condition) then then_expr) else else_expr
						let if_cond = Node::Key(Box::new(Empty), Op::If, cond.clone());
						let if_then =
							Node::Key(Box::new(if_cond), Op::Then, then_expr.clone());
						Node::Key(Box::new(if_then), Op::Else, Box::new(else_expr))
					} else {
						// Just if cond:then - no else
						let if_cond = Node::Key(Box::new(Empty), Op::If, cond.clone());
						Node::Key(Box::new(if_cond), Op::Then, then_expr.clone())
					}
				} else if let Node::List(items, _, _) = rhs.drop_meta() {
					// Handle implicit application: if cond {block} parsed as List([cond, {block}])
					if items.len() == 2 {
						if let Node::List(_, Bracket::Curly, _) = items[1].drop_meta() {
							let condition = items[0].clone();
							let then_block = items[1].clone();
							self.skip_spaces();
							let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(condition));
							let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
							return if self.matches_keyword("else") {
								self.advance_by(4);
								self.skip_spaces();
								Node::Key(Box::new(if_then), Op::Else, Box::new(self.parse_atom()))
							} else {
								if_then
							};
						}
					}
					Node::Key(Box::new(Empty), op, Box::new(rhs))
				} else {
					Node::Key(Box::new(Empty), op, Box::new(rhs))
				}
			} else if op == Op::While {
				// Special handling for `while condition { block }` or `while condition do body`
				self.skip_spaces();
				if self.current_char() == '{' {
					let body_block = self.parse_atom(); // parse { block }
					// Structure: (while condition) do body
					let while_cond = Node::Key(Box::new(Empty), Op::While, Box::new(rhs));
					Node::Key(Box::new(while_cond), Op::Do, Box::new(body_block))
				} else if let Node::List(items, _, _) = rhs.drop_meta() {
					// Handle implicit application: while(cond){block} parsed as List([(cond), {block}])
					if items.len() == 2 {
						if let Node::List(_, Bracket::Curly, _) = items[1].drop_meta() {
							let condition = items[0].clone();
							let body_block = items[1].clone();
							let while_cond = Node::Key(Box::new(Empty), Op::While, Box::new(condition));
							return Node::Key(Box::new(while_cond), Op::Do, Box::new(body_block));
						}
					}
					Node::Key(Box::new(Empty), op, Box::new(rhs))
				} else {
					Node::Key(Box::new(Empty), op, Box::new(rhs))
				}
			} else {
				Node::Key(Box::new(Empty), op, Box::new(rhs))
			}
		} else {
			self.parse_atom()
		};

		loop {
			self.skip_spaces(); // Only spaces, not newlines (newlines are separators)

			// Step 2: Check for suffix operators first (they bind tightest)
			if let Some((op, chars)) = self.peek_suffix_operator() {
				let (l_bp, _) = op.binding_power();
				if l_bp >= min_bp {
					self.advance_by(chars);
					lhs = Node::Key(Box::new(lhs), op, Box::new(Empty));
					continue;
				}
			}

			// Step 2b: Check for subscript access [index] or [i,j,...] - binds tight like Op::Hash
			// binding power 170 matches Op::Hash
			// Comma-index syntax: a[i,j] means a[i][j] (nested indexing)
			if self.current_char() == '[' && min_bp <= 170 {
				self.advance(); // skip '['
				self.skip_whitespace();

				// Collect all comma-separated indices
				let mut indices = vec![self.parse_expr(0)];
				self.skip_whitespace();

				while self.current_char() == ',' {
					self.advance(); // skip ','
					self.skip_whitespace();
					indices.push(self.parse_expr(0));
					self.skip_whitespace();
				}

				if self.current_char() == ']' {
					self.advance(); // skip ']'
					// Chain indexing operations for each index
					// [i,j] becomes ((lhs#(i+1))#(j+1))
					for index in indices {
						let index_unwrapped = index.drop_meta();
						let adjusted_index = match index_unwrapped {
							Node::Number(n) => Node::Number(*n + crate::extensions::numbers::Number::Int(1)),
							_ => Node::Key(Box::new(index), Op::Add, Box::new(Node::Number(crate::extensions::numbers::Number::Int(1)))),
						};
						lhs = Node::Key(Box::new(lhs), Op::Hash, Box::new(adjusted_index));
					}
					continue;
				}
			}

			// Step 3: Check for infix operator
			let (op, chars) = match self.peek_operator() {
				Some(pair) => pair,
				None => {
					// Step 3b: Check for implicit function application (space between atoms)
					// This makes `x = f y` parse as `x = (f y)` and `252 > f y` as `252 > (f y)`
					// Only apply when:
					// - min_bp > 0 (not at top level)
					// - min_bp <= 130 (inside assignment/comparison/arithmetic)
					// - lhs is a Symbol or a function call (List with Bracket::None)
					// - NOT parenthesized expressions (List with Bracket::Round)
					// - Argument conditions vary by context:
					//   - In assignment (min_bp <= 60): allow ANY argument (including identifiers)
					//   - In colon/comparison (min_bp > 60): only non-identifier args
					//   This allows `fetch url` but prevents `a: b c d` from chaining
					const APPLICATION_BP: u8 = 165;
					const MAX_BP_FOR_APPLICATION: u8 = 130;
					let lhs_is_callable = match lhs.drop_meta() {
						Node::Symbol(_) => true,
						Node::List(_, Bracket::None, _) => true, // Function call from implicit application
						_ => false,
					};
					let ch = self.current_char();
					let in_assignment_context = min_bp <= 60; // Assignment r_bp is 59
					let arg_is_non_identifier = ch.is_numeric() || ch == '"' || ch == '\'' || ch == '(' || ch == '[' || ch == '{' || ch == '-';
					let should_apply = in_assignment_context || arg_is_non_identifier;
					if min_bp > 0 && min_bp <= MAX_BP_FOR_APPLICATION && lhs_is_callable && self.can_start_atom() && should_apply {
						// Parse argument with high binding power (tighter than application itself)
						let arg = self.parse_expr(APPLICATION_BP + 1);
						if arg != Empty {
							// Create function application as List[func, arg]
							lhs = Node::List(
								vec![lhs, arg],
								Bracket::None,
								Separator::Space,
							);
							continue;
						}
					}
					break;
				}
			};

			let (l_bp, r_bp) = op.binding_power();

			// Stop if operator binds less tightly than our minimum
			if l_bp < min_bp {
				break;
			}

			// Consume the operator
			self.advance_by(chars);
			self.skip_whitespace();

			// Parse right-hand side with appropriate binding power
			let rhs = self.parse_expr(r_bp);

			// Build the Key node
			lhs = Node::Key(Box::new(lhs), op, Box::new(rhs));
		}

		lhs
	}

	/// Parse a complete value/expression - calls parse_expr(0) for operator chaining
	fn parse_value(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();

		// Capture position before parsing
		let (line_nr, column) = self.get_position();
		if self.is_at_line_end() {
			return Empty;
		};

		let ch = self.current_char();

		// Handle special non-expression cases first
		let node = match ch {
			';' => return Empty, // Semicolons handled by main parse loop
			'>' => return Empty, // Closing bracket handled by parse_bracketed
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			// Everything else goes through parse_expr for operator chaining
			_ => self.parse_expr(0),
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
		let is_double_quote = quote == '"';
		self.advance(); // skip opening quote

		let mut s = String::new();
		loop {
			let ch = self.current_char();
			if ch == '\0' {
				return error("Unterminated string");
			}
			if ch == quote {
				self.advance(); // skip closing quote
				// Hint for double quotes (only for multi-char strings)
				if is_double_quote && s.len() > 1 {
					norm::double_quotes(&s);
				}
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
					if !ch.is_ascii_hexdigit() {
						break;
					}
					hex_str.push(ch);
					self.advance();
				}
				return i64::from_str_radix(&hex_str, 16)
					.map(Node::int)
					.unwrap_or_else(|_| error(&format!("Invalid hex: 0x{}", hex_str)));
			}
		}

		loop {
			let ch = self.current_char();
			// '三'.is_numeric() is true but not ASCII
			// Exclude ² and ³ as they are suffix operators, not part of the number
			if ch.is_numeric() && ch != '²' && ch != '³' {
				num_str.push(ch);
				self.advance();
			} else if ch == '.' && !has_dot && self.peek_char(1) != '.' {
				// Only consume . as decimal if NOT followed by another . (range operator)
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
				.unwrap_or_else(|_| error(&format!("Invalid float: {}", num_str)))
		} else {
			num_str
				.parse::<i64>()
				.map(Node::int)
				.unwrap_or_else(|_| error(&format!("Invalid int: {}", num_str)))
		}
	}

	fn parse_symbol(&mut self) -> Result<String, String> {
		let mut symbol = String::new();
		loop {
			let ch = self.current_char();
			// Include alphanumeric, underscore, and hyphen (for kebab-case)
			// BUT: don't include hyphen if followed by a digit (that's subtraction)
			let is_hyphen_in_symbol = ch == '-' && !self.peek_char(1).is_numeric();
			if (ch.is_alphanumeric() && ch != '²' && ch != '³') || ch == '_' || is_hyphen_in_symbol {
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
			return error(&format!("Unmatched closing tag </{}>", tag_name));
		}

		// Parse tag name
		let tag_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(e) => return error(&e),
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
						Err(_) => Empty,
					}
				};

				// Store attribute as dotted key
				attributes.push(key_ops(attr_name, Op::Assign, attr_value));
				// attributes.push(Node::Key(Box::new(Symbol(format!(".{}", attr_name))), Op::Assign, Box::new(attr_value)));
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
				Node::Key(Box::new(Symbol(tag_name)), Op::Colon, Box::new(Empty))
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
					return error(&format!(
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
			Node::Key(
				Box::new(Symbol(tag_name.clone())),
				Op::Colon,
				Box::new(Empty),
			)
		} else if body_items.len() == 1 {
			Node::Key(
				Box::new(Symbol(tag_name)),
				Op::Colon,
				Box::new(body_items.into_iter().next().unwrap()),
			)
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
			let item = if had_newline && line_indent > self.base_indent && bracket == Bracket::None
			{
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

	/// Transform field definitions: Key(name, op, Symbol) -> Key(name, op, Type)
	/// Used for class/struct definitions to convert type names to Type nodes
	fn transform_fields_to_types(node: Node) -> Node {
		match node {
			Node::List(items, bracket, sep) => {
				let transformed: Vec<Node> = items.into_iter().map(Self::transform_fields_to_types).collect();
				Node::List(transformed, bracket, sep)
			}
			Node::Key(name, op, value) => {
				let type_node = Self::symbol_to_type(*value);
				Node::Key(name, op, Box::new(type_node))
			}
			Node::Meta { node, data } => {
				Node::Meta { node: Box::new(Self::transform_fields_to_types(*node)), data }
			}
			other => other,
		}
	}

	/// Convert a Symbol to a Type node (for type references)
	fn symbol_to_type(node: Node) -> Node {
		match node {
			Node::Symbol(s) => Node::Type {
				name: Box::new(Node::Symbol(s)),
				body: Box::new(Empty),
			},
			Node::Meta { node, data } => {
				Node::Meta { node: Box::new(Self::symbol_to_type(*node)), data }
			}
			other => other,
		}
	}
}

fn check_constants(s: &str) -> Option<Node> {
	match s.to_lowercase().as_str() {
		"⊤" | "true" | "yes" | "✓" | "🗸" | "✔" | "✓️" | "🗹" | "☑" | "✅" | "⊨" => Some(Node::True),
		"⊥" | "false" | "no" | "⊭" | "❌" | "" => Some(Node::False),
		"ø" | "null" | "nul" | "none" | "nil" | "nill" | "nix" | "nada" | "nothing" | "empty" | "void" => Some(Empty),
		"π" | "pi" => Some(float(std::f64::consts::PI)),
		"τ" | "tau" => Some(float(std::f64::consts::TAU)),
		"euler" | "ℯ" => Some(float(std::f64::consts::E)),
		"⚠️" | "⚡" | "⚡️" => Some(error(s)),
		_ => None,
	}
}
// Tests moved to tests/test_parser.rs
