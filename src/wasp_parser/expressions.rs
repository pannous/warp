//! Expressions: the Pratt loop, special infix operators, prefix operators, if as a prefix

use super::*;

/// Characters that end a statement: a body cannot start with them
const BODY_ENDS: [char; 7] = ['\0', ';', ',', '\n', '}', ')', ']'];

impl WaspParser {
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
		} else if let Some(awaited) = self.try_parse_await() {
			awaited
		} else if let Some((op, chars)) = self.peek_prefix_operator().filter(|_| !self.at_member_name(0)) {
			let (prefix_line, prefix_column) = self.get_position();
			self.hint_operator(chars, true);
			self.advance_by(chars);
			self.skip_spaces();
			if chars == 1 && op == Op::Abs {
				self.parse_norm_bars()
			} else {
				let rhs = self.parse_prefix_operand(op);
				if op == Op::Hash {
					set_hint_position(prefix_line, prefix_column);
					norm::length_operator(&crate::normalize::operand_text(&rhs), false);
				}
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
		let before_keyword = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.advance_by(AWAIT_KEYWORD.len());
		let called = self.current_char() == '(';
		self.skip_spaces();
		let names_a_variable = self.peek_operator().is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		if called || names_a_variable || matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0' | ')') {
			(self.pos, self.line_nr, self.column, self.current_line) = before_keyword;
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

	/// `return` takes the whole expression after it: `return -1` is no subtraction from `return`
	pub(super) fn try_parse_return(&mut self) -> Option<Node> {
		if !self.matches_keyword(RETURN_KEYWORD) {
			return None;
		}
		let before_keyword = (self.pos, self.line_nr, self.column, self.current_line.clone());
		self.advance_by(RETURN_KEYWORD.len());
		self.skip_spaces();
		let names_a_variable = self.peek_operator().is_some_and(|(op, _)| matches!(op, Op::Assign | Op::Define | Op::Colon) || op.is_compound_assign());
		// a bare `return` at the end of a statement returns ø: `if t == ø { return }`
		let value = if matches!(self.current_char(), '}' | ';' | '\n' | '\r' | '\0') && !names_a_variable {
			Node::Empty
		} else if !self.at_body_start() || names_a_variable {
			(self.pos, self.line_nr, self.column, self.current_line) = before_keyword; // `return := …` as a name
			return None;
		} else {
			self.parse_expr(0)
		};
		Some(Node::List(vec![Symbol(RETURN_KEYWORD.to_string()), value], Bracket::None, Separator::Space))
	}

	/// The operator after `lhs` that the infix table does not hold, its width in characters and its binding power
	pub(super) fn special_infix(&self, lhs: &Node) -> Option<(SpecialInfix, usize, (u8, u8))> {
		// `a mod b` is `a % b` (Euclidean, 0 ≤ r < |b|), `a rem b` the truncated remainder (sign of the dividend, as C)
		for (word, op) in [("mod", Op::Mod), ("rem", Op::Rem)] {
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
		if self.current_char() == '|' && self.peek_char(1) == '>' {
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
		if self.matches_keyword(IN_KEYWORD) && !counts_units {
			return Some((SpecialInfix::Membership, IN_KEYWORD.len(), Op::Eq.binding_power()));
		}
		None
	}

	/// The right operand of a special infix operator, None when it binds looser than `min_bp` (nothing consumed)
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

			// Step 3a–e: the operators the infix table does not hold (`mod`, `//`, `|>`, `.+`, `in`, see SpecialInfix)
			if let Some((infix, width, binding)) = self.special_infix(&lhs) {
				let Some(operand) = self.special_operand(width, binding, min_bp) else { break };
				lhs = infix.applied(lhs, operand);
				previous_comparand = None;
				continue;
			}

			// Step 3: Check for infix operator
			let (op, chars) = match self.peek_operator() {
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
			self.advance_by(chars);
			let block_body = match op {
				Op::Colon if self.only_blanks_before_newline() => {
					self.with_equals_comparing(false, |parser| parser.parse_indented_block()) // the block of `if c:` assigns
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
				None if matches!(op, Op::Then | Op::Else) => {
					let outer = self.branch_bp.replace(r_bp);
					let branch = self.parse_expr(r_bp);
					let branch = self.branch_assignment(branch, r_bp);
					self.branch_bp = outer;
					branch
				}
				None => self.parse_expr(r_bp),
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

			if op == Op::Hash {
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
			if let Some(warning) = hash_range_warning(&lhs, op, &written, &rhs) {
				if let Err(strict) = crate::diagnostic::report(&[warning]) {
					lhs = strict;
					continue;
				}
			}
			lhs = match previous_comparand.take() {
				Some(middle) if op.is_ordering() => {
					let next_comparison = Node::Key(Box::new(middle), op, Box::new(rhs.clone()));
					Node::Key(Box::new(lhs), Op::And, Box::new(next_comparison))
				}
				_ if op == Op::Hash && hash_slice_bounds(&rhs).is_some() => {
					let (start, end) = hash_slice_bounds(&rhs).expect("guarded");
					Node::List(vec![Symbol(SLICE_WORD.to_string()), lhs, start, end], Bracket::Round, Separator::None)
				}
				_ => Node::Key(Box::new(lhs), op, Box::new(rhs.clone())),
			};
			if op.is_ordering() {
				previous_comparand = Some(rhs);
			}
		}

		lhs
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
			return error("Missing closing ‖");
		}
		self.advance();
		self.finish_prefix(Op::Abs, inner)
	}

	/// Operand of a prefix operator; `++i` binds like `i++`, conditions compare with `=`
	pub(super) fn parse_prefix_operand(&mut self, op: Op) -> Node {
		let (left_bp, right_bp) = op.binding_power();
		match op {
			Op::Inc | Op::Dec => self.parse_expr(left_bp),
			Op::If | Op::While => self.with_equals_comparing(true, |parser| parser.parse_expr(right_bp)),
			// `#m#1` counts `m#1`: indexing a count is never meant
			Op::Hash => self.parse_expr(left_bp - 1),
			Op::Add => self.parse_expr(Op::Neg.binding_power().1), // `+2^2` like `-2^2`
			_ => self.parse_expr(right_bp),
		}
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
			let then_expr = self.continue_expr(then_expr.as_ref().clone(), 0);
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
