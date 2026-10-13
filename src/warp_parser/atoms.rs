//! Atoms: symbols with suffixes, definitions and their bodies, indented and end blocks, URLs, type declarations

use super::*;
use crate::node::children_rewritten;
use crate::words::SWITCH_WORDS;

mod blocks_and_lambdas;
mod type_declarations;
use blocks_and_lambdas::*;

const VOID_WORD: &str = "void";
/// `none` is the null ø; called, `none(xs, f)` (Kotlin's "no element matches") is an error naming it (P185)
const NONE_WORD: &str = "none";
const NONE_CALL_ERROR: &str = "none is the null ø, not a function: for \"no element matches\" write not any(xs, f)";
const STYLE_WORD: &str = "style";
/// What follows the name of a lowercase type declaration: `type point {…}`, `type size = u32`, `type pair<T>`
const TYPE_BODY_STARTS: [char; 4] = ['{', '=', ':', '<'];
/// The first argument of a block, what Elixir's `&1` and the element of Ruby's `&:to_s` are
const CAPTURED_ARGUMENT: &str = "$0";
/// The most code points an emoji in code may have (a family ZWJ sequence has 7 to 11, a subdivision flag 7)
const LONGEST_EMOJI: usize = 32;

/// `T, F: Fn(i32) -> i32` → `T F`: the names before each bound
fn type_parameter_names(written: &str) -> Vec<String> {
	let mut depth = 0;
	let mut parts = vec![String::new()];
	for ch in written.chars() {
		match ch {
			'<' | '(' => depth += 1,
			'>' | ')' => depth -= 1,
			',' if depth == 0 => parts.push(String::new()),
			_ => {}
		}
		if ch != ',' || depth != 0 {
			parts.last_mut().expect("one part").push(ch);
		}
	}
	parts
		.iter()
		.filter_map(|part| {
			part.split(':')
				.next()
				.map(str::trim)
				.filter(|name| !name.is_empty())
				.map(str::to_string)
		})
		.collect()
}

impl WarpParser {
	/// The path literal at the cursor, up to whitespace, a closing bracket or a separator
	pub(super) fn parse_path_literal(&mut self) -> Node {
		let path: String = (0..)
			.map(|offset| self.peek_char(offset))
			.take_while(|&c| !super::lookahead::ends_path(c))
			.collect();
		self.advance_by(path.chars().count());
		Node::Text(path)
	}

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
			'&' if self.starts_function_reference() => self.symbol_after(1, crate::closures::function_reference),
			// Elixir's capture `&(&1 * 2)`: the block `{$0 * 2}`; its `&1`, `&2` are `$0`, `$1`
			'&' if self.starts_capture() => {
				self.advance();
				let outer = std::mem::replace(&mut self.in_capture, true);
				let group = self.parse_bracketed('(');
				self.in_capture = outer;
				match group {
					Node::List(items, Bracket::Round, separator) => Node::List(items, Bracket::Curly, separator),
					other => Node::List(vec![other], Bracket::Curly, Separator::None),
				}
			}
			'&' if self.peek_char(1).is_ascii_digit() && self.in_capture => {
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
			'.' if self.starts_path_literal() => self.parse_path_literal(),
			// `a, *rest = xs`: the starred name takes the items the other names leave (src/lowering/tuples.rs); `...rest`
			// (JS) is the starred `*rest` too: a rest parameter or a spread argument (src/lowering/variadic.rs)
			'.' if self.peek_char(1) == '.' && self.peek_char(2) == '.' && self.is_identifier_start(3) => {
				self.parse_starred(3)
			}
			// Python's `**kw`: the keyword arguments as one object (src/lowering/variadic.rs)
			'*' if self.peek_char(1) == '*' && self.is_identifier_start(2) => match self.parse_starred(2) {
				Node::Symbol(name) => Node::Symbol(format!("{}{name}", crate::words::STARRED)),
				other => other,
			},
			'*' if self.is_identifier_start(1) => self.parse_starred(1),
			// `.card { … }`: a leading-dot name, a class selector in a style block (style_rules.rs)
			'.' if self.is_identifier_start(1) => self.symbol_after(1, |name| Node::Symbol(format!(".{name}"))),
			'-' if self.peek_char(1) == '>' && !self.options.data_mode => self.parse_stabby_lambda(),
			'(' | '[' | '{' => self.parse_bracketed(self.current_char()),
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
			crate::units::DEGREE if self.peek_char(1).is_alphabetic() => {
				self.symbol_after(1, |name| Node::Symbol(format!("{}{name}", crate::units::DEGREE)))
			}
			crate::units::DEGREE => {
				self.advance();
				Node::Symbol(crate::units::DEGREE.to_string())
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
				Node::Symbol(format!(
					"{ATTRIBUTE_MARK}{}",
					self.take_while_char(|ch| ch.is_ascii_digit())
				))
			}
			'$' if self.peek_char(1).is_alphabetic() || self.peek_char(1) == '_' => self.parse_dollar_name(),
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
					.parse_elixir_function()
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
			'\\' if let Some((name, length)) = crate::uniscript_entities::entity_name_at(&self.chars, self.pos) => {
				let warned = self.warn_unknown_entity(&name, self.column);
				self.advance_by(length);
				warned.map_or_else(|strict| strict, |()| Node::Text(format!("\\:{name}")))
			}
			'\\' if let Some(name) = crate::uniscript_entities::bare_entity_name_at(&self.chars, self.pos) => {
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
		let node = node.with_meta_data(LineInfo::at(line_nr, column, || self.current_line.clone()));
		if let Some(c) = comment {
			node.with_comment(c)
		} else {
			node
		}
	}

	/// The word of `length` characters just parsed is the first thing of its statement: nothing, a newline, `;` or `{` before it
	pub(super) fn word_starts_statement(&self, length: usize) -> bool {
		let word_start = self.pos.saturating_sub(length);
		let before = self.chars[..word_start]
			.iter()
			.rev()
			.find(|ch| !matches!(ch, ' ' | '\t' | '\r'));
		matches!(before, None | Some('\n' | ';' | '{'))
	}

	/// `to name params: body`, after the word `to`: the function `name(params) := body`.
	/// The body is the rest of the statement, a `{…}` block or an indented block. Parameters are plain names or, with a
	/// known type word, typed slots (`to square a number:`, type_name_matching::parameter_slots). Anything else is no definition.
	pub(super) fn try_parse_to_definition(&mut self) -> Option<Node> {
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
		let body = self.parse_definition_body();
		self.functions.insert(name.clone());
		let words: Vec<&str> = parameters.iter().map(String::as_str).collect();
		let declared_types = &self.declared_types;
		let is_known_type = |word: &str| {
			crate::analyzer::type_word_kind(word).is_some()
				|| crate::analyzer::plural_element_type(word).is_some()
				|| declared_types.contains(word)
		};
		let in_phrase = crate::type_name_matching::phrase_words(&words, &body, &is_known_type);
		let (parameters, body) = match crate::type_name_matching::parameter_slots(&words, &body, &is_known_type) {
			Ok(slots) => slots,
			Err(message) => return Some(error(&message)),
		};
		let parameters: Vec<Node> = parameters
			.into_iter()
			.map(
				|parameter| match shapes.iter().find(|(name, _)| *name == parameter.name()) {
					Some((_, fields)) => Node::Key(Box::new(parameter), Op::Colon, Box::new(fields.clone())),
					None => parameter,
				},
			)
			.collect();
		if let Some(clash) = self.phrase_redefinition(&name, &parameters) {
			return Some(clash);
		}
		let head = if parameters.is_empty() {
			Symbol(name)
		} else {
			self.functions_with_parameters.insert(name.clone());
			self.phrase_definitions.insert(name.clone(), parameters.clone());
			let head = Node::List(
				[vec![Symbol(name)], parameters].concat(),
				Bracket::Round,
				Separator::None,
			);
			// P52: the prepositions of the phrase, for phrase_calls to read calls like `add 1 to 2`
			let pattern = crate::phrase_calls::pattern_text(&words, &in_phrase);
			// `To square a number: …` of one slot is called in English as `square of x` too (`square 3` stays a call)
			let pattern = if pattern == crate::words::SLOT {
				format!("{OF_WORD} {pattern}")
			} else {
				pattern
			};
			if pattern.split(' ').any(|part| part != crate::words::SLOT) {
				Node::Meta {
					node: Box::new(head),
					data: Box::new(Node::key(crate::words::PHRASE_MARK, Node::Text(pattern))),
				}
			} else {
				head
			}
		};
		let head = match result_type {
			Some(result_type) => Node::Key(Box::new(head), Op::As, Box::new(Symbol(result_type))),
			None => head,
		};
		Some(Node::Key(Box::new(head), Op::Define, Box::new(body)))
	}

	/// `to square a number: …` or `to square a number { … }`
	fn at_to_header_end(&self) -> bool {
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
	pub(super) fn phrase_redefinition(&self, name: &str, parameters: &[Node]) -> Option<Node> {
		let earlier = self
			.phrase_definitions
			.get(name)
			.filter(|earlier| earlier.len() == parameters.len())?;
		let untyped = |parameter: &Node| matches!(parameter.drop_meta(), Symbol(_));
		let differing: Vec<(&Node, &Node)> = earlier
			.iter()
			.zip(parameters)
			.filter(|(before, now)| before != now)
			.collect();
		if differing.is_empty() || !differing.iter().all(|(before, now)| untyped(before) && untyped(now)) {
			return None;
		}
		let classes: Vec<String> = differing
			.iter()
			.flat_map(|(before, now)| [before.name(), now.name()])
			.map(|noun| format!("class {noun}"))
			.collect();
		Some(error(&format!(
			"{name} is defined twice; declare {} to dispatch on them",
			classes.join(" and ")
		)))
	}

	/// Move back to the end of the last thing before the whitespace run behind the cursor
	pub(super) fn seek_back_over_whitespace(&mut self) {
		let mut position = self.pos.min(self.chars.len());
		while position > 0 && self.chars[position - 1].is_whitespace() {
			position -= 1;
		}
		let line_start = self.chars[..position]
			.iter()
			.rposition(|ch| *ch == '\n')
			.map_or(0, |newline| newline + 1);
		self.pos = position;
		self.line_nr = 1 + self.chars[..position].iter().filter(|ch| **ch == '\n').count();
		self.column = position - line_start + 1;
		self.current_line = self.chars[line_start..].iter().take_while(|ch| **ch != '\n').collect();
	}

	/// `🌍`, `🇩🇪`, `👍🏽` in code, one user-perceived character: one code point is a codepoint, several a text
	/// `⊤`, `✅`, `❌` …: the truth value a glyph at the position spells, and its length in chars
	fn truth_glyph_at(&self) -> Option<(Node, usize)> {
		super::TRUTH_GLYPHS.iter().find_map(|(glyph, _)| {
			let length = glyph.chars().count();
			let ahead: String = self.chars[self.pos..].iter().take(length).collect();
			(ahead == *glyph).then(|| (super::truth_glyph(glyph).expect("a truth glyph"), length))
		})
	}

	fn parse_emoji(&mut self) -> Node {
		let ahead: String = self.chars[self.pos..].iter().take(LONGEST_EMOJI).collect();
		let emoji = crate::extensions::strings::grapheme_clusters(&ahead)[0].to_string();
		self.advance_by(emoji.chars().count());
		let mut code_points = emoji.chars();
		match (code_points.next(), code_points.next()) {
			(Some(single), None) => Node::Char(single),
			_ => Node::Text(emoji),
		}
	}

	/// `*rest`, `...rest`: the name after the `prefix` characters, starred
	fn parse_starred(&mut self, prefix: usize) -> Node {
		self.symbol_after(prefix, |name| Node::Symbol(format!("{}{name}", crate::words::STARRED)))
	}

	/// ` name(`: blanks, a name and its parenthesis
	fn named_call_follows(&self) -> bool {
		let blanks = (0..)
			.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
			.count();
		let name = (blanks..)
			.take_while(|&offset| is_identifier_char(self.peek_char(offset)))
			.count();
		blanks > 0 && name > 0 && self.is_identifier_start(blanks) && self.peek_char(blanks + name) == '('
	}

	/// ` () {` after a name
	fn empty_parameters_and_block_follow(&self) -> bool {
		let blanks_from = |start: usize| {
			(start..)
				.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
				.count()
		};
		let blanks = blanks_from(0);
		let inner = blanks_from(blanks + 1);
		let closed = blanks + 1 + inner;
		blanks > 0
			&& self.peek_char(blanks) == '('
			&& self.peek_char(closed) == ')'
			&& self.peek_char(closed + 1 + blanks_from(closed + 1)) == '{'
	}

	pub(super) fn parameters_follow_after_blanks(&self) -> bool {
		let blanks = (0..)
			.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
			.count();
		blanks > 0 && self.peek_char(blanks) == '('
	}

	/// Parse symbol with optional suffix: name{...}, name<...>, name(...)
	/// Does NOT handle infix operators like : or = (those are handled by parse_expr)
	/// The soft keyword `version` before digits takes them as written: `version 1.10` is one value, not the float 1.1
	pub(super) fn version_operand(&mut self, symbol: &str) -> Option<Node> {
		if symbol != crate::words::VERSION_KEYWORD || self.options.data_mode {
			return None;
		}
		let blanks = self.chars[self.pos..].iter().take_while(|c| **c == ' ').count();
		let length = crate::versions::operand_len(&self.chars[self.pos + blanks..]);
		if blanks == 0 || length == 0 {
			return None;
		}
		let operand: String = self.chars[self.pos + blanks..self.pos + blanks + length]
			.iter()
			.collect();
		self.advance_by(blanks + length);
		Some(Node::List(
			vec![Node::Symbol(symbol.to_string()), Node::Symbol(operand)],
			Bracket::None,
			Separator::Space,
		))
	}

	pub(super) fn parse_symbol_with_suffix(&mut self) -> Node {
		let version_len = crate::versions::tagged_literal_len(&self.chars[self.pos..]);
		if version_len > 0 {
			let literal = self.take_chars(version_len);
			return Node::Symbol(literal);
		}
		let symbol = or_return_error!(self.parse_symbol());
		let symbol = self.joined_type_name(symbol);

		if let Some(version) = self.version_operand(&symbol) {
			return version;
		}
		// `x°`: a variable in degrees, as `30°` (card angles-degrees)
		if self.current_char() == crate::units::DEGREE && !self.peek_char(1).is_alphabetic() {
			self.advance();
			return Node::Key(
				Box::new(Node::Symbol(symbol)),
				Op::Mul,
				Box::new(Node::Symbol(crate::units::DEGREE.to_string())),
			);
		}

		self.read_function_name_suffix(&symbol);

		if let Some(reference) = self.function_reference_after(&symbol) {
			return reference;
		}

		if self.url_follows() {
			return self.parse_url(symbol);
		}

		// C's `void f() { … }`: the result type of a function, not ø (src/lowering/declarations.rs c_function)
		if symbol == VOID_WORD && self.named_call_follows() {
			return Node::Symbol(symbol);
		}

		if symbol == NONE_WORD && self.peek_char(0) == '(' && !self.options.data_mode {
			return error(NONE_CALL_ERROR);
		}
		if let Some(constant) =
			check_constants(&symbol, self.options.data_mode).filter(|_| !self.at_member_name(symbol.chars().count()))
		{
			if !self.names_field(&symbol, &constant) {
				return self.refuse_constant_assignment(&symbol).unwrap_or(constant); // if true {} fall through :?
			}
		}

		if let Some(declaration) = self.try_parse_operator_declaration(&symbol) {
			return declaration;
		}

		// Possessive: `p's name` is the field `p.name`
		if !self.options.data_mode
			&& self.current_char() == '\''
			&& self.peek_char(1) == 's'
			&& self.peek_char(2) == ' '
			&& self.is_identifier_start(3)
		{
			return self.symbol_after(3, |field| {
				Node::Key(Box::new(Symbol(symbol)), Op::Dot, Box::new(Symbol(field)))
			});
		}

		// Optional type: `x:int?=ø`, `f(x:int?)`, the field `right? }` (wiki/null.md); a ternary `?` is followed by its branch instead
		if self.current_char() == '?'
			&& (self.ends_optional_type(self.peek_char(1), self.peek_char(2))
				|| self.closes_after_blanks(1)
				|| self.next_field_follows(1))
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
		let spaced_child = self.in_data_literal && !self.in_for_header && !declared && self.blanks_then('{');
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
		if let Some(length) = if names_function {
			self.generic_parameters_length()
		} else {
			None
		} {
			let written: String = self.chars[self.pos + 1..self.pos + length - 1].iter().collect();
			self.advance_by(length);
			self.generic_names = Some((symbol.to_string(), type_parameter_names(&written)));
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
			return Some(self.parse_type_declaration());
		}
		// P178: `enum Shape { Circle(r), Rect(w, h), Dot }`, cases with values: the sum type `Circle(r) | Rect(w, h) | Dot`
		if symbol == ENUM_WORD && self.in_code() && self.enum_with_values_ahead() {
			return Some(self.parse_enum_with_values());
		}
		self.foreign_form(symbol)
	}

	/// Another language's way of writing a warp form: `new Point(1, 2)`, `constant`, Ruby's `attr_accessor :x`,
	/// `operator +`, Kotlin's `enum class` and `data class`, `mixin Name {…}`
	fn foreign_form(&mut self, symbol: &str) -> Option<Node> {
		// `new Point(1, 2)` (Java, JavaScript, C#) of a declared class: the construction `Point(1, 2)`
		if symbol == NEW_WORD && !self.options.data_mode && self.declared_type_after_blanks() {
			self.skip_blanks();
			let construction = self.parse_atom();
			let class = crate::node::leading_name(&construction);
			crate::diagnostic::note_alias(&format!("{NEW_WORD} {class}"), &class);
			return Some(construction);
		}
		if symbol == CONSTANT_ALIAS && !self.options.data_mode && self.identifier_after_blanks() {
			crate::diagnostic::note_alias(CONSTANT_ALIAS, CONST_WORD);
			return Some(Symbol(CONST_WORD.to_string()));
		}
		// Ruby's `attr_accessor :x, :y` in a class body: the fields x and y
		if RUBY_FIELD_WORDS.contains(&symbol) && self.type_fields.is_some() {
			let line: String = (0..)
				.map(|offset| self.peek_char(offset))
				.take_while(|ch| !matches!(ch, '\n' | '\0' | ';' | '}'))
				.collect();
			let names: Vec<String> = line
				.split(',')
				.map(|name| name.trim().trim_start_matches(':').to_string())
				.collect();
			if names
				.iter()
				.all(|name| !name.is_empty() && name.chars().all(is_identifier_char))
			{
				self.advance_by(line.chars().count());
				crate::diagnostic::note_alias(&format!("{symbol}{line}"), &names.join("; "));
				return Some(Node::List(
					names.into_iter().map(Symbol).collect(),
					Bracket::None,
					Separator::Semicolon,
				));
			}
		}
		if symbol == OPERATOR_WORD && !self.options.data_mode && matches!(self.current_char(), ' ' | '\t') {
			self.skip_blanks();
			if let Some(head) = self.try_parse_operator_method_head() {
				crate::diagnostic::note_alias(
					&format!("{OPERATOR_WORD} {}", head.first().name()),
					&head.first().name(),
				);
				return Some(head);
			}
		}
		// Kotlin's `enum class Color {…}`: the enum
		if symbol == ENUM_WORD && !self.options.wit_mode && self.class_keyword_after_blanks() == Some("class") {
			self.skip_blanks();
			self.advance_by("class".len());
			crate::diagnostic::note_alias(&format!("{ENUM_WORD} class"), ENUM_WORD);
			return Some(Symbol(symbol.to_string()));
		}
		// `data class P(…)` (Kotlin), `open class`, `abstract class`: a modifier of a class declaration, the class itself
		if !self.options.wit_mode && CLASS_MODIFIERS.contains(&symbol) {
			if let Some(keyword) = self.class_keyword_after_blanks() {
				crate::diagnostic::note_alias(&format!("{symbol} {keyword}"), keyword);
				self.skip_blanks();
				self.advance_by(keyword.len());
				return Some(self.parse_type_declaration());
			}
		}
		if !self.options.wit_mode && symbol == MIXIN_WORD && self.name_and_block_follow() {
			return Some(match self.parse_type_declaration() {
				Node::Type { name, body } => Node::Type {
					name: Box::new(name.with_attribute(MIXIN_WORD, Node::True)),
					body,
				},
				other => other,
			});
		}
		None
	}

	/// `class circle{pi = 3}`, `class C{pi:int}`: a named number declared in a type body names the type's own field, which
	/// shadows nothing outside; the type's methods read the field
	fn names_field(&mut self, symbol: &str, constant: &Node) -> bool {
		// `{empty: yes}`, `class Cell{empty: bool}`: a key named like ø is a name (card object-empty-field)
		let is_word = symbol.chars().all(char::is_alphabetic);
		if matches!(constant, Node::Empty) && is_word && self.type_annotation_follows() && self.peek_char(1) != ':' {
			return true;
		}
		if !matches!(constant, Node::Number(_)) {
			return false;
		}
		let declares = self.assignment_follows() || self.type_annotation_follows();
		let Some(fields) = self.type_fields.as_mut() else {
			return false;
		};
		if declares {
			fields.insert(symbol.to_string());
		}
		fields.contains(symbol)
	}

	/// `:type` right after the name (`pi:int`, `pi: int`), not the definition `:=` nor a ternary's `c ? pi : 0`
	fn type_annotation_follows(&self) -> bool {
		self.current_char() == ':' && self.peek_char(1) != '='
	}

	/// `pi = 4` (P130, user: "loud error, if it was declared constant before which it should be"): a named number is a
	/// declared constant, an assignment to it is an error
	fn refuse_constant_assignment(&self, symbol: &str) -> Option<Node> {
		let column = self.column.saturating_sub(symbol.chars().count());
		let message = format!("{symbol} is a constant");
		self.assignment_follows().then(|| {
			Diagnostic {
				message,
				line: self.line_nr,
				column,
				..Default::default()
			}
			.fix("another name")
			.into_error()
		})
	}

	/// `= …` or `:= …` after blanks, not the comparison `==` nor the arrow `=>`
	fn assignment_follows(&self) -> bool {
		let blanks = (0..)
			.take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t'))
			.count();
		let defines = self.peek_char(blanks) == ':' && self.peek_char(blanks + 1) == '=';
		let assigns = self.peek_char(blanks) == '=' && !matches!(self.peek_char(blanks + 1), '=' | '>'); // `None => 0` is a match arm
		defines || assigns
	}

	/// `http://…`, `file://…`, `ssh://…`: the scheme of a URL, the rest of which reads as one text
	pub(super) fn url_follows(&self) -> bool {
		URL_MARK
			.chars()
			.enumerate()
			.all(|(offset, mark)| self.peek_char(offset) == mark)
	}

	pub(super) fn parse_url(&mut self, scheme: String) -> Node {
		let mut url = scheme;
		while !self.is_url_terminator(self.current_char()) {
			url.push(self.current_char());
			self.advance();
		}
		Node::Text(url)
	}

	/// What is glued to a word: `name{…}`, `List<int>`, `p@unit`, `f(args)`, `f(params) {body}`; else the word itself
	pub(super) fn parse_glued_suffix(&mut self, symbol: String) -> Node {
		let ch = self.current_char();
		match ch {
			'{' if GLUED_BLOCK_KEYWORDS.contains(&symbol.as_str()) && self.in_code() => Node::symbol(&symbol),
			'{' => {
				// `point{x:1}` of a declared type constructs a point, `point:{x:1}` and any other `name{…}` stay data (D4)
				let op = if self.declared_types.contains(&symbol) {
					Op::None
				} else {
					Op::Colon
				};
				let outer_data_literal = std::mem::replace(&mut self.in_data_literal, op == Op::Colon);
				// a rule `p { … }` of a style sheet is still the sheet
				let in_style_sheet = op == Op::Colon && (symbol == STYLE_WORD || self.in_style_sheet);
				let outer_style_sheet = std::mem::replace(&mut self.in_style_sheet, in_style_sheet);
				self.tag_block = op == Op::Colon;
				let mut blocks = vec![self.parse_bracketed('{')];
				// `a{x:1}{y:2}{3}`: glued blocks are the children of a, `a{x}{y z}` is no `a{x, {y z}}`
				while self.current_char() == '{' {
					blocks.push(self.parse_bracketed('{'));
				}
				self.in_data_literal = outer_data_literal;
				self.in_style_sheet = outer_style_sheet;
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
				Node::Key(
					Box::new(Symbol(symbol)),
					Op::Dot,
					Box::new(Symbol(format!("{ATTRIBUTE_MARK}{key}"))),
				)
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
				if self.current_char() == '{' && !self.equals_compares && !self.in_for_header && !is_switch_subject {
					// Function with body: name(params) { body }
					let body = self.parse_bracketed('{');
					let signature = call(&symbol, vec![typed_parameters(args_node)]);
					Node::List(vec![signature, body], Bracket::Round, Separator::None)
				} else if symbol == PRINT_WORD {
					print_call(print_arguments(args_node))
				} else {
					function_call(symbol, args_node)
				}
			}
			_ => Node::symbol(&symbol),
		}
	}
}

/// Function call: name(params) -> List([symbol, args...])
fn function_call(symbol: String, args_node: Node) -> Node {
	let mut items = vec![Symbol(symbol)];
	match typed_parameters(args_node) {
		Node::List(args, _, _) => items.extend(args),
		Node::Empty => {}
		other => items.push(other),
	}
	Node::List(items, Bracket::Round, Separator::None)
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
