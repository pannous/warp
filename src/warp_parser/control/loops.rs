//! Loops: for-in, C-style for, while/until, `n times`

use super::*;
use crate::warp_parser::context::Context;

/// The words an operand may start counting with: `thrice {…}` is `3 times {…}`
const COUNT_WORDS: [(&str, i64); 3] = [("once", 1), ("twice", 2), ("thrice", 3)];
const COUNT_WORD: &str = "count-word";

impl WarpParser {
	/// The iterable of a for header and the word that opens the body: `:`, `do` or none.
	/// In the header `xs {` is the collection and the body, as in any for loop, not a construction of xs
	fn for_iterable_and_body_word(&mut self) -> (Node, Option<&'static str>) {
		let header = Context { in_for_header: true, equals_compares: false, ..self.context };
		let iterable = self.within(header, Self::parse_iterable);
		self.skip_spaces();
		let body_word =
			if self.current_char() == ':' { Some(":") } else { Some("do").filter(|word| self.matches_keyword(word)) };
		(iterable, body_word)
	}

	/// The name of `for x in …`, or the names each element destructures into: `for (r, c) in …`, `for k, v in …`
	/// A loop variable and the word `in` follow the blanks here: `x in xs` after `each`; nothing consumed
	pub(crate) fn loop_variable_in_follows(&mut self) -> bool {
		let before = self.mark();
		self.skip_blanks();
		let follows = self.parse_loop_variable().is_some() && {
			self.skip_blanks();
			self.matches_keyword("in")
		};
		self.rewind(before);
		follows
	}

	pub(crate) fn parse_loop_variable(&mut self) -> Option<Node> {
		let bracket = if self.current_char() == '(' {
			self.advance();
			Bracket::Round
		} else {
			Bracket::None
		};
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
		Some(Node::single_or_list(names, bracket, Separator::Comma))
	}

	/// `(i=0;i<n;i++)`: a group here whose top level holds a `;`
	fn at_c_style_for_head(&self) -> bool {
		if self.current_char() != '(' {
			return false;
		}
		let (mut position, mut depth) = (self.pos, 0);
		while position < self.chars.len() {
			if let Some(after) = text_or_comment_end(&self.chars, position) {
				position = after;
				continue;
			}
			match self.chars[position] {
				'(' | '[' | '{' => depth += 1,
				')' | ']' | '}' if depth == 1 => return false,
				')' | ']' | '}' => depth -= 1,
				';' if depth == 1 => return true,
				_ => {}
			}
			position += 1;
		}
		false
	}

	/// `for x in iterable: body` and `for x in iterable {body}`, after the word `for`; the body of a colon runs to the end of
	/// the statement. `for 1..10 : print it` names no variable (wiki/for.md): `for iterable {body}`, whose items are `it`
	pub(crate) fn try_parse_for_in(&mut self) -> Option<Node> {
		let before_header = self.mark();
		let starts_statement = self.word_starts_statement("for".len());
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
			self.rewind(before_header.clone());
			self.skip_spaces();
			if self.at_c_style_for_head() {
				// `for(i=0;i<n;i++)`; `for (1…5).filter(f):` walks the list
				return Some(self.c_style_for());
			}
		}
		let (iterable, body_word) = self.for_iterable_and_body_word();
		let Some(variable) = named else {
			// only the colon and do forms: `for 1..3 {…}` is read where for loops are lowered
			let body = match body_word {
				Some(word) => self.colon_body(word),
				None if self.current_char() == '{' => self.parse_atom(), // `for (1…5).filter(odd) {print it}`
				None => match starts_statement.then(|| self.indented_lines_below()).flatten() {
					// `play 440Hz for: 0.05s` owns no lines below
					Some(block) => block, // `for 0 to 2` and the lines indented below it
					None => {
						self.rewind(before_header);
						return None;
					}
				},
			};
			return Some(Node::List(vec![Symbol("for".to_string()), iterable, body], Bracket::None, Separator::Space));
		};
		// `for pi in xs` binds no name, as `pi = 4` assigns none (P130, card library-word)
		if let Symbol(name) = variable.drop_meta() {
			if check_constants(name, self.options.data_mode).is_some() {
				let message = format!("{name} is a constant");
				return Some(
					crate::errors::diagnostic::Diagnostic::at(&variable, message).fix("another name").into_error(),
				);
			}
		}
		let key_variable = match variable.drop_meta() {
			Symbol(name) if iterates_keys(&iterable) => Some(name.clone()),
			_ => None,
		};
		self.key_variables.extend(key_variable.clone());
		let body = match body_word {
			Some(word) => self.colon_body(word), // `for i in 0..n: body`, `for i in 0..n do body`
			None if self.current_char() == ';' && self.closing_end_follows(&END_BLOCK_OPENERS) => {
				self.semicolon_end_body(&variable, &iterable)
			}
			None => self.indented_lines_below().unwrap_or_else(|| self.parse_atom()),
		};
		if key_variable.is_some() {
			self.key_variables.pop();
		}
		// kept as written: lowering/control/for_loop/headers.rs gives `for int in xs`, `for chars in text` their meaning
		Some(for_in_loop(variable, iterable, body))
	}

	/// Ruby's `for x in xs; print x; end` (card print-end): the statements up to `end`, with a hint naming the braces
	fn semicolon_end_body(&mut self, variable: &Node, iterable: &Node) -> Node {
		let head = format!("for {} in {}", variable.serialize(), crate::normalize::operand_text(iterable));
		crate::normalize::hint(
			&format!("{head}; … end"),
			&format!("{head} {{ … }}"),
			"a loop's body goes in braces; `; … end` is Ruby's form",
		);
		self.advance();
		self.parse_end_block(false)
	}

	/// The iterable of a loop header, its words up to the body: `for todo in todos sorted by priority {…}`,
	/// `for x in xs where it > 1: …`; a comprehension's iterable is one word, its condition follows
	/// (`[x for x in xs where x > 1]`)
	fn parse_iterable(&mut self) -> Node {
		let first = self.parse_expr(Op::Colon.binding_power().0 + 1);
		let after_first = self.mark();
		let mut words = vec![first];
		loop {
			self.skip_spaces();
			let start = self.pos;
			if self.at_loop_body() || ITERABLE_ENDS.contains(&self.current_char()) || self.matches_keyword(IF_WORD) {
				break;
			}
			let word = self.parse_expr(Op::Colon.binding_power().0 + 1);
			if self.pos == start {
				break;
			}
			words.push(word);
		}
		if words.len() == 1 || !self.at_loop_body() {
			self.rewind(after_first);
			return words.remove(0);
		}
		Node::List(words, Bracket::None, Separator::Space)
	}

	/// A loop's body starts here: `{…}`, `: …`, `do …`, or the next line
	fn at_loop_body(&self) -> bool {
		matches!(self.current_char(), '{' | ':' | '\n')
			|| (self.current_char() == '/' && self.peek_char(1) == '/')
			|| self.matches_keyword("do")
	}

	/// `for (it>2) in xs: body` (wiki/for.md, P46): the items the condition holds for, as `it`; None (nothing consumed)
	/// for any other header
	pub(crate) fn try_parse_condition_loop(&mut self) -> Option<Node> {
		let before = self.mark();
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
		// `(even number)`: an adjective and a type word
		let is_condition = crate::warp_parser::mentions(&condition, crate::words::IMPLICIT_PARAMETER)
			|| self.names_adjective_and_type(&condition);
		if !is_condition || self.current_char() != ')' {
			self.rewind(before);
			return None;
		}
		self.advance();
		self.skip_spaces();
		if !self.matches_keyword("in") {
			self.rewind(before);
			return None;
		}
		self.advance_by("in".len());
		let (iterable, body_word) = self.for_iterable_and_body_word();
		let body = match body_word {
			Some(word) => self.colon_body(word),
			None => self.indented_lines_below().unwrap_or_else(|| self.parse_atom()),
		};
		// kept as written, the condition in round brackets: lowering/control/for_loop/headers.rs filters the loop
		let condition = Node::List(vec![condition], Bracket::Round, Separator::None);
		Some(for_in_loop(condition, iterable, body))
	}

	/// `(even number)` in a loop header: two words, spaced or read as the call `even(number)`, the second a declared type
	/// or a type word
	fn names_adjective_and_type(&self, words: &Node) -> bool {
		let Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) = words.drop_meta()
		else {
			return false;
		};
		let [adjective, type_word] = items.as_slice() else {
			return false;
		};
		let is_type =
			|name: &String| self.declared_types.contains(name) || crate::analyzer::type_word_kind(name).is_some();
		matches!(adjective.drop_meta(), Symbol(_)) && matches!(type_word.drop_meta(), Symbol(name) if is_type(name))
	}

	/// `for (i=0;i<n;i++) {body}`, `for(…) print i`, `for(…): body`, at its head: `((for (head)) {body})` as lowering reads it
	fn c_style_for(&mut self) -> Node {
		let head = self.parse_atom();
		self.skip_blanks();
		let body = match self.current_char() {
			'{' => self.parse_atom(),
			':' => self.colon_body(":"),
			_ if self.matches_keyword(DO_WORD) => self.colon_body(DO_WORD),
			_ => self.indented_lines_below().unwrap_or_else(|| self.rest_of_statement()),
		};
		Node::List(vec![call("for", vec![head]), as_block(body)], Bracket::None, Separator::None)
	}

	pub(crate) fn finish_while_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let body_block = self.parse_atom(); // parse { block }
			return while_do(rhs, body_block);
		}

		if let Node::Key(condition, Op::Colon, body) = &rhs {
			let body = self.continue_colon_body(body); // `while c: i+=2`, `while x-->0 : print x`
			return while_do(condition.as_ref().clone(), body);
		}

		if matches!(rhs.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1) && self.at_body_start() {
			let body = self.parse_expr(0); // `while (i<9) i++`
			return while_do(rhs, body);
		}

		if let Some((condition, body)) = split_trailing_block(&rhs, true) {
			// `{}` parses as ø
			return while_do(condition, body);
		}

		if let Some(body) = self.indented_lines_below() {
			self.skip_end_line(); // Ruby's `while c` … `end`
			return while_do(rhs, body);
		}
		// `while x-->0` alone loops for the condition's effects, as `while x-->0 {}` does
		if BODY_ENDS.contains(&self.current_char()) {
			return while_do(rhs, Empty);
		}
		Node::Key(Box::new(Empty), Op::While, Box::new(rhs))
	}

	/// `3 times {body}`; `x = it times it`: with a value after it (no block) `times` binds like `*` anywhere
	pub(crate) fn try_parse_times(&mut self, lhs: &Node, min_bp: u8) -> Option<Node> {
		let binds = min_bp <= TIMES_BP
			|| self.times_fills_list()
			|| (self.value_after_times() && min_bp <= Op::Mul.binding_power().0);
		if self.options.data_mode || lhs.is_empty_node() || !binds || !self.matches_keyword(TIMES_WORD) {
			return None;
		}
		self.advance_by(TIMES_WORD.len());
		Some(self.parse_times_loop(lhs.clone()))
	}

	/// `thrice {…}`, `twice [x]`, `once "ab"` wherever an operand starts in code: `3 times {…}` (card thrice-block).
	/// `twice: …` stays a key and `twice{a: 1}`, a block of `key: value` pairs, a tag; a function named so (`def twice`),
	/// a member (`p.twice`) and a binding of the word in scope keep it, the binding asked loudly
	pub(crate) fn try_parse_count_word(&mut self) -> Option<Node> {
		if self.options.data_mode
			|| self.context.in_data_literal
			|| self.after_function_keyword
			|| self.prev_char() == '.'
		{
			return None;
		}
		let (word, count) = COUNT_WORDS.into_iter().find(|(word, _)| self.matches_keyword(word))?;
		let after = (word.len()..).find(|at| !matches!(self.peek_char(*at), ' ' | '\t')).unwrap_or(word.len());
		if !matches!(self.peek_char(after), '{' | '[' | '"' | '\'') || self.keeps_bound_count_word(word, count) {
			return None;
		}
		let mark = self.mark();
		self.advance_by(word.len());
		let counted = self.parse_times_loop(Node::int(count));
		if counts_pairs(&counted) {
			self.rewind(mark);
			return None;
		}
		Some(counted)
	}

	/// A variable, function or parameter named like the count word is the program's own: the warning asks which is meant
	fn keeps_bound_count_word(&self, word: &str, count: i64) -> bool {
		let bound = self.variables.contains(word)
			|| self.functions.contains(word)
			|| self.program_functions.contains(word)
			|| self.parameters_in_scope.iter().any(|parameter| parameter == word);
		if !bound {
			return false;
		}
		let (line, column) = self.get_position();
		let question = format!("is `{word} {{…}}` your {word} or the block {count} times? (your {word} wins here)");
		let readings = vec![
			reading(&format!("your {word}"), &format!("{word}({{…}})")),
			reading(&format!("{count} times"), &format!("{count} times {{…}}")),
		];
		let _ = ask(&Ask::new(COUNT_WORD, question, readings, Fallback::Warning).written(word).at(line, column));
		true
	}

	/// `times [x]` fills a list, binding to the count right before it: `xs = 100 times [0]`
	pub(crate) fn times_fills_list(&self) -> bool {
		let after_word = TIMES_WORD.len();
		let blanks = (after_word..).take_while(|&offset| self.peek_char(offset) == ' ').count();
		self.peek_char(after_word + blanks) == '['
	}

	/// `N times {body}` and `N times: body` count with a hidden variable of its own, so nested loops do not meet
	/// `times` at the cursor followed by a value (a name, a number or a text), no block
	fn value_after_times(&self) -> bool {
		if !self.matches_keyword(TIMES_WORD) {
			return false;
		}
		let after =
			(TIMES_WORD.len()..).find(|at| !matches!(self.peek_char(*at), ' ' | '\t')).unwrap_or(TIMES_WORD.len());
		let next = self.peek_char(after);
		next.is_ascii_digit()
			|| matches!(next, '"' | '\'')
			|| self.is_identifier_start(after)
			|| self.is_hole_start(after)
	}

	/// `n times {body}`, `n times: body`, `n times [x]`, `n times "ab"`: the phrase [n times operand] as written, which
	/// lowering/words/times_phrases.rs makes a loop, a filled list or a repeated text
	pub(crate) fn parse_times_loop(&mut self, count: Node) -> Node {
		self.skip_spaces();
		let operand = match self.current_char() {
			'{' | '[' => self.parse_atom(),
			// `3 times: print it` runs the rest of the line, or the lines indented below
			':' => as_block(self.with_equals_comparing(false, |parser| parser.colon_body(":"))),
			quote_or_letter
				if matches!(quote_or_letter, '"' | '\'')
					|| self.is_identifier_start(0)
					|| self.is_hole_start(0)
					|| quote_or_letter.is_ascii_digit() =>
			{
				self.parse_atom()
			}
			_ => return error("`times` needs a body: `3 times {…}`"),
		};
		Node::List(vec![count, Symbol(TIMES_WORD.to_string()), operand], Bracket::None, Separator::Space)
	}
}

/// `keys(m)`, `m.keys`, `m.keys()`: an iterable whose items are a map's keys
pub(crate) fn iterates_keys(iterable: &Node) -> bool {
	let is_keys_word = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word) if word == "keys" || word == crate::words::MAP_KEYS);
	let is_keys_call = |call: &Node| {
		is_keys_word(call)
			|| matches!(call.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(is_keys_word))
	};
	match iterable.drop_meta() {
		Node::Key(_, Op::Dot, method) => is_keys_call(method),
		call => is_keys_call(call),
	}
}

/// `for variable in iterable body`, the loop for_loop.rs lowers
pub(crate) fn for_in_loop(variable: Node, iterable: Node, body: Node) -> Node {
	Node::List(
		vec![Symbol("for".to_string()), variable, Symbol("in".to_string()), iterable, body],
		Bracket::None,
		Separator::Space,
	)
}

/// `while condition body`: the loop head `ø while condition` applied `do` to its body
pub(crate) fn while_do(condition: Node, body: Node) -> Node {
	let head = Node::Key(Box::new(Empty), Op::While, Box::new(condition));
	Node::Key(Box::new(head), Op::Do, Box::new(body))
}

/// `[2 times {a: 1}]`: the block is `key: value` data, the word its tag
fn counts_pairs(counted: &Node) -> bool {
	let Node::List(items, _, _) = counted else { return false };
	matches!(items.last().map(Node::drop_meta), Some(Node::List(entries, Bracket::Curly, _))
		if !entries.is_empty() && entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))))
}
