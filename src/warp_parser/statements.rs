//! Statements: global and nonlocal declarations, modifiers, for/while/condition loops, else branches

use super::*;

impl WarpParser {
	/// `global [modifiers] name[=value]` and the same after `export`: a variable declared at module level.
	/// `export` without a name after it, and `export f:=it*2`, a function, stay ordinary code.
	pub(super) fn try_parse_global_declaration(&mut self, keyword: &str) -> Option<Node> {
		let before_declaration = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.skip_whitespace();
		self.skip_declaration_modifiers();
		if keyword == "export" && !self.at_identifier_start() {
			(self.pos, self.line_nr, self.column, self.current_line) = before_declaration;
			return None;
		}
		let declaration = self.parse_expr(0); // name, name=value or name:=value
		let defines_function = matches!(declaration.drop_meta(), Node::Key(name, Op::Define, _) if self.functions.contains(&name.name()));
		if keyword == "export" && defines_function {
			return Some(declaration);
		}
		let global = |declaration: Node| Node::Key(Box::new(Symbol(crate::node::GLOBAL_DECLARATION.to_string())), Op::Colon, Box::new(declaration));
		// `global a, b` (Python): one declaration per name
		let mut names = vec![declaration];
		while keyword == crate::node::GLOBAL_DECLARATION && matches!(names.last().map(Node::drop_meta), Some(Symbol(_))) && self.current_char() == ',' {
			self.advance();
			self.skip_spaces();
			names.push(self.parse_atom());
		}
		Some(match names.len() {
			1 => global(names.remove(0)),
			_ => Node::List(names.into_iter().map(global).collect(), Bracket::None, Separator::Semicolon),
		})
	}

	/// A name follows after at least one blank: `nonlocal y`, not `nonlocal = 3` or `nonlocal(…)`
	pub(super) fn name_after_blanks(&self) -> bool {
		let blanks = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let next = self.peek_char(blanks);
		blanks > 0 && (next.is_alphabetic() || next == '_')
	}

	/// `nonlocal y` → `nonlocal:y`, like `global y`; `nonlocal a, b` declares each name
	pub(super) fn parse_nonlocal_declaration(&mut self) -> Node {
		let mut names = vec![];
		loop {
			self.skip_spaces();
			match self.parse_symbol() {
				Ok(name) if !name.is_empty() => names.push(Symbol(name)),
				Ok(_) => break,
				Err(e) => return error(&e),
			}
			self.skip_spaces();
			if self.current_char() != ',' || !self.name_after_comma() {
				break;
			}
			self.advance();
		}
		let declared = match names.len() {
			1 => names.remove(0),
			_ => Node::List(names, Bracket::None, Separator::Colon),
		};
		Node::Key(Box::new(Symbol(NONLOCAL_WORD.to_string())), Op::Colon, Box::new(declared))
	}

	/// `, b` after a declared name: another name, not the next item of a list
	pub(super) fn name_after_comma(&self) -> bool {
		let blanks = (1..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
		let next = self.peek_char(1 + blanks);
		next.is_alphabetic() || next == '_'
	}

	pub(super) fn at_identifier_start(&self) -> bool {
		self.current_char().is_alphabetic() || self.current_char() == '_'
	}

	/// `public static fun f(){…}`, `private x = 4` (P78): a run of modifiers that ends before a function definition, or of
	/// modifiers without warp meaning before a name, is skipped, each meaningless one with a note; the statement after it
	pub(super) fn skip_foreign_modifiers(&mut self, first: &str) -> Option<Node> {
		let is_modifier = |word: &str| FOREIGN_MODIFIERS.contains(&word) || DEFINITION_MODIFIERS.contains(&word);
		if !is_modifier(first) {
			return None;
		}
		let mut words = vec![first.to_string()];
		let mut offset = 0;
		let next = loop {
			let blanks = (offset..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t')).count();
			if blanks == 0 {
				return None; // `public = 3`, `public(…)`: a name
			}
			let start = offset + blanks;
			let word: String = (start..).map(|at| self.peek_char(at)).take_while(|ch| is_identifier_char(*ch)).collect();
			offset = start + word.chars().count();
			if !is_modifier(&word) {
				break word;
			}
			words.push(word);
		};
		let before_definition = crate::operators::is_function_keyword(&next);
		let foreign_only = words.iter().all(|word| FOREIGN_MODIFIERS.contains(&word.as_str()));
		if next.is_empty() || !(before_definition || foreign_only) {
			return None;
		}
		let (line, column) = (self.line_nr, self.column.saturating_sub(first.chars().count()));
		let is_static = words.iter().any(|word| word == STATIC_KEYWORD);
		for word in words.iter().filter(|word| FOREIGN_MODIFIERS.contains(&word.as_str()) && *word != STATIC_KEYWORD) {
			let question = Ask::new(FOREIGN_MODIFIER_TOPIC, format!("{word} has no meaning in warp"), vec![reading("skip it", &next)], Fallback::Warning)
				.written(word).at(line, column);
			if let Err(error) = ask(&question) {
				return Some(error);
			}
		}
		self.advance_by(offset - next.chars().count());
		self.skip_spaces();
		let statement = self.parse_expr(0);
		Some(if is_static { statement.with_attribute(STATIC_KEYWORD, Node::True) } else { statement })
	}

	/// Modifier and type words between the keyword and the name (`global const int k=7`): the global holds the value, the words are dropped
	pub(super) fn skip_declaration_modifiers(&mut self) {
		loop {
			let before_word = (self.pos, self.line_nr, self.column, self.current_line.clone());
			let word = self.parse_symbol().unwrap_or_default();
			let is_modifier = DECLARATION_MODIFIERS.contains(&word.as_str()) || crate::analyzer::CONSTANT_KEYWORDS.contains(&word.as_str()) || crate::analyzer::type_word_kind(&word).is_some();
			self.skip_spaces();
			if !is_modifier || !self.at_identifier_start() {
				(self.pos, self.line_nr, self.column, self.current_line) = before_word; // `global int` names the variable int
				return;
			}
		}
	}

	/// The name of `for x in …`, or the names each element destructures into: `for (r, c) in …`, `for k, v in …`
	pub(super) fn parse_loop_variable(&mut self) -> Option<Node> {
		let bracket = if self.current_char() == '(' { self.advance(); Bracket::Round } else { Bracket::None };
		let mut names = vec![];
		loop {
			self.skip_spaces();
			names.push(Symbol(self.parse_symbol().ok()?));
			self.skip_spaces();
			if self.current_char() != ',' {
				break;
			}
			self.advance();
		}
		if bracket == Bracket::Round {
			if self.current_char() != ')' {
				return None;
			}
			self.advance();
		}
		Some(if names.len() == 1 { names.remove(0) } else { Node::List(names, bracket, Separator::Colon) })
	}

	/// `for x in iterable: body` and `for x in iterable {body}`, after the word `for`; the body of a colon runs to the end of
	/// the statement. `for 1..10 : print it` names no variable (wiki/for.md): `for iterable {body}`, whose items are `it`
	pub(super) fn try_parse_for_in(&mut self) -> Option<Node> {
		let before_header = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.skip_spaces();
		if let Some(filtered) = self.try_parse_condition_loop() {
			return Some(filtered);
		}
		if self.matches_keyword(EACH_WORD) && self.word_at(EACH_WORD.len() + 1) != "in" {
			self.advance_by(EACH_WORD.len());
			self.skip_spaces();
		}
		let variable = self.parse_loop_variable();
		self.skip_spaces();
		let named = variable.filter(|_| self.matches_keyword("in"));
		if named.is_some() {
			self.advance_by("in".len());
		} else {
			(self.pos, self.line_nr, self.column, self.current_line) = before_header.clone();
			self.skip_spaces();
			if self.current_char() == '(' { // `for(i=0;i<n;i++)`
				(self.pos, self.line_nr, self.column, self.current_line) = before_header;
				return None;
			}
		}
		let outer_header = std::mem::replace(&mut self.in_for_header, true);
		let iterable = self.with_equals_comparing(false, |parser| parser.parse_expr(Op::Colon.binding_power().0 + 1));
		self.in_for_header = outer_header;
		self.skip_spaces();
		let body_word = if self.current_char() == ':' { Some(":") } else { Some("do").filter(|word| self.matches_keyword(word)) };
		let Some(variable) = named else {
			// only the colon and do forms: `for 1..3 {…}` is read where for loops are lowered
			let Some(word) = body_word else {
				(self.pos, self.line_nr, self.column, self.current_line) = before_header;
				return None;
			};
			let body = self.colon_body(word);
			let block = Node::List(vec![body], Bracket::Curly, Separator::Semicolon);
			return Some(Node::List(vec![Symbol("for".to_string()), iterable, block], Bracket::None, Separator::Space));
		};
		let key_variable = match variable.drop_meta() {
			Symbol(name) if iterates_keys(&iterable) => Some(name.clone()),
			_ => None,
		};
		self.key_variables.extend(key_variable.clone());
		let body = match body_word {
			Some(word) => self.colon_body(word), // `for i in 0..n: body`, `for i in 0..n do body`
			None => self.parse_atom(),
		};
		if key_variable.is_some() {
			self.key_variables.pop();
		}
		// `for friend in xs`: a declared type's name visits only its instances, as `it`; the name in the body is the item
		// (P46), a call `friend(…)` still constructs
		let (variable, body) = match variable.drop_meta() {
			Symbol(name) if self.declared_types.contains(name) => {
				let it = Symbol(crate::lambdas::IMPLICIT_PARAMETER.to_string());
				let test = Node::List(vec![Symbol(crate::traits::INSTANCE_OF.to_string()), it.clone(), Node::Text(name.clone())], Bracket::Round, Separator::None);
				let header = type_filter_header(name, &iterable, &body);
				let body = item_named(body, name, &it);
				match self.filtered_body(&format!("for {name} in …"), &format!("for x in … {{ if x is {name} {{ … }} }}"), header, test, body) {
					Ok(body) => (it, body),
					Err(error) => return Some(error),
				}
			}
			// `for number in xs`: a type word visits only the items of that type, under its own name; a literal list whose items
			// all are of the type needs no filter
			Symbol(name) if crate::analyzer::type_word_kind(name).is_some() && !literal_items_of_type(&iterable, name) => {
				let test = self.type_test(&variable, name).expect("a type word");
				let header = type_filter_header(name, &iterable, &body);
				match self.filtered_body(&format!("for {name} in …"), &format!("for x in … {{ if x is {name} {{ … }} }}"), header, test, body) {
					Ok(body) => (variable, body),
					Err(error) => return Some(error),
				}
			}
			_ => (variable, body),
		};
		// `for chars in text: print it`: a unit word walks the text by that unit, the item is `it`
		if let Some((unit, iterable)) = unit_iteration(&variable, &iterable, &body) {
			return Some(Node::List(vec![Symbol("for".to_string()), unit, Symbol("in".to_string()), iterable, body], Bracket::None, Separator::Space));
		}
		Some(Node::List(vec![Symbol("for".to_string()), variable, Symbol("in".to_string()), iterable, body], Bracket::None, Separator::Space))
	}

	/// `for (it>2) in xs: body` (wiki/for.md, P46): the items the condition holds for, as `it`; None (nothing consumed)
	/// for any other header
	pub(super) fn try_parse_condition_loop(&mut self) -> Option<Node> {
		let before = (self.pos, self.line_nr, self.column, self.current_line.clone());
		if self.current_char() != '(' {
			return None;
		}
		self.advance();
		// the expressions up to `)`: one condition `it>2`, or the words `even number`
		let mut parts = vec![];
		loop {
			self.skip_spaces();
			if matches!(self.current_char(), ')' | '\0' | '\n') {
				break;
			}
			let start = self.pos;
			parts.push(self.with_equals_comparing(true, |parser| parser.parse_expr(0)));
			if self.pos == start {
				break;
			}
		}
		let condition = match parts.len() {
			1 => parts.remove(0),
			_ => Node::List(parts, Bracket::None, Separator::Space),
		};
		let mentions_it = crate::warp_parser::mentions(&condition, crate::lambdas::IMPLICIT_PARAMETER);
		// `(even number)`: an adjective and a type word
		let condition = if mentions_it { Some(condition) } else { self.adjective_condition(&condition) };
		let Some(condition) = condition.filter(|_| self.current_char() == ')') else {
			(self.pos, self.line_nr, self.column, self.current_line) = before;
			return None;
		};
		self.advance();
		self.skip_spaces();
		if !self.matches_keyword("in") {
			(self.pos, self.line_nr, self.column, self.current_line) = before;
			return None;
		}
		self.advance_by("in".len());
		// in the header `xs {` is the collection and the body, as in any for loop, not a construction of xs
		let outer_header = std::mem::replace(&mut self.in_for_header, true);
		let iterable = self.with_equals_comparing(false, |parser| parser.parse_expr(Op::Colon.binding_power().0 + 1));
		self.in_for_header = outer_header;
		self.skip_spaces();
		let body_word = if self.current_char() == ':' { Some(":") } else { Some("do").filter(|word| self.matches_keyword(word)) };
		let body = match body_word {
			Some(word) => self.colon_body(word),
			None => self.parse_atom(),
		};
		let shown = crate::normalize::operand_text(&condition);
		let collection = crate::normalize::operand_text(&iterable);
		let header = (format!("for ({shown}) in {collection}"), format!("for it in {collection}.filter(it => {shown})"));
		let body = match self.filtered_body(&format!("for ({shown}) in …"), &format!("for it in … {{ if {shown} {{ … }} }}"), Some(header), condition, body) {
			Ok(body) => body,
			Err(error) => return Some(error),
		};
		let it = Symbol(crate::lambdas::IMPLICIT_PARAMETER.to_string());
		Some(Node::List(vec![Symbol("for".to_string()), it, Symbol("in".to_string()), iterable, body], Bracket::None, Separator::Space))
	}

	/// `(even number)` in a loop header: `even(it)` (built in: `it%2==0`, `odd` its opposite, else the user's function) and
	/// `it is number`; None for anything else
	pub(super) fn adjective_condition(&self, words: &Node) -> Option<Node> {
		// the two words, spaced or read as the call `even(number)`
		let Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) = words.drop_meta() else { return None };
		let [adjective, type_word] = items.as_slice() else { return None };
		let Symbol(adjective) = adjective.drop_meta() else { return None };
		let Symbol(type_name) = type_word.drop_meta() else { return None };
		let it = Symbol(crate::lambdas::IMPLICIT_PARAMETER.to_string());
		let type_test = self.type_test(&it, type_name)?;
		let parity = |op: Op| Node::Key(Box::new(Node::Key(Box::new(it.clone()), Op::Mod, Box::new(Node::int(2)))), op, Box::new(Node::int(0)));
		let adjective_test = match adjective.as_str() {
			word if self.functions.contains(word) => Node::List(vec![Symbol(word.to_string()), it.clone()], Bracket::Round, Separator::None),
			EVEN_WORD => parity(Op::Eq),
			ODD_WORD => parity(Op::Ne),
			word => Node::List(vec![Symbol(word.to_string()), it.clone()], Bracket::Round, Separator::None),
		};
		Some(Node::Key(Box::new(type_test), Op::And, Box::new(adjective_test)))
	}

	/// `value is T` for a loop filter: a declared type by its instances, a type word by its runtime kind; None for any other word
	pub(super) fn type_test(&self, value: &Node, type_name: &str) -> Option<Node> {
		let test = if self.declared_types.contains(type_name) {
			crate::traits::INSTANCE_OF
		} else if crate::analyzer::type_word_kind(type_name).is_some() {
			crate::type_tests::IS_TYPE
		} else {
			return None;
		};
		Some(Node::List(vec![Symbol(test.to_string()), value.clone(), Node::Text(type_name.to_string())], Bracket::Round, Separator::None))
	}

	/// The body of a filtering loop: run only when `test` holds; the filter is announced once (got-it warning, P46).
	/// `header` is the loop's header as written and its explicit form, the fix ("I meant: …"); None when the body
	/// needs the edit too
	pub(super) fn filtered_body(&self, written: &str, explicit: &str, header: Option<(String, String)>, test: Node, body: Node) -> Result<Node, Node> {
		let question = format!("`{written}` visits only the items that pass its filter");
		let filter = reading("filter the items", explicit);
		let readings = vec![match header {
			Some((written, replacement)) => filter.replacing(written, replacement),
			None => filter,
		}];
		ask(&Ask::new(FILTER_LOOP_TOPIC, question, readings, Fallback::Warning).written(written).at(self.line_nr, self.column))?;
		let guarded = Node::Key(Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(test))), Op::Then, Box::new(body));
		Ok(Node::List(vec![guarded], Bracket::Curly, Separator::Semicolon))
	}

	/// The body after `:` or `do`: an indented block under a colon at the end of the line, else the rest of the line
	pub(super) fn colon_body(&mut self, word: &str) -> Node {
		self.advance_by(word.len());
		let indented_block = if word == ":" && self.only_blanks_before_newline() { self.parse_indented_block() } else { None };
		indented_block.unwrap_or_else(|| self.rest_of_statement())
	}

	/// The words up to the end of the line, one expression: the colon body `print i` of `for i in 1..3 : print i`
	pub(super) fn rest_of_statement(&mut self) -> Node {
		let mut words = vec![self.with_equals_comparing(false, |parser| parser.parse_expr(0))];
		loop {
			while matches!(self.current_char(), ' ' | '\t') {
				self.advance();
			}
			if !self.can_start_atom() {
				return one_expression(&words);
			}
			words.push(self.with_equals_comparing(false, |parser| parser.parse_expr(0)));
		}
	}

	pub(super) fn finish_while_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let body_block = self.parse_atom(); // parse { block }
			return while_do(rhs, body_block);
		}

		if let Node::Key(condition, Op::Colon, body) = &rhs {
			let body = self.continue_expr(body.as_ref().clone(), 0); // `while c: i+=2`
			return while_do(condition.as_ref().clone(), body);
		}

		if matches!(rhs.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1) && self.at_body_start() {
			let body = self.parse_expr(0); // `while (i<9) i++`
			return while_do(rhs, body);
		}

		if let Some((condition, body)) = split_trailing_block(&rhs, true) { // `{}` parses as ø
			return while_do(condition, body);
		}

		Node::Key(Box::new(Empty), Op::While, Box::new(rhs))
	}

	/// `else body`, and `else if` / `elif` / `elsif` / `elseif` chaining a whole new `if … {…} else …`
	pub(super) fn parse_optional_else(&mut self, if_then: Node, mode: ElseParseMode) -> Node {
		self.skip_spaces();
		// `}` then `else …` on the next line belongs to this if
		if self.else_on_a_later_line() {
			while matches!(self.current_char(), ' ' | '\t' | '\n' | '\r') {
				self.advance();
			}
		}
		let else_expr = if let Some(word) = self.at_else_if_word() {
			self.advance_by(word.len());
			self.parse_else_if()
		} else if self.matches_keyword("else") {
			self.advance_by("else".len());
			self.skip_spaces();
			match mode {
				_ if self.matches_keyword("if") => {
					self.advance_by("if".len());
					self.parse_else_if()
				}
				_ if self.current_char() == ':' => {
					self.advance(); // Python's `else: body`
					self.parse_expr(0)
				}
				ElseParseMode::Atom => self.parse_atom(),
				ElseParseMode::Expr => self.parse_expr(0),
			}
		} else {
			return if_then;
		};
		Node::Key(Box::new(if_then), Op::Else, Box::new(else_expr))
	}

	/// Do line breaks and blanks, then the word `else`, follow the cursor
	pub(super) fn else_on_a_later_line(&self) -> bool {
		let gap = (0..).take_while(|at| matches!(self.peek_char(*at), ' ' | '\t' | '\n' | '\r')).count();
		let crosses_line = (0..gap).any(|at| self.peek_char(at) == '\n');
		let word = ELSE_KEYWORD.chars().enumerate().all(|(i, c)| self.peek_char(gap + i) == c) && !is_identifier_char(self.peek_char(gap + ELSE_KEYWORD.len()));
		crosses_line && word
	}

	pub(super) fn at_else_if_word(&self) -> Option<&'static str> {
		ELSE_IF_WORDS.into_iter().find(|word| self.matches_keyword(word))
	}

	pub(super) fn parse_else_if(&mut self) -> Node {
		let condition = self.parse_prefix_operand(Op::If);
		self.finish_if_prefix(condition)
	}
}
