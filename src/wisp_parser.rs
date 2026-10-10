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
//!   final ø is optional in notation but not in wasm / node structure
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
use crate::operators::Op;

/// The metadata a `def` keeps its parameter list under: `(def square (typed x int) (mul it it))`
const PARAMS_WORD: &str = "params";

/// The forms of an unbracketed list `(group a b)` and of a parenthesized one `(round f x)`: `(f x)` alone is a call
const GROUP: &str = "group";
const ROUND: &str = "round";

pub struct WispParser {
	chars: Vec<char>,
	pos: usize,
	/// The first thing wrong with the input (a missing `)`, a number that is none): the parse is that error, never a
	/// silently repaired value
	problem: Option<String>,
}

impl WispParser {
	pub fn new(input: &str) -> Self {
		WispParser {
			chars: input.chars().collect(),
			pos: 0,
			problem: None,
		}
	}

	pub fn parse(input: &str) -> Node {
		let mut parser = WispParser::new(input);
		let node = parser.parse_expr();
		match parser.problem {
			Some(problem) => Error(Box::new(Text(format!("wisp: {problem}")))),
			None => node,
		}
	}

	fn complain(&mut self, problem: String) {
		self.problem.get_or_insert(problem);
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
			'[' => self.parse_list(']', Bracket::Square),
			'{' => self.parse_list('}', Bracket::Curly),
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
			"number" | "num" | "int" | "float" => self.finish_one(Self::parse_number),
			"char" => self.finish_one(Self::parse_char_or_symbol),
			"bool" => self.parse_bool_node(),
			"true" => self.finish_constant(True),
			"false" => self.finish_constant(False),
			"nil" | "ø" | "empty" => self.closing(Empty),
			"list" => self.finish_one(Self::parse_expr),
			GROUP => self.finish_as_bracketed(Bracket::None),
			ROUND => self.finish_as_bracketed(Bracket::Round),
			"key" | "tag" => self.finish_key(Op::Colon),
			"pair" => self.finish_key(Op::Assign),
			"cons" => self.finish_key(Op::Dot),
			"meta" => self.parse_meta_node(),
			"defn" | "def" => self.parse_defn_node(),
			"call" => self.finish_key(Op::None),
			"error" | "err" => Error(Box::new(self.finish_one(Self::parse_expr))),
			// `(* it it)`, as emit_wisp writes an operation
			_ => match crate::operators::op_named(kind) {
				Some(op) => self.finish_as_operation(op, first),
				None => self.finish_as_call(first),
			},
		}
	}

	/// `(op left right)` is the operation; with another number of operands it stays a call
	fn finish_as_operation(&mut self, op: Op, name: Node) -> Node {
		match <[Node; 2]>::try_from(self.items_until(')')) {
			Ok([left, right]) => Key(Box::new(left), op, Box::new(right)),
			Err(args) => call_of(name, args),
		}
	}

	/// The expressions up to `close`, which is consumed
	fn items_until(&mut self, close: char) -> Vec<Node> {
		let mut items = vec![];
		loop {
			self.skip_whitespace();
			if self.current() == close || self.end() {
				break;
			}
			items.push(self.parse_expr());
		}
		self.expect(close);
		items
	}

	fn finish_as_list(&mut self, first: Node) -> Node {
		let items = [vec![first], self.items_until(')')].concat();
		List(items, Bracket::Round, Separator::Space)
	}

	/// `(group a b)`, `(round f x)`: the items of a list with that bracket
	fn finish_as_bracketed(&mut self, bracket: Bracket) -> Node {
		List(self.items_until(')'), bracket, Separator::Space)
	}

	fn finish_as_call(&mut self, name: Node) -> Node {
		call_of(name, self.items_until(')'))
	}

	/// `(word value)`: the value `read` reads, then an optional placeholder and the closing paren
	fn finish_one(&mut self, read: fn(&mut Self) -> Node) -> Node {
		self.skip_whitespace();
		let node = read(self);
		self.closing(node)
	}

	fn closing(&mut self, node: Node) -> Node {
		self.skip_optional_value();
		self.expect(')');
		node
	}

	/// `(true)` or `(true 1)`: the value written after the word is ignored
	fn finish_constant(&mut self, node: Node) -> Node {
		self.skip_optional_value();
		self.closing(node)
	}

	/// `(word left right)`: the two parts
	fn finish_two(&mut self) -> (Node, Node) {
		self.skip_whitespace();
		let left = self.parse_expr();
		self.skip_whitespace();
		let right = self.parse_expr();
		self.expect(')');
		(left, right)
	}

	fn finish_key(&mut self, op: Op) -> Node {
		let (left, right) = self.finish_two();
		Key(Box::new(left), op, Box::new(right))
	}

	fn parse_text_node(&mut self) -> Node {
		match self.finish_one(Self::parse_expr) {
			Char(c) => Text(c.to_string()),
			Symbol(s) => Text(s),
			node => node,
		}
	}

	fn parse_symbol_node(&mut self) -> Node {
		match self.finish_one(Self::parse_expr) {
			Text(s) => Symbol(s),
			node => node,
		}
	}

	fn parse_bool_node(&mut self) -> Node {
		match self.finish_one(Self::parse_expr) {
			Number(Number::Int(0)) => False,
			Number(Number::Int(_)) => True,
			Symbol(s) if s == "0" || s.eq_ignore_ascii_case("false") => False,
			_ => True,
		}
	}

	fn parse_meta_node(&mut self) -> Node {
		let (node, data) = self.finish_two();
		Meta {
			node: Box::new(node),
			data: Box::new(data),
		}
	}

	/// `(def name body)` → name:=body; `(def name params body)`: the last part is the body, the parameter list before it
	/// its metadata `(params …)`, as `(def name ((meta params …) body))` writes it
	fn parse_defn_node(&mut self) -> Node {
		self.skip_whitespace();
		let name = self.parse_expr();
		let parts = self.items_until(')');
		let body = match parts.as_slice() {
			[body] => body.clone(),
			[params, body] => Meta {
				node: Box::new(body.clone()),
				data: Box::new(List(vec![Symbol(PARAMS_WORD.to_string()), params.clone()], Bracket::Round, Separator::Space)),
			},
			_ => Error(Box::new(Text(format!("def {} takes a body, or parameters and a body; got {} parts", name.serialize(), parts.len())))),
		};
		Key(Box::new(name), Op::Define, Box::new(body))
	}

	/// a placeholder (ø) or any extra value before the closing paren is read and ignored
	fn skip_optional_value(&mut self) {
		self.skip_whitespace();
		if self.current() != ')' {
			self.parse_expr();
		}
	}

	/// `[a b]` or `{a b}`, as emit_wisp writes a list or a block
	fn parse_list(&mut self, close: char, bracket: Bracket) -> Node {
		self.advance(); // skip the opening bracket
		List(self.items_until(close), bracket, Separator::Space)
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
		if self.end() {
			self.complain(format!("unclosed text \"{s}"));
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
		while !self.end() && (self.current().is_ascii_digit() || self.current() == '.' || self.current() == '_') {
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
		let digits = s.replace('_', "");
		let number = match digits.contains('.') {
			true => digits.parse::<f64>().ok().map(Number::Float),
			false => digits.parse::<num_bigint::BigInt>().ok().map(Number::from_bigint),
		};
		number.map(Number).unwrap_or_else(|| {
			let found: String = self.chars[self.pos..].iter().take_while(|c| !c.is_whitespace() && **c != ')').collect();
			self.complain(format!("expected a number, got {}", if s.is_empty() { found } else { s }));
			Empty
		})
	}

	fn parse_symbol_or_shorthand(&mut self) -> Node {
		let mut s = String::new();
		while !self.end() {
			let c = self.current();
			if c.is_whitespace() || c == '(' || c == ')' || c == '[' || c == ']' || c == '"' || c == '\'' {
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
		let (op, width) = match self.current() {
			':' if self.peek(1) == '=' => (Op::Define, 2),
			':' if self.peek(1) == ':' => (Op::Scope, 2),
			':' => (Op::Colon, 1),
			'=' if self.peek(1) != '=' => (Op::Assign, 1),
			_ => return sym,
		};
		for _ in 0..width {
			self.advance();
		}
		self.skip_whitespace();
		Key(Box::new(sym), op, Box::new(self.parse_expr()))
	}

	fn expect(&mut self, ch: char) {
		self.skip_whitespace();
		if self.current() != ch {
			let found = if self.end() { "the end".to_string() } else { format!("{:?}", self.current()) };
			self.complain(format!("expected {ch:?}, got {found}"));
		}
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

/// `name(args…)`: the name keyed to its round argument list
fn call_of(name: Node, args: Vec<Node>) -> Node {
	Key(Box::new(name), Op::None, Box::new(List(args, Bracket::Round, Separator::Space)))
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
			Error(e) => Self::emit_form("error", [e.as_ref()], out),
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
				Self::emit_form(kind, [l.as_ref(), r.as_ref()], out)
			}
			List(items, bracket, _sep) => {
				let (open, close) = match bracket {
					Bracket::None => return Self::emit_form(GROUP, items, out),
					Bracket::Round => return Self::emit_form(ROUND, items, out),
					Bracket::Square => ('[', ']'),
					Bracket::Curly => ('{', '}'),
					Bracket::Less => ('<', '>'),
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
			// a Rust value (the line positions) has no text form: the node alone
			Meta { node, data } if matches!(data.as_ref(), Data(_)) => Self::emit_node(node, out),
			Meta { node, data } => Self::emit_form("meta", [node.as_ref(), data.as_ref()], out),
			Type { name, body } => Self::emit_form("type", [name.as_ref(), body.as_ref()], out),
			Data(d) => {
				out.push_str(&format!("(data {})", d.type_name));
			}
		}
	}

	/// `(word part part …)`
	fn emit_form<'a>(word: &str, parts: impl IntoIterator<Item = &'a Node>, out: &mut String) {
		out.push('(');
		out.push_str(word);
		for part in parts {
			out.push(' ');
			Self::emit_node(part, out);
		}
		out.push(')');
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
macro_rules! wisp {
	// Wisp roundtrip: parse wisp -> Node -> emit wisp -> parse again -> compare
	($input:expr) => {{
		let node = parse_wisp($input);
		let emitted = emit_wisp(&node);
		let reparsed = parse_wisp(&emitted);
		assert_eq!(
			node, reparsed,
			"roundtrip failed:\n  input: {}\n  emitted: {}",
			$input, emitted
		);
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
	use crate::type_kinds::Kind;
	use crate::expression;

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
	fn test_wisp_defn() {
		// todo: param list vs body!!
		let _result = parse_wisp("(def square (mul it it))"); // how is that already legal?
		let _result = parse_wisp("(def square (op mul [it it]))");
		let _result = parse_wisp("(def square ((meta params (x int)) (mul it it)))");
		let result = parse_wisp("(def square (typed x int) (mul it it)))");
		match result {
			Key(name, Op::Define, body) => {
				assert_eq!(*name, Symbol("square".to_string()));
				assert_eq!(body.drop_meta().serialize(), "mul(it it)"); // (mul it it)
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
		wisp!("ø", (&Empty));
		wisp!("true", (&True));
		wisp!("false", (&False));
		wisp!("(int 42)", (&Number(Number::Int(42))));
		wisp!("(float 3.11)", (&Number(Number::Float(3.11))));
		wisp!("(char 'x')", (&Char('x')));
		wisp!("(text 'hello')", (&Text("hello".into())));
		wisp!("foo", (&Symbol("foo".into())));
	}

	#[test]
	fn test_wisp_emit_compound() {
		// let list = texts!["a", "b"];
		let list = expression!["a", "b"];
		wisp!("[a b]", list);

		let key = Key(
			Box::new(Symbol("x".into())),
			Op::Colon,
			Box::new(Number(Number::Int(1))),
		);
		wisp!("(key x (int 1))", &key);

		let pair = Key(
			Box::new(Symbol("y".into())),
			Op::Assign,
			Box::new(Number(Number::Int(2))),
		);
		wisp!("(pair y (int 2))", &pair);
	}

	// ==================== Roundtrip Tests ====================

	fn roundtrip(input: &str) {
		let node = parse_wisp(input);
		let emitted = emit_wisp(&node);
		let reparsed = parse_wisp(&emitted);
		assert_eq!(
			node, reparsed,
			"roundtrip failed:\n  input: {}\n  emitted: {}",
			input, emitted
		);
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
