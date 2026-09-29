use crate::diagnostic::Diagnostic;
use crate::extensions::numbers::Number;
use crate::extensions::strings::StringExtensions;
use crate::meta::LineInfo;
use crate::node::Node::{Empty, Symbol};
use crate::extensions::reals::{Exact, Rational, Real};
use crate::node::{error, key_ops, Bracket, Node, Separator};
use crate::operators::Op;
use crate::normalize::{hints as norm, set_hint_position};
use crate::*;
use log::warn;
use std::fs::read_to_string;
use unicode_normalization::UnicodeNormalization;

/// Largest exponent written out as an exact integer literal (1e4096 has 4097 digits)
const MAX_INTEGER_EXPONENT: i64 = 4096;
/// Literal suffixes of C, Java and C#, a tight conversion: `0.1f`, `0.1d` (double) → float; `0.1l` (long double) → exact, the default anyway
const LITERAL_SUFFIXES: [(char, &str); 6] = [('f', "float"), ('F', "float"), ('d', "float"), ('D', "float"), ('l', "exact"), ('L', "exact")];
/// `0.1:float`, `1.5:int`: a number literal directly typed with one of these binds tightly, unlike the loose `as`
const LITERAL_NUMBER_TYPES: [&str; 11] = ["int", "i64", "integer", "exact", "real", "float", "fast", "f64", "double", "f32", "i32"];

/// Superscript digits ⁰…⁹ in digit order: `3⁴` is 3^4 and a run of them is one exponent (`2¹⁰` is 2^10)
const SUPERSCRIPT_DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
/// A vulgar fraction such as ⅓ decomposes (NFKD) into numerator, this slash and denominator: 1⁄3
const FRACTION_SLASH: char = '⁄';

/// Parser options for handling different file formats
#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(Default)]
pub struct ParserOptions {
	/// XML mode: treat <tag> as XML tags, not C++ generics
	pub xml_mode: bool,
	/// Data mode (untrusted or foreign data, never evaluated): English words stay symbols
	/// (`NO` is Norway, not false) and numbers keep their source literal (`01234`, `1.10`)
	pub data_mode: bool,
	/// WIT syntax: `name<a, b>` applies a type (`tuple<s64, s64>`), `struct`/`type` are plain words
	pub wit_mode: bool,
}


impl ParserOptions {
	pub fn wit() -> Self {
		ParserOptions { wit_mode: true, ..Default::default() }
	}

	pub fn xml() -> Self {
		ParserOptions { xml_mode: true, ..Default::default() }
	}

	pub fn data() -> Self {
		ParserOptions { data_mode: true, ..Default::default() }
	}
}

fn superscript_digit(ch: char) -> Option<i64> {
	SUPERSCRIPT_DIGITS.chars().position(|digit| digit == ch).map(|position| position as i64)
}

/// Numerator and denominator of a Unicode vulgar fraction character: ½ → (1, 2), ⅚ → (5, 6), ↉ → (0, 3)
fn vulgar_fraction(ch: char) -> Option<(i64, i64)> {
	let decomposed: String = std::iter::once(ch).nfkd().collect();
	let (numerator, denominator) = decomposed.split_once(FRACTION_SLASH)?;
	Some((numerator.parse().ok()?, denominator.parse().ok()?))
}

/// Characters that are numeric for Unicode but written next to a number as operators or literals of their own
fn is_number_glyph(ch: char) -> bool {
	superscript_digit(ch).is_some() || vulgar_fraction(ch).is_some()
}

fn is_identifier_char(c: char) -> bool {
	c.is_alphanumeric() || c == '_'
}

/// Read and parse a WASP file
pub fn parse_file(path: &str) -> Node {
	let options = if path.ends_with(".wit") { ParserOptions::wit() } else { ParserOptions::default() };
	match read_to_string(path) {
		Ok(content) => WaspParser::parse_with_options(&content, options),
		_ => error(&format!("Failed to read {}", path)),
	}
}

pub fn parse(input: &str) -> Node {
	if input.ends_with(".wasp") {
		return parse_file(input);
	}
	WaspParser::parse(input)
}

/// The number a text spells if the whole text is one number literal, so `int "12a"` never guesses
pub fn number_in_text(text: &str) -> Option<Number> {
	let text = text.trim();
	let spells_number = !text.is_empty() && text.chars().all(|c| c.is_ascii_digit() || "+-._eE".contains(c));
	if !spells_number {
		return None;
	}
	match WaspParser::parse(text).drop_meta() {
		Node::Number(number) => Some(*number),
		_ => None,
	}
}

/// Parse-only path for untrusted data: nothing is evaluated, no word is guessed to be a boolean
pub fn parse_data(input: &str) -> Node {
	WaspParser::parse_with_options(input, ParserOptions::data())
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
	/// Inside an `if`/`while` condition `=` compares instead of assigning (wiki/Bad.md)
	equals_compares: bool,
	/// Names defined with `:=` so far: a braceless call of one may take an identifier argument anywhere (`fac it-1`)
	functions: std::collections::HashSet<String>,
	/// Those of them declared with named parameters (`f x y := …`), the rest take the implicit `it`
	functions_with_parameters: std::collections::HashSet<String>,
}

enum ElseParseMode {
	Atom,
	Expr,
}

impl WaspParser {
	pub fn new(input: String) -> Self {
		Self::new_with_options(input, ParserOptions::default())
	}

	/// Source text is normalized to NFC, so equal-looking text and identifiers are equal
	pub fn new_with_options(input: String, options: ParserOptions) -> Self {
		let input: String = input.nfc().collect();
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
			equals_compares: false,
			functions: Default::default(),
			functions_with_parameters: Default::default(),
		}
	}

	/// Set the hint position to current parser position
	fn set_hint_pos(&self) {
		set_hint_position(self.line_nr, self.column);
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
			&& !is_identifier_char(self.peek_char(keyword.len()))
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

			// # line comment (shell-style) - only at line start; `#(…)` counts, as `#x` does after a statement
			if c1 == '#' && c2 != '(' && self.is_at_line_start() {
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

	/// A `;` that ends its line is a statement terminator of the same strength as the newline after it
	fn only_blanks_before_newline(&self) -> bool {
		self.chars[self.pos..].iter().find(|ch| !matches!(ch, ' ' | '\t' | '\r')) == Some(&'\n')
	}

	/// Check if current character can start an atom (for implicit application)
	fn can_start_atom(&self) -> bool {
		let ch = self.current_char();
		ch.is_alphanumeric() || ch == '_' || ch == '"' || ch == '\'' || ch == '(' || ch == '[' || ch == '{'
			|| self.number_starts_at(0)
	}

	/// A digit, or a leading-dot decimal like `.5`
	fn number_starts_at(&self, offset: usize) -> bool {
		let ch = self.peek_char(offset);
		ch.is_ascii_digit() || (ch == '.' && self.peek_char(offset + 1).is_ascii_digit())
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
			('*', '*') => { self.set_hint_pos(); norm::power_operator("**"); return Some((Op::Pow, 2)); }
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
			('&', '&') => { self.set_hint_pos(); norm::and_operator("&&"); return Some((Op::And, 2)); }
			('|', '|') => { self.set_hint_pos(); norm::or_operator("||"); return Some((Op::Or, 2)); }
			_ => {}
		}
		// Keywords (2-char)
		if self.matches_keyword("or") { return Some((Op::Or, 2)); }
		if self.matches_keyword("is") { return Some((Op::Eq, 2)); } // wiki/equality.md: `is` compares by value like ==
		if self.matches_keyword("if") { return Some((Op::If, 2)); }
		if self.matches_keyword("do") { return Some((Op::Do, 2)); }
		if self.matches_keyword("to") { return Some((Op::To, 2)); }

		// 1-char operators
		match c1 {
			':' => Some((Op::Colon, 1)),
			'=' if self.equals_compares => Some((Op::Eq, 1)),
			'=' => Some((Op::Assign, 1)),
			// `a .5` is a list of two values, `a.5` a member access
			'.' if !(self.prev_char().is_whitespace() && self.number_starts_at(0)) => Some((Op::Dot, 1)),
			'+' => Some((Op::Add, 1)),
			'-' => Some((Op::Sub, 1)),
			'*' => Some((Op::Mul, 1)),
			'/' if c2 != '/' => Some((Op::Div, 1)), // Don't treat // as division - it's a comment
			'%' => Some((Op::Mod, 1)),
			'^' => Some((Op::Pow, 1)),
			'×' | '⋅' => Some((Op::Mul, 1)),
			'÷' => Some((Op::Div, 1)),
			'<' | '>' if self.options.wit_mode => None, // angle brackets only delimit type arguments
			'<' => Some((Op::Lt, 1)),
			'>' => Some((Op::Gt, 1)),
			'≤' => Some((Op::Le, 1)),
			'≥' => Some((Op::Ge, 1)),
			'≠' => Some((Op::Ne, 1)),
			'!' => { self.set_hint_pos(); norm::not_operator("!"); Some((Op::Not, 1)) }
			'¬' => Some((Op::Not, 1)),
			'&' => { self.set_hint_pos(); norm::and_operator("&"); Some((Op::And, 1)) }
			'|' => { self.set_hint_pos(); norm::or_operator("|"); Some((Op::Or, 1)) }
			'∧' => Some((Op::And, 1)),
			'⋁' => Some((Op::Or, 1)),
			'⊻' => Some((Op::Xor, 1)),
			'#' => Some((Op::Hash, 1)),
			'?' => { self.set_hint_pos(); norm::conditional(true); Some((Op::Question, 1)) }
			'…' => Some((Op::To, 1)),
			_ => None,
		}
	}

	/// Peek for prefix operators (unary operators that bind to right operand)
	fn peek_prefix_operator(&self) -> Option<(Op, usize)> {
		if self.matches_keyword("while") { return Some((Op::While, 5)); }
		if self.matches_keyword("sqrt") { return Some((Op::Sqrt, 4)); }
		if self.matches_keyword("cbrt") { return Some((Op::Cbrt, 4)); }
		if self.matches_keyword("not") { return Some((Op::Not, 3)); }
		if self.matches_keyword("abs") { return Some((Op::Abs, 3)); }
		if self.matches_keyword("if") { return Some((Op::If, 2)); }

		let (c1, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		let variable_follows = c3.is_alphabetic() || c3 == '_';
		match c1 {
			'+' if c2 == '+' && variable_follows => Some((Op::Inc, 2)),
			'-' if c2 == '-' && variable_follows => Some((Op::Dec, 2)),
			'-' => Some((Op::Neg, 1)),
			'!' | '¬' => Some((Op::Not, 1)),
			'√' => Some((Op::Sqrt, 1)),
			'∛' => Some((Op::Cbrt, 1)),
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
			_ => None,
		}
	}

	/// `$main`, `$ii_i`: names keep their sigil (WAT identifiers, DOM selectors)
	fn parse_dollar_name(&mut self) -> Node {
		self.advance(); // skip '$'
		match self.parse_symbol() {
			Ok(name) => Symbol(format!("${name}")),
			Err(message) => error(&message),
		}
	}

	fn unwrap_single(group: Node) -> Node {
		match group {
			Node::List(mut items, _, _) if items.len() == 1 => items.remove(0),
			other => other,
		}
	}

	/// `@name` or `@name(value)` annotates the atom that follows: `@version(2) @draft tee{a:1}`
	fn parse_attribute(&mut self) -> Node {
		self.advance(); // skip '@'
		let name = match self.parse_symbol() {
			Ok(name) => name,
			Err(message) => return error(&message),
		};
		let value = if self.current_char() == '(' { Self::unwrap_single(self.parse_bracketed('(')) } else { Node::True };
		self.parse_atom().with_attribute(&name, value)
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
			';' | '>' | '}' | ')' | ']' => Empty, // Closing brackets/terminators handled by caller
			'ø' => { self.advance(); return Empty }
			// $n parameter reference (e.g., $0 = first param)
			'@' if self.peek_char(1).is_alphabetic() => self.parse_attribute(),
			'$' if self.peek_char(1).is_alphabetic() || self.peek_char(1) == '_' => self.parse_dollar_name(),
			'$' if self.peek_char(1).is_numeric() => {
				self.advance(); // skip '$'
				let mut num_str = String::new();
				while self.current_char().is_numeric() {
					num_str.push(self.current_char());
					self.advance();
				}
				Node::Symbol(format!("${}", num_str))
			}
			ch if ch.is_numeric() || self.number_starts_at(0) || (ch == '-' && self.number_starts_at(1)) => {
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
			#[cfg(debug_assertions)]
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

		if let Some(constant) = check_constants(&symbol, self.options.data_mode) {
			return constant; // if true {} fall through :?
		}

		// Optional type: `x:int?=ø`, `f(x:int?)` (wiki/null.md); a ternary `?` is followed by its branch instead
		if self.current_char() == '?' && self.ends_optional_type(self.peek_char(1), self.peek_char(2)) {
			self.advance();
			return Symbol(format!("{symbol}?"));
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
		if self.options.wit_mode && self.current_char() == '<' {
			return self.parse_type_application(symbol);
		}
		if !self.options.wit_mode && (symbol == "class" || symbol == "struct" || (symbol == "type" && self.current_char() != '(')) {
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
		const ARGUMENT_BP: u8 = 140; // a braceless argument takes arithmetic, stops at ranges and comparisons: f 3-1 > 5
		const MAX_BP_FOR_APPLICATION: u8 = 151; // operand of + - * / takes a braceless call: 1 + f 3
		const SUBSCRIPT_BP: u8 = 170; // Matches Op::Hash

		self.skip_spaces();

		// Step 1: Prefix (nud)
		let mut lhs = if let Some((op, chars)) = self.peek_prefix_operator() {
			self.advance_by(chars);
			self.skip_spaces();
			if chars == 1 && op == Op::Abs {
				self.parse_norm_bars()
			} else {
				let rhs = self.parse_prefix_operand(op);
				self.finish_prefix(op, rhs)
			}
		} else {
			self.parse_atom()
		};

		// Right operand of the last comparison, to chain a<b<c into a<b and b<c
		let mut previous_comparand: Option<Node> = None;
		loop {
			self.skip_spaces(); // Only spaces, not newlines (newlines are separators)

			// Step 2: Suffix (led)
			if let Some(updated) = self.try_parse_suffix(&lhs, min_bp) {
				lhs = updated;
				continue;
			}

			// Step 2b: Subscript (tight, like Op::Hash)
			if let Some(updated) = self.try_parse_subscript(&lhs, min_bp, SUBSCRIPT_BP) {
				lhs = updated;
				continue;
			}

			// Step 3a: `a mod b` is `a % b` (Euclidean, 0 ≤ r < |b|), `a rem b` the truncated remainder (sign of the dividend, as C)
			let keyword_op = if self.matches_keyword("mod") { Some(Op::Mod) } else if self.matches_keyword("rem") { Some(Op::Rem) } else { None };
			if let Some(op) = keyword_op {
				let (l_bp, r_bp) = op.binding_power();
				if l_bp < min_bp {
					break;
				}
				self.advance_by(3);
				self.skip_whitespace();
				let divisor = self.parse_expr(r_bp);
				lhs = Node::Key(Box::new(lhs), op, Box::new(divisor));
				previous_comparand = None;
				continue;
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
					if let Some(updated) = self.try_parse_implicit_application(
						&lhs,
						min_bp,
						ARGUMENT_BP,
						MAX_BP_FOR_APPLICATION,
					) {
						lhs = updated;
						continue;
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
			let (op_line, op_column) = self.get_position();
			let bare_symbol = Some(self.current_char()).filter(|symbol| chars == 1 && matches!(symbol, '&' | '|'));
			self.advance_by(chars);
			self.skip_whitespace();
			if op == Op::Define {
				if let Some(name) = defined_function_name(&lhs) {
					if matches!(lhs.drop_meta(), Node::List(..)) {
						self.functions_with_parameters.insert(name.clone());
					}
					self.functions.insert(name);
				}
			}

			// Parse right-hand side with appropriate binding power
			let rhs = if op == Op::Colon {
				self.with_equals_comparing(false, |parser| parser.parse_expr(r_bp))
			} else {
				self.parse_expr(r_bp)
			};

			if op == Op::Define && !matches!(lhs.drop_meta(), Node::List(..)) && !mentions(&rhs, "it") {
				self.functions.remove(&lhs.name()); // `x := 5` defines a value, not a function
			}

			if let Some(symbol) = bare_symbol {
				if let Some(diagnostic) = logic_mixed_with_comparison(&lhs, symbol, &rhs) {
					lhs = Diagnostic { line: op_line, column: op_column, ..diagnostic }.into_error();
					previous_comparand = None;
					continue;
				}
			}

			if op.is_equality() && ungrouped_equality(&lhs) {
				lhs = Diagnostic { line: op_line, column: op_column, ..chained_equality(&lhs, op, &rhs) }.into_error();
				previous_comparand = None;
				continue;
			}

			lhs = match previous_comparand.take() {
				Some(middle) if op.is_ordering() => {
					let next_comparison = Node::Key(Box::new(middle), op, Box::new(rhs.clone()));
					Node::Key(Box::new(lhs), Op::And, Box::new(next_comparison))
				}
				_ => Node::Key(Box::new(lhs), op, Box::new(rhs.clone())),
			};
			if op.is_ordering() {
				previous_comparand = Some(rhs);
			}
		}

		lhs
	}

	/// `‖x‖` brackets a whole expression like parentheses: `‖3-5‖*2` → 4
	fn parse_norm_bars(&mut self) -> Node {
		let inner = self.parse_expr(0);
		self.skip_spaces();
		if self.current_char() != '‖' {
			return error("Missing closing ‖");
		}
		self.advance();
		self.finish_prefix(Op::Abs, inner)
	}

	/// Operand of a prefix operator; `++i` binds like `i++`, conditions compare with `=`
	fn parse_prefix_operand(&mut self, op: Op) -> Node {
		let (left_bp, right_bp) = op.binding_power();
		match op {
			Op::Inc | Op::Dec => self.parse_expr(left_bp),
			Op::If | Op::While => self.with_equals_comparing(true, |parser| parser.parse_expr(right_bp)),
			_ => self.parse_expr(right_bp),
		}
	}

	fn with_equals_comparing(&mut self, compares: bool, parse: impl FnOnce(&mut Self) -> Node) -> Node {
		let outer = std::mem::replace(&mut self.equals_compares, compares);
		let node = parse(self);
		self.equals_compares = outer;
		node
	}

	fn finish_prefix(&mut self, op: Op, rhs: Node) -> Node {
		match (op, rhs.drop_meta()) {
			(Op::If, _) => self.finish_if_prefix(rhs),
			(Op::While, _) => self.finish_while_prefix(rhs),
			(Op::Neg, Node::Number(number)) => Node::Number(-*number),
			(Op::Inc | Op::Dec, _) => Node::Key(Box::new(rhs), op, Box::new(Empty)), // ++i is i++: increment is immediate
			_ => Node::Key(Box::new(Empty), op, Box::new(rhs)),
		}
	}

	fn finish_if_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let then_block = self.parse_atom(); // parse { block }
			let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(rhs));
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
			return self.parse_optional_else(if_then, ElseParseMode::Atom);
		}

		if let Node::Key(cond, Op::Colon, then_expr) = &rhs {
			let if_cond = Node::Key(Box::new(Empty), Op::If, cond.clone());
			let if_then = Node::Key(Box::new(if_cond), Op::Then, then_expr.clone());
			return self.parse_optional_else(if_then, ElseParseMode::Expr);
		}

		if let Node::List(items, _, _) = rhs.drop_meta() {
			if items.len() == 2 {
				if let Node::List(_, Bracket::Curly, _) = items[1].drop_meta() {
					let condition = items[0].clone();
					let then_block = items[1].clone();
					self.skip_spaces();
					let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(condition));
					let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
					return self.parse_optional_else(if_then, ElseParseMode::Atom);
				}
			}
		}

		Node::Key(Box::new(Empty), Op::If, Box::new(rhs))
	}

	/// A statement body follows: not a separator, closing bracket, end of input or the `do` keyword
	fn at_body_start(&self) -> bool {
		!matches!(self.current_char(), '\0' | ';' | ',' | '\n' | '}' | ')' | ']') && !self.matches_keyword("do")
	}

	fn finish_while_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let body_block = self.parse_atom(); // parse { block }
			return while_do(rhs, body_block);
		}

		if let Node::Key(condition, Op::Colon, body) = &rhs {
			return while_do(condition.as_ref().clone(), body.as_ref().clone());
		}

		if matches!(rhs.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1) && self.at_body_start() {
			let body = self.parse_expr(0); // `while (i<9) i++`
			return while_do(rhs, body);
		}

		if let Node::List(items, _, _) = rhs.drop_meta() {
			if items.len() == 2 {
				if let Node::List(_, Bracket::Curly, _) | Empty = items[1].drop_meta() { // `{}` parses as ø
					return while_do(items[0].clone(), items[1].clone());
				}
			}
		}

		Node::Key(Box::new(Empty), Op::While, Box::new(rhs))
	}

	fn parse_optional_else(&mut self, if_then: Node, mode: ElseParseMode) -> Node {
		self.skip_spaces();
		if self.matches_keyword("else") {
			self.advance_by(4);
			self.skip_spaces();
			let else_expr = match mode {
				ElseParseMode::Atom => self.parse_atom(),
				ElseParseMode::Expr => self.parse_expr(0),
			};
			Node::Key(Box::new(if_then), Op::Else, Box::new(else_expr))
		} else {
			if_then
		}
	}

	/// The exponent written in superscript digits at the cursor and its length in characters: ⁴ → (4, 1), ¹² → (12, 2)
	fn superscript_exponent(&self) -> Option<(i64, usize)> {
		let digits: Vec<i64> = self.chars.iter().skip(self.pos).map_while(|ch| superscript_digit(*ch)).collect();
		let exponent = digits.iter().fold(0i64, |exponent, digit| exponent.saturating_mul(10).saturating_add(*digit));
		(!digits.is_empty()).then_some((exponent, digits.len()))
	}

	/// `x⁴` is x^4; the single digits ² and ³ keep their dedicated square and cube operators
	fn try_parse_superscript_power(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let (exponent, length) = self.superscript_exponent()?;
		let (op, right) = match (exponent, length) {
			(2, 1) => (Op::Square, Empty),
			(3, 1) => (Op::Cube, Empty),
			_ => (Op::Pow, Node::int(exponent)),
		};
		if op.binding_power().0 < min_bp {
			return None;
		}
		self.advance_by(length);
		Some(Node::Key(Box::new(lhs.clone()), op, Box::new(right)))
	}

	fn try_parse_suffix(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		if let Some(power) = self.try_parse_superscript_power(lhs, min_bp) {
			return Some(power);
		}
		let (op, chars) = self.peek_suffix_operator()?;
		let (l_bp, _) = op.binding_power();
		if l_bp < min_bp {
			return None;
		}
		self.advance_by(chars);
		Some(Node::Key(Box::new(lhs.clone()), op, Box::new(Empty)))
	}

	fn try_parse_subscript(&mut self, lhs: &Node, min_bp: u8, subscript_bp: u8) -> Option<Node> {
		if self.current_char() != '[' || min_bp > subscript_bp {
			return None;
		}
		self.advance(); // skip '['
		self.skip_whitespace();

		let mut indices = vec![self.parse_expr(0)];
		self.skip_whitespace();

		while self.current_char() == ',' {
			self.advance(); // skip ','
			self.skip_whitespace();
			indices.push(self.parse_expr(0));
			self.skip_whitespace();
		}

		if self.current_char() != ']' {
			return None;
		}
		self.advance(); // skip ']'

		Some(indices.into_iter().fold(lhs.clone(), subscript))
	}

	fn try_parse_implicit_application(
		&mut self,
		lhs: &Node,
		min_bp: u8,
		argument_bp: u8,
		max_bp_for_application: u8,
	) -> Option<Node> {
		let lhs_is_callable = match lhs.drop_meta() {
			Node::Symbol(_) => true,
			Node::List(_, Bracket::None, _) => true, // Function call from implicit application
			_ => false,
		};
		let lhs_is_defined_function = matches!(lhs.drop_meta(), Node::Symbol(name) if self.functions.contains(name));
		let ch = self.current_char();
		let in_assignment_context = min_bp <= 60; // Assignment r_bp is 59
		let arg_is_non_identifier = ch.is_numeric()
			|| self.number_starts_at(0)
			|| ch == '"'
			|| ch == '\''
			|| ch == '('
			|| ch == '['
			|| ch == '{'
			|| ch == '-';
		let should_apply = in_assignment_context || arg_is_non_identifier || lhs_is_defined_function;

		// At statement level a list `f a b` is a call with all its items; a function of the implicit `it` takes one argument,
		// so `f 3-1 > 15` compares `f(3-1)` just like the operand `1 + f 3-1 > 15` does
		let takes_one_argument = lhs_is_defined_function && !self.functions_with_parameters.contains(&lhs.name());
		if (min_bp == 0 && !takes_one_argument) || min_bp > max_bp_for_application {
			return None;
		}
		if !lhs_is_callable || !self.can_start_atom() || !should_apply {
			return None;
		}

		let arg = self.parse_expr(argument_bp);
		if arg == Empty {
			return None;
		}

		Some(Node::List(
			vec![lhs.clone(), arg],
			Bracket::None,
			Separator::Space,
		))
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
			#[cfg(debug_assertions)]
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
		// let is_double_quote = quote == '"';
		let is_single_quote = quote == '\'';
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
				if is_single_quote && s.len() > 1 {
					self.set_hint_pos();
					norm::single_quotes(&s);
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
		let start = self.pos;
		let number = self.parse_number_value();
		let literal: String = self.chars[start..self.pos].iter().collect();
		let number = self.keep_source_literal(number, &literal);
		match self.literal_type_suffix() {
			Some(number_type) => Node::Key(Box::new(number), Op::As, Box::new(Node::Symbol(number_type))),
			None => number,
		}
	}

	/// Tight conversion written on the literal: `0.1f` → float, `0.1l` → exact, `0.1:float` / `1.5:int` → that type.
	/// Code only: in data `version: 1.10` and `n:int` stay key/value.
	fn literal_type_suffix(&mut self) -> Option<String> {
		if self.options.data_mode {
			return None;
		}
		let suffix = LITERAL_SUFFIXES.iter().find(|(letter, _)| *letter == self.current_char());
		if let Some((_, number_type)) = suffix.filter(|_| !is_identifier_char(self.peek_char(1))) {
			self.advance();
			return Some(number_type.to_string());
		}
		if self.current_char() != ':' {
			return None;
		}
		let name: String = self.chars[self.pos + 1..].iter().take_while(|c| is_identifier_char(**c)).collect();
		if !LITERAL_NUMBER_TYPES.contains(&name.as_str()) {
			return None;
		}
		self.advance_by(1 + name.chars().count());
		Some(name)
	}

	/// Data round-trips its literals: `zip: 01234` stays `01234`, `version: 1.10` stays `1.10`
	fn keep_source_literal(&self, number: Node, literal: &str) -> Node {
		let is_lossy = matches!(number, Node::Number(_)) && number.serialize() != literal;
		if self.options.data_mode && is_lossy {
			number.with_source_literal(literal)
		} else {
			number
		}
	}

	fn parse_number_value(&mut self) -> Node {
		// RFC 3339 / RFC 9557 literal: 2024-01-31, 2024-01-31T10:00Z, 2024-01-31T10:00[Europe/Berlin]
		// Not in data mode yet: the data path round-trips source text and has no date serialization
		let literal_len = if self.options.data_mode { 0 } else { crate::time::literal_len(&self.chars[self.pos..]) };
		if literal_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + literal_len].iter().collect();
			self.advance_by(literal_len);
			return Node::data(crate::time::TimeLiteral(literal));
		}
		let mut num_str = String::new();

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

		if let Some(fraction) = self.vulgar_fraction_literal() {
			return fraction;
		}

		self.push_digits(&mut num_str);
		let mut is_float = false;
		// Only consume . as decimal if NOT followed by another . (range operator)
		if self.current_char() == '.' && self.peek_char(1) != '.' {
			is_float = true;
			num_str.push('.');
			self.advance();
			self.push_digits(&mut num_str);
		}
		let exponent = match self.parse_exponent() {
			Ok(exponent) => exponent,
			Err(e) => return error(&e),
		};

		match exponent {
			// 1e3 is the exact integer 1000, like the literal it abbreviates
			Some(exp) if !is_float && exp >= 0 => {
				if exp > MAX_INTEGER_EXPONENT {
					return error(&format!("Exponent too large: {}e{}", num_str, exp));
				}
				num_str.push_str(&"0".repeat(exp as usize));
				self.integer_node(&num_str)
			}
			Some(exp) => format!("{}e{}", num_str, exp)
				.parse::<f64>()
				.map(Node::float)
				.unwrap_or_else(|_| error(&format!("Invalid float: {}e{}", num_str, exp))),
			None if is_float => num_str
				.parse::<f64>()
				.map(Node::float)
				.unwrap_or_else(|_| error(&format!("Invalid float: {}", num_str))),
			None => {
				let number = self.integer_node(&num_str);
				self.with_time_unit(number)
			}
		}
	}

	/// `⅓` is the exact ratio 1/3; a number written directly after it multiplies: `⅓9` is 3
	fn vulgar_fraction_literal(&mut self) -> Option<Node> {
		let (numerator, denominator) = vulgar_fraction(self.current_char())?;
		self.advance();
		let ratio = Number::ratio(Number::Int(numerator), Number::Int(denominator));
		let fraction = Node::Number(ratio);
		if !self.current_char().is_ascii_digit() {
			return Some(fraction);
		}
		let factor = self.parse_number_value();
		Some(Node::Key(Box::new(fraction), Op::Mul, Box::new(factor)))
	}

	/// `1 month`, `24 hours`: an integer followed by a time unit word is a duration
	fn with_time_unit(&mut self, number: Node) -> Node {
		let Node::Number(Number::Int(count)) = &number else {
			return number;
		};
		if self.options.data_mode {
			return number;
		}
		let Some((unit, length)) = crate::time::unit_after(&self.chars[self.pos..]) else {
			return number;
		};
		self.advance_by(length);
		Node::data(unit.times(*count))
	}

	fn integer_node(&self, digits: &str) -> Node {
		crate::extensions::numbers::Number::parse_integer(digits)
			.map(Node::Number)
			.unwrap_or_else(|| error(&format!("Invalid int: {}", digits)))
	}

	/// Digits with `_` separators between them: 1_000_000
	fn push_digits(&mut self, num_str: &mut String) {
		loop {
			let ch = self.current_char();
			// '三'.is_numeric() is true but not ASCII
			// Exclude superscripts and vulgar fractions: suffix operators and literals of their own
			if ch.is_numeric() && !is_number_glyph(ch) {
				num_str.push(ch);
				self.advance();
			} else if ch == '_' && self.prev_char().is_ascii_digit() && self.peek_char(1).is_ascii_digit() {
				self.advance();
			} else {
				break;
			}
		}
	}

	/// Scientific notation suffix: e3, E-3, e+03 (only when digits follow, so `2em` stays a unit)
	fn parse_exponent(&mut self) -> Result<Option<i64>, String> {
		let (e, c2, c3) = (self.current_char(), self.peek_char(1), self.peek_char(2));
		let signed = (c2 == '+' || c2 == '-') && c3.is_ascii_digit();
		if !(e == 'e' || e == 'E') || !(c2.is_ascii_digit() || signed) {
			return Ok(None);
		}
		self.advance();
		let mut exponent = String::new();
		if signed {
			exponent.push(self.current_char());
			self.advance();
		}
		self.push_digits(&mut exponent);
		exponent.parse::<i64>().map(Some).map_err(|_| format!("Invalid exponent: e{}", exponent))
	}

	/// `?` directly after a type name, then `=` (not `==`) or a closing bracket or separator
	fn ends_optional_type(&self, next: char, after: char) -> bool {
		matches!(next, ')' | ']' | '}' | ',' | ';') || (next == '=' && after != '=')
	}

	fn parse_symbol(&mut self) -> Result<String, String> {
		let mut symbol = String::new();
		loop {
			let ch = self.current_char();
			// Include alphanumeric, underscore, and hyphen (for kebab-case)
			// BUT: a hyphen joins only a following word, so `i--` and `x-1` end the symbol at the hyphen
			let next = self.peek_char(1);
			let is_hyphen_in_symbol = ch == '-' && (next.is_alphabetic() || next == '_');
			if (ch.is_alphanumeric() && !is_number_glyph(ch)) || ch == '_' || is_hyphen_in_symbol {
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

	/// `tuple<s64, list<node>>` → `tuple` followed by the angle-bracketed argument list
	fn parse_type_application(&mut self, type_name: String) -> Node {
		self.advance(); // skip '<'
		let arguments = self.parse_list_with_separators(Some('>'), Bracket::Less);
		Node::List(vec![Symbol(type_name), arguments], Bracket::None, Separator::None)
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
		let compares = self.equals_compares && bracket_type != Bracket::Curly; // a block is not the condition
		self.with_equals_comparing(compares, |parser| parser.parse_list_with_separators(Some(close), bracket_type))
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

			// Check for end condition (also check for end-of-input to avoid infinite loop)
			let ch = self.current_char();
			let at_end = match close {
				Some(c) => ch == c || ch == '\0',
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
				if self.only_blanks_before_newline() { Separator::Newline } else { Separator::Semicolon }
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

		let list = self.group_by_separators(items_with_seps, bracket);
		match list.duplicate_key() {
			Some(key) => error(&format!("duplicate key '{}'", key)),
			None => list,
		}
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

/// In data only JSON's words are literals; aliases like `yes`, `no`, `none`, `pi` stay symbols.
const DATA_WORD_LITERALS: [&str; 3] = ["true", "false", "null"];

fn real(exact: Exact) -> Node {
	Node::Number(Number::real(Real::Exact(exact)))
}

fn check_constants(s: &str, data_mode: bool) -> Option<Node> {
	let is_word = s.chars().all(|c| c.is_ascii_alphabetic());
	if data_mode && is_word && !DATA_WORD_LITERALS.contains(&s) {
		return None;
	}
	match s.to_lowercase().as_str() {
		"⊤" | "true" | "yes" | "✓" | "🗸" | "✔" | "✓️" | "🗹" | "☑" | "✅" | "⊨" => Some(Node::True),
		"⊥" | "false" | "no" | "⊭" | "❌" | "" => Some(Node::False),
		"ø" | "null" | "nul" | "none" | "nil" | "nill" | "nix" | "nada" | "nothing" | "empty" | "void" => Some(Empty),
		// exact generators (extensions/reals.rs); a bare `e` or `i` stays a free name
		"π" | "pi" => Some(real(Exact::pi())),
		"τ" | "tau" => Some(real(Exact::pi().scale(&Rational::integer(2)))),
		"euler" | "ℯ" => Some(real(Exact::euler())),
		"ⅈ" => Some(real(Exact::imaginary())),
		"⚠️" | "⚡" | "⚡️" => Some(error(s)),
		_ => None,
	}
}
// Tests moved to tests/test_parser.rs

/// `f := …`, `f x := …`, `f(x) := …` define f
fn defined_function_name(target: &Node) -> Option<String> {
	match target.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

fn mentions(node: &Node, name: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol == name,
		Node::Key(left, _, right) => mentions(left, name) || mentions(right, name),
		Node::List(items, _, _) => items.iter().any(|item| mentions(item, name)),
		_ => false,
	}
}

/// `a==b` built in the same expression, not grouped by parentheses (a group is parsed as its own node)
fn ungrouped_equality(node: &Node) -> bool {
	matches!(node, Node::Key(_, op, _) if op.is_equality())
}

/// `1==1==1` chains in Python (true) but is `(1==1)==1` in C: ambiguous, like in Rust
fn chained_equality(lhs: &Node, op: Op, rhs: &Node) -> Diagnostic {
	let (left, right) = (lhs.serialize(), rhs.serialize());
	let middle = match lhs {
		Node::Key(_, _, middle) => middle.serialize(),
		_ => left.clone(),
	};
	let symbol = op.as_str();
	Diagnostic {
		message: format!("ambiguous: equality does not chain in {left} {symbol} {right}"),
		line: 0,
		column: 0,
		fix: Some(format!("{left} and {middle} {symbol} {right} or ({left}) {symbol} {right}")),
	}
}

/// `3 & 4 == 4` is `(3&4)==4` in Python but `3 & (4==4)` in C: `&`/`|` next to an ungrouped comparison is ambiguous.
/// The word forms `and`/`or` read unambiguously and are not affected: `x==1 and y==2`.
fn logic_mixed_with_comparison(lhs: &Node, symbol: char, rhs: &Node) -> Option<Diagnostic> {
	let comparison = |node: &Node| match node.drop_meta() {
		Node::Key(left, op, right) if op.is_comparison() => Some((left.serialize(), op.as_str(), right.serialize())),
		_ => None,
	};
	let (left, right) = (lhs.serialize(), rhs.serialize());
	let fix = match (comparison(lhs), comparison(rhs)) {
		(None, Some((a, op, b))) => format!("{left} {symbol} ({a} {op} {b}) or ({left} {symbol} {a}) {op} {b}"),
		(Some((a, op, b)), None) => format!("({a} {op} {b}) {symbol} {right} or {a} {op} ({b} {symbol} {right})"),
		(Some(_), Some(_)) => format!("({left}) {symbol} ({right})"),
		(None, None) => return None,
	};
	let word = if symbol == '&' { "and" } else { "or" };
	Some(Diagnostic {
		message: format!("ambiguous: `{symbol}` mixed with a comparison in {left} {symbol} {right}; group it or write `{word}`"),
		line: 0,
		column: 0,
		fix: Some(fix),
	})
}


/// `target[index]` is the 1-based `target#(index+1)`; a numeric index is shifted at parse time
pub fn subscript(target: Node, index: Node) -> Node {
	let one = Node::Number(crate::extensions::numbers::Number::Int(1));
	let one_based = match index.drop_meta() {
		Node::Number(n) => Node::Number(*n + crate::extensions::numbers::Number::Int(1)),
		_ => Node::Key(Box::new(index), Op::Add, Box::new(one)),
	};
	Node::Key(Box::new(target), Op::Hash, Box::new(one_based))
}

/// The written index of a subscript's 1-based index `index+1`, when it was not a number (inverse of `subscript`)
pub fn subscript_key(one_based_index: &Node) -> Option<&Node> {
	match one_based_index.drop_meta() {
		Node::Key(key, Op::Add, one) if matches!(one.drop_meta(), Node::Number(crate::extensions::numbers::Number::Int(1))) => Some(key),
		_ => None,
	}
}

/// `while condition body`: the loop head `ø while condition` applied `do` to its body
fn while_do(condition: Node, body: Node) -> Node {
	let head = Node::Key(Box::new(Empty), Op::While, Box::new(condition));
	Node::Key(Box::new(head), Op::Do, Box::new(body))
}
