//! Atoms: symbols with suffixes, function calls, keyword forms and glued suffixes

use super::*;
use crate::warp_parser::context::Context;
use crate::words::STYLE_WORD;
use crate::words::SWITCH_WORDS;

use crate::words::VOID_WORD;
use crate::words::{WASM_INSTRUCTION_CALL, WASM_WORD};

/// Rust's endless `loop { … }`
const LOOP_WORD: &str = "loop";
/// The words of a block of code, also glued to it: `go{…}` starts a task as `go {…}` does (lowering/control/go_blocks.rs,
/// card error-beep), `loop{…}` loops, `do{…}` runs its block (card glued-loop); no tags `go:{…}`
const GLUED_BLOCK_KEYWORDS: [&str; 3] = ["go", "loop", "do"];
/// Other languages' print words (wiki/Features.md); `log` stays the logarithm, `puts` the WASI text output
const PRINT_SPELLINGS: [&str; 2] = ["println", "writeln"];
const TO_SENTENCE_WORD: &str = "To";
/// `none` is the null ø; called, `none(xs, f)` (Kotlin's "no element matches") is an error naming it (P185)
const NONE_WORD: &str = "none";
const NONE_CALL_ERROR: &str = "none is the null ø, not a function: for \"no element matches\" write not any(xs, f)";
/// The first argument of a block, what Elixir's `&1` and the element of Ruby's `&:to_s` are
const CAPTURED_ARGUMENT: &str = "$0";

impl WarpParser {
	/// Parse an atomic expression (no infix operators)
	/// Handles: numbers, strings, brackets, symbols with named blocks
	pub(super) fn parse_atom(&mut self) -> Node {
		let (_, _, comment) = self.skip_whitespace_and_comments();
		let (line_nr, column) = self.get_position();

		if self.is_at_line_end() {
			return Empty;
		}

		let node = match self.current_char() {
			// Rust's `&x`, `&mut x`, `&&x` and `*r`: values need no references, so a reference and its dereference read as the
			// value itself (card rust-refs); `&name` of a function stays below
			'&' | '*' if self.starts_rust_reference() => {
				self.skip_reference_prefix();
				return self.parse_atom();
			}
			// `&name` is the function `name` itself: a name is already a function value where a function is expected
			'&' if self.starts_function_reference() => self.symbol_after(1, crate::closures::function_reference),
			// Elixir's capture `&(&1 * 2)`: the block `{$0 * 2}`; its `&1`, `&2` are `$0`, `$1`
			'&' if self.starts_capture() => {
				self.advance();
				let group =
					self.within(Context { in_capture: true, ..self.context }, |parser| parser.parse_bracketed('('));
				match group {
					Node::List(items, Bracket::Round, separator) => Node::List(items, Bracket::Curly, separator),
					other => Node::List(vec![other], Bracket::Curly, Separator::None),
				}
			}
			'&' if self.peek_char(1).is_ascii_digit() && self.context.in_capture => {
				self.advance();
				let position: usize = self.take_while_char(|ch| ch.is_ascii_digit()).parse().unwrap_or(1);
				Node::Symbol(format!("${}", position.saturating_sub(1)))
			}
			// Ruby's `&:to_s`: the block `{$0.to_s}`, the method called on each element
			'&' if self.peek_char(1) == ':' && self.peek_char(2).is_alphabetic() => self.symbol_after(2, |method| {
				let call = Node::Key(
					Box::new(Node::Symbol(CAPTURED_ARGUMENT.to_string())),
					Op::Dot,
					Box::new(Node::Symbol(method)),
				);
				Node::List(vec![call], Bracket::Curly, Separator::None)
			}),
			'"' | '\'' | '«' | '`' => self.parse_string(),
			'#' if let Some(length) = self.css_color_length() => Node::Text(self.take_chars(length)),
			// `://warp.pannous.com/lib/libc.h`: a URL taking the page's protocol (card pannous-com, with_default_scheme)
			':' if self.url_follows() => self.parse_url(String::new()),
			':' if self.at_pseudo_class_selector() => self.symbol_after(1, |name| Node::Symbol(format!(":{name}"))),
			// `:src` (wiki/symbol.md): the symbol itself, never evaluated; data keeps it as the name
			':' if self.at_explicit_symbol() => {
				let data_mode = self.options.data_mode;
				self.symbol_after(1, |name| match data_mode {
					true => Node::Symbol(name),
					false => call(crate::words::SYMBOL_WORD, vec![Node::Text(name)]),
				})
			}
			'.' | '~' | '/' if self.starts_path_literal() => self.parse_path_literal(),
			// `a, *rest = xs`: the starred name takes the items the other names leave (src/lowering/types/tuples.rs); `...rest`
			// (JS) is the starred `*rest` too: a rest parameter or a spread argument (src/lowering/definitions/variadic.rs)
			'.' if self.peek_char(1) == '.' && self.peek_char(2) == '.' && self.is_identifier_start(3) => {
				self.parse_starred(3)
			}
			// Python's `**kw`: the keyword arguments as one object (src/lowering/definitions/variadic.rs)
			'*' if self.peek_char(1) == '*' && self.is_identifier_start(2) => match self.parse_starred(2) {
				Node::Symbol(name) => Node::Symbol(format!("{}{name}", crate::words::STARRED)),
				other => other,
			},
			'*' if self.is_identifier_start(1) => self.parse_starred(1),
			// `.card { … }`: a leading-dot name, a class selector in a style block (lib/markup.warp style)
			'.' if self.is_identifier_start(1) => self.symbol_after(1, |name| Node::Symbol(format!(".{name}"))),
			'-' if self.peek_char(1) == '>' && !self.options.data_mode => self.parse_stabby_lambda(),
			'[' => {
				let list = self.parse_bracketed('[');
				self.with_unit_after_list(list)
			}
			'(' | '{' => self.parse_bracketed(self.current_char()),
			'<' if self.options.xml_mode => self.parse_xml_tag(),
			'<' => self.parse_bracketed('<'),
			';' | '>' | '}' | ')' | ']' => Empty, // Closing brackets/terminators handled by caller
			'ø' => {
				self.advance();
				return Empty;
			}
			'∞' => {
				self.advance();
				return Node::Number(Number::Inf);
			} // the float infinity (P56)
			// `°C`, `°F`: a temperature unit; `30°`: an angle in degrees (units.rs)
			crate::numbers::units::DEGREE if self.peek_char(1).is_alphabetic() => {
				self.symbol_after(1, |name| Node::Symbol(format!("{}{name}", crate::numbers::units::DEGREE)))
			}
			crate::numbers::units::DEGREE => {
				self.advance();
				Node::Symbol(crate::numbers::units::DEGREE.to_string())
			}
			_ if let Some((truth, length)) = self.truth_glyph_at() => {
				self.advance_by(length);
				return truth;
			}
			'@' if self.peek_char(1).is_alphabetic() => self.parse_attribute(),
			'@' if self.peek_char(1) == '(' && !self.options.data_mode => self.parse_matlab_lambda(),
			'\\' if self.backslash_lambda_ahead() && !self.options.data_mode => self.parse_backslash_lambda(),
			// `@1`: a reference to the node whose @id is 1 (wiki/reference.md, P160)
			'@' if self.peek_char(1).is_ascii_digit() => {
				self.advance(); // skip '@'
				Node::Symbol(format!("{ATTRIBUTE_MARK}{}", self.take_while_char(|ch| ch.is_ascii_digit())))
			}
			'$' if self.peek_char(1).is_alphabetic() || self.peek_char(1) == '_' => self.parse_dollar_name(),
			// jQuery's `$('#who')`: the call of `$`, which page_names.rs answers with a hint naming `$who` and `who`
			'$' if self.peek_char(1) == '(' => {
				self.advance(); // skip '$'
				let selector = self.parse_bracketed('(');
				function_call("$".to_string(), selector)
			}
			// `$0`, the first parameter
			'$' if self.peek_char(1).is_numeric() => {
				self.advance(); // skip '$'
				Node::Symbol(format!("${}", self.take_while_char(char::is_numeric)))
			}
			ch if ch.is_numeric() || self.number_starts_at(0) || (ch == '-' && self.number_starts_at(1)) => {
				self.parse_number()
			}
			ch if ch.is_alphabetic() || ch == '_' => {
				let atom = self
					.try_parse_count_word()
					.or_else(|| self.parse_elixir_function())
					.or_else(|| self.parse_python_lambda())
					.or_else(|| self.parse_f_string())
					.or_else(|| self.parse_shader_block())
					.unwrap_or_else(|| self.parse_symbol_with_suffix());
				// the function's own atom takes the mark, not an atom parsed inside it (its parameters)
				let head = match atom.drop_meta() {
					Node::List(items, _, _) => items.first().map(Node::name),
					other => Some(other.name()),
				};
				match self.generic_names.take() {
					Some((name, names)) if head.as_ref() == Some(&name) => Node::Meta {
						node: Box::new(atom),
						data: Box::new(Node::key(crate::words::GENERIC_MARK, Node::Text(names.join(" ")))),
					},
					pending => {
						self.generic_names = pending;
						atom
					}
				}
			}
			'\\' if let Some((name, length)) =
				crate::text::uniscript_entities::entity_name_at(&self.chars, self.pos) =>
			{
				let warned = self.warn_unknown_entity(&name, self.column);
				self.advance_by(length);
				warned.map_or_else(|strict| strict, |()| Node::Text(format!("\\:{name}")))
			}
			'\\' if let Some(name) = crate::text::uniscript_entities::bare_entity_name_at(&self.chars, self.pos) => {
				self.advance_by(name.len() + 1);
				Diagnostic::default()
					.message(format!("a uniscript entity is written \\:{name}, not \\{name}"))
					.offer("the uniscript entity", format!("\\{name}"), format!("\\:{name}"))
					.into_error()
			}
			_ if let Some((operator, length)) = self.bare_operator_in_block() => {
				self.advance_by(length);
				error(&format!(
					"{{{operator}}} is an operator without operands, no function: write them, e.g. map xs {{it + 1}}"
				))
			}
			ch if crate::extensions::strings::starts_an_emoji(ch) => self.parse_emoji(),
			ch if crate::warp_parser::definitions::is_free_symbol(ch) => {
				self.advance();
				Node::Symbol(ch.to_string())
			}
			ch => {
				warn!("Unexpected character '{}' at line {}, column {}", ch, line_nr, column);
				self.advance();
				error(&format!("Unexpected character '{}'", ch))
			}
		};

		if node == Empty {
			return node;
		}

		// Attach metadata
		let node = node.with_meta_data(LineInfo::at(line_nr, column, &self.current_line));
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

	/// `*rest`, `...rest`: the name after the `prefix` characters, starred
	fn parse_starred(&mut self, prefix: usize) -> Node {
		self.symbol_after(prefix, |name| Node::Symbol(format!("{}{name}", crate::words::STARRED)))
	}

	/// ` name(`: blanks, a name and its parenthesis
	fn named_call_follows(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let name = (blanks..).take_while(|&offset| is_identifier_char(self.peek_char(offset))).count();
		blanks > 0 && name > 0 && self.is_identifier_start(blanks) && self.peek_char(blanks + name) == '('
	}

	/// ` () {` after a name
	fn empty_parameters_and_block_follow(&self) -> bool {
		let blanks_from =
			|start: usize| (start..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		let blanks = blanks_from(0);
		let inner = blanks_from(blanks + 1);
		let closed = blanks + 1 + inner;
		blanks > 0
			&& self.peek_char(blanks) == '('
			&& self.peek_char(closed) == ')'
			&& self.peek_char(closed + 1 + blanks_from(closed + 1)) == '{'
	}

	pub(super) fn parameters_follow_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		blanks > 0 && self.peek_char(blanks) == '('
	}

	pub(super) fn parse_symbol_with_suffix(&mut self) -> Node {
		let version_len = crate::versions::literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal = self.take_chars(version_len);
			return Node::Symbol(literal);
		}
		let symbol = or_return_error!(self.parse_symbol());
		let symbol = self.joined_type_name(symbol);
		let symbol = self.print_spelled(symbol);

		if let Some(version) = self.version_operand(&symbol) {
			return version;
		}
		// `x°`: a variable in degrees, as `30°` (card angles-degrees)
		if self.current_char() == crate::numbers::units::DEGREE && !self.peek_char(1).is_alphabetic() {
			self.advance();
			return Node::Key(
				Box::new(Node::Symbol(symbol)),
				Op::Mul,
				Box::new(Node::Symbol(crate::numbers::units::DEGREE.to_string())),
			);
		}

		self.read_function_name_suffix(&symbol);
		self.skip_turbofish();

		if let Some(reference) = self.function_reference_after(&symbol) {
			return reference;
		}

		if self.url_follows() {
			return self.parse_url(symbol);
		}

		// C's `void f() { … }`: the result type of a function, not ø (src/lowering/definitions/declarations.rs c_function)
		if symbol == VOID_WORD && self.named_call_follows() {
			return Node::Symbol(symbol);
		}

		if symbol == NONE_WORD && self.peek_char(0) == '(' && !self.options.data_mode {
			return error(NONE_CALL_ERROR);
		}
		if let Some(constant) = check_constants(&symbol, self.options.data_mode).filter(|_| {
			let length = symbol.chars().count();
			!self.at_member_name(length) && !self.at_scoped_name(length) && !self.context.in_enum_cases
		}) {
			if !self.names_field(&symbol, &constant) {
				return self.refuse_constant_assignment(&symbol).unwrap_or(constant); // if true {} fall through :?
			}
		}

		if let Some(declaration) = self.try_parse_operator_declaration(&symbol) {
			return declaration;
		}
		if let Some(instruction) = self.wasm_instruction_after(&symbol) {
			return instruction;
		}

		// Possessive: `p's name` is the field `p.name`
		if !self.options.data_mode
			&& self.current_char() == '\''
			&& self.peek_char(1) == 's'
			&& self.peek_char(2) == ' '
			&& self.is_identifier_start(3)
		{
			return self.symbol_after(3, |field| Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(field))));
		}

		// Optional type: `x:int?=ø`, `f(x:int?)`, the field `right? }` (wiki/null.md); a ternary `?` is followed by its branch instead
		if self.current_char() == '?'
			&& (self.ends_optional_type(self.peek_char(1), self.peek_char(2))
				|| self.closes_after_blanks(1)
				|| self.next_field_follows(1)
				|| self.declaration_follows(&symbol, 1))
		{
			self.advance();
			return Symbol(format!("{symbol}?"));
		}

		if let Some(form) = self.keyword_form(&symbol) {
			return form;
		}

		// `point {x:1}` with blanks constructs a declared type like the glued `point{x:1}` (open decision 41)
		// a capitalized word of no declared type before a block of fields is the tagged object `Person:{…}` (data, D4), as
		// glued `Person{…}` is: the README's `Person { name: "Alice" … }` (card person-name)
		let tagged = symbol.starts_with(|first: char| first.is_uppercase());
		let declared = self.declared_types.contains(&symbol);
		// inside a data literal any spaced `c { … }` is the child c{ … } (card spaced-child), but in a for header
		// `for t in todos { … }` it is the collection and the body (card markup-ul)
		let spaced_child =
			self.context.in_data_literal && !self.context.in_for_header && !declared && self.blanks_then('{');
		if spaced_child || ((declared || tagged) && self.block_after_blanks(declared)) {
			self.skip_blanks();
		}
		self.parse_glued_suffix(symbol)
	}

	/// The blanks and type parameters after a function's name: `def square (n) {…}` names its function like
	/// `def square(n) {…}`
	fn read_function_name_suffix(&mut self, symbol: &str) {
		let in_code = self.in_code();
		let names_function =
			std::mem::replace(&mut self.after_function_keyword, is_function_keyword(symbol) && in_code);
		// C's `int boom () { -1 }` (wiki/type.md): a name, `()` and a block are a definition like `boom() {…}`
		let spaced_definition = in_code && self.empty_parameters_and_block_follow();
		if (names_function && self.parameters_follow_after_blanks()) || spaced_definition {
			self.skip_spaces();
		}
		// Rust's and C#'s `fn apply<F: Fn(i32) -> i32>(f: F)`, `id<T>(x: T)`: the type parameters say nothing warp needs
		// P157 (user): no generics in warp; the name carries the type parameters' names (GENERIC_MARK), welcome_forms.rs
		// drops their annotations: `fn id<T>(x: T) -> T` is `def id(x)`
		if let Some(length) = if names_function { self.generic_parameters_length() } else { None } {
			let written: String = self.chars[self.pos + 1..self.pos + length - 1].iter().collect();
			self.advance_by(length);
			self.generic_names = Some((symbol.to_string(), type_parameter_names(&written)));
		}
	}

	/// Rust's turbofish `sum::<i32>()`, `collect::<Vec<i32>>()`: the type arguments say nothing warp needs, the call is
	/// `sum()` (card rust-paths)
	fn skip_turbofish(&mut self) {
		if self.current_char() != ':' || self.peek_char(1) != ':' || self.peek_char(2) != '<' {
			return;
		}
		let start = self.mark();
		self.advance_by(2);
		match self.generic_parameters_length() {
			Some(length) => self.advance_by(length),
			None => self.rewind(start),
		}
	}

	/// A form a keyword starts: `to square a number: …`, `for`, `global`, `nonlocal`, a type or enum declaration, another
	/// language's way of writing a warp form
	fn keyword_form(&mut self, symbol: &str) -> Option<Node> {
		// `To square a number: …` starts an English sentence with a capital
		if (symbol == TO_WORD || symbol == TO_SENTENCE_WORD)
			&& self.in_code()
			&& self.word_starts_statement(symbol.len())
		{
			if let Some(definition) = self.try_parse_to_definition() {
				return Some(definition);
			}
		}

		// Handle "global" keyword: global name = value
		// `each x in xs: body` is `for each x in xs: body`
		if symbol == "for" || (symbol == EACH_WORD && self.loop_variable_in_follows()) {
			if let Some(loop_node) = self.try_parse_for_in() {
				return Some(loop_node);
			}
		}

		if self.word_starts_statement(symbol.len()) {
			if let Some(skipped) = self.skip_foreign_modifiers(symbol) {
				return Some(skipped);
			}
		}

		// Rust's `loop { … }` is `while true { … }`, its statement ends with the block as a while's does (card rust-option)
		if symbol == LOOP_WORD && self.in_code() && self.blanks_then('{') {
			self.skip_blanks();
			let body = self.parse_bracketed('{');
			return Some(while_do(Node::True, body));
		}

		if symbol == "global" || symbol == "export" {
			if let Some(declaration) = self.try_parse_global_declaration(symbol) {
				return Some(declaration);
			}
		}

		if symbol == NONLOCAL_WORD && self.word_starts_statement(symbol.len()) && self.name_after_blanks() {
			return Some(self.parse_nonlocal_declaration());
		}

		// Handle "class"/"struct"/"type" keyword: class Name { fields }
		// But NOT type(x) which is a function call for type introspection
		if self.options.wit_mode && self.current_char() == '<' {
			return Some(self.parse_type_application(symbol.to_string()));
		}
		if !self.options.wit_mode && self.declares_type(symbol) {
			return Some(if symbol == CLASS_KEYWORD {
				self.parse_class_declaration()
			} else {
				self.parse_type_declaration()
			});
		}
		// P178: `enum Shape { Circle(r), Rect(w, h), Dot }`, cases with values: the sum type `Circle(r) | Rect(w, h) | Dot`
		if symbol == ENUM_WORD && self.in_code() {
			match self.enum_body_ahead() {
				Some(true) => return Some(self.parse_enum_with_values()),
				Some(false) => return Some(self.parse_plain_enum()),
				None => {}
			}
		}
		self.foreign_form(symbol)
	}

	/// `println x`, `writeln x`: print, with a note; a function of the program keeps its name
	fn print_spelled(&self, symbol: String) -> String {
		let named = self.functions.contains(&symbol) || self.defines_named(0);
		if !PRINT_SPELLINGS.contains(&symbol.as_str()) || !self.in_code() || named {
			return symbol;
		}
		note_alias_at(self.start_of(&symbol), &symbol, PRINT_WORD);
		PRINT_WORD.to_string()
	}

	/// Where the symbol just read starts: line and column
	pub(crate) fn start_of(&self, symbol: &str) -> (usize, usize) {
		(self.line_nr, self.column.saturating_sub(symbol.chars().count()))
	}

	/// What is glued to a word: `name{…}`, `List<int>`, `p@unit`, `f(args)`, `f(params) {body}`; else the word itself
	pub(super) fn parse_glued_suffix(&mut self, symbol: String) -> Node {
		let ch = self.current_char();
		match ch {
			'{' if GLUED_BLOCK_KEYWORDS.contains(&symbol.as_str()) && self.in_code() => Node::symbol(&symbol),
			'{' => {
				// `point{x:1}` of a declared type constructs a point, `point:{x:1}` and any other `name{…}` stay data (D4)
				let op = if self.declared_types.contains(&symbol) { Op::None } else { Op::Colon };
				// a rule `p { … }` of a style sheet is still the sheet
				let in_style_sheet = op == Op::Colon && (symbol == STYLE_WORD || self.context.in_style_sheet);
				self.tag_block = op == Op::Colon;
				let data_literal = Context { in_data_literal: op == Op::Colon, in_style_sheet, ..self.context };
				let blocks = self.within(data_literal, |parser| {
					let mut blocks = vec![parser.parse_bracketed('{')];
					// `a{x:1}{y:2}{3}`: glued blocks are the children of a, `a{x}{y z}` is no `a{x, {y z}}`
					while parser.current_char() == '{' {
						blocks.push(parser.parse_bracketed('{'));
					}
					blocks
				});
				let block = Node::single_or_list(blocks, Bracket::None, Separator::None);
				Node::Key(Box::new(Symbol(symbol)), op, Box::new(block))
			}
			'<' if !self.options.xml_mode
				&& !self.options.data_mode
				&& let Some(length) = self.type_application_length() =>
			{
				let arguments: String = (1..length - 1).map(|offset| self.peek_char(offset)).collect();
				if symbol == "list" && !arguments.contains(',') {
					let element_words: Vec<&str> = arguments
						.split(|c: char| c == '<' || c == '>' || c.is_whitespace())
						.filter(|word| !word.is_empty())
						.collect();
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
			'<' if !self.options.xml_mode
				&& !self.peek_char(1).is_numeric()
				&& self.peek_char(1) != '<'
				&& names_a_type(&symbol) =>
			{
				// Only treat as generic if immediately after a type name (no space)
				// and NOT followed by a number (that would be comparison: i<9); `it<k` and `a<b` compare
				let generic = self.parse_bracketed('<');
				Node::Key(Box::new(Symbol(symbol)), Op::Colon, Box::new(generic))
			}
			// `info@pannous.com` in data is one word; `p@unit` in code reads the meta key: `p.@unit`
			'@' if self.peek_char(1).is_alphabetic() && self.options.data_mode => {
				let rest: String = self.chars[self.pos..]
					.iter()
					.take_while(|c| is_identifier_char(**c) || matches!(c, '@' | '.' | '-'))
					.collect();
				let rest = rest.trim_end_matches(['.', '-']).to_string();
				self.advance_by(rest.chars().count());
				Symbol(format!("{symbol}{rest}"))
			}
			'@' if self.peek_char(1).is_alphabetic() => self.symbol_after(1, |key| {
				Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(format!("{ATTRIBUTE_MARK}{key}"))))
			}),
			'(' => {
				// `match f(3, 4) {…}`: the block holds the cases, no body of a definition of f
				let is_switch_subject =
					SWITCH_WORDS.contains(&self.word_ending_before(self.pos - symbol.chars().count()).as_str());
				// Parse arguments as a proper Node
				let args_node = self.parse_bracketed('(');
				// `add(2)(5)`: each glued group calls what the call before returned (closures.rs), one operand
				if self.current_char() == '(' && symbol != PRINT_WORD {
					let mut chain = vec![function_call(symbol, args_node)];
					while self.current_char() == '(' {
						chain.push(argument_group(self.parse_bracketed('(')));
					}
					return Node::List(chain, Bracket::None, Separator::Space);
				}
				self.skip_spaces(); // Only spaces, preserve newlines as statement separators

				// in a condition `if f(1, 2) {…}` the block is the body of the `if`, not of a definition of f
				if self.current_char() == '{'
					&& !self.context.equals_compares
					&& !self.context.in_for_header
					&& !is_switch_subject
				{
					// Function with body: name(params) { body }, its head the call
					let body = self.parse_bracketed('{');
					Node::List(vec![function_call(symbol, args_node), body], Bracket::Round, Separator::None)
				} else if symbol == PRINT_WORD {
					print_call(print_arguments(args_node))
				} else {
					function_call(symbol, args_node)
				}
			}
			_ => Node::symbol(&symbol),
		}
	}

	/// `wasm f64.floor`, the word wasm and an instruction name (a type, a dot, its operation): the call
	/// `wasm_instruction("f64.floor")`; any other `wasm` stays a word
	fn wasm_instruction_after(&mut self, symbol: &str) -> Option<Node> {
		if symbol != WASM_WORD || self.options.data_mode || self.current_char() != ' ' {
			return None;
		}
		let rest = &self.chars[self.pos + 1..];
		let is_name_char = |c: &&char| c.is_ascii_lowercase() || c.is_ascii_digit() || **c == '_';
		let type_length = rest.iter().take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).count();
		if type_length == 0 || rest.get(type_length) != Some(&'.') {
			return None;
		}
		let operation_length = rest[type_length + 1..].iter().take_while(is_name_char).count();
		if operation_length == 0 {
			return None;
		}
		self.advance();
		let name = self.take_chars(type_length + 1 + operation_length);
		if crate::wasm_emitter::functions::wasm_bodies::instruction(&name).is_none() {
			return Some(error(&format!(
				"wasm {name} is no instruction warp takes: the ones on i64 and f64, e.g. wasm f64.sqrt"
			)));
		}
		Some(crate::node::call(WASM_INSTRUCTION_CALL, vec![Node::Text(name)]))
	}

	pub(super) fn parse_symbol(&mut self) -> Result<String, String> {
		let mut symbol = String::new();
		loop {
			let ch = self.current_char();
			// Include alphanumeric, underscore, and hyphen (for kebab-case)
			// BUT: a hyphen joins only a following word, so `i--` and `x-1` end the symbol at the hyphen
			let is_hyphen_in_symbol = ch == '-' && hyphen_joins_word(self.peek_char(1));
			if (ch.is_alphanumeric() && !is_number_glyph(ch))
				|| ch == '_' || is_hyphen_in_symbol
				|| !symbol.is_empty() && is_music_accidental(ch)
			{
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
}

/// Function call: name(params) -> List([symbol, args...])
fn function_call(symbol: String, args_node: Node) -> Node {
	let mut items = vec![Symbol(symbol)];
	match typed_parameters(args_node) {
		// Swift's labeled parameter `greet(person name: String)` stays one argument, unlike Go's `f(a, b int)`
		Node::List(args, bracket, Separator::Space) if is_labeled_parameter(&args) => {
			items.push(Node::List(args, bracket, Separator::Space))
		}
		Node::List(args, _, _) => items.extend(args),
		Node::Empty => {}
		other => items.push(other),
	}
	Node::List(items, Bracket::Round, Separator::None)
}

/// `person name: String`: a label, then the typed parameter
fn is_labeled_parameter(words: &[Node]) -> bool {
	matches!(words, [label, typed] if matches!(label.drop_meta(), Symbol(_)) && matches!(typed.drop_meta(), Node::Key(name, Op::Colon, _) if matches!(name.drop_meta(), Symbol(_))))
}

/// The arguments of a glued group `(5)`, `(1, 2)`, `()`, as a round list
fn argument_group(arguments: Node) -> Node {
	match arguments {
		Node::List(items, Bracket::Round, separator) => Node::List(items, Bracket::Round, separator),
		Node::List(items, _, _) => Node::List(items, Bracket::Round, Separator::None),
		Node::Empty => Node::List(vec![], Bracket::Round, Separator::None),
		other => Node::List(vec![other], Bracket::Round, Separator::None),
	}
}

/// `full-name`: inside a name a hyphen joins the word that follows (`x-1` and `i--` end the name at the hyphen)
pub(crate) fn hyphen_joins_word(next: char) -> bool {
	next.is_alphabetic() || next == '_'
}

/// A got-it note on another language's word, pointing at where the word starts
pub(crate) fn note_alias_at((line, column): (usize, usize), written: &str, warp_word: &str) {
	crate::normalize::set_hint_position(line, column);
	crate::errors::diagnostic::note_alias(written, warp_word);
}
