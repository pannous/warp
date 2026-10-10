//! Expressions: the Pratt loop, special infix operators, prefix operators, if as a prefix

use super::*;

/// Characters that end a statement: a body cannot start with them
const BODY_ENDS: [char; 7] = ['\0', ';', ',', '\n', '}', ')', ']'];
/// The type words of a text: `k:text` names a key in `m[k]` (card text-key)
const TEXT_TYPE_WORDS: [&str; 3] = ["text", "string", "str"];

impl WarpParser {
	/// Pratt parser: parse expression with given minimum binding power
	/// Handles prefix, infix, and suffix operators
	pub(super) fn parse_expr(&mut self, min_bp: u8) -> Node {
		self.skip_spaces();

		// Step 1: Prefix (nud)
		let lhs = if let Some((op, chars)) = self.peek_negated_control_word() {
			self.advance_by(chars);
			self.skip_spaces();
			let rhs = self.parse_prefix_operand(op);
			negate_condition(self.finish_prefix(op, rhs))
		} else if let Some(marker) = self.peek_guard_word() {
			self.parse_guard(marker)
		} else if let Some(phrase) = self.try_parse_after_return() {
			phrase
		} else if let Some(call) = self.try_parse_user_prefix() {
			call
		} else if let Some(statement) = self.try_parse_return() {
			statement
		} else if let Some(data) = self.try_parse_data() {
			data
		} else if let Some(awaited) = self.try_parse_await() {
			awaited
		} else if let Some(emitted) = self.try_parse_emit() {
			emitted
		} else if let Some(head) = self.try_parse_operator_method_head() {
			head
		} else if let Some((op, chars)) = self.peek_prefix_operator().filter(|_| !self.at_member_name(0)) {
			let (prefix_line, prefix_column) = self.get_position();
			// `-7 abs`, `f = abs`, `x abs + 1`: an operator word without its operand is the operator itself, which
			// lowering (ambiguous_forms) applies to the value before it or makes a function
			let operator_word = chars > 1 && matches!(op, Op::Abs | Op::Sqrt | Op::Cbrt);
			let fourth_root = self.current_char() == super::lookahead::FOURTH_ROOT;
			// `sort by abs.take first 1`: a glued method applies to the word, it is no operand
			let method_follows = self.peek_char(chars) == '.' && self.peek_char(chars + 1).is_alphabetic();
			let bare = operator_word && (self.expression_ends_after(chars) || method_follows);
			// `sqrt(2).round(2)`: the glued parentheses are the whole operand, the method applies to the result
			let call_with_method = operator_word && self.parenthesis_length(chars).is_some_and(|length| self.peek_char(chars + length) == '.' && self.peek_char(chars + length + 1).is_alphabetic());
			self.hint_operator(chars, true);
			self.advance_by(chars);
			self.skip_spaces();
			if bare {
				Node::Key(Box::new(Empty), op, Box::new(Empty))
			} else if call_with_method {
				let operand = self.parse_atom();
				self.finish_prefix(op, operand)
			} else if chars == 1 && op == Op::Abs {
				self.parse_norm_bars()
			} else {
				let rhs = self.parse_prefix_operand(op);
				if op == Op::Hash {
					set_hint_position(prefix_line, prefix_column);
					norm::length_operator(&crate::normalize::operand_text(&rhs), false);
				}
				let rhs = if fourth_root { self.finish_prefix(op, rhs) } else { rhs };
				self.finish_prefix(op, rhs)
			}
		} else if let Some(operator) = self.try_parse_operator_value() {
			operator
		} else if let Some(parameters) = self.pipe_parameters() {
			let body = self.parse_expr(Op::FatArrow.binding_power().1);
			Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
		} else {
			self.parse_atom()
		};

		self.continue_expr(lhs, min_bp)
	}

	/// `+(o) := …` defines the method of an operator by its glyph (a class's `plus`): the head is the call `+(o)`, no
	/// unary plus
	pub(super) fn try_parse_operator_method_head(&mut self) -> Option<Node> {
		let (op, chars) = self.peek_operator()?;
		crate::lowering::class_methods::operator_method(op)?;
		let mut offset = chars + self.parenthesis_length(chars)?;
		while matches!(self.peek_char(offset), ' ' | '\t') {
			offset += 1;
		}
		if (self.peek_char(offset), self.peek_char(offset + 1)) != (':', '=') {
			return None;
		}
		self.advance_by(chars);
		let parameters = match self.parse_atom().drop_meta().clone() {
			Node::List(parameters, Bracket::Round, _) => parameters,
			Node::Empty => vec![],
			parameter => vec![parameter],
		};
		Some(Node::List([vec![Symbol(op.to_string())], parameters].concat(), Bracket::Round, Separator::None))
	}

	/// The length of the balanced `(…)` at `offset`, None when none starts there or it never closes
	fn parenthesis_length(&self, offset: usize) -> Option<usize> {
		if self.peek_char(offset) != '(' {
			return None;
		}
		let mut depth = 0;
		for length in 0.. {
			match self.peek_char(offset + length) {
				'(' => depth += 1,
				')' if depth == 1 => return Some(length + 1),
				')' => depth -= 1,
				'\0' => return None,
				_ => {}
			}
		}
		None
	}

	/// An infix operator alone as an argument or item, `sorted(xs, >)`, `[+, *]`: the operator as a value `(> ø ø)`, as
	/// `by: >` gives
	pub(super) fn try_parse_operator_value(&mut self) -> Option<Node> {
		let (op, chars) = self.peek_operator()?;
		if self.current_char().is_alphabetic() {
			return None; // a word operator alone is a name: the parameter `to` of `hanoi(n, from, to, via)`
		}
		let blanks = (chars..).take_while(|&offset| matches!(self.peek_char(offset), ' ' | '\t')).count();
		if !matches!(self.peek_char(chars + blanks), ')' | ',' | ']') {
			return None;
		}
		self.advance_by(chars);
		Some(Node::Key(Box::new(Node::Empty), op, Box::new(Node::Empty)))
	}

	/// `await a + await b`: `await` takes its operand like a unary minus, so each task is awaited before the sum.
	/// `await(x)` (a call or a definition of a function `await`) and `await = …` stay as they are
	pub(super) fn try_parse_await(&mut self) -> Option<Node> {
		if !self.matches_keyword(AWAIT_KEYWORD) {
			return None;
		}
		let before_keyword = self.mark();
		self.advance_by(AWAIT_KEYWORD.len());
		let called = self.current_char() == '(';
		self.skip_spaces();
		let names_a_variable = self.peek_operator().is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		if called || names_a_variable || matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0' | ')') {
			self.rewind(before_keyword);
			return None;
		}
		// `await all jobs` (P47): every task of a list
		if self.matches_keyword(AWAIT_ALL_WORD) && self.peek_char(AWAIT_ALL_WORD.len()) == ' ' && (self.is_identifier_start(AWAIT_ALL_WORD.len() + 1) || matches!(self.peek_char(AWAIT_ALL_WORD.len() + 1), '[' | '(')) {
			self.advance_by(AWAIT_ALL_WORD.len());
			self.skip_spaces();
			let list = self.parse_expr(AWAIT_OPERAND_BP);
			let all = Node::List(vec![Symbol(AWAIT_ALL_WORD.to_string()), list], Bracket::None, Separator::Space);
			return Some(Node::List(vec![Symbol(AWAIT_KEYWORD.to_string()), all], Bracket::None, Separator::Space));
		}
		let operand = self.parse_expr(AWAIT_OPERAND_BP);
		Some(Node::List(vec![Symbol(AWAIT_KEYWORD.to_string()), operand], Bracket::None, Separator::Space))
	}

	/// `1 + emit ask`, `2 * emit stop the machine{reason: "x"} + 1`: the phrase `emit ask` is one operand, the words
	/// of the event and its data bind like the operand of a unary minus; the same Space list a statement `emit ask` is
	pub(super) fn try_parse_emit(&mut self) -> Option<Node> {
		let keyword = EMIT_KEYWORDS.into_iter().find(|keyword| self.matches_keyword(keyword))?;
		let before_keyword = self.mark();
		self.advance_by(keyword.len());
		self.skip_spaces();
		if !self.is_identifier_start(0) || self.peek_operator().is_some() {
			self.rewind(before_keyword);
			return None;
		}
		let mut phrase = vec![Symbol(keyword.to_string())];
		while self.is_identifier_start(0) || (phrase.len() > 1 && matches!(self.current_char(), '0'..='9' | '"' | '\'')) {
			// `send alarm{} to "x"`: a word operator takes the whole phrase, as without this
			if self.peek_operator().is_some() {
				self.rewind(before_keyword);
				return None;
			}
			phrase.push(self.parse_expr(AWAIT_OPERAND_BP));
			self.skip_spaces();
		}
		Some(Node::List(phrase, Bracket::None, Separator::Space))
	}

	/// `return` takes the whole expression after it: `return -1` is no subtraction from `return`
	pub(super) fn try_parse_return(&mut self) -> Option<Node> {
		if !self.matches_keyword(RETURN_KEYWORD) {
			return None;
		}
		let before_keyword = self.mark();
		self.advance_by(RETURN_KEYWORD.len());
		self.skip_spaces();
		let names_a_variable = self.peek_operator().is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		// a bare `return` at the end of a statement returns ø: `if t == ø { return }`
		let value = if matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0') && !names_a_variable {
			Node::Empty
		} else if !self.at_body_start() || names_a_variable {
			self.rewind(before_keyword); // `return := …` as a name
			return None;
		} else {
			// the binding of an assigned value: `return square n` returns the braceless call, like `x = square n`; then
			// what binds looser, Python's `return a if c else b`
			let value = self.parse_expr(Op::Assign.binding_power().1);
			self.continue_expr(value, 0)
		};
		Some(Node::List(vec![Symbol(RETURN_KEYWORD.to_string()), value], Bracket::None, Separator::Space))
	}

	/// `data a and b` is the data `a and b`, also as an operand (`x = data a and b`, `string(data a and b)`); parentheses
	/// around all of it only group it (`data (a and b)`). `data = …`, `data.x`, glued `data(x)` and `data class` stay
	pub(super) fn try_parse_data(&mut self) -> Option<Node> {
		// `data = [4 2]; for x in data {…}`: a variable named data is read, not a prefix
		let is_variable = self.variables.contains(DATA_KEYWORD);
		if is_variable || self.options.data_mode || !self.matches_keyword(DATA_KEYWORD) || self.peek_char(DATA_KEYWORD.len()) != ' ' {
			return None;
		}
		let before_keyword = self.mark();
		self.advance_by(DATA_KEYWORD.len());
		self.skip_spaces();
		let ends = matches!(self.current_char(), '}' | ']' | ')' | ';' | ',' | '\n' | '\r' | '\0');
		if ends || self.peek_operator().is_some() || self.matches_keyword(CLASS_KEYWORD) {
			self.rewind(before_keyword);
			return None;
		}
		let parsed = self.parse_expr(Op::Assign.binding_power().1);
		let data = match parsed.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() == 1 => items[0].clone(),
			_ => parsed,
		};
		Some(Node::List(vec![Symbol(DATA_KEYWORD.to_string()), data], Bracket::None, Separator::Space))
	}

	/// The operator after `lhs` that the infix table does not hold, its width in characters and its binding power
	pub(super) fn special_infix(&self, lhs: &Node) -> Option<(SpecialInfix, usize, (u8, u8))> {
		// `a mod b` (`a modulo b`) is `a % b` (Euclidean, 0 ≤ r < |b|), `a rem b` the truncated remainder (sign of the
		// dividend, as C)
		for (word, op) in [("mod", Op::Mod), ("modulo", Op::Mod), ("rem", Op::Rem)] {
			if self.matches_keyword(word) {
				return Some((SpecialInfix::Keyword(op), word.len(), op.binding_power()));
			}
		}
		// Python floor division glued to its operand, `7//2` and `x//=2` (`x // note` stays a comment), and `7 div 2`;
		// `div{…}` and `div {…}` are the html tag, and data never computes
		let word_division = self.matches_keyword("div") && !self.options.data_mode && !self.word_opens_block("div");
		if word_division || self.at_floor_division() {
			let compound = !word_division && self.peek_char(2) == '=';
			let op = if compound { Op::Assign } else { Op::Div };
			return Some((SpecialInfix::FloorDivision { compound }, if compound || word_division { 3 } else { 2 }, op.binding_power()));
		}
		// the pipeline `xs |> f(a)` is the call `f(xs, a)`, `xs |> f` is `f(xs)` (F#, Elixir); it binds below arithmetic
		// and above comparison: `xs |> sum > 3` is `sum(xs) > 3`
		if self.at_pipeline() {
			return Some((SpecialInfix::Pipeline, 2, PIPELINE_BINDING_POWER));
		}
		// Julia's dot call `f.(xs)`: a name, a dot, an opening parenthesis
		let is_name = matches!(lhs.drop_meta(), Node::Symbol(_));
		if is_name && self.current_char() == '.' && self.peek_char(1) == '(' && !self.options.data_mode {
			return Some((SpecialInfix::DotCall, 1, Op::Dot.binding_power()));
		}
		// element-wise arithmetic `xs .+ 4` maps the operator over the list (D3: `xs + 4` asks, `xs + [4]` joins)
		if let Some(op) = self.element_wise_operator() {
			return Some((SpecialInfix::ElementWise(op), 2, op.binding_power()));
		}
		// membership `x in xs` binds like `==`, kept as the list [x in xs]: `if "Z" in v {…}`, `found = x in xs`;
		// a unit word stays in the flat counting phrase: `number of chars in t`
		let counts_units = matches!(lhs.drop_meta(), Node::Symbol(word) if crate::analyzer::text_unit(word).is_some());
		// `item is in basket` reads as `item in basket`
		let membership = [IS_IN_PHRASE, IN_KEYWORD].into_iter().find(|word| self.matches_keyword(word));
		if let (Some(word), false) = (membership, counts_units) {
			return Some((SpecialInfix::Membership(IN_KEYWORD), word.len(), Op::Eq.binding_power()));
		}
		// `xs contains x` binds like `x in xs`, in code only (data keeps the words as they are)
		let containment = CONTAINMENT_WORDS.into_iter().find(|word| !self.options.data_mode && self.matches_keyword(word) && self.operand_after(word.len()));
		containment.map(|word| (SpecialInfix::Membership(word), word.len(), Op::Eq.binding_power()))
	}

	/// The right operand of a special infix operator, None when it binds looser than `min_bp` (nothing consumed)
	pub(super) fn at_pipeline(&self) -> bool {
		self.current_char() == '|' && self.peek_char(1) == '>'
	}

	pub(super) fn special_operand(&mut self, width: usize, (left, right): (u8, u8), min_bp: u8) -> Option<Node> {
		if left < min_bp {
			return None;
		}
		self.advance_by(width);
		self.skip_whitespace();
		Some(self.parse_expr(right))
	}

	/// The infix and suffix operators after an already parsed left operand, binding tighter than `min_bp`
	pub(super) fn continue_expr(&mut self, mut lhs: Node, min_bp: u8) -> Node {
		const ARGUMENT_BP: u8 = Op::Add.binding_power().0; // a braceless argument takes arithmetic, stops at ranges and comparisons: f 3-1 > 5
		const MAX_BP_FOR_APPLICATION: u8 = Op::Mul.binding_power().1; // operand of + - * / takes a braceless call: 1 + f 3
		const SUBSCRIPT_BP: u8 = Op::Hash.binding_power().0;
		// Right operand of the last comparison, to chain a<b<c into a<b and b<c
		let mut previous_comparand: Option<Node> = None;
		loop {
			self.skip_spaces_and_inline_comments(); // not newlines: they are separators

			// a whole statement or assigned value, never an operand inside it: `ys = xs.map(f) @parallel` marks the map call
			let whole_expression = min_bp <= Op::Assign.binding_power().1;
			if let Some(annotated) = whole_expression.then(|| self.try_parse_trailing_attribute(&lhs)).flatten() {
				lhs = annotated;
				continue;
			}

			// Step 2: Suffix (led)
			// P48: a declared suffix operator wins over the built-in one of the same glyph (`suffix operator ³`)
			if let Some(updated) = self.try_parse_user_suffix(&lhs, min_bp).or_else(|| self.try_parse_suffix(&lhs, min_bp)) {
				lhs = updated;
				continue;
			}
			if let Some(updated) = self.try_parse_user_infix(&lhs, min_bp) {
				lhs = updated;
				continue;
			}

			// `(() => 7)()`: a parenthesized function called with no arguments, the `()` glued to the group
			let parenthesized = matches!(lhs.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1);
			if parenthesized && self.prev_char() == ')' && self.current_char() == '(' && self.peek_char(1) == ')' {
				self.advance_by(2);
				lhs = Node::List(vec![lhs, Node::List(vec![], Bracket::Round, Separator::None)], Bracket::None, Separator::Space);
				continue;
			}

			// Step 2b: Subscript (tight, like Op::Hash)
			if let Some(updated) = self.try_parse_subscript(&lhs, min_bp, SUBSCRIPT_BP) {
				lhs = updated;
				continue;
			}

			if let Some(updated) = self.try_parse_evaluate_bang(&lhs, min_bp)
				.or_else(|| self.try_parse_control_suffix(&lhs, min_bp))
				.or_else(|| self.try_parse_test_word(&lhs, min_bp))
				.or_else(|| self.try_parse_nand(&lhs, min_bp))
				.or_else(|| self.try_parse_elvis(&lhs, min_bp)) {
				lhs = updated;
				continue;
			}

			// the argument of `square xs |> sum` ends at the pipeline, which takes the whole call (lists.rs)
			if self.pipe_takes_call && self.at_pipeline() {
				break;
			}
			// Step 3a–e: the operators the infix table does not hold (`mod`, `//`, `|>`, `.+`, `in`, see SpecialInfix)
			if let Some((infix, width, binding)) = self.special_infix(&lhs) {
				let Some(operand) = self.special_operand(width, binding, min_bp) else { break };
				lhs = infix.applied(lhs, operand);
				previous_comparand = None;
				continue;
			}

			// Step 3: Check for infix operator
			// `whenever not x {…}`, `waiting = xs where not done`: the word `not` negates what follows, it is no infix
			// (the effect constraint `f := … ! IO` is written with `!`)
			let prefix_not = self.matches_keyword("not");
			let (op, chars) = match self.peek_operator().filter(|_| !prefix_not) {
				Some(pair) => pair,
				None if self.stops_at_else && self.at_else_if_word().is_some() => break, // `if c: x elif d: y`
				None if self.at_block_close() => break, // `do x end`: the `end` is no argument of x
				None => {
					// Step 3b: Check for implicit function application (space between atoms)
					// This makes `x = f y` parse as `x = (f y)` and `252 > f y` as `252 > (f y)`
					// Only apply when:
					// - min_bp > 0 (not at top level)
					// - min_bp <= 130 (inside assignment/comparison/arithmetic)
					// - lhs is a Symbol or a function call (List with Bracket::None)
					// - NOT parenthesized expressions (List with Bracket::Round)
					// - Argument conditions vary by context:
					//   - In assignment (min_bp <= 60): allow ANY argument (including identifiers)
					//   - In colon/comparison (min_bp > 60): only non-identifier args
					//   This allows `fetch url` but prevents `a: b c d` from chaining
					if let Some(updated) = self.try_parse_implicit_application(
						&lhs,
						min_bp,
						ARGUMENT_BP,
						MAX_BP_FOR_APPLICATION,
					) {
						lhs = updated;
						continue;
					}
					break;
				}
			};

			let (l_bp, r_bp) = op.binding_power();

			// `sleep 1s and print "x"`: the statement ends before the `and`, the statement list runs both (parse_list_with_separators)
			let ends_command = op == Op::And && (self.in_command || (min_bp == 0 && is_command(&lhs))) && self.and_starts_statement();
			// Stop if operator binds less tightly than our minimum
			if ends_command || l_bp < min_bp || (op == Op::Else && self.stops_at_else) {
				break;
			}
			if let Err(refused) = self.left_arrow_assignment(op, &lhs) {
				lhs = refused;
				break;
			}
			if matches!(op, Op::Add | Op::Sub) {
				match self.signed_number_starts_a_list(&lhs) {
					Ok(false) => {}
					Ok(true) => {
						if op == Op::Add {
							self.advance_by(1); // `+1` is the element 1
						}
						break; // `1 -1`: the signed number is the next element
					}
					Err(strict) => {
						lhs = strict;
						break;
					}
				}
			}

			// Consume the operator
			// a single `|` may pipe into a function (pipes.rs): its `or` hint waits for the operand
			let may_pipe = chars == 1 && self.current_char() == '|' && !self.options.data_mode;
			let written = if may_pipe { PIPE_GLYPH.to_string() } else { self.hint_operator(chars, false) };
			let (op_line, op_column) = self.get_position();
			let bare_symbol = Some(self.current_char()).filter(|symbol| chars == 1 && matches!(symbol, '&' | '|'));
			let glued_before = !self.prev_char().is_whitespace();
			self.advance_by(chars);
			// `for:email "Email"`: the glued pair is for:email, the text the next item (card parser-tag)
			let glued_pair = op == Op::Colon && glued_before && !self.current_char().is_whitespace();
			let block_body = match op {
				Op::Colon | Op::Define if self.only_blanks_before_newline() => {
					self.with_equals_comparing(false, |parser| parser.parse_indented_block()) // the block of `if c:` and `f(n):=` assigns
				}
				Op::Do | Op::Then | Op::Else if self.closing_end_follows(&END_BLOCK_OPENERS) => Some(self.parse_end_block(op == Op::Then)),
				Op::Do if self.closing_end_follows(&["do"]) => Some(error(AMBIGUOUS_END)), // `do a; if c then b end`
				_ => None,
			};
			if block_body.is_none() {
				self.skip_whitespace();
			}
			if op == Op::Define {
				if let Some(name) = defined_function_name(&lhs) {
					if matches!(lhs.drop_meta(), Node::List(..)) {
						self.functions_with_parameters.insert(name.clone());
					}
					self.functions.insert(name);
				}
			}

			// Parse right-hand side with appropriate binding power
			if op == Op::Colon {
				self.equals_compares = false; // in `if c: x=1` the colon ends the condition, the rest is the body
			}
			let rhs_start = self.pos;
			let rhs = match block_body {
				Some(block) => block,
				None => self.parse_right_operand(op, glued_pair, r_bp),
			};
			let rhs_end = self.pos.min(self.chars.len());
			let rhs_written: String = self.chars[rhs_start.min(rhs_end)..rhs_end].iter().collect();

			if op == Op::Define && !matches!(lhs.drop_meta(), Node::List(..)) && !mentions(&rhs, "it") {
				self.functions.remove(&lhs.name()); // `x := 5` defines a value, not a function
			}

			if let Some(symbol) = bare_symbol {
				if let Some(diagnostic) = logic_mixed_with_comparison(&lhs, symbol, &rhs) {
					lhs = Diagnostic { line: op_line, column: op_column, ..diagnostic }.into_error();
					previous_comparand = None;
					continue;
				}
			}

			let op = match self.range_reading(op, &written, &rhs, op_line, op_column) {
				Ok(op) => op,
				Err(unanswered) => {
					lhs = unanswered;
					previous_comparand = None;
					continue;
				}
			};

			if op == Op::Hash && !self.names_a_key(&rhs) {
				crate::normalize::set_position_of(&lhs);
				norm::index_operator(&crate::normalize::operand_text(&lhs), &crate::normalize::operand_text(&rhs), false);
			}

			if op.is_equality() && ungrouped_equality(&lhs) {
				lhs = Diagnostic { line: op_line, column: op_column, ..chained_equality(&lhs, op, &rhs) }.into_error();
				previous_comparand = None;
				continue;
			}

			let rhs = if may_pipe { self.pipe_operand(rhs, op_line, op_column) } else { rhs };
			let rhs = if op == Op::Eq && written != IS_WORD { crate::type_tests::equality_operand(rhs) } else { rhs };
			if op == Op::Eq && written == IS_WORD {
				lhs = crate::type_tests::with_compared_text(lhs, rhs_written.trim());
			}
			let op = if op == Op::Assign && is_function_block(&lhs, &rhs) {
				self.functions.insert(lhs.name());
				Op::Define
			} else {
				op
			};
			self.note_variable(&lhs, op, &rhs);
			if let Some(warning) = hash_range_warning(&lhs, op, &written, &rhs) {
				if let Err(strict) = crate::diagnostic::report(&[warning]) {
					lhs = strict;
					continue;
				}
			}
			lhs = combined(lhs, op, &written, rhs.clone(), previous_comparand.take());
			if op.is_ordering() {
				previous_comparand = Some(rhs);
			}
		}

		lhs
	}

	/// The right operand of `op` without a block: a branch, a glued pair's value or an expression
	fn parse_right_operand(&mut self, op: Op, glued_pair: bool, r_bp: u8) -> Node {
		if matches!(op, Op::Then | Op::Else) {
			let outer = self.branch_bp.replace(r_bp);
			let branch = self.parse_branch(r_bp);
			let branch = self.branch_assignment(branch, r_bp);
			self.branch_bp = outer;
			branch
		} else if glued_pair {
			let outer = self.glued_pair_bp.replace(r_bp);
			let value = self.parse_expr(r_bp);
			self.glued_pair_bp = outer;
			value
		} else if matches!(op, Op::And | Op::Or) && self.take_braceless_print() {
			// `it%2 and print it`, `x || print "none"`: print takes the rest of the statement, as at its start
			print_call([self.rest_of_statement()])
		} else {
			self.parse_expr(r_bp)
		}
	}

	/// A branch without braces takes the whole statement: `if c then s += 5 else s = 0` assigns in the branch, where
	/// assignment's weaker binding would end the branch at `s` and assign to the whole if. The value stops at `else`.
	pub(super) fn branch_assignment(&mut self, target: Node, branch_bp: u8) -> Node {
		self.skip_spaces();
		let Some((assignment, chars)) = self.peek_operator().filter(|(op, _)| *op == Op::Assign || op.is_compound_assign()) else { return target };
		self.advance_by(chars);
		self.skip_whitespace();
		let value = self.parse_expr(branch_bp);
		Node::Key(Box::new(target), assignment, Box::new(value))
	}

	/// `‖x‖` brackets a whole expression like parentheses: `‖3-5‖*2` → 4
	pub(super) fn parse_norm_bars(&mut self) -> Node {
		let inner = self.parse_expr(0);
		self.skip_spaces();
		if self.current_char() != '‖' {
			return missing_closer("Missing closing ‖".into(), "‖", crate::fixits::end_of_line(&self.input, self.line_nr));
		}
		self.advance();
		self.finish_prefix(Op::Abs, inner)
	}

	/// Operand of a prefix operator; `++i` binds like `i++`, conditions compare with `=`
	pub(super) fn parse_prefix_operand(&mut self, op: Op) -> Node {
		let (left_bp, right_bp) = op.binding_power();
		match op {
			Op::Inc | Op::Dec => self.parse_expr(left_bp),
			Op::If | Op::While => self.parse_condition(),
			// `#m#1` counts `m#1`: indexing a count is never meant
			Op::Hash => self.parse_expr(left_bp - 1),
			Op::Add => self.parse_expr(Op::Neg.binding_power().1), // `+2^2` like `-2^2`
			_ => self.parse_expr(right_bp),
		}
	}

	/// The condition of `if` and `while`: `=` compares, and a braceless call takes a variable argument as in a branch
	/// (`if area certainly > 10 then`, `while square i < 10`)
	pub(super) fn parse_condition(&mut self) -> Node {
		let condition_bp = Op::If.binding_power().1;
		let outer = self.branch_bp.replace(condition_bp);
		let condition = self.with_equals_comparing(true, |parser| parser.parse_expr(condition_bp));
		self.branch_bp = outer;
		condition
	}

	pub(super) fn with_equals_comparing<T>(&mut self, compares: bool, parse: impl FnOnce(&mut Self) -> T) -> T {
		let outer = std::mem::replace(&mut self.equals_compares, compares);
		let node = parse(self);
		self.equals_compares = outer;
		node
	}

	pub(super) fn finish_prefix(&mut self, op: Op, rhs: Node) -> Node {
		match (op, rhs.drop_meta()) {
			(Op::If, _) => self.finish_if_prefix(rhs),
			(Op::While, _) => self.finish_while_prefix(rhs),
			(Op::Neg, Node::Number(number)) => Node::Number(-*number),
			(Op::Add, _) => rhs,
			(Op::Inc | Op::Dec, _) => Node::Key(Box::new(rhs), op, Box::new(Empty)), // ++i is i++: increment is immediate
			_ => Node::Key(Box::new(Empty), op, Box::new(rhs)),
		}
	}

	pub(super) fn finish_if_prefix(&mut self, rhs: Node) -> Node {
		self.skip_spaces();
		if self.current_char() == '{' {
			let then_block = self.parse_atom(); // parse { block }
			let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(rhs));
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
			return self.parse_optional_else(if_then, ElseParseMode::Atom);
		}

		if let Node::Key(cond, Op::Colon, then_expr) = &rhs {
			let if_cond = Node::Key(Box::new(Empty), Op::If, cond.clone());
			// the body runs to the end of the statement, `if c: x+=1`, but not into the `else`
			let outer = std::mem::replace(&mut self.stops_at_else, true);
			let then_expr = match then_expr.drop_meta() {
				_ if is_print_word(then_expr) && self.braceless_argument_follows() => { // `if it%2: print it`, `if c: print ord x`
					let first = self.expression_of_words();
					self.print_call_from(first, Self::expression_of_words)
				}
				// `if c: print a, b`: the colon took `print a`, the comma continues its arguments
				Node::List(words, Bracket::None, Separator::Space) if words.len() > 1 && is_print_word(&words[0]) && self.current_char() == ',' => {
					self.print_call_from(one_expression(&words[1..]), |parser| parser.parse_expr(0))
				}
				_ => self.continue_expr(then_expr.as_ref().clone(), 0),
			};
			self.stops_at_else = outer;
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_expr));
			return self.parse_optional_else(if_then, ElseParseMode::Expr);
		}

		if let Some((condition, then_block)) = split_trailing_block(&rhs, false) {
			self.skip_spaces();
			let if_cond = Node::Key(Box::new(Empty), Op::If, Box::new(condition));
			let if_then = Node::Key(Box::new(if_cond), Op::Then, Box::new(then_block));
			return self.parse_optional_else(if_then, ElseParseMode::Atom);
		}

		// `if (x < 0) return -1`: C's parenthesized condition, then the statement on the same line
		let is_grouped = matches!(rhs.drop_meta(), Node::List(_, Bracket::Round, _));
		if is_grouped && self.at_body_start() && !self.matches_keyword("then") && !self.matches_keyword("else") {
			let outer = std::mem::replace(&mut self.stops_at_else, true);
			let then_expr = self.parse_expr(0);
			self.stops_at_else = outer;
			let if_then = Node::Key(Box::new(Node::Key(Box::new(Empty), Op::If, Box::new(rhs))), Op::Then, Box::new(then_expr));
			return self.parse_optional_else(if_then, ElseParseMode::Expr);
		}

		Node::Key(Box::new(Empty), Op::If, Box::new(rhs))
	}

	/// `|x| x*x`, `|a, b| a+b` (Rust, and Ruby's `{ |x| x*x }`): the parameters of a lambda between bars, which the
	/// body follows; nothing else starts with a bar
	pub(super) fn pipe_parameters(&mut self) -> Option<Node> {
		if self.current_char() != '|' || self.options.data_mode {
			return None;
		}
		let mut length = 1;
		while matches!(self.peek_char(length), ' ' | ',') || self.peek_char(length).is_alphanumeric() || self.peek_char(length) == '_' {
			length += 1;
		}
		let written: String = (1..length).map(|offset| self.peek_char(offset)).collect();
		let names: Vec<Node> = written.split([' ', ',']).filter(|name| !name.is_empty()).map(|name| Node::Symbol(name.to_string())).collect();
		let mut body_start = length + 1;
		while self.peek_char(body_start) == ' ' {
			body_start += 1;
		}
		let first_name = 1 + written.len() - written.trim_start().len();
		if self.peek_char(length) != '|' || names.is_empty() || !self.is_identifier_start(first_name) || BODY_ENDS.contains(&self.peek_char(body_start)) {
			return None;
		}
		self.advance_by(body_start);
		Some(match <[Node; 1]>::try_from(names) {
			Ok([name]) => name,
			Err(names) => Node::List(names, Bracket::Round, Separator::Colon),
		})
	}

	/// A statement body follows: not a separator, closing bracket, end of input or the `do` keyword
	pub(super) fn at_body_start(&self) -> bool {
		!BODY_ENDS.contains(&self.current_char()) && !self.matches_keyword("do")
	}
}

impl WarpParser {
	/// A loop's key variable (`for k in keys(m)`) or a name holding a text: `m[k]` looks a key up, no indexing hint
	pub(super) fn names_a_key(&self, index: &Node) -> bool {
		matches!(index.drop_meta(), Node::Symbol(name) if self.key_variables.contains(name) || self.text_variables.contains(name))
	}

	/// `l = "en"` or `k:text` (a typed parameter or declaration) makes l, k text variables; any `x = …` a variable
	fn note_variable(&mut self, lhs: &Node, op: Op, rhs: &Node) {
		let Node::Symbol(name) = lhs.drop_meta() else { return };
		if matches!(op, Op::Assign | Op::Define) {
			self.variables.insert(name.clone());
		}
		let holds_text = match (op, rhs.drop_meta()) {
			(Op::Assign | Op::Define, Node::Text(_)) => true,
			(Op::Colon, Node::Symbol(type_word)) => TEXT_TYPE_WORDS.contains(&type_word.as_str()),
			_ => false,
		};
		if holds_text {
			self.text_variables.insert(name.clone());
		}
	}
}

/// `lhs op rhs`; a chained comparison `a<b<c` as `a<b and b<c` (`middle` is b), a range subscript `xs#(a..b)` as a slice,
/// `a down to b` as the reversed `b to a`
fn combined(lhs: Node, op: Op, written: &str, rhs: Node, middle: Option<Node>) -> Node {
	match (middle, hash_slice_bounds(&rhs)) {
		(Some(middle), _) if op.is_ordering() => {
			let next_comparison = Node::Key(Box::new(middle), op, Box::new(rhs));
			Node::Key(Box::new(lhs), Op::And, Box::new(next_comparison))
		}
		(_, Some((start, end))) if op == Op::Hash => call(SLICE_WORD, vec![lhs, start, end]),
		_ if op == Op::To && written.starts_with(DOWN_WORD) => call(REVERSE_WORD, vec![Node::Key(Box::new(rhs), Op::To, Box::new(lhs))]),
		_ => Node::Key(Box::new(lhs), op, Box::new(rhs)),
	}
}
