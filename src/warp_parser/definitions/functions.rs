//! Function definitions: heads, typed parameters, result types, definition blocks, function references
use super::*;

/// A lowercase word that names no type, or a parameter group `(x, y)`
pub(crate) fn is_parameter_word(node: &Node) -> bool {
	match node.drop_meta() {
		Symbol(word) => word.starts_with(char::is_lowercase) && crate::analyzer::type_word_kind(word).is_none(),
		Node::List(_, Bracket::Round, _) => true,
		_ => false,
	}
}

/// The first word of a class-body item that is code: a function keyword, or the constructor call `init(n)`
/// `int bar(){ 42 }` as words: a type word and a call with its block
pub(crate) fn is_typed_method(words: &[Node]) -> bool {
	matches!(words, [type_word, head] if matches!(type_word.drop_meta(), Node::Symbol(word) if crate::analyzer::builtin_type_kind(word).is_some())
		&& matches!(head.drop_meta(), Node::List(parts, _, _) if matches!(parts.last().map(Node::drop_meta), Some(Node::List(_, Bracket::Curly, _)))))
}

/// `f(xs) := xs.map`, `ys = xs.filter`, `x -> x.map`: a definition or lambda whose value ends in a method name, waiting
/// for its trailing block
pub(crate) fn assigned_method_call(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Assign | Op::Define | Op::Arrow | Op::FatArrow, value) => {
			matches!(value.drop_meta(), Node::Key(_, Op::Dot, method) if matches!(method.drop_meta(), Symbol(_)))
				|| assigned_method_call(value)
		}
		_ => false,
	}
}

/// The single argument `(int x)` of `f(int x)` is the typed parameter `x:int`, so that flattening the call `f(T, y)` into
/// `(f T y)` never reads a parameter named like a type (`offset_of(text, byte, start)`) as the type of the next one
pub(crate) fn typed_parameter(arguments: Node) -> Node {
	let is_type_word =
		|node: &Node| matches!(node.drop_meta(), Symbol(word) if crate::analyzer::type_word_kind(word).is_some());
	match arguments.drop_meta() {
		Node::List(items, _, Separator::Space) => match items.as_slice() {
			[type_word, name] if is_type_word(type_word) && matches!(name.drop_meta(), Symbol(_)) => {
				Node::Key(Box::new(name.clone()), Op::Colon, Box::new(type_word.clone()))
			}
			// `x of int`: the parameter x of type int
			[name, of, type_word]
				if of.drop_meta().is_symbol(OF_WORD)
					&& is_type_word(type_word)
					&& matches!(name.drop_meta(), Symbol(_))
					&& !is_type_word(name) =>
			{
				Node::Key(Box::new(name.clone()), Op::Colon, Box::new(type_word.clone()))
			}
			// Kotlin `vararg xs: Int`, C# `params int[] xs`: the rest parameter `*xs` (lowering/variadic.rs)
			[marker, .., name] if matches!(marker.drop_meta(), Symbol(word) if REST_MARKERS.contains(&word.as_str())) => {
				match name.drop_meta() {
					Symbol(name) => Symbol(format!("{}{name}", crate::words::STARRED)),
					Node::Key(name, Op::Colon, _) => Symbol(format!("{}{}", crate::words::STARRED, name.name())),
					_ => arguments,
				}
			}
			// `int b = 2` (C#, C++): `b:int = 2`
			[type_word, default] if is_type_word(type_word) => match default.drop_meta() {
				Node::Key(name, Op::Assign, value) if matches!(name.drop_meta(), Symbol(_)) => Node::Key(
					Box::new(Node::Key(name.clone(), Op::Colon, Box::new(type_word.clone()))),
					Op::Assign,
					value.clone(),
				),
				_ => arguments,
			},
			_ => arguments,
		},
		_ => arguments,
	}
}

/// Every argument of `f(int x, float y)` as its typed parameter
pub(crate) fn typed_parameters(arguments: Node) -> Node {
	match arguments.drop_meta() {
		Node::List(items, bracket, Separator::Colon) => Node::List(
			items.iter().cloned().map(typed_parameter).collect(),
			bracket.clone(),
			Separator::Colon,
		),
		_ => typed_parameter(arguments),
	}
}

/// `f := …`, `f x := …`, `f(x) := …` define f
pub(crate) fn defined_function_name(target: &Node) -> Option<String> {
	match target.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// `f={it*2}` defines a function like `f:={it*2}`: a name assigned a block that uses `it`; other blocks are data
pub(crate) fn is_function_block(name: &Node, block: &Node) -> bool {
	matches!(name.drop_meta(), Node::Symbol(_))
		&& matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))
		&& mentions(block, "it")
}

impl WarpParser {
	/// Elixir's `fn a, b -> body end` at the cursor: the parameter names and where the body starts; Rust's
	/// `fn(x) -> i32 { … }` names a result type, no body
	pub(crate) fn elixir_function_head(&self) -> Option<(Vec<String>, usize)> {
		if !self.matches_keyword(ELIXIR_FUNCTION_KEYWORD) {
			return None;
		}
		let start = self.pos + ELIXIR_FUNCTION_KEYWORD.len();
		let line_end = (start..self.chars.len())
			.find(|&at| self.chars[at] == '\n')
			.unwrap_or(self.chars.len());
		let arrow =
			(start..line_end.saturating_sub(1)).find(|&at| self.chars[at] == '-' && self.chars[at + 1] == '>')?;
		let head: String = self.chars[start..arrow].iter().collect();
		let is_parameter_text = head
			.chars()
			.all(|ch| is_identifier_char(ch) || matches!(ch, ' ' | ',' | '(' | ')'));
		let names: Vec<String> = head
			.split([',', '(', ')', ' '])
			.filter(|name| !name.is_empty())
			.map(str::to_string)
			.collect();
		let body_start = arrow + 2;
		let after: String = self.chars[body_start..].iter().collect();
		let mut words = after.trim_start().splitn(2, |ch: char| !is_identifier_char(ch));
		let is_result_type = words.next().is_some_and(|word| !word.is_empty())
			&& words.next().is_some_and(|rest| rest.trim_start().starts_with('{'));
		let is_function = is_parameter_text
			&& !is_result_type
			&& self.closing_end_follows_from(body_start, &[ELIXIR_FUNCTION_KEYWORD, "do"]);
		is_function.then_some((names, body_start))
	}

	/// The head of a definition, `f`, `f x` or `f(x)`, makes f a function the rest of the source calls
	pub(crate) fn register_defined_function(&mut self, head: &Node) {
		if let Some(name) = defined_function_name(head) {
			if matches!(head.drop_meta(), Node::List(..)) {
				self.functions_with_parameters.insert(name.clone());
			}
			self.functions.insert(name);
		}
	}

	/// The lines of a definition `def f(n)` indented below it by spaces, as `def f(n):` (by tabs the indentation rule of
	/// parse_list_with_separators takes them); Ruby's `def f(n) … end` is read by its end
	pub(crate) fn with_definition_block(&mut self, definition: Node) -> Node {
		let ruby_definition = self.closing_end_follows(&RUBY_END_OPENERS);
		// a top-level Ruby `def f(n)` with its lines indented by tabs: the indented block, then its `end` (the tab rule
		// would make each line the block of the one before)
		let tab_indented_ruby = ruby_definition && self.base_indent == 0 && self.next_line_starts_with_tab();
		if (ruby_definition && !tab_indented_ruby) || !self.only_blanks_before_newline() {
			return definition;
		}
		let Some(body) = self.parse_indented_block() else {
			return definition;
		};
		if tab_indented_ruby {
			self.skip_end_line();
		}
		Node::Key(Box::new(definition), Op::Colon, Box::new(body))
	}

	/// `function add` (P82): the function itself where a name and then the end of the expression follow the keyword;
	/// `function add(x) {…}` and `function add x := …` define it
	pub(crate) fn function_reference_after(&mut self, keyword: &str) -> Option<Node> {
		if !FUNCTION_REFERENCE_WORDS.contains(&keyword) || self.options.data_mode {
			return None;
		}
		let blanks = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		if blanks == 0 || !self.peek_char(blanks).is_alphabetic() {
			return None;
		}
		let name: String = (blanks..)
			.map(|at| self.peek_char(at))
			.take_while(|ch| is_identifier_char(*ch))
			.collect();
		let after = blanks + name.chars().count();
		let rest = (after..)
			.find(|at| !matches!(self.peek_char(*at), ' ' | '\t'))
			.unwrap_or(after);
		// `map function square on [1 2 3]`: the iteration words' `on` ends it too
		let on_follows = (rest..)
			.map(|at| self.peek_char(at))
			.take_while(|ch| is_identifier_char(*ch))
			.collect::<String>()
			== crate::words::ON_WORD;
		if !matches!(self.peek_char(rest), '\0' | ';' | ',' | ')' | ']' | '}' | '\n') && !on_follows {
			return None;
		}
		self.advance_by(after);
		self.after_function_keyword = false;
		Some(crate::closures::function_reference(name))
	}

	pub(crate) fn starts_function_reference(&self) -> bool {
		self.current_char() == '&' && self.peek_char(1).is_alphabetic() && !is_identifier_char(self.prev_char())
	}

	/// Elixir's capture `&(&1 * 2)`: an `&(` not glued to a word before it, whose group reads an `&1`
	pub(crate) fn starts_capture(&self) -> bool {
		if self.current_char() != '&' || self.peek_char(1) != '(' || is_identifier_char(self.prev_char()) {
			return false;
		}
		let mut depth = 0;
		for at in 1.. {
			match self.peek_char(at) {
				'(' => depth += 1,
				')' if depth == 1 => return false,
				')' => depth -= 1,
				'&' if self.peek_char(at + 1).is_ascii_digit() => return true,
				'\0' => return false,
				_ => {}
			}
		}
		false
	}

	/// `norm() := …`, `def abs(): …`, `fun sqrt() { … }`: the word `length` characters ahead names what is defined, not
	/// the operator. A function of that name is refused (analyzer check_operator_word_functions, P141); a method is
	/// called as `p.norm()` (card class-method-named)
	pub(crate) fn defines_named(&self, length: usize) -> bool {
		if self.peek_char(length) != '(' {
			return false;
		}
		let mut depth = 0;
		let mut offset = length;
		loop {
			match self.peek_char(offset) {
				'(' => depth += 1,
				')' if depth == 1 => break,
				')' => depth -= 1,
				'\0' | '\n' => return false,
				_ => {}
			}
			offset += 1;
		}
		let mut next = offset + 1;
		while matches!(self.peek_char(next), ' ' | '\t') {
			next += 1;
		}
		match (self.peek_char(next), self.peek_char(next + 1)) {
			(':', '=') => true,
			(':' | '{', _) => self.after_function_keyword_word(),
			_ => false,
		}
	}

	/// `def ` right before here: what follows heads a definition
	pub(crate) fn after_function_keyword_word(&self) -> bool {
		let before: String = self.chars[..self.pos]
			.iter()
			.rev()
			.skip_while(|c| c.is_whitespace())
			.take_while(|c| c.is_alphanumeric())
			.collect();
		crate::operators::is_function_keyword(&before.chars().rev().collect::<String>())
	}

	/// `Int` or `float` after the colon of a definition's head: its result type, not its body
	pub(crate) fn result_type_follows(&self) -> bool {
		let word: String = self.chars[self.pos..]
			.iter()
			.skip_while(|c| **c == ' ' || **c == '\t')
			.take_while(|c| c.is_alphanumeric() || **c == '_')
			.collect();
		word.starts_with(char::is_uppercase) || crate::analyzer::type_word_kind(&word).is_some()
	}

	/// `fn x -> x * 2 end`: the lambda `x => {x * 2}`
	pub(in crate::warp_parser) fn parse_elixir_function(&mut self) -> Option<Node> {
		let (names, body_start) = self.elixir_function_head()?;
		self.advance_by(body_start - self.pos);
		let body = self.parse_end_block(false);
		let parameters = match names.as_slice() {
			[single] => Node::Symbol(single.clone()),
			several => Node::List(
				several.iter().cloned().map(Node::Symbol).collect(),
				Bracket::Round,
				Separator::Colon,
			),
		};
		Some(Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body)))
	}
}
