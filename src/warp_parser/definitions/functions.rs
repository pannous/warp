//! Function definitions: heads, typed parameters, result types, definition blocks, function references
use super::*;

/// Words that make a parameter the rest parameter: Kotlin `vararg xs: Int`, C# `params int[] xs`
const REST_MARKERS: [&str; 2] = ["vararg", "params"];
/// `function add`, `func add`: a reference to a function (P82), like `&add`
const FUNCTION_REFERENCE_WORDS: [&str; 2] = ["function", "func"];
const ELIXIR_FUNCTION_KEYWORD: &str = "fn";
/// `to render text as pdf: …`: the word before a `to` definition's result type (card render-as)
const RESULT_TYPE_WORD: &str = "as";

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
		// a list `[C3 G3]` stays a list; Dart's named parameters `{int b = 2}` are typed
		Node::List(items, bracket, Separator::Space) if !matches!(bracket, Bracket::Square) => match items.as_slice() {
			[type_word, name] if is_type_word(type_word) && matches!(name.drop_meta(), Symbol(_)) => {
				Node::Key(Box::new(name.clone()), Op::Colon, Box::new(type_word.clone()))
			}
			// Go's `x int`, `s Shape`: the type after the name
			[name, type_word]
				if matches!(name.drop_meta(), Symbol(_))
					&& !is_type_word(name)
					&& (is_type_word(type_word)
						|| matches!(type_word.drop_meta(), Symbol(word) if word.starts_with(char::is_uppercase))) =>
			{
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
		Node::List(items, bracket, Separator::Comma) => {
			Node::List(items.iter().cloned().map(typed_parameter).collect(), bracket.clone(), Separator::Comma)
		}
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
		&& mentions(block, crate::words::IMPLICIT_PARAMETER)
}

impl WarpParser {
	/// Elixir's `fn a, b -> body end` at the cursor: the parameter names and where the body starts; Rust's
	/// `fn(x) -> i32 { … }` names a result type, no body
	pub(crate) fn elixir_function_head(&self) -> Option<(Vec<String>, usize)> {
		if !self.matches_keyword(ELIXIR_FUNCTION_KEYWORD) {
			return None;
		}
		let start = self.pos + ELIXIR_FUNCTION_KEYWORD.len();
		let line_end = (start..self.chars.len()).find(|&at| self.chars[at] == '\n').unwrap_or(self.chars.len());
		let arrow =
			(start..line_end.saturating_sub(1)).find(|&at| self.chars[at] == '-' && self.chars[at + 1] == '>')?;
		let head: String = self.chars[start..arrow].iter().collect();
		let is_parameter_text = head.chars().all(|ch| is_identifier_char(ch) || matches!(ch, ' ' | ',' | '(' | ')'));
		let names: Vec<String> =
			head.split([',', '(', ')', ' ']).filter(|name| !name.is_empty()).map(str::to_string).collect();
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

	/// The parameters of a definition head (`f(a, b: int)`, `def f(a)`) or of a lambda (`x`, `(a, b)`) are in scope until
	/// the statement holding it ends: ` /name` after one divides there, a path elsewhere (card path-parameter-scope)
	pub(crate) fn bind_parameters(&mut self, head: &Node, is_lambda: bool) {
		let parameters = match head.drop_meta() {
			Node::List(items, Bracket::Round, _) if is_lambda => items.as_slice(),
			Node::List(items, Bracket::Round, _)
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) =>
			{
				&items[1..]
			}
			// `area(w, h)` after `def`: the name, then its parameter group
			Node::List(items, Bracket::None, _)
				if items.len() == 2 && matches!(items[1].drop_meta(), Node::List(_, Bracket::Round, _)) =>
			{
				return self.bind_parameters(&items[1], true);
			}
			// `def f(a)`: the head after the keyword
			Node::List(items, _, _) if !is_lambda => {
				if let Some(last) = items.last() {
					self.bind_parameters(last, false);
				}
				return;
			}
			_ if is_lambda => std::slice::from_ref(head),
			_ => return,
		};
		let names = parameters.iter().filter_map(|parameter| match parameter.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			Node::Key(name, _, _) => name.symbol_name().map(str::to_string),
			_ => None,
		});
		self.parameters_in_scope.extend(names);
	}

	/// `def area(w, h) {`: the parameters of the definition head before the block opening at `open`
	pub(crate) fn parameters_before_block(&self, open: usize) -> Vec<String> {
		let before = |at: usize| (0..at).rev().find(|at| !self.chars[*at].is_whitespace());
		let Some(close) = before(open).filter(|close| self.chars[*close] == ')') else {
			return vec![];
		};
		let mut depth = 0;
		let Some(opening) = (0..=close).rev().find(|at| {
			match self.chars[*at] {
				')' => depth += 1,
				'(' => depth -= 1,
				_ => {}
			}
			depth == 0
		}) else {
			return vec![];
		};
		let name_start = (0..opening).rev().take_while(|at| is_identifier_char(self.chars[*at])).last();
		let keyword_end = name_start.and_then(before);
		let keyword: String = keyword_end.map_or_else(String::new, |end| {
			let start = (0..=end).rev().take_while(|at| self.chars[*at].is_alphanumeric()).last().unwrap_or(end);
			self.chars[start..=end].iter().collect()
		});
		if !crate::operators::is_function_keyword(&keyword) {
			return vec![];
		}
		parameter_names(&self.chars[opening + 1..close].iter().collect::<String>())
	}

	/// The lines of a definition `def f(n)` indented below it by spaces, as `def f(n):` (by tabs the indentation rule of
	/// parse_list_with_separators takes them); Ruby's `def f(n) … end` is read by its end
	pub(crate) fn with_definition_block(&mut self, definition: Node) -> Node {
		let ruby_definition = self.closing_end_follows(&RUBY_END_OPENERS);
		// a top-level Ruby `def f(n)` with its lines indented by tabs: the indented block, then its `end` (the tab rule
		// would make each line the block of the one before)
		let tab_indented_ruby = ruby_definition && self.context.base_indent == 0 && self.next_line_starts_with_tab();
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
		let name: String = (blanks..).map(|at| self.peek_char(at)).take_while(|ch| is_identifier_char(*ch)).collect();
		let after = blanks + name.chars().count();
		let rest = (after..).find(|at| !matches!(self.peek_char(*at), ' ' | '\t')).unwrap_or(after);
		// `map function square on [1 2 3]`: the iteration words' `on` ends it too, and `function floor is function` a test
		let next_word: String =
			(rest..).map(|at| self.peek_char(at)).take_while(|ch| is_identifier_char(*ch)).collect();
		let word_follows = [crate::words::ON_WORD, crate::words::IS_WORD].contains(&next_word.as_str());
		if !matches!(self.peek_char(rest), '\0' | ';' | ',' | ')' | ']' | '}' | '\n') && !word_follows {
			return None;
		}
		self.advance_by(after);
		self.after_function_keyword = false;
		Some(crate::closures::function_reference(name))
	}

	/// `&name`: a reference to the function `name`, an `&` glued to the word after it and not to a word before it (`a &b`, `f(&g)`)
	pub(crate) fn starts_function_reference(&self) -> bool {
		self.current_char() == '&' && self.peek_char(1).is_alphabetic() && !is_identifier_char(self.prev_char())
	}

	/// Rust's reference `&x`, `&mut x`, `&&x`, `&5`, `&(a + b)` and dereference `*r`, `**r`, `*(&x)`: where an operand starts,
	/// not a function reference `&name` nor a capture. A `*name` is a dereference unless it is a star of a rest or spread
	/// (`a, *rest = xs`, `f(*xs)`, `*init, last = xs`, Python's `**kw`): after a comma or an opening bracket, or at a statement start
	/// followed by a comma or `=`
	pub(crate) fn starts_rust_reference(&self) -> bool {
		let next = self.peek_char(1);
		match self.current_char() {
			'&' => {
				let reference_to_value = next == '&'
					|| next == '(' && !self.starts_capture()
					|| next == '[' || next == '"'
					|| next.is_ascii_digit() && !self.context.in_capture;
				reference_to_value || self.mutable_reference_follows()
			}
			'*' => {
				let operand = self.is_identifier_start(1) || matches!(next, '(' | '*' | '&');
				operand && !self.star_is_rest_or_spread()
			}
			_ => false,
		}
	}

	fn mutable_reference_follows(&self) -> bool {
		let word: String = (1..).map(|at| self.peek_char(at)).take_while(|ch| is_identifier_char(*ch)).collect();
		word == crate::words::MUT_WORD && matches!(self.peek_char(1 + word.len()), ' ' | '\t')
	}

	fn star_is_rest_or_spread(&self) -> bool {
		let before = (1..=self.pos).map(|back| self.chars[self.pos - back]).find(|ch| !matches!(ch, ' ' | '\t'));
		match before {
			Some(',' | '(' | '[') => true,
			None | Some('\n' | ';' | '{') => {
				let name = (1..).take_while(|&at| is_identifier_char(self.peek_char(at))).count();
				let after = (1 + name..).map(|at| self.peek_char(at)).find(|ch| !matches!(ch, ' ' | '\t'));
				let assigned = after == Some('=') && self.peek_char(1 + name + 1) != '=';
				after == Some(',') || assigned
			}
			Some(_) => false,
		}
	}

	/// The `&`, `&mut ` or `*` of a reference
	pub(crate) fn skip_reference_prefix(&mut self) {
		let mutable = self.mutable_reference_follows();
		self.advance_by(if mutable { 1 + crate::words::MUT_WORD.len() } else { 1 });
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
			several => {
				Node::List(several.iter().cloned().map(Node::Symbol).collect(), Bracket::Round, Separator::Comma)
			}
		};
		Some(Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body)))
	}

	/// `to name params: body`, after the word `to`: the function `name(params) := body`.
	/// The body is the rest of the statement, a `{…}` block or an indented block. Parameters are plain names or, with a
	/// known type word, typed slots (`to square a number:`, type_name_matching::parameter_slots). Anything else is no definition.
	pub(crate) fn try_parse_to_definition(&mut self) -> Option<Node> {
		let before_header = self.mark();
		let restore = |parser: &mut Self| parser.rewind(before_header.clone());
		self.skip_spaces();
		let Some(name) = self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() else {
			restore(self);
			return None;
		};
		let mut parameters = Vec::new();
		// `to call person{name?, phone number} do …` (wiki/argument.md): the parameter's fields, glued to it
		let mut shapes = Vec::new();
		let mut result_type = None;
		loop {
			self.skip_spaces();
			if self.at_to_header_end() {
				break;
			}
			let word = self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten();
			match word.map(|word| self.joined_type_name(word)) {
				Some(word) if word == DO_WORD && !parameters.is_empty() => break,
				Some(word) if word == RESULT_TYPE_WORD && !parameters.is_empty() => {
					result_type = self.phrase_result_type();
					if result_type.is_some() {
						break;
					}
					parameters.push(word)
				}
				Some(parameter) => {
					if self.current_char() == '{' {
						shapes.push((parameter.clone(), self.parse_atom()));
					}
					parameters.push(parameter)
				}
				None => {
					restore(self);
					return None;
				}
			}
		}
		if self.current_char() == ':' {
			self.advance();
			self.skip_spaces();
		}
		self.parameters_in_scope.extend(parameters.iter().cloned());
		let body = self.parse_definition_body();
		self.functions.insert(name.clone());
		let words: Vec<&str> = parameters.iter().map(String::as_str).collect();
		let declared_types = &self.declared_types;
		let is_known_type = |word: &str| {
			crate::analyzer::type_word_kind(word).is_some()
				|| crate::analyzer::plural_element_type(word).is_some()
				|| declared_types.contains(word)
		};
		let (parameters, body) = match crate::type_name_matching::parameter_slots(&words, &body, &is_known_type, true) {
			Ok(slots) => slots,
			Err(message) => return Some(error(&message)),
		};
		let parameters: Vec<Node> = parameters
			.into_iter()
			.map(|parameter| match shapes.iter().find(|(name, _)| *name == parameter.name()) {
				Some((_, fields)) => Node::Key(Box::new(parameter), Op::Colon, Box::new(fields.clone())),
				None => parameter,
			})
			.collect();
		if let Some(clash) = self.phrase_redefinition(&name, &parameters) {
			return Some(clash);
		}
		let head = if parameters.is_empty() {
			Symbol(name)
		} else {
			self.functions_with_parameters.insert(name.clone());
			self.phrase_definitions.insert(name.clone(), parameters.clone());
			Node::List([vec![Symbol(name)], parameters].concat(), Bracket::Round, Separator::None)
		};
		let head = match result_type {
			Some(result_type) => Node::Key(Box::new(head), Op::As, Box::new(Symbol(result_type))),
			None => head,
		};
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// `to square a number: …` or `to square a number { … }`
	pub(crate) fn at_to_header_end(&self) -> bool {
		(self.current_char() == ':' && self.peek_char(1) != '=') || self.current_char() == '{'
	}

	/// The type after `as` when it ends the header: `to render text as pdf: …`
	fn phrase_result_type(&mut self) -> Option<String> {
		let after_as = self.mark();
		self.skip_spaces();
		let result_type = self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten();
		self.skip_spaces();
		if result_type.is_some() && (self.at_to_header_end() || self.matches_keyword(DO_WORD)) {
			return result_type;
		}
		self.rewind(after_as);
		None
	}

	/// `to kill a person: …` after `to kill a dog: …`: the same phrase with other untyped nouns can only be told apart by types
	pub(crate) fn phrase_redefinition(&self, name: &str, parameters: &[Node]) -> Option<Node> {
		let earlier = self.phrase_definitions.get(name).filter(|earlier| earlier.len() == parameters.len())?;
		let untyped = |parameter: &Node| matches!(parameter.drop_meta(), Symbol(_));
		let differing: Vec<(&Node, &Node)> =
			earlier.iter().zip(parameters).filter(|(before, now)| before != now).collect();
		if differing.is_empty() || !differing.iter().all(|(before, now)| untyped(before) && untyped(now)) {
			return None;
		}
		let classes: Vec<String> = differing
			.iter()
			.flat_map(|(before, now)| [before.name(), now.name()])
			.map(|noun| format!("class {noun}"))
			.collect();
		Some(error(&format!("{name} is defined twice; declare {} to dispatch on them", classes.join(" and "))))
	}
}
