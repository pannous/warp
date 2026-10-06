//! Atoms: symbols with suffixes, definitions and their bodies, indented and end blocks, URLs, type declarations

use super::*;

const VOID_WORD: &str = "void";

impl WaspParser {
	/// Parse an atomic expression (no infix operators)
	/// Handles: numbers, strings, brackets, symbols with named blocks
	pub(super) fn parse_atom(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();
		let (line_nr, column) = self.get_position();

		if self.is_at_line_end() {
			return Empty;
		}

		let node = match self.current_char() {
			// `&name` is the function `name` itself: a name is already a function value where a function is expected
			'&' if self.starts_function_reference() => {
				self.advance();
				match self.parse_symbol() {
					Ok(name) => crate::closures::function_reference(name),
					Err(message) => error(&message),
				}
			}
			'"' | '\'' | '«' => self.parse_string(),
			// `a, *rest = xs`: the starred name takes the items the other names leave (src/lowering/tuples.rs); `...rest`
			// (JS) is the starred `*rest` too: a rest parameter or a spread argument (src/lowering/variadic.rs)
			'.' if self.peek_char(1) == '.' && self.peek_char(2) == '.' && self.is_identifier_start(3) => self.parse_starred(3),
			// Python's `**kw`: the keyword arguments as one object (src/lowering/variadic.rs)
			'*' if self.peek_char(1) == '*' && self.is_identifier_start(2) => match self.parse_starred(2) {
				Node::Symbol(name) => Node::Symbol(format!("{}{name}", crate::tuples::STARRED)),
				other => other,
			},
			'*' if self.is_identifier_start(1) => self.parse_starred(1),
			'(' | '[' | '{' => self.parse_bracketed(self.current_char()),
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			'<' => self.parse_bracketed('<'),
			';' | '>' | '}' | ')' | ']' => Empty, // Closing brackets/terminators handled by caller
			'ø' => { self.advance(); return Empty }
			'∞' => { self.advance(); return Node::Number(Number::Inf) } // the float infinity (P56)
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
			'\\' if let Some((name, length)) = crate::uniscript_entities::entity_name_at(&self.chars, self.pos) => {
				(0..length).for_each(|_| self.advance());
				error(&crate::uniscript_entities::unknown_entity(&name))
			}
			'\\' if let Some(name) = crate::uniscript_entities::bare_entity_name_at(&self.chars, self.pos) => {
				(0..=name.len()).for_each(|_| self.advance());
				error(&format!("a uniscript entity is written \\:{name}, not \\{name}"))
			}
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

	/// The word of `length` characters just parsed is the first thing of its statement: nothing, a newline, `;` or `{` before it
	pub(super) fn word_starts_statement(&self, length: usize) -> bool {
		let word_start = self.pos.saturating_sub(length);
		let before = self.chars[..word_start].iter().rev().find(|ch| !matches!(ch, ' ' | '\t' | '\r'));
		matches!(before, None | Some('\n' | ';' | '{'))
	}

	/// `to name params: body`, after the word `to`: the function `name(params) := body`.
	/// The body is the rest of the statement, a `{…}` block or an indented block. Parameters are plain names or, with a
	/// known type word, typed slots (`to square a number:`, type_name_matching::parameter_slots). Anything else is no definition.
	pub(super) fn try_parse_to_definition(&mut self) -> Option<Node> {
		let before_header = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let restore = |parser: &mut Self| (parser.pos, parser.line_nr, parser.column, parser.current_line) = before_header.clone();
		self.skip_spaces();
		let Some(name) = self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() else {
			restore(self);
			return None;
		};
		let mut parameters = Vec::new();
		loop {
			self.skip_spaces();
			// `to square a number: …` or `to square a number { … }`
			if (self.current_char() == ':' && self.peek_char(1) != '=') || self.current_char() == '{' {
				break;
			}
			match self.at_identifier_start().then(|| self.parse_symbol().ok()).flatten() {
				Some(parameter) => parameters.push(parameter),
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
		let body = self.parse_definition_body();
		self.functions.insert(name.clone());
		let words: Vec<&str> = parameters.iter().map(String::as_str).collect();
		let declared_types = &self.declared_types;
		let is_known_type = |word: &str| {
			crate::analyzer::type_word_kind(word).is_some() || crate::analyzer::plural_element_type(word).is_some() || declared_types.contains(word)
		};
		let parameters = match crate::type_name_matching::parameter_slots(&words, &body, &is_known_type) {
			Ok(parameters) => parameters,
			Err(message) => return Some(error(&message)),
		};
		if let Some(clash) = self.phrase_redefinition(&name, &parameters) {
			return Some(clash);
		}
		let head = if parameters.is_empty() {
			Symbol(name)
		} else {
			self.functions_with_parameters.insert(name.clone());
			self.phrase_definitions.insert(name.clone(), parameters.clone());
			let head = Node::List([vec![Symbol(name)], parameters].concat(), Bracket::Round, Separator::None);
			// P52: the prepositions of the phrase, for phrase_calls to read calls like `add 1 to 2`
			let pattern = crate::phrase_calls::pattern_text(&words);
			if pattern.split(' ').any(|part| part != crate::phrase_calls::SLOT) {
				Node::Meta { node: Box::new(head), data: Box::new(Node::key(crate::phrase_calls::PHRASE_MARK, Node::Text(pattern))) }
			} else {
				head
			}
		};
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// `to kill a person: …` after `to kill a dog: …`: the same phrase with other untyped nouns can only be told apart by types
	pub(super) fn phrase_redefinition(&self, name: &str, parameters: &[Node]) -> Option<Node> {
		let earlier = self.phrase_definitions.get(name).filter(|earlier| earlier.len() == parameters.len())?;
		let untyped = |parameter: &Node| matches!(parameter.drop_meta(), Symbol(_));
		let differing: Vec<(&Node, &Node)> = earlier.iter().zip(parameters).filter(|(before, now)| before != now).collect();
		if differing.is_empty() || !differing.iter().all(|(before, now)| untyped(before) && untyped(now)) {
			return None;
		}
		let classes: Vec<String> = differing.iter().flat_map(|(before, now)| [before.name(), now.name()]).map(|noun| format!("class {noun}")).collect();
		Some(error(&format!("{name} is defined twice; declare {} to dispatch on them", classes.join(" and "))))
	}

	/// Move back to the end of the last thing before the whitespace run behind the cursor
	pub(super) fn seek_back_over_whitespace(&mut self) {
		let mut position = self.pos.min(self.chars.len());
		while position > 0 && self.chars[position - 1].is_whitespace() {
			position -= 1;
		}
		let line_start = self.chars[..position].iter().rposition(|ch| *ch == '\n').map_or(0, |newline| newline + 1);
		self.pos = position;
		self.line_nr = 1 + self.chars[..position].iter().filter(|ch| **ch == '\n').count();
		self.column = position - line_start + 1;
		self.current_line = self.chars[line_start..].iter().take_while(|ch| **ch != '\n').collect();
	}

	/// The rest of the line, or the indented lines below it, or a `{…}` block
	pub(super) fn parse_definition_body(&mut self) -> Node {
		if self.current_char() == '{' {
			return self.parse_atom();
		}
		if !self.only_blanks_before_newline() {
			return self.parse_expr(0);
		}
		self.parse_indented_block().unwrap_or_else(|| error("a definition needs a body: `to name params: body`"))
	}

	/// Offside rule: the lines indented (by tabs or spaces) below a line ending in `:` are its `{…}` block;
	/// None, with nothing consumed, when the next line is not indented deeper
	pub(super) fn parse_indented_block(&mut self) -> Option<Node> {
		let next_line = self.chars[self.pos..].iter().skip_while(|ch| **ch != '\n').skip(1);
		let indented_by_spaces = next_line.take_while(|ch| matches!(ch, ' ' | '\t')).any(|ch| *ch == ' ');
		let before_block = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let outer_counts_spaces = self.indent_counts_spaces;
		self.indent_counts_spaces |= indented_by_spaces;
		let (_, indent) = self.skip_whitespace();
		if indent <= self.base_indent {
			self.indent_counts_spaces = outer_counts_spaces;
			(self.pos, self.line_nr, self.column, self.current_line) = before_block;
			return None;
		}
		let outer_indent = std::mem::replace(&mut self.base_indent, indent);
		let block = self.parse_list_with_separators(None, Bracket::None);
		self.base_indent = outer_indent;
		self.indent_counts_spaces = outer_counts_spaces;
		self.seek_back_over_whitespace(); // the newline after the block still separates the next statement
		Some(curly_block(block))
	}

	/// Ruby/Lua `do … end`, `then … end`, `else … end`: an `end` closes the statements opened here,
	/// counting the `do`/`then` openers in between; an `end` that closes nothing leaves the one-statement body
	pub(super) fn closing_end_follows(&self, openers: &[&str]) -> bool {
		let mut open = 1;
		let mut position = self.pos;
		while position < self.chars.len() {
			let ch = self.chars[position];
			if ch == '"' || ch == '\'' {
				position += 2 + self.chars[position + 1..].iter().position(|quoted| *quoted == ch).unwrap_or(self.chars.len());
				continue;
			}
			if !is_identifier_char(ch) {
				position += 1;
				continue;
			}
			let word_start = position;
			while position < self.chars.len() && is_identifier_char(self.chars[position]) {
				position += 1;
			}
			if word_start > 0 && self.chars[word_start - 1] == '.' {
				continue; // `range.end` is a property
			}
			let word: String = self.chars[word_start..position].iter().collect();
			match word.as_str() {
				END_KEYWORD => open -= 1,
				opener if openers.contains(&opener) => open += 1,
				_ => {}
			}
			if open == 0 {
				return true;
			}
		}
		false
	}

	/// The statements up to the closing `end` (consumed) as a `{…}` block; a `then` block also ends at its `else`
	pub(super) fn parse_end_block(&mut self, stops_at_else: bool) -> Node {
		let outer_else = std::mem::replace(&mut self.stops_at_else, stops_at_else);
		let outer_end = std::mem::replace(&mut self.stops_at_end, true);
		let outer_indent = self.base_indent;
		let before_first_line = (self.pos, self.line_nr, self.column, self.current_line.clone());
		let (_, indent) = self.skip_whitespace();
		(self.pos, self.line_nr, self.column, self.current_line) = before_first_line;
		self.base_indent = self.base_indent.max(indent); // its lines may be indented, the `end` line not
		let block = self.parse_list_with_separators(None, Bracket::None);
		self.base_indent = outer_indent;
		self.stops_at_else = outer_else;
		self.stops_at_end = outer_end;
		self.skip_whitespace();
		if self.matches_keyword(END_KEYWORD) {
			self.advance_by(END_KEYWORD.len());
		}
		curly_block(block)
	}

	/// The `end` (or, in a `then` block, the `else`) that closes the block being parsed
	pub(super) fn at_block_close(&self) -> bool {
		self.stops_at_end && (self.matches_keyword(END_KEYWORD) || (self.stops_at_else && self.matches_keyword(ELSE_KEYWORD)))
	}

	/// `*rest`, `...rest`: the name after the `prefix` characters, starred
	fn parse_starred(&mut self, prefix: usize) -> Node {
		(0..prefix).for_each(|_| self.advance());
		match self.parse_symbol() {
			Ok(name) => Node::Symbol(format!("{}{name}", crate::tuples::STARRED)),
			Err(message) => error(&message),
		}
	}

	/// ` name(`: blanks, a name and its parenthesis
	fn named_call_follows(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name = (blanks..).take_while(|&offset| is_identifier_char(self.peek_char(offset))).count();
		blanks > 0 && name > 0 && self.is_identifier_start(blanks) && self.peek_char(blanks + name) == '('
	}

	pub(super) fn parameters_follow_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		blanks > 0 && self.peek_char(blanks) == '('
	}

	/// Parse symbol with optional suffix: name{...}, name<...>, name(...)
	/// Does NOT handle infix operators like : or = (those are handled by parse_expr)
	/// The soft keyword `version` before digits takes them as written: `version 1.10` is one value, not the float 1.1
	pub(super) fn version_operand(&mut self, symbol: &str) -> Option<Node> {
		if symbol != crate::versions::VERSION_KEYWORD || self.options.data_mode {
			return None;
		}
		let blanks = self.chars[self.pos..].iter().take_while(|c| **c == ' ').count();
		let length = crate::versions::operand_len(&self.chars[self.pos + blanks..]);
		if blanks == 0 || length == 0 {
			return None;
		}
		let operand: String = self.chars[self.pos + blanks..self.pos + blanks + length].iter().collect();
		self.advance_by(blanks + length);
		Some(Node::List(vec![Node::Symbol(symbol.to_string()), Node::Symbol(operand)], Bracket::None, Separator::Space))
	}

	pub(super) fn parse_symbol_with_suffix(&mut self) -> Node {
		let version_len = crate::versions::tagged_literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal: String = self.chars[self.pos..self.pos + version_len].iter().collect();
			self.advance_by(version_len);
			return Node::Symbol(literal);
		}
		let symbol = match self.parse_symbol() {
			Ok(s) => s,
			Err(e) => return error(&e),
		};

		if let Some(version) = self.version_operand(&symbol) {
			return version;
		}

		// `def square (n) {…}` names its function like `def square(n) {…}`
		let names_function = std::mem::replace(&mut self.after_function_keyword, is_function_keyword(&symbol) && !self.options.wit_mode && !self.options.data_mode);
		if names_function && self.parameters_follow_after_blanks() {
			self.skip_spaces();
		}

		if let Some(reference) = self.function_reference_after(&symbol) {
			return reference;
		}

		if self.url_follows(&symbol) {
			return self.parse_url(symbol);
		}

		// C's `void f() { … }`: the result type of a function, not ø (src/lowering/declarations.rs c_function)
		if symbol == VOID_WORD && self.named_call_follows() {
			return Node::Symbol(symbol);
		}

		if let Some(constant) = check_constants(&symbol, self.options.data_mode).filter(|_| !self.at_member_name(symbol.chars().count())) {
			if !self.names_field(&constant) {
				return self.refuse_constant_assignment(&symbol).unwrap_or(constant); // if true {} fall through :?
			}
		}

		if let Some(declaration) = self.try_parse_operator_declaration(&symbol) {
			return declaration;
		}

		// Possessive: `p's name` is the field `p.name`
		if !self.options.data_mode && self.current_char() == '\'' && self.peek_char(1) == 's' && self.peek_char(2) == ' ' && self.is_identifier_start(3) {
			self.advance_by(3);
			return match self.parse_symbol() {
				Ok(field) => Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(field))),
				Err(message) => error(&message),
			};
		}

		// Optional type: `x:int?=ø`, `f(x:int?)`, the field `right? }` (wiki/null.md); a ternary `?` is followed by its branch instead
		if self.current_char() == '?' && (self.ends_optional_type(self.peek_char(1), self.peek_char(2)) || self.closes_after_blanks(1)) {
			self.advance();
			return Symbol(format!("{symbol}?"));
		}

		if symbol == HEX_WORD && !self.options.data_mode {
			if let Some(number) = self.try_parse_hex_word() {
				return number;
			}
		}

		if symbol == TO_WORD && !self.options.data_mode && !self.options.wit_mode && self.word_starts_statement(symbol.len()) {
			if let Some(definition) = self.try_parse_to_definition() {
				return definition;
			}
		}

		// Handle "global" keyword: global name = value
		if symbol == "for" {
			if let Some(loop_node) = self.try_parse_for_in() {
				return loop_node;
			}
		}

		if self.word_starts_statement(symbol.len()) {
			if let Some(skipped) = self.skip_foreign_modifiers(&symbol) {
				return skipped;
			}
		}

		if symbol == "global" || symbol == "export" {
			if let Some(declaration) = self.try_parse_global_declaration(&symbol) {
				return declaration;
			}
		}

		if symbol == NONLOCAL_WORD && self.word_starts_statement(symbol.len()) && self.name_after_blanks() {
			return self.parse_nonlocal_declaration();
		}

		// Handle "class"/"struct"/"type" keyword: class Name { fields }
		// But NOT type(x) which is a function call for type introspection
		if self.options.wit_mode && self.current_char() == '<' {
			return self.parse_type_application(symbol);
		}
		if !self.options.wit_mode && self.declares_type(&symbol) {
			return self.parse_type_declaration();
		}

		// `point {x:1}` with blanks constructs a declared type like the glued `point{x:1}` (open decision 41)
		if self.declared_types.contains(&symbol) && self.block_after_blanks() {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			let block = self.parse_bracketed('{');
			return Node::Key(Box::new(Symbol(symbol)), Op::None, Box::new(block));
		}

		self.parse_glued_suffix(symbol)
	}

	/// `class circle{pi = 3}`: a named number assigned in a type body names the type's own field, which shadows nothing
	fn names_field(&self, constant: &Node) -> bool {
		self.in_type_body && matches!(constant, Node::Number(_)) && self.assignment_follows()
	}

	/// `pi = 4` (P130, user: "loud error, if it was declared constant before which it should be"): a named number is a
	/// declared constant, an assignment to it is an error
	fn refuse_constant_assignment(&self, symbol: &str) -> Option<Node> {
		let column = self.column.saturating_sub(symbol.chars().count());
		let message = format!("{symbol} is a constant");
		self.assignment_follows().then(|| Diagnostic { message, line: self.line_nr, column, ..Default::default() }.fix("another name").into_error())
	}

	/// `= …` or `:= …` after blanks, not the comparison `==`
	fn assignment_follows(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let defines = self.peek_char(blanks) == ':' && self.peek_char(blanks + 1) == '=';
		let assigns = self.peek_char(blanks) == '=' && self.peek_char(blanks + 1) != '=';
		defines || assigns
	}

	/// `http://…`, `file://…`: the scheme of a URL, the rest of which reads as one text
	pub(super) fn url_follows(&self, symbol: &str) -> bool {
		URL_SCHEMES.contains(&symbol) && self.current_char() == ':' && self.peek_char(1) == '/' && self.peek_char(2) == '/'
	}

	pub(super) fn parse_url(&mut self, scheme: String) -> Node {
		let mut url = scheme;
		while !self.is_url_terminator(self.current_char()) {
			url.push(self.current_char());
			self.advance();
		}
		Node::Text(url)
	}

	/// `class Name {…}`, `struct`, `type Name {…}`, `record Name {…}`; `class:"btn"`, `type:email` (html attributes),
	/// `x.class` and `type(x)` are no declarations
	pub(super) fn declares_type(&self, symbol: &str) -> bool {
		let is_key = self.current_char() == ':' && self.peek_char(1) != '=';
		let start = self.pos.saturating_sub(symbol.chars().count());
		let is_field = start > 0 && self.chars[start - 1] == '.';
		!is_key && !is_field && (TYPE_DECLARATION_WORDS.contains(&symbol)
			|| (symbol == RECORD_WORD && self.name_and_block_follow())
			|| (symbol == "type" && self.current_char() != '('))
	}

	/// The name and the `{fields}` of a type declaration, after its keyword
	pub(super) fn parse_type_declaration(&mut self) -> Node {
		let outer = std::mem::replace(&mut self.in_type_body, true);
		let declaration = self.parse_type_declaration_body();
		self.in_type_body = outer;
		declaration
	}

	fn parse_type_declaration_body(&mut self) -> Node {
		self.skip_whitespace();
		let type_name = match self.parse_symbol() {
			Ok(name) => name,
			Err(message) => return error(&message),
		};
		// `class Box<T>{item:T}`: a field or parameter of a type parameter holds any value
		let type_parameters = match self.current_char() {
			'<' => match self.parse_type_parameters() {
				Ok(parameters) => parameters,
				Err(message) => return error(&message),
			},
			_ => vec![],
		};
		self.skip_whitespace();
		// `class dog extends animal {…}` (P117): the parent rides on the name, class_methods copies its fields and methods
		let mut name = Symbol(type_name);
		if self.matches_keyword(EXTENDS_KEYWORD) {
			self.advance_by(EXTENDS_KEYWORD.len());
			self.skip_whitespace();
			match self.parse_symbol() {
				Ok(parent) => name = name.with_attribute(EXTENDS_KEYWORD, Symbol(parent)),
				Err(message) => return error(&message),
			}
			self.skip_whitespace();
		}
		let body = if self.current_char() == '{' { Self::transform_fields_to_types(self.parse_bracketed('{')) } else { Empty };
		let body = if type_parameters.is_empty() { body } else { any_for_type_parameters(body, &type_parameters) };
		Node::Type { name: Box::new(name), body: Box::new(body) }
	}

	/// `<T>`, `<A, B>`: the names of the type parameters of a declared type
	fn parse_type_parameters(&mut self) -> Result<Vec<String>, String> {
		self.advance(); // <
		let mut names = vec![];
		loop {
			self.skip_whitespace();
			names.push(self.parse_symbol()?);
			self.skip_whitespace();
			match self.current_char() {
				',' => self.advance(),
				'>' => {
					self.advance();
					return Ok(names);
				}
				other => return Err(format!("type parameters <{}…> end with >, got {other}", names.join(", "))),
			}
		}
	}

	/// What is glued to a word: `name{…}`, `List<int>`, `p@unit`, `f(args)`, `f(params) {body}`; else the word itself
	pub(super) fn parse_glued_suffix(&mut self, symbol: String) -> Node {
		let ch = self.current_char();
		match ch {
			'{' => {
				let mut blocks = vec![self.parse_bracketed('{')];
				// `a{x:1}{y:2}{3}`: glued blocks are the children of a, `a{x}{y z}` is no `a{x, {y z}}`
				while self.current_char() == '{' {
					blocks.push(self.parse_bracketed('{'));
				}
				let block = match blocks.len() {
					1 => blocks.remove(0),
					_ => Node::List(blocks, Bracket::None, Separator::None),
				};
				// `point{x:1}` of a declared type constructs a point, `point:{x:1}` and any other `name{…}` stay data (D4)
				let op = if self.declared_types.contains(&symbol) { Op::None } else { Op::Colon };
				Node::Key(Box::new(Symbol(symbol)), op, Box::new(block))
			}
			'<' if !self.options.xml_mode && !self.options.data_mode && self.type_application_length().is_some() => {
				let length = self.type_application_length().expect("guarded");
				let arguments: String = (1..length - 1).map(|offset| self.peek_char(offset)).collect();
				if symbol == "list" && !arguments.contains(',') {
					let element_words: Vec<&str> = arguments.split(|c: char| c == '<' || c == '>' || c.is_whitespace()).filter(|word| !word.is_empty()).collect();
					set_hint_position(self.line_nr, self.column.saturating_sub(symbol.chars().count()));
					norm::list_type(ListTypeStyle::Generic, &element_words);
				}
				self.advance_by(length);
				// `Box<int>(3)`, `Box<int>{item:3}` of a declared class `Box<T>`: its type parameters hold any value
				if self.declared_types.contains(&symbol) {
					return self.parse_glued_suffix(symbol);
				}
				Symbol(type_application_name(&symbol, &arguments))
			}
			'<' if !self.options.xml_mode && !self.peek_char(1).is_numeric() && self.peek_char(1) != '<' && names_a_type(&symbol) => {
				// Only treat as generic if immediately after a type name (no space)
				// and NOT followed by a number (that would be comparison: i<9); `it<k` and `a<b` compare
				let generic = self.parse_bracketed('<');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(generic))
			}
			// `info@pannous.com` in data is one word; `p@unit` in code reads the meta key: `p.@unit`
			'@' if self.peek_char(1).is_alphabetic() && self.options.data_mode => {
				let rest: String = self.chars[self.pos..].iter().take_while(|c| is_identifier_char(**c) || matches!(c, '@' | '.' | '-')).collect();
				let rest = rest.trim_end_matches(['.', '-']).to_string();
				self.advance_by(rest.chars().count());
				Symbol(format!("{symbol}{rest}"))
			}
			'@' if self.peek_char(1).is_alphabetic() => {
				self.advance();
				match self.parse_symbol() {
					Ok(key) => Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(format!("{ATTRIBUTE_MARK}{key}")))),
					Err(message) => error(&message),
				}
			}
			'(' => {
				// Parse arguments as a proper Node
				let args_node = self.parse_bracketed('(');
				self.skip_spaces(); // Only spaces, preserve newlines as statement separators

				// in a condition `if f(1, 2) {…}` the block is the body of the `if`, not of a definition of f
				if self.current_char() == '{' && !self.equals_compares && !self.in_for_header {
					// Function with body: name(params) { body }
					let body = self.parse_bracketed('{');
					let signature = Node::List(
						vec![Symbol(symbol), typed_parameters(args_node)],
						Bracket::Round,
						Separator::None,
					);
					Node::List(vec![signature, body], Bracket::Round, Separator::None)
				} else if symbol == PRINT_WORD {
					print_call(print_arguments(args_node))
				} else {
					// Function call: name(params) -> List([symbol, args...])
					let mut items = vec![Symbol(symbol)];
					match typed_parameters(args_node) {
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
	pub(super) fn advance_by(&mut self, n: usize) {
		for _ in 0..n {
			self.advance();
		}
	}
}

/// A class body with each type parameter used as a type read as any type: a field `item:T` holds any value, a
/// parameter `with(x:T)` takes any
fn any_for_type_parameters(node: Node, parameters: &[String]) -> Node {
	let is_parameter = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) => parameters.contains(name),
		Node::Type { name, body } => matches!(body.drop_meta(), Node::Empty) && parameters.contains(&name.drop_meta().name()),
		_ => false,
	};
	match node {
		Node::Key(name, Op::Colon, kind) if is_parameter(&kind) => Node::Key(name, Op::Colon, Box::new(Symbol(crate::type_kinds::UNTYPED_FIELD.to_string()))),
		Node::Key(head, Op::Define, body) => Node::Key(Box::new(untyped_parameters(*head, &is_parameter)), Op::Define, body),
		other => other.map_children(|child| any_for_type_parameters(child, parameters)),
	}
}

/// A method head with each parameter of a type parameter untyped: `with(x:T)` is `with(x)`
fn untyped_parameters(head: Node, is_parameter: &dyn Fn(&Node) -> bool) -> Node {
	match head {
		Node::Key(name, Op::Colon, kind) if is_parameter(&kind) => *name,
		Node::Meta { node, data } => Node::Meta { node: Box::new(untyped_parameters(*node, is_parameter)), data },
		other => other.map_children(|child| untyped_parameters(child, is_parameter)),
	}
}
