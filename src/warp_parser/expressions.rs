//! Expressions: the Pratt loop, special infix operators, prefix operators

use super::*;
use crate::warp_parser::context::Context;

const AMBIGUOUS_END: &str = "ambiguous `end`: it closes either the `then` or the `do`; as in Ruby and Lua every `then … end` and `do … end` needs its own: write `while c do … if x then … end end` or `while c { … if x { … } }`";
/// Words between two operands besides the operator words (operators::is_operator_word): `of`, `matches`
const JOINING_WORDS: [&str; 2] = [OF_WORD, "matches"];
const COMPARISON_FIRST: &str = "the comparison first, as C";

/// Characters that end a statement: a body cannot start with them
pub(crate) const BODY_ENDS: [char; 7] = ['\0', ';', ',', '\n', '}', ')', ']'];
/// A braceless argument takes arithmetic and ranges, stops at comparisons: f 3-1 > 5, sqrt 4 + 5 > 2, y = sum 1..4
/// (card braceless-argument: every callable as at statement level, `sum 1..4`)
const ARGUMENT_BP: u8 = Op::Range.binding_power().0;
/// `x not in xs`: the negated membership, no prefix `not`
const NOT_IN_SPELLING: &str = "not in";

impl WarpParser {
	/// Pratt parser: parse expression with given minimum binding power
	/// Handles prefix, infix, and suffix operators
	pub(super) fn parse_expr(&mut self, min_bp: u8) -> Node {
		self.skip_spaces();
		let heads_definition = self.after_function_keyword_word();

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
			// `sqrt(16) + 1`, `sqrt(2).round(2)`: glued parentheses are the whole operand, like a call's
			let glued_call = operator_word && self.parenthesis_length(chars).is_some();
			self.hint_operator(chars, true);
			self.advance_by(chars);
			self.skip_spaces();
			if bare {
				Node::Key(Box::new(Empty), op, Box::new(Empty))
			} else if glued_call {
				let operand = self.parse_atom();
				self.finish_prefix(op, operand)
			} else if operator_word {
				// `sqrt 4 + 5` is sqrt(9): an operator word takes its braceless argument like a call (f 3-1 is f(2));
				// `map sqrt xs`, `apply abs -3`: lowering may read the word as the function instead
				let operand = self.parse_expr(ARGUMENT_BP);
				let applied = self.finish_prefix(op, operand);
				Node::Meta {
					node: Box::new(applied),
					data: Box::new(Node::key(crate::words::OPERATOR_WORD_MARK, Node::True)),
				}
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
			self.bind_parameters(&parameters, true);
			let body = self.parse_expr(Op::FatArrow.binding_power().1);
			Node::Key(Box::new(parameters), Op::FatArrow, Box::new(body))
		} else {
			self.parse_atom()
		};

		self.continues_definition_head = heads_definition;
		self.continue_expr(lhs, min_bp)
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
		const MAX_BP_FOR_APPLICATION: u8 = Op::Mul.binding_power().1; // operand of + - * / takes a braceless call: 1 + f 3
		const SUBSCRIPT_BP: u8 = Op::Hash.binding_power().0;
		// Right operand of the last comparison, to chain a<b<c into a<b and b<c
		let mut previous_comparand: Option<Node> = None;
		let heads_definition = std::mem::take(&mut self.continues_definition_head);
		loop {
			self.skip_spaces_and_inline_comments(); // not newlines: they are separators

			// a whole statement or assigned value, never an operand inside it: `ys = xs.map(f) @parallel` marks the map call
			let whole_expression = min_bp <= Op::Assign.binding_power().1;
			// P48: a declared suffix operator wins over the built-in one of the same glyph (`suffix operator ³`)
			if let Some(updated) = whole_expression
				.then(|| self.try_parse_trailing_attribute(&lhs))
				.flatten()
				.or_else(|| self.try_parse_user_suffix(&lhs, min_bp))
				.or_else(|| self.try_parse_suffix(&lhs, min_bp))
				.or_else(|| self.try_parse_user_infix(&lhs, min_bp))
				.or_else(|| self.try_parse_empty_call(&lhs))
				.or_else(|| self.try_parse_subscript(&lhs, min_bp, SUBSCRIPT_BP))
				.or_else(|| self.try_parse_rust_macro(&lhs))
				.or_else(|| self.try_parse_evaluate_bang(&lhs, min_bp))
				.or_else(|| self.try_parse_times(&lhs, min_bp))
				.or_else(|| self.try_parse_test_word(&lhs, min_bp))
				.or_else(|| self.try_parse_elvis(&lhs, min_bp))
			{
				lhs = updated;
				continue;
			}

			// the argument of `square xs |> sum` ends at the pipeline, which takes the whole call (lists.rs)
			if self.context.pipe_takes_call && self.at_pipeline() {
				break;
			}
			// Julia's dot call `f.(xs)`: a name, a dot, an opening parenthesis
			let is_name = matches!(lhs.drop_meta(), Node::Symbol(_));
			if is_name && self.current_char() == '.' && self.peek_char(1) == '(' && !self.options.data_mode {
				let Some(arguments) = self.special_operand(1, Op::Dot.binding_power(), min_bp) else {
					break;
				};
				lhs = dot_call(lhs, arguments);
				previous_comparand = None;
				continue;
			}

			// Step 3: Check for infix operator
			// `whenever not x {…}`, `waiting = xs where not done`: the word `not` negates what follows, it is no infix
			// (the effect constraint `f := … ! IO` is written with `!`)
			let prefix_not = self.matches_keyword("not") && !self.matches_keyword(NOT_IN_SPELLING);
			let (op, chars) = match self.peek_operator().filter(|_| !prefix_not) {
				Some(pair) => pair,
				None if self.context.stops_at_else && self.at_else_if_word().is_some() => break, // `if c: x elif d: y`
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
					//   - In assignment (min_bp <= 60) and in an operand of or/and/comparison/arithmetic: ANY argument
					//     (including identifiers), `1 + count xs` is `1 + count(xs)`
					//   - In colon and the like (between): only non-identifier args
					//   This allows `fetch url` but prevents `a: b c d` from chaining
					if let Some(updated) = self.try_parse_free_infix(&lhs, min_bp).or_else(|| {
						self.try_parse_implicit_application(&lhs, min_bp, ARGUMENT_BP, MAX_BP_FOR_APPLICATION)
					}) {
						lhs = updated;
						continue;
					}
					break;
				}
			};

			let (l_bp, r_bp) = op.binding_power();
			// a unit word stays in the flat counting phrase `number of chars in t`; `def contains(a,b) = …` names the
			// function after its keyword
			let membership_word = op == Op::In
				&& matches!(lhs.drop_meta(), Node::Symbol(word)
					if crate::analyzer::text_unit(word).is_some() || crate::operators::is_function_keyword(word));

			// `sleep 1s and print "x"`: the statement ends before the `and`, the statement list runs both (parse_list_with_separators);
			// between two comparisons it joins conditions: `sq 2 == 4 and sq 3 == 8`
			let joins_conditions = crate::library_words::is_comparison(&lhs) && self.comparison_follows_and();
			let ends_command = op == Op::And
				&& (self.context.in_command || (min_bp == 0 && is_command(&lhs)))
				&& !joins_conditions
				&& self.and_starts_statement();
			// Stop if operator binds less tightly than our minimum
			if ends_command || membership_word || l_bp < min_bp || (op == Op::Else && self.context.stops_at_else) {
				break;
			}
			// `xs where #it > 1`, `[i for i in xs where -i < 0]`, `where to#i == n`: after `where` a glued `#` or `-`, or an
			// operator word (a variable `to`), starts the condition
			let starts_operand = (matches!(op, Op::Hash | Op::Sub) && !self.peek_char(chars).is_whitespace())
				|| self.current_char().is_alphabetic();
			if starts_operand && ends_with_clause_word(&lhs) {
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
			// a single `|` may pipe into a function (pipes.rs), a single `&` or `|` may combine bits (may_be_whole_number):
			// their `and`/`or` hint waits for the operands
			let may_pipe = chars == 1 && self.current_char() == '|' && !self.options.data_mode;
			let deferred_hint = may_pipe || (chars == 1 && self.current_char() == '&');
			let written =
				if deferred_hint { self.current_char().to_string() } else { self.hint_operator(chars, false) };
			// `a nand b` is `not (a and b)`, `x is not int` is `not (x is int)`
			let negated = crate::operators::negated_spelling(&written);
			let written = negated.map_or(written, str::to_string);
			let (op_line, op_column) = self.get_position();
			let bare_symbol = Some(self.current_char()).filter(|symbol| chars == 1 && matches!(symbol, '&' | '|'));
			let glued_before = !self.prev_char().is_whitespace();
			self.advance_by(chars);
			// `f - 2` subtracts, `f -2` may pass -2 to f (SpacedMinus)
			let spaced_minus = op == Op::Sub
				&& !glued_before
				&& self.current_char().is_whitespace()
				&& matches!(lhs.drop_meta(), Node::Symbol(_));
			// `for:email "Email"`: the glued pair is for:email, the text the next item (card parser-tag)
			let glued_pair = op == Op::Colon && glued_before && !self.current_char().is_whitespace();
			// `def foo: …` defines foo by what follows, its body may call foo like the body of `foo := …`
			let colon_defines = op == Op::Colon && heads_definition && !glued_pair && !self.result_type_follows();
			if colon_defines {
				self.register_defined_function(&lhs);
				self.bind_parameters(&lhs, false);
			}
			let block_body = match op {
				Op::Colon | Op::Define if self.only_blanks_before_newline() => {
					self.with_equals_comparing(false, |parser| parser.parse_indented_block()) // the block of `if c:` and `f(n):=` assigns
				}
				// `def foo: hi=2*2` defines foo by the rest of the line, no `(foo: hi) = 2*2`; Scala's `def f(x: Int): Int = …`
				// declares its result type
				Op::Colon if colon_defines => {
					self.skip_spaces();
					Some(self.with_equals_comparing(false, |parser| parser.rest_of_statement()))
				}
				Op::Do | Op::Then | Op::Else if self.closing_end_follows(&END_BLOCK_OPENERS) => {
					Some(self.parse_end_block(op == Op::Then))
				}
				Op::Do if self.closing_end_follows(&["do"]) => Some(error(AMBIGUOUS_END)), // `do a; if c then b end`
				_ => None,
			};
			if block_body.is_none() {
				self.skip_whitespace();
			}
			if op == Op::Define {
				self.register_defined_function(&lhs);
				self.bind_parameters(&lhs, false);
			}
			if matches!(op, Op::FatArrow | Op::Arrow) {
				self.bind_parameters(&lhs, true);
			}

			// Parse right-hand side with appropriate binding power
			if op == Op::Colon {
				self.context.equals_compares = false; // in `if c: x=1` the colon ends the condition, the rest is the body
			}
			let rhs_start = self.pos;
			let rhs = match block_body {
				Some(block) => block,
				None => self.parse_right_operand(op, glued_pair, r_bp),
			};
			let rhs_end = self.pos.min(self.chars.len());
			let rhs_written: String = self.chars[rhs_start.min(rhs_end)..rhs_end].iter().collect();

			if op == Op::Define
				&& !matches!(lhs.drop_meta(), Node::List(..))
				&& !mentions(&rhs, crate::words::IMPLICIT_PARAMETER)
			{
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
				norm::index_operator(
					&crate::normalize::operand_text(&lhs),
					&crate::normalize::operand_text(&rhs),
					false,
				);
			}

			if op.is_equality() && ungrouped_equality(&lhs) {
				lhs = Diagnostic { line: op_line, column: op_column, ..chained_equality(&lhs, op, &rhs) }.into_error();
				previous_comparand = None;
				continue;
			}

			let bitwise = deferred_hint
				&& crate::lowering::nodes::may_be_whole_number(&lhs)
				&& crate::lowering::nodes::may_be_whole_number(&rhs);
			let mut rhs = match bare_symbol.filter(|_| deferred_hint) {
				Some(_) if self.in_arm_key() => rhs, // `1 | 2 =>`: alternatives, no pipe, no bit hint
				Some(_) if may_pipe && crate::pipes::may_be_stage(&rhs) => self.pipe_operand(rhs, op_line, op_column),
				// any operand after `|` may be a pipe stage (`n | f(x)` or `xs | sort`): pipes.rs decides, by the functions
				Some(_) if bitwise && may_pipe => crate::pipes::pipe_stage(rhs),
				Some(_) if bitwise => rhs,
				Some(_) if may_pipe => self.pipe_operand(rhs, op_line, op_column),
				Some(symbol) => {
					set_hint_position(op_line, op_column);
					norm::operator(&symbol.to_string(), false);
					rhs
				}
				None => rhs,
			};
			// a type word compares as its type on either side of == and != (`int != integer` is no)
			if op.is_equality() && written != IS_WORD {
				rhs = crate::type_tests::equality_operand(&lhs, rhs);
				lhs = crate::type_tests::equality_operand(&rhs, lhs);
			}
			if op == Op::Eq && written == IS_WORD {
				lhs = crate::type_tests::with_compared_text(lhs, rhs_written.trim());
			}
			let op = if op == Op::Assign && is_function_block(&lhs, &rhs) {
				self.functions.insert(lhs.name());
				Op::Define
			} else if bitwise && !crate::pipes::is_pipe_stage(&rhs) {
				if op == Op::And {
					Op::BitAnd
				} else {
					Op::BitOr
				}
			} else {
				op
			};
			self.note_variable(&lhs, op, &rhs);
			if let Some(warning) = hash_range_warning(&lhs, op, &written, &rhs) {
				if let Err(strict) = crate::errors::diagnostic::report(&[warning]) {
					lhs = strict;
					continue;
				}
			}
			let lhs_operand = if spaced_minus { lhs.with_meta_data(super::SpacedMinus) } else { lhs };
			lhs = combined(lhs_operand, op, &written, rhs.clone(), previous_comparand.take());
			if negated.is_some() {
				lhs = Node::Key(Box::new(Empty), Op::Not, Box::new(lhs));
			}
			if op.is_ordering() {
				previous_comparand = Some(rhs);
			}
		}

		lhs
	}

	/// `(() => 7)()`: a parenthesized function called with no arguments, the `()` glued to the group
	fn try_parse_empty_call(&mut self, lhs: &Node) -> Option<Node> {
		let parenthesized = matches!(lhs.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1);
		if !(parenthesized && self.prev_char() == ')' && self.current_char() == '(' && self.peek_char(1) == ')') {
			return None;
		}
		self.advance_by(2);
		let no_arguments = Node::List(vec![], Bracket::Round, Separator::None);
		Some(Node::List(vec![lhs.clone(), no_arguments], Bracket::None, Separator::Space))
	}

	/// The right operand of `op` without a block: a branch, a glued pair's value or an expression
	fn parse_right_operand(&mut self, op: Op, glued_pair: bool, r_bp: u8) -> Node {
		if matches!(op, Op::If | Op::While) {
			self.parse_condition()
		} else if matches!(op, Op::Then | Op::Else) {
			self.within(Context { branch_bp: Some(r_bp), ..self.context }, |parser| {
				let branch = parser.parse_branch(r_bp);
				parser.branch_assignment(branch, r_bp)
			})
		} else if glued_pair {
			self.within(Context { glued_pair_bp: Some(r_bp), ..self.context }, |parser| parser.parse_expr(r_bp))
		} else if matches!(op, Op::Range | Op::To) && self.peek_operator().is_some_and(|(next, _)| next == Op::FatArrow)
		{
			Empty // the match arm `65.. => "senior"`: a range without end
		} else if matches!(op, Op::And | Op::Or) && self.take_braceless_print() {
			// `it%2 and print it`, `x || print "none"`: print takes the rest of the statement, as at its start
			print_call([self.rest_of_statement()])
		} else {
			self.parse_expr(r_bp)
		}
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
			// `-floor 2.5` negates the call `floor 2.5`, as `1 + floor 2.5` adds it (card leading-minus); a variable takes
			// no argument, and in `[-x 1]` the space starts the next item
			Op::Neg => {
				let operand = self.parse_expr(right_bp);
				let summand_bp = Op::Add.binding_power().1;
				match operand.drop_meta() {
					Node::Symbol(name) if !self.context.in_spaced_list && !self.variables.contains(name) => self
						.try_parse_implicit_application(&operand, summand_bp, ARGUMENT_BP, summand_bp)
						.unwrap_or(operand),
					_ => operand,
				}
			}
			_ => self.parse_expr(right_bp),
		}
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
}

impl WarpParser {}

/// A word joining two operands, never a call of the word before it: `a and b`, `n mod 100`, `x in xs`, `p is int`
pub(crate) fn joins_operands(word: &str) -> bool {
	crate::operators::is_operator_word(word) || JOINING_WORDS.contains(&word)
}

/// `a==b` built in the same expression, not grouped by parentheses (a group is parsed as its own node)
fn ungrouped_equality(node: &Node) -> bool {
	matches!(node, Node::Key(_, op, _) if op.is_equality())
}

/// `1==1==1` chains in Python (true) but is `(1==1)==1` in C: ambiguous, like in Rust
fn chained_equality(lhs: &Node, op: Op, rhs: &Node) -> Diagnostic {
	let (left, right) = (lhs.serialize(), rhs.serialize());
	let middle = match lhs {
		Node::Key(_, _, middle) => middle.serialize(),
		_ => left.clone(),
	};
	let symbol = op.as_str();
	let (chained, grouped) = (format!("{left} and {middle} {symbol} {right}"), format!("({left}) {symbol} {right}"));
	Diagnostic::default()
		.fix(format!("{chained} or {grouped}"))
		.message(format!("ambiguous: equality does not chain in {left} {symbol} {right}"))
		.offer("both equalities, as Python chains them", format!("{left} {symbol} {right}"), chained)
		.offer("compare the first result, as C does", format!("{left} {symbol} {right}"), grouped)
}

/// `3 & 4 == 4` is `(3&4)==4` in Python but `3 & (4==4)` in C: `&`/`|` next to an ungrouped comparison is ambiguous.
/// The word forms `and`/`or` read unambiguously and are not affected: `x==1 and y==2`.
fn logic_mixed_with_comparison(lhs: &Node, symbol: char, rhs: &Node) -> Option<Diagnostic> {
	let comparison = |node: &Node| match node.drop_meta() {
		Node::Key(left, op, right) if op.is_comparison() => Some((left.serialize(), op.as_str(), right.serialize())),
		_ => None,
	};
	let (left, right) = (lhs.serialize(), rhs.serialize());
	let readings = match (comparison(lhs), comparison(rhs)) {
		(None, Some((a, op, b))) => vec![
			(COMPARISON_FIRST.to_string(), format!("{left} {symbol} ({a} {op} {b})")),
			(format!("`{symbol}` first, as Python"), format!("({left} {symbol} {a}) {op} {b}")),
		],
		(Some((a, op, b)), None) => vec![
			(COMPARISON_FIRST.to_string(), format!("({a} {op} {b}) {symbol} {right}")),
			(format!("`{symbol}` first, as Python"), format!("{a} {op} ({b} {symbol} {right})")),
		],
		(Some(_), Some(_)) => vec![("both comparisons first".to_string(), format!("({left}) {symbol} ({right})"))],
		(None, None) => return None,
	};
	let word = if symbol == '&' { "and" } else { "or" };
	let forms: Vec<&str> = readings.iter().map(|(_, form)| form.as_str()).collect();
	let written = format!("{left} {symbol} {right}");
	let diagnostic = Diagnostic::default()
		.fix(forms.join(" or "))
		.message(format!("ambiguous: `{symbol}` mixed with a comparison in {written}; group it or write `{word}`"));
	Some(readings.iter().fold(diagnostic, |diagnostic, (meaning, form)| diagnostic.offer(meaning, &written, form)))
}
