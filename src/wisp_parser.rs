//! Wisp Parser - Wasm Lisp (almost) S-expression format mapping directly to WASM GC Node layout
//!
//! why? so far only to demonstrate the simple wasm node layout
//! let result = parse("[a b c]#2");
//! assert!(result.first().length() == 3); // (# [a b c] 2)
//!
//! Format: (kind data value) where:
//! - kind: node type (text, symbol, number, list, key, pair, tag, meta, ...)
//! - data: primary payload of arbitrary data type depending on kind (also node)
//! - value: secondary payload of type node, can be ø (empty, null)
//!  final ø is optional in notation but not in wasm / node structure
//!
//! Shorthands:
//! - "ok"       → (text "ok") → (text "ok" ø)
//! - 42         → (int 42) → (int 42 ø)
//! - 3.14       → (float 3.14)
//! - 'a'        → (char 'a')
//! - True/False → (bool 1/0)
//! - [a b c]    → (list [a b c])
//! - a=b        → (pair a b)
//! - "a":b      → (key "a" b)

use crate::extensions::numbers::Number;
use crate::node::Node::*;
use crate::node::*;
use crate::type_kinds::Kind;


pub struct WispParser {
	chars: Vec<char>,
	pos: usize,
}

impl WispParser {
	pub fn new(input: &str) -> Self {
		WispParser {
			chars: input.chars().collect(),
			pos: 0,
		}
	}

	pub fn parse(input: &str) -> Node {
		let mut parser = WispParser::new(input);
		parser.parse_expr()
	}

	fn end(&self) -> bool {
		self.pos >= self.chars.len()
	}

	fn current(&self) -> char {
		*self.chars.get(self.pos).unwrap_or(&'\0')
	}

	fn peek(&self, offset: usize) -> char {
		*self.chars.get(self.pos + offset).unwrap_or(&'\0')
	}

	fn advance(&mut self) -> char {
		let ch = self.current();
		self.pos += 1;
		ch
	}

	fn skip_whitespace(&mut self) {
		while !self.end() && self.current().is_whitespace() {
			self.advance();
		}
		// skip comments
		if self.current() == ';' {
			while !self.end() && self.current() != '\n' {
				self.advance();
			}
			self.skip_whitespace();
		}
	}

	fn parse_expr(&mut self) -> Node {
		self.skip_whitespace();
		if self.end() {
			return Empty;
		}
		match self.current() {
			'(' => self.parse_sexpr(),
			'[' => self.parse_list(),
			'"' => self.parse_string(),
			'\'' => self.parse_char_or_symbol(),
			'0'..='9' | '-' => self.parse_number(),
			_ => self.parse_symbol_or_shorthand(),
		}
	}

	fn parse_sexpr(&mut self) -> Node {
		self.advance(); // skip '('
		self.skip_whitespace();

		// check for dotted pair (a . b)
		let first = self.parse_expr();
		self.skip_whitespace();
		if self.current() == '.' && self.peek(1).is_whitespace() {
			self.advance(); // skip '.'
			self.skip_whitespace();
			let second = self.parse_expr();
			self.skip_whitespace();
			self.expect(')');
			return Key(Box::new(first), Op::Dot, Box::new(second));
		}

		// handle True/False/Empty at start of sexpr - just consume remaining and return
		match &first {
			True | False | Empty => {
				self.consume_until_close();
				self.expect(')');
				return first;
			}
			_ => {}
		}

		// regular s-expr: (kind data value) or (kind data)
		let kind = match &first {
			Symbol(s) => s.as_str(),
			_ => return self.finish_as_list(first),
		};

		match kind {
			"text" => self.parse_text_node(),
			"symbol" | "sym" => self.parse_symbol_node(),
			"number" | "num" => self.parse_number_node(),
			"int" => self.parse_int_node(),
			"float" => self.parse_float_node(),
			"char" => self.parse_char_node(),
			"bool" => self.parse_bool_node(),
			"true" => self.finish_true(),
			"false" => self.finish_false(),
			"nil" | "ø" | "empty" => self.finish_empty(),
			"list" => self.parse_list_node(),
			"key" => self.parse_key_node(),
			"pair" => self.parse_pair_node(),
			"cons" => self.parse_cons_node(),
			"tag" => self.parse_tag_node(),
			"meta" => self.parse_meta_node(),
			"defn" | "def" => self.parse_defn_node(),
			"call" => self.parse_call_node(),
			"error" | "err" => self.parse_error_node(),
			_ => self.finish_as_call(first),
		}
	}

	fn finish_as_list(&mut self, first: Node) -> Node {
		let mut items = vec![first];
		loop {
			self.skip_whitespace();
			if self.current() == ')' || self.end() {
				break;
			}
			items.push(self.parse_expr());
		}
		self.expect(')');
		List(items, Bracket::Round, Separator::Space)
	}

	fn finish_as_call(&mut self, name: Node) -> Node {
		let mut args = vec![];
		loop {
			self.skip_whitespace();
			if self.current() == ')' || self.end() {
				break;
			}
			args.push(self.parse_expr());
		}
		self.expect(')');
		// call is: name:args or key with call semantics
		let args_node = List(args, Bracket::Round, Separator::Space);
		Key(Box::new(name), Op::None, Box::new(args_node))
	}

	fn parse_text_node(&mut self) -> Node {
		self.skip_whitespace();
		let node = self.parse_expr();
		self.skip_optional_value();
		self.expect(')');
		// convert to Text if needed
		match node {
			Text(s) => Text(s),
			Char(c) => Text(c.to_string()),
			Symbol(s) => Text(s),
			_ => node,
		}
	}

	fn parse_symbol_node(&mut self) -> Node {
		self.skip_whitespace();
		let sym = self.parse_expr();
		self.skip_optional_value();
		self.expect(')');
		match sym {
			Text(s) | Symbol(s) => Symbol(s),
			_ => sym,
		}
	}

	fn parse_number_node(&mut self) -> Node {
		self.skip_whitespace();
		let num = self.parse_number();
		self.skip_whitespace();
		// optional type hint
		if self.current() != ')' {
			let _type_hint = self.parse_expr();
		}
		self.expect(')');
		num
	}

	fn parse_int_node(&mut self) -> Node {
		self.skip_whitespace();
		let num = self.parse_number();
		self.skip_optional_value();
		self.expect(')');
		num
	}

	fn parse_float_node(&mut self) -> Node {
		self.skip_whitespace();
		let num = self.parse_number();
		self.skip_optional_value();
		self.expect(')');
		num
	}

	fn parse_char_node(&mut self) -> Node {
		self.skip_whitespace();
		let ch = self.parse_char_or_symbol();
		self.skip_optional_value();
		self.expect(')');
		ch
	}

	fn parse_bool_node(&mut self) -> Node {
		self.skip_whitespace();
		let val = self.parse_expr();
		self.skip_optional_value();
		self.expect(')');
		match val {
			Number(Number::Int(0)) => False,
			Number(Number::Int(_)) => True,
			Symbol(s) if s == "0" || s.eq_ignore_ascii_case("false") => False,
			_ => True,
		}
	}

	fn finish_true(&mut self) -> Node {
		self.skip_whitespace();
		if self.current() != ')' {
			self.parse_expr(); // skip any value
		}
		self.skip_optional_value();
		self.expect(')');
		True
	}

	fn finish_false(&mut self) -> Node {
		self.skip_whitespace();
		if self.current() != ')' {
			self.parse_expr();
		}
		self.skip_optional_value();
		self.expect(')');
		False
	}

	fn finish_empty(&mut self) -> Node {
		self.skip_optional_value();
		self.expect(')');
		Empty
	}

	fn parse_list_node(&mut self) -> Node {
		self.skip_whitespace();
		let list = self.parse_expr();
		self.skip_optional_value();
		self.expect(')');
		list
	}

	fn parse_key_node(&mut self) -> Node {
		self.skip_whitespace();
		let key = self.parse_expr();
		self.skip_whitespace();
		let val = self.parse_expr();
		self.expect(')');
		Key(Box::new(key), Op::Colon, Box::new(val))
	}

	fn parse_pair_node(&mut self) -> Node {
		self.skip_whitespace();
		let left = self.parse_expr();
		self.skip_whitespace();
		let right = self.parse_expr();
		self.expect(')');
		Key(Box::new(left), Op::Assign, Box::new(right))
	}

	fn parse_cons_node(&mut self) -> Node {
		self.skip_whitespace();
		let car = self.parse_expr();
		self.skip_whitespace();
		let cdr = self.parse_expr();
		self.expect(')');
		Key(Box::new(car), Op::Dot, Box::new(cdr))
	}

	fn parse_tag_node(&mut self) -> Node {
		self.skip_whitespace();
		let name = self.parse_expr();
		self.skip_whitespace();
		let body = self.parse_expr();
		self.expect(')');
		Key(Box::new(name), Op::Colon, Box::new(body))
	}

	fn parse_meta_node(&mut self) -> Node {
		self.skip_whitespace();
		let node = self.parse_expr();
		self.skip_whitespace();
		let data = self.parse_expr();
		self.expect(')');
		Meta {
			node: Box::new(node),
			data: Box::new(data),
		}
	}

	fn parse_defn_node(&mut self) -> Node {
		self.skip_whitespace();
		let name = self.parse_expr();
		self.skip_whitespace();
		let body = self.parse_expr();
		self.expect(')');
		// defn name body → name:=body
		Key(Box::new(name), Op::Define, Box::new(body))
	}

	fn parse_call_node(&mut self) -> Node {
		self.skip_whitespace();
		let name = self.parse_expr();
		self.skip_whitespace();
		let args = self.parse_expr();
		self.expect(')');
		Key(Box::new(name), Op::None, Box::new(args))
	}

	fn parse_error_node(&mut self) -> Node {
		self.skip_whitespace();
		let msg = self.parse_expr();
		self.skip_optional_value();
		self.expect(')');
		Error(Box::new(msg))
	}

	fn skip_optional_value(&mut self) {
		self.skip_whitespace();
		if self.current() != ')' {
			// could be ø or any placeholder
			let val = self.parse_expr();
			match val {
				Symbol(s) if s == "ø" || s == "nil" || s == "null" => {}
				_ => {} // ignore extra value
			}
		}
	}

	fn parse_list(&mut self) -> Node {
		self.advance(); // skip '['
		let mut items = vec![];
		loop {
			self.skip_whitespace();
			if self.current() == ']' || self.end() {
				break;
			}
			items.push(self.parse_expr());
		}
		self.expect(']');
		List(items, Bracket::Square, Separator::Space)
	}

	fn parse_string(&mut self) -> Node {
		self.advance(); // skip '"'
		let mut s = String::new();
		while !self.end() && self.current() != '"' {
			if self.current() == '\\' {
				self.advance();
				match self.current() {
					'n' => s.push('\n'),
					't' => s.push('\t'),
					'r' => s.push('\r'),
					'\\' => s.push('\\'),
					'"' => s.push('"'),
					c => s.push(c),
				}
			} else {
				s.push(self.current());
			}
			self.advance();
		}
		self.advance(); // skip closing '"'
		Text(s)
	}

	fn parse_char_or_symbol(&mut self) -> Node {
		self.advance(); // skip '\''
		if self.current() == '\'' {
			self.advance();
			return Empty; // empty char ''
		}
		let mut chars = vec![];
		while !self.end() && self.current() != '\'' {
			if self.current() == '\\' {
				self.advance();
				match self.current() {
					'n' => chars.push('\n'),
					't' => chars.push('\t'),
					'r' => chars.push('\r'),
					'\\' => chars.push('\\'),
					'\'' => chars.push('\''),
					c => chars.push(c),
				}
			} else {
				chars.push(self.current());
			}
			self.advance();
		}
		self.advance(); // skip closing '\''
		if chars.len() == 1 {
			Char(chars[0])
		} else {
			// multi-char becomes text
			Text(chars.into_iter().collect())
		}
	}

	fn parse_number(&mut self) -> Node {
		let mut s = String::new();
		if self.current() == '-' {
			s.push(self.advance());
		}
		while !self.end()
			&& (self.current().is_ascii_digit() || self.current() == '.' || self.current() == '_')
		{
			if self.current() == '.' && self.peek(1) == '.' {
				break; // range operator
			}
			s.push(self.advance());
		}
		// hex
		if s.starts_with("0x") || s.starts_with("0X") || s.starts_with("-0x") {
			let hex_str = s
				.trim_start_matches('-')
				.trim_start_matches("0x")
				.trim_start_matches("0X");
			if let Ok(n) = i64::from_str_radix(hex_str, 16) {
				return Number(Number::Int(if s.starts_with('-') { -n } else { n }));
			}
		}
		if s.contains('.') {
			Number(Number::Float(s.replace('_', "").parse().unwrap_or(0.0)))
		} else {
			Number(Number::Int(s.replace('_', "").parse().unwrap_or(0)))
		}
	}

	fn parse_symbol_or_shorthand(&mut self) -> Node {
		let mut s = String::new();
		while !self.end() {
			let c = self.current();
			if c.is_whitespace()
				|| c == '(' || c == ')'
				|| c == '[' || c == ']'
				|| c == '"' || c == '\''
			{
				break;
			}
			// check for key operator
			if c == ':' || c == '=' {
				break;
			}
			s.push(self.advance());
		}
		if s.is_empty() {
			return Empty;
		}
		// keywords
		match s.as_str() {
			"true" | "True" | "TRUE" => return True,
			"false" | "False" | "FALSE" => return False,
			"nil" | "null" | "ø" | "empty" | "Empty" => return Empty,
			_ => {}
		}
		let sym = Symbol(s);
		self.skip_whitespace();
		// check for shorthand operators
		match self.current() {
			':' if self.peek(1) == '=' => {
				self.advance();
				self.advance();
				self.skip_whitespace();
				let val = self.parse_expr();
				Key(Box::new(sym), Op::Define, Box::new(val))
			}
			':' if self.peek(1) == ':' => {
				self.advance();
				self.advance();
				self.skip_whitespace();
				let val = self.parse_expr();
				Key(Box::new(sym), Op::Scope, Box::new(val))
			}
			':' => {
				self.advance();
				self.skip_whitespace();
				let val = self.parse_expr();
				Key(Box::new(sym), Op::Colon, Box::new(val))
			}
			'=' if self.peek(1) != '=' => {
				self.advance();
				self.skip_whitespace();
				let val = self.parse_expr();
				Key(Box::new(sym), Op::Assign, Box::new(val))
			}
			_ => sym,
		}
	}

	fn expect(&mut self, ch: char) {
		self.skip_whitespace();
		if self.current() == ch {
			self.advance();
		}
	}

	fn consume_until_close(&mut self) {
		while !self.end() && self.current() != ')' {
			self.parse_expr();
			self.skip_whitespace();
		}
	}
}

pub fn parse_wisp(input: &str) -> Node {
	WispParser::parse(input)
}

/// Emit Node as wisp s-expression format
pub fn emit_wisp(node: &Node) -> String {
	WispEmitter::emit(node)
}

pub struct WispEmitter;

impl WispEmitter {
	pub fn emit(node: &Node) -> String {
		let mut out = String::new();
		Self::emit_node(node, &mut out);
		out
	}

	fn emit_node(node: &Node, out: &mut String) {
		match node {
			Empty => out.push('ø'),
			True => out.push_str("true"),
			False => out.push_str("false"),
			Number(n) => match n {
				Number::Int(i) => out.push_str(&format!("(int {})", i)),
				Number::Float(f) => out.push_str(&format!("(float {})", f)),
				_ => out.push_str(&format!("(num {})", n)),
			},
			Char(c) => out.push_str(&format!("(char '{}')", c)),
			Text(s) => {
				out.push_str("(text '");
				Self::emit_escaped(s, out);
				out.push_str("')");
			}
			Symbol(s) => out.push_str(s),
			Error(e) => {
				out.push_str("(error ");
				Self::emit_node(e, out);
				out.push(')');
			}
			Key(l, op, r) => {
				let kind = match op {
					Op::Colon => "key",
					Op::Assign => "pair",
					Op::Define => "def",
					Op::Dot => "cons",
					Op::Scope => "scope",
					Op::Arrow => "arrow",
					Op::FatArrow => "fatarrow",
					Op::None => "call",
					// Arithmetic/comparison/logical ops use op symbol
					_ => op.as_str(),
				};
				out.push('(');
				out.push_str(kind);
				out.push(' ');
				Self::emit_node(l, out);
				out.push(' ');
				Self::emit_node(r, out);
				out.push(')');
			}
			List(items, bracket, _sep) => {
				let (open, close) = match bracket {
					Bracket::Square => ('[', ']'),
					Bracket::Curly => ('{', '}'),
					Bracket::Round => ('(', ')'),
					Bracket::Less => ('<', '>'),
					Bracket::None => ('[', ']'),
					Bracket::Other(o, c) => (*o, *c),
				};
				out.push(open);
				for (i, item) in items.iter().enumerate() {
					if i > 0 {
						out.push(' ');
					}
					Self::emit_node(item, out);
				}
				out.push(close);
			}
			Meta { node, data } => {
				out.push_str("(meta ");
				Self::emit_node(node, out);
				out.push(' ');
				Self::emit_node(data, out);
				out.push(')');
			}
			Type { name, body } => {
				out.push_str("(type ");
				Self::emit_node(name, out);
				out.push(' ');
				Self::emit_node(body, out);
				out.push(')');
			}
			Data(d) => {
				out.push_str(&format!("(data {})", d.type_name));
			}
		}
	}

	fn emit_escaped(s: &str, out: &mut String) {
		for c in s.chars() {
			match c {
				'\n' => out.push_str("\\n"),
				'\t' => out.push_str("\\t"),
				'\r' => out.push_str("\\r"),
				'\\' => out.push_str("\\\\"),
				'\'' => out.push_str("\\'"),
				_ => out.push(c),
			}
		}
	}
}


#[macro_export]
macro_rules! wis {
	// Wisp roundtrip: parse wisp -> Node -> emit wisp -> parse again -> compare
	($input:expr) => {{
		let node = parse_wisp($input);
		let emitted = emit_wisp(&node);
		let reparsed = parse_wisp(&emitted);
		assert_eq!(node, reparsed, "roundtrip failed:\n  input: {}\n  emitted: {}", $input, emitted);
		node
	}};
	// Wisp eval: parse wisp -> Node -> compare to expected
	($input:expr, $expected:expr) => {{
		let node = parse_wisp($input);
		assert_eq!(node, $expected, "wisp parse mismatch for: {}", $input);
		node
	}};
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::{expression, is, put, wis, NodeKind, Strings};
	use crate::DataType::String;

	#[test]
	fn test_wisp_basic_atom_types() {
		assert_eq!(parse_wisp("42"), Number(Number::Int(42)));
		assert_eq!(parse_wisp("-7"), Number(Number::Int(-7)));
		assert_eq!(parse_wisp("3.11"), Number(Number::Float(3.11)));
		assert_eq!(parse_wisp("'hello'"), Text("hello".to_string()));
		assert_eq!(parse_wisp("'a'"), Char('a'));
		assert_eq!(parse_wisp("true"), True);
		assert_eq!(parse_wisp("false"), False);
		assert_eq!(parse_wisp("nil"), Empty);
		assert_eq!(parse_wisp("ø"), Empty);
	}

	#[test]
	fn test_wisp_sexpr_types_superfluous_empty_node() {
		assert_eq!(parse_wisp("(text 'ok' ø)"), Text("ok".to_string()));
		assert_eq!(parse_wisp("(int 42 ø)"), Number(Number::Int(42)));
		assert_eq!(parse_wisp("(float 3.11 ø)"), Number(Number::Float(3.11)));
		assert_eq!(parse_wisp("(char 'x' ø)"), Char('x'));
		assert_eq!(parse_wisp("(bool 1 ø)"), True);
		assert_eq!(parse_wisp("(bool 0 ø)"), False);
		assert_eq!(parse_wisp("(true 1 True)"), True);
		assert_eq!(parse_wisp("(nil)"), Empty);
	}

	#[test]
	fn test_wisp_sexpr_types() {
		assert_eq!(parse_wisp("(text 'ok')"), Text("ok".to_string()));
		assert_eq!(parse_wisp("(int 42)"), Number(Number::Int(42)));
		assert_eq!(parse_wisp("(float 3.11)"), Number(Number::Float(3.11)));
		assert_eq!(parse_wisp("(char 'x')"), Char('x'));
		assert_eq!(parse_wisp("(bool 1)"), True);
		assert_eq!(parse_wisp("(bool 0)"), False);
		assert_eq!(parse_wisp("(nil)"), Empty);
	}

	#[test]
	fn test_wisp_list() {
		let result = parse_wisp("[a b c]");
		match result {
			List(items, Bracket::Square, _) => {
				assert_eq!(items.len(), 3);
				assert_eq!(items[0], Symbol("a".to_string()));
			}
			_ => panic!("expected list"),
		}
	}

	#[test]
	fn test_wisp_cons_dotted_pair() {
		let result = parse_wisp("(a . b)");
		match result {
			Key(l, Op::Dot, r) => {
				assert_eq!(*l, Symbol("a".to_string()));
				assert_eq!(*r, Symbol("b".to_string()));
			}
			_ => panic!("expected cons cell"),
		}
	}

	#[test]
	fn test_wisp_key_pair() {
		let result = parse_wisp("(key 'name' value)");
		match result {
			Key(l, Op::Colon, r) => {
				assert_eq!(*l, Text("name".to_string()));
				assert_eq!(*r, Symbol("value".to_string()));
			}
			_ => panic!("expected key"),
		}

		let result2 = parse_wisp("(pair x 42)");
		match result2 {
			Key(l, Op::Assign, r) => {
				assert_eq!(*l, Symbol("x".to_string()));
				assert_eq!(*r, Number(Number::Int(42)));
			}
			_ => panic!("expected pair"),
		}
	}

	#[test]
	fn test_wisp_tag() {
		let result = parse_wisp("(tag html [body])");
		match result {
			Key(l, Op::Colon, r) => {
				assert_eq!(*l, Symbol("html".to_string()));
				match *r {
					List(items, Bracket::Square, _) => {
						assert_eq!(items.len(), 1);
					}
					_ => panic!("expected list body"),
				}
			}
			_ => panic!("expected tag"),
		}
	}

	#[test]
	fn test_wisp_meta() {
		let result = parse_wisp("(meta value (comment 'test'))");
		match result {
			Meta { node, data } => {
				assert_eq!(*node, Symbol("value".to_string()));
				assert_eq!(data.value(), "test") // wait, comment is no legal node type!?
			}
			_ => panic!("expected meta"),
		}
	}

	#[test]
	// #[todo]
	#[ignore]
	fn test_wisp_defn() {
		// todo: param list vs body!!
		let _result = parse_wisp("(def square (mul it it))"); // how is that already legal?
		let _result = parse_wisp("(def square (op mul [it it]))");
		let _result = parse_wisp("(def square ((meta params (x int)) (mul it it)))");
		let result = parse_wisp("(def square (typed x int) (mul it it)))");
		match result {
			Key(name, Op::Define, body) => {
				assert_eq!(*name, Symbol("square".to_string()));
				put!(body);
				assert_eq!(body.len(), 3); // (mul it it)
			}
			_ => panic!("expected defn"),
		}
	}

	#[test]
	fn test_wisp_shorthand_operators() {
		let result = parse_wisp("x:42");
		match result {
			Key(l, Op::Colon, r) => {
				assert_eq!(*l, Symbol("x".to_string()));
				assert_eq!(*r, Number(Number::Int(42)));
			}
			_ => panic!("expected key"),
		}

		let result2 = parse_wisp("x=42");
		match result2 {
			Key(l, Op::Assign, r) => {
				assert_eq!(*l, Symbol("x".to_string()));
				assert_eq!(*r, Number(Number::Int(42)));
			}
			_ => panic!("expected pair"),
		}

		let result3 = parse_wisp("x:=42");
		match result3 {
			Key(l, Op::Define, r) => {
				assert_eq!(*l, Symbol("x".to_string()));
				assert_eq!(*r, Number(Number::Int(42)));
			}
			_ => panic!("expected define"),
		}
	}

	#[test]
	fn test_wisp_nested() {
		let result = parse_wisp("(tag div [(meta (text 'hello') (class 'item')) (tag span ø)])");
		match result {
			Key(name, Op::Colon, body) => {
				assert_eq!(*name, Symbol("div".to_string()));
				assert_eq!(body.kind(), Kind::List)
			}
			_ => panic!("expected nested structure"),
		}
	}

	#[test]
	fn test_wisp_call() {
		let result = parse_wisp("(call print ['hello' 'world'])");
		match result {
			Key(name, Op::None, args) => {
				assert_eq!(*name, Symbol("print".to_string()));
				assert_eq!(args.first(), Text("hello".to_string()));
			}
			_ => panic!("expected call"),
		}
	}

	// ==================== Emitter Tests ====================

	#[test]
	fn test_wisp_emit_atoms() {
		wis!("ø",(&Empty));
		wis!("true",(&True));
		wis!("false",(&False));
		wis!("(int 42)",(&Number(Number::Int(42))));
		wis!("(float 3.11)",(&Number(Number::Float(3.11))));
		wis!("(char 'x')",(&Char('x')));
		wis!("(text 'hello')",(&Text("hello".into())));
		wis!("foo",(&Symbol("foo".into())));
	}

	#[test]
	fn test_wisp_emit_compound() {
		// let list = Strings!["a", "b"];
		let list = expression!["a", "b"];
		wis!("[a b]",list);

		let key = Key(Box::new(Symbol("x".into())), Op::Colon, Box::new(Number(Number::Int(1))));
		wis!("(key x (int 1))",&key);

		let pair = Key(Box::new(Symbol("y".into())), Op::Assign, Box::new(Number(Number::Int(2))));
		wis!("(pair y (int 2))",&pair);
	}

	// ==================== Roundtrip Tests ====================

	fn roundtrip(input: &str) {
		let node = parse_wisp(input);
		let emitted = emit_wisp(&node);
		let reparsed = parse_wisp(&emitted);
		assert_eq!(node, reparsed, "roundtrip failed:\n  input: {}\n  emitted: {}", input, emitted);
	}

	#[test]
	fn test_wisp_roundtrip_atoms() {
		roundtrip("42"); // todo how can that be: ?
		roundtrip("(int 42)");
		roundtrip("-7");
		roundtrip("3.14");
		roundtrip("true");
		roundtrip("false");
		roundtrip("ø");
	}

	#[test]
	fn test_wisp_roundtrip_sexpr() {
		roundtrip("(int 42)");
		roundtrip("(float 3.14)");
		roundtrip("(char 'x')");
		roundtrip("(text 'hello')");
	}

	#[test]
	fn test_wisp_roundtrip_compound() {
		roundtrip("[a b c]");
		roundtrip("(key x 42)");
		roundtrip("(pair y 3)");
		roundtrip("(cons a b)");
		roundtrip("(meta value info)");
	}

	#[test]
	fn test_wisp_roundtrip_nested() {
		roundtrip("(key x [1 2 3])");
		roundtrip("(meta (text 'hi') (key class 'item'))");
		roundtrip("[a [b c] d]");
	}
}
