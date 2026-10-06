//! Symbols, type applications and bracketed lists, separators and grouping

use super::*;

impl WaspParser {
	pub(super) fn parse_symbol(&mut self) -> Result<String, String> {
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
	/// Length of the `<int>`, `<list<int>>`, `<text, int>` directly at the cursor when everything inside is a type word;
	/// `a<b`, `a<<b` and `a<b>c` are not type applications
	pub(super) fn type_application_length(&self) -> Option<usize> {
		let mut depth = 0;
		let mut word = String::new();
		let mut words = Vec::new();
		for offset in 0.. {
			let ch = self.peek_char(offset);
			match ch {
				'<' => depth += 1,
				'>' => depth -= 1,
				ch if ch.is_alphanumeric() || ch == '_' || ch == '?' => {
					word.push(ch);
					continue;
				}
				',' | ' ' => {}
				_ => return None,
			}
			if !word.is_empty() {
				words.push(std::mem::take(&mut word));
			}
			if depth == 0 {
				let is_type_argument = |word: &String| {
					let bare = word.trim_end_matches('?');
					crate::analyzer::type_word_kind(bare).is_some()
						|| crate::analyzer::plural_element_type(bare).is_some()
						|| GENERIC_TYPE_HEADS.contains(&bare)
						|| bare.chars().next().is_some_and(char::is_uppercase)
				};
				return (offset > 1 && words.iter().all(is_type_argument)).then_some(offset + 1);
			}
		}
		None
	}

	pub(super) fn parse_type_application(&mut self, type_name: String) -> Node {
		self.advance(); // skip '<'
		let arguments = self.parse_list_with_separators(Some('>'), Bracket::Less);
		Node::List(vec![Symbol(type_name), arguments], Bracket::None, Separator::None)
	}

	pub(super) fn parse_bracketed(&mut self, open: char) -> Node {
		let (close, bracket_type) = match open {
			'(' => (')', Bracket::Round),
			'[' => (']', Bracket::Square),
			'{' => ('}', Bracket::Curly),
			'<' => ('>', Bracket::Round),
			_ => panic!("Invalid bracket: {}", open),
		};
		let start = self.get_position();
		let outer_start = std::mem::replace(&mut self.group_start, start);
		self.advance(); // skip opening bracket
		let compares = self.equals_compares && bracket_type != Bracket::Curly; // a block is not the condition
		// `for i in (0 until n)` and `(0..n-1)` are still the header; a block or a list inside it is not
		let inner_header = self.in_for_header && bracket_type == Bracket::Round;
		let outer_header = std::mem::replace(&mut self.in_for_header, inner_header);
		let list = self.with_equals_comparing(compares, |parser| parser.parse_list_with_separators(Some(close), bracket_type));
		self.in_for_header = outer_header;
		self.group_start = outer_start;
		list
	}

	pub(super) fn parse_list_with_separators(&mut self, close: Option<char>, bracket: Bracket) -> Node {
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
				None => self.end_of_input() || self.at_block_close(),
			};
			if at_end {
				if let Some(closer) = close.filter(|_| ch == '\0') {
					let (line, column) = self.group_start;
					return error(&format!("`{closer}` is missing: the group opened at {line}:{column} runs to the end of the input"));
				}
				if close.is_some() {
					self.advance(); // consume closing bracket
				}
				break;
			}

			let pos_before = self.pos;
			// an argument of a braceless call at statement level (`sleep 1s`): `and print "x"` after it starts the next statement
			let statement_start = items_with_seps.iter().rposition(|(_, separator)| *separator != Separator::Space).map_or(0, |last| last + 1);
			let in_command = items_with_seps.get(statement_start).is_some_and(|(first, _)| matches!(first.drop_meta(), Symbol(_)));
			let outer_command = std::mem::replace(&mut self.in_command, in_command);
			let item = self.parse_value();
			self.in_command = outer_command;

			let consumed_input = self.pos != pos_before;
			// `==` is loose (false equals ø): only a real ø is skipped
			if matches!(item, Empty) && !(consumed_input && close.is_some()) {
				if !consumed_input {
					self.advance();
				}
				continue;
			}

			let (had_newline, line_indent, comment) = self.skip_whitespace_and_comments();
			self.pending_comment = comment;

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
			} else if bracket == Bracket::Curly {
				field_holding_lambda(item)
			} else {
				item
			};

			// Determine separator after this item
			let ch = self.current_char();
			let at_end = match close {
				Some(c) => ch == c || ch == '\0',
				None => self.end_of_input() || self.at_block_close(),
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
			} else if (in_command || is_command(&item)) && self.and_starts_statement() {
				self.advance_by(AND_KEYWORD.len());
				Separator::Semicolon
			} else {
				Separator::Space
			};

			items_with_seps.push((item, sep));

			if self.pos == pos_before {
				self.advance();
			}
		}

		self.pending_comment = None; // a comment closing a list documents nothing after it
		let list = self.group_by_separators(items_with_seps, bracket);
		match list.duplicate_key() {
			Some(key) => error(&format!("duplicate key '{}'", key)),
			None => list,
		}
	}
	pub(super) fn group_by_separators(
		&self,
		items_with_seps: Vec<(Node, Separator)>,
		bracket: Bracket,
	) -> Node {
		if items_with_seps.is_empty() {
			return if bracket == Bracket::Curly { Node::List(Vec::new(), bracket, Separator::None) } else { Empty }; // an empty block is a value
		}

		if items_with_seps.len() == 1 && bracket == Bracket::None {
			// Only unwrap single items for implicit groupings
			return only(items_with_seps).0;
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
				return only(items);
			}
			return grouped_list(items, bracket, Separator::Space);
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
					only(group).0
				} else {
					// Has multiple items or tighter separators - recurse
					// Inner groups use Bracket::None to avoid extra braces in serialization
					self.group_by_separators(group, Bracket::None)
				}
			})
			.collect();

		if grouped_nodes.len() == 1 && bracket == Bracket::None {
			// Only unwrap single items for implicit groupings (Bracket::None)
			// Explicit brackets like {x} or [x] should preserve the wrapper
			only(grouped_nodes)
		} else {
			grouped_list(grouped_nodes, bracket, split_sep)
		}
	}

	/// Transform field definitions: Key(name, op, Symbol) -> Key(name, op, Type)
	/// Used for class/struct definitions to convert type names to Type nodes
	pub(super) fn transform_fields_to_types(node: Node) -> Node {
		match node {
			// a keyword method (`def scaled(k) { side * k }`) or a constructor (`value(n) {…}`) in a class body: its body is
			// code, no field types
			Node::List(items, bracket, sep) if items.first().is_some_and(starts_code) => {
				Node::List(items, bracket, sep)
			}
			Node::List(items, bracket, sep) => {
				let transformed: Vec<Node> = items.into_iter().map(Self::transform_fields_to_types).collect();
				Node::List(transformed, bracket, sep)
			}
			// a method in a class body (`greet() := …`): its body is code, no field type
			Node::Key(name, Op::Define, value) => Node::Key(name, Op::Define, value),
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
	pub(super) fn symbol_to_type(node: Node) -> Node {
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

/// `{f: x => x * 2}`: `:` binds tighter than `=>`, so the entry parses as the lambda `(f:x) => x*2` with a parameter f of
/// type x; in an object a lowercase word that is no type after the key is the lambda's parameter: the field f holds
/// `x => x * 2`. A typed lambda `{x: int => x * 2}` or `{p: Person => p.name}` stays one.
fn field_holding_lambda(item: Node) -> Node {
	match item {
		Node::Meta { node, data } => Node::Meta { node: Box::new(field_holding_lambda(*node)), data },
		Node::Key(head, Op::FatArrow, body) => match head.drop_meta() {
			Node::Key(key, Op::Colon, parameter) if matches!(key.drop_meta(), Symbol(_)) && is_parameter_word(parameter) => {
				let lambda = Node::Key(parameter.clone(), Op::FatArrow, body);
				Node::Key(key.clone(), Op::Colon, Box::new(lambda))
			}
			_ => Node::Key(head, Op::FatArrow, body),
		},
		other => other,
	}
}

/// A lowercase word that names no type, or a parameter group `(x, y)`
fn is_parameter_word(node: &Node) -> bool {
	match node.drop_meta() {
		Symbol(word) => word.starts_with(char::is_lowercase) && crate::analyzer::type_word_kind(word).is_none(),
		Node::List(_, Bracket::Round, _) => true,
		_ => false,
	}
}

/// The first word of a class-body item that is code: a function keyword, or the constructor call `value(n)`
fn starts_code(first: &Node) -> bool {
	match first.drop_meta() {
		Node::Symbol(word) => crate::operators::is_function_keyword(word) || super::ACCESSOR_WORDS.contains(&word.as_str()) || super::MEMBER_MODIFIERS.contains(&word.as_str()),
		// `value(n) {…}`, JavaScript's `sum() {…}` and `constructor(x, y) {…}`
		Node::List(call, Bracket::Round, _) => matches!(call.first().map(Node::drop_meta), Some(Node::Symbol(_))),
		_ => false,
	}
}
