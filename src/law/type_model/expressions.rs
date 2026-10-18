//! Exporting functions, blocks and expressions to W0: loops, lambdas, extrema, tallies, conversions and checked results
use super::*;
use crate::words::{ERROR_WORD, FOR_WORD, IN_WORD, IS_ERROR_WORD, MAP_WORD};

impl Exporter {
	/// A function of one parameter as it is; of none, a function of `unit`; of several, a function of its
	/// arguments object, whose fields the parameters read; a declared result converts the body's value (`body as int`)
	pub(super) fn function(&mut self, definition: &crate::lowering::FunctionHead) -> Lean {
		let (name, parameters, body) = (definition.name, definition.parameters(), definition.body);
		let declared = format!("(some ({}))", self.functions[name]);
		let (parameter, fields, parameter_type) = match parameters.as_slice() {
			[parameter] => {
				let inferred = parameter_type_word(parameter).is_none();
				(self.parameter_type(parameter)?.0, vec![], if inferred { "none".to_string() } else { declared })
			}
			[] => (UNIT_PARAMETER.to_string(), vec![], declared),
			parameters => (
				arguments_class(name),
				parameters.iter().map(|parameter| parameter_word(parameter)).collect(),
				declared,
			),
		};
		let references: Vec<String> = parameters
			.iter()
			.filter(|parameter| parameter_type_word(parameter).is_some_and(|word| self.cell_classes.contains(&word)))
			.map(|parameter| parameter_word(parameter))
			.collect();
		let cells: Vec<String> =
			assigned_locals(body, &self.globals).into_iter().filter(|cell| !references.contains(cell)).collect();
		// an assigned parameter's cell starts with the argument (read before the cell's name hides it)
		let first_values: Vec<String> = cells
			.iter()
			.map(|cell| match () {
				_ if *cell == parameter => format!(".loc {}", quoted(cell)),
				_ if fields.contains(cell) => format!(".get (.loc {}) {}", quoted(&parameter), quoted(cell)),
				_ => UNIT_TYPE.to_string(),
			})
			.collect();
		self.cell_lists = cells.iter().filter(|cell| first_value_is_list(body, cell)).cloned().collect();
		self.cells = cells.iter().chain(&references).cloned().collect();
		self.locals.push(parameter.clone());
		self.argument_fields = fields.clone();
		let body = &without_tail_return(body);
		let return_event = format!("{name}{RETURN_EVENT_SUFFIX}");
		self.returning = returns(body).then(|| return_event.clone());
		let body = self.block(body).map(|body| match self.returning.take() {
			Some(event) => format!(
				".handle {event} (.abort {event} none (.loc {})) ({body})",
				quoted(EVENT_LOCAL),
				event = quoted(&event)
			),
			None => body,
		});
		self.locals.pop();
		self.argument_fields.clear();
		self.cells.clear();
		let body = cells.iter().zip(&first_values).rev().fold(body?, |body, (cell, first_value)| {
			let path = lean_strings(&[cell_class(name, cell)]);
			let new_cell = match first_value.as_str() {
				UNIT_TYPE => format!(".new {path}"),
				_ => format!(
						".letIn \"·new\" (.cls {path}) (.new {path}) (.seq (.set (.loc \"·new\") {} ({first_value})) (.loc \"·new\"))",
						quoted(CELL_FIELD)
					),
			};
			format!(".letIn {} (.cls {path}) ({new_cell}) ({body})", quoted(cell))
		});
		// a cell parameter among several is read from the arguments object once
		let body = references.iter().filter(|reference| fields.contains(reference)).fold(body, |body, reference| {
			let cell = parameters
				.iter()
				.find(|parameter| parameter_word(parameter) == **reference)
				.and_then(|parameter| parameter_type_word(parameter))
				.expect("a cell parameter");
			let path = lean_strings(&[cell]);
			format!(
				".letIn {} (.cls {path}) (.get (.loc {}) {}) ({body})",
				quoted(reference),
				quoted(&parameter),
				quoted(reference)
			)
		});
		let body = match definition.result {
			Some(result) if crate::type_name_matching::is_checked_result(&result.name()) => {
				self.checked_result(body, result)?
			}
			Some(result) => self.conversion_of(body, result)?,
			None => body,
		};
		Ok(format!(".function {} {} {parameter_type} ({body})", quoted(name), quoted(&parameter)))
	}

	/// a function body: `{ a; b }` is the sequence
	pub(super) fn block(&mut self, body: &Node) -> Lean {
		match body.drop_meta() {
			// `{emit too big{value: x}}`: the words of one statement
			Node::List(items, Bracket::Curly, Separator::Space) if items.len() > 1 => {
				self.expression(&Node::List(items.clone(), Bracket::None, Separator::Space))
			}
			Node::List(items, Bracket::Curly, _) if !items.is_empty() => self.sequence(items),
			_ => self.expression(body),
		}
	}

	/// Statements, at least one, in order: `.seq (a) (.seq (b) (c))`
	pub(super) fn sequence(&mut self, statements: &[Node]) -> Lean {
		let mut statements = self.expressions(statements)?;
		let last = statements.pop().expect("a statement");
		Ok(statements.iter().rev().fold(last, |rest, statement| format!(".seq ({statement}) ({rest})")))
	}

	pub(super) fn expressions(&mut self, items: &[Node]) -> Result<Vec<String>, String> {
		items.iter().map(|item| self.expression(item)).collect()
	}

	pub(super) fn assignment(&mut self, name: &str, value: &Node) -> Lean {
		if self.cells.iter().any(|cell| cell == name) {
			return Ok(format!(".set (.loc {}) {} ({})", quoted(name), quoted(CELL_FIELD), self.expression(value)?));
		}
		let declared = self.names.get(name).cloned().flatten();
		let value = match place_literal(self.bool_names.contains(&name.to_string()).then_some(BOOL_TYPE), value) {
			Some(yes_or_no) => yes_or_no,
			None => self.shared_list(name, declared.as_deref(), value)?,
		};
		if !self.names.contains_key(name) {
			return Err(format!("not in W0: a first binding of {name} inside an expression"));
		}
		Ok(format!(".assign {} ({value})", quoted(name)))
	}

	/// `for x in xs { body }`: x is a local of the body; a `break` in it ends the loop. A type word or `it` as the
	/// variable (type filters, unit walks) is not in W0, nor a main-level name (warp's loop assigns it, W0's shadows it)
	pub(super) fn for_loop(&mut self, variable: &Node, list: &Node, body: &Node) -> Lean {
		let variable = match variable.drop_meta() {
			Node::Symbol(name)
				if name != crate::words::IMPLICIT_PARAMETER
					&& type_of_word(name).is_none()
					&& !self.names.contains_key(name)
					&& !self.cells.contains(name) =>
			{
				name.clone()
			}
			_ => return Err(format!("not in W0: the loop variable {}", variable.serialize().trim())),
		};
		let list = self.expression(list)?;
		self.locals.push(variable.clone());
		let body = self.block(body);
		self.locals.pop();
		Ok(format!(".forIn {} ({list}) ({}) {UNIT_TYPE}", quoted(&variable), body?))
	}

	/// `x => body`: x is a local of the body; a lambda of no or several parameters is not in W0
	pub(super) fn lambda(&mut self, parameter: &Node, body: &Node) -> Lean {
		let parameter = match parameter.drop_meta() {
			Node::Symbol(name) => name.clone(),
			other => return Err(format!("not in W0: the lambda parameters {}", other.serialize().trim())),
		};
		self.locals.push(parameter.clone());
		let body = self.block(body);
		self.locals.pop();
		Ok(format!(".lam {} ({})", quoted(&parameter), body?))
	}

	/// a variable, parameter or local (not a function or class): what a call of it calls is its value
	pub(super) fn holds_value(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Symbol(name) => {
				!self.functions.contains_key(name)
					&& !self.classes.contains_key(name)
					&& (self.names.contains_key(name)
						|| self.locals.contains(name)
						|| self.argument_fields.contains(name)
						|| self.cells.contains(name))
			}
			_ => false,
		}
	}

	/// `min(a, b)` is `b < a ? b : a`, `max(a, b)` is `a < b ? b : a`, a and b bound once
	pub(super) fn extremum(&mut self, word: &str, a: &Node, b: &Node) -> Lean {
		self.hidden_names += 1;
		let (first, second) =
			(quoted(&format!("·a{}", self.hidden_names)), quoted(&format!("·b{}", self.hidden_names)));
		let (left, right) = if word == EXTREMUM_WORDS[0] { (&second, &first) } else { (&first, &second) };
		let comparison = format!(".ite (.lt (.loc {left}) (.loc {right})) (.loc {second}) (.loc {first})");
		Ok(format!(
			".letIn {first} {ANY_TYPE} ({}) (.letIn {second} {ANY_TYPE} ({}) ({comparison}))",
			self.expression(a)?,
			self.expression(b)?
		))
	}

	/// a literal, a name (not a function's), or operators on such: evaluating it twice changes nothing
	pub(super) fn is_pure(&self, node: &Node) -> bool {
		match node.ungrouped() {
			Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty => true,
			Node::Symbol(name) => !self.functions.contains_key(name),
			Node::Key(left, op, right) => {
				!matches!(op, Op::Assign | Op::Define | Op::Dot)
					&& !op.is_compound_assign()
					&& self.is_pure(left)
					&& self.is_pure(right)
			}
			_ => false,
		}
	}

	/// the value given to a place of the given type word: to a cell parameter, the caller's cell itself
	pub(super) fn argument(&mut self, type_word: Option<&str>, value: &Node) -> Lean {
		match value.drop_meta() {
			Node::Symbol(name)
				if type_word.is_some_and(|word| self.cell_classes.iter().any(|cell| cell == word))
					&& self.cells.contains(name) =>
			{
				Ok(format!(".loc {}", quoted(name)))
			}
			_ => self.expression(value),
		}
	}

	pub(super) fn cell_value(&self, cell: &str) -> String {
		format!(".get (.loc {}) {}", quoted(cell), quoted(CELL_FIELD))
	}

	/// `count list`, `count item in list`, and with `position` `item in list`: a walk of the list with a fresh `·tally`
	pub(super) fn tally(&mut self, list: &Node, item: Option<&Node>, position: bool) -> Lean {
		let (tally, path) = (".loc \"·t\"", lean_strings(&[TALLY_CLASS.to_string()]));
		let field = |name: &str| format!(".get ({tally}) {}", quoted(name));
		let write = |name: &str, value: String| format!(".set ({tally}) {} ({value})", quoted(name));
		let increment = |name: &str| write(name, format!(".add ({}) (.int 1)", field(name)));
		let matches = |then: String| format!(".ite (.eq false (.loc \"·item\") (.loc \"·x\")) ({then}) .unit");
		let step = match (item, position) {
			(None, _) => increment(CELL_FIELD),
			(Some(_), false) => matches(increment(CELL_FIELD)),
			(Some(_), true) => {
				let first = format!(
					".ite (.eq false ({}) (.int 0)) ({}) .unit",
					field(CELL_FIELD),
					write(CELL_FIELD, field(TALLY_INDEX))
				);
				format!(".seq ({}) ({})", increment(TALLY_INDEX), matches(first))
			}
		};
		let walk = format!(".forIn \"·item\" ({}) ({step}) {UNIT_TYPE}", self.expression(list)?);
		let start = format!(
			".seq ({}) ({})",
			write(CELL_FIELD, ".int 0".to_string()),
			write(TALLY_INDEX, ".int 0".to_string())
		);
		let counted = format!(
			".letIn \"·t\" (.cls {path}) (.new {path}) (.seq ({start}) (.seq ({walk}) ({})))",
			field(CELL_FIELD)
		);
		match item {
			Some(item) => Ok(format!(".letIn \"·x\" .any ({}) ({counted})", self.expression(item)?)),
			None => Ok(counted),
		}
	}

	/// `k` of `m[k]` (parsed `m#(k+1)`) of a map name m: a computed key
	pub(super) fn map_key<'a>(&self, map: &Node, index: &'a Node) -> Option<&'a Node> {
		matches!(map.drop_meta(), Node::Symbol(name) if self.map_names.contains(name))
			.then(|| bracket_index(index))
			.flatten()
	}

	pub(super) fn binary(&mut self, constructor: &str, left: &Node, right: &Node) -> Lean {
		Ok(format!("{constructor} ({}) ({})", self.expression(left)?, self.expression(right)?))
	}

	/// `x as int`: warp's conversion to a scalar type (to a class or a list it is another operation)
	pub(super) fn conversion(&mut self, value: &Node, target: &Node) -> Lean {
		let value = self.expression(value)?;
		self.conversion_of(value, target)
	}

	pub(super) fn conversion_of(&self, value: String, target: &Node) -> Lean {
		let Some(target) = type_of_word(&target.name()).filter(|lean| !lean.starts_with(LIST_TYPE_PREFIX)) else {
			return Err(format!("not in W0: as {}", target.serialize().trim()));
		};
		Ok(format!(".conv ({value}) ({target})"))
	}

	/// `-> int`, `-> int or text`: the result checked when it runs, as warp lowers it (type_name_matching
	/// checked_result), where converting could lose something
	pub(super) fn checked_result(&self, value: String, target: &Node) -> Lean {
		let declared = self.annotation_type(target)?;
		Ok(match union_alternatives(&declared) {
			Some(alternatives) => format!(".cast ({value}) {alternatives}"),
			None => format!(".cast ({value}) [{declared}]"),
		})
	}

	/// `if c then a else b` and `c ? a : b`
	pub(super) fn conditional(&mut self, condition: &Node, then: &Node, otherwise: &Node) -> Lean {
		Ok(format!(
			".ite ({}) ({}) ({})",
			self.expression(condition)?,
			self.expression(then)?,
			self.expression(otherwise)?
		))
	}

	pub(super) fn expression(&mut self, node: &Node) -> Lean {
		if let Some(value) = returned(node) {
			let event = self.returning.clone().ok_or("not in W0: a return outside a function body")?;
			return Ok(format!(".emit {} ({})", quoted(&event), self.expression(&value)?));
		}
		// an item written into a list or field raises when it is a stored error, as warp's emitter checks it
		if let Some((item, place)) = crate::lowering::data_items::marked_item(node) {
			return Ok(format!(".written ({}) {}", self.expression(item)?, quoted(&place.holds_no_error())));
		}
		match node.drop_meta() {
			Node::Number(Number::Int(n)) => Ok(format!(".int ({n})")),
			Node::Number(Number::Float(x)) if Number::is_exact_decimal(*x) => {
				let (numerator, denominator) = crate::wasm_emitter::numbers::exact::decimal_fraction(*x);
				Ok(exact_number(numerator, denominator))
			}
			Node::Number(Number::Float(x)) => Ok(format!(".flt (Float.toBits ({x:?} : Float))")),
			Node::Number(Number::Quotient(numerator, denominator)) => Ok(exact_number(numerator, denominator)),
			Node::True => Ok(".bool true".to_string()),
			Node::False => Ok(".bool false".to_string()),
			Node::Text(text) => Ok(format!(".text {}", quoted(text))),
			Node::Char(c) => Ok(format!(".text {}", quoted(&c.to_string()))), // `"a"` parses as a codepoint
			Node::Empty => Ok(EMPTY_LIST.to_string()),
			Node::Symbol(name) if self.cells.contains(name) => Ok(self.cell_value(name)),
			// the function's first local is its arguments object; loops and lambdas push theirs after it
			Node::Symbol(name) if self.argument_fields.contains(name) => Ok(format!(
				".get (.loc {}) {}",
				quoted(self.locals.first().expect("the arguments object")),
				quoted(name)
			)),
			Node::Symbol(name) if self.locals.contains(name) => Ok(format!(".loc {}", quoted(name))),
			// `two()` of a function of no parameters parses as `(two)`
			Node::Symbol(name) if self.functions.get(name).is_some_and(|parameter| parameter == UNIT_TYPE) => {
				Ok(format!(".call {} .unit", quoted(name)))
			}
			Node::Symbol(name) if self.lambda_names.contains(name) => {
				Err(format!("not in W0: {name} as a value (warp calls it; `function {name}` is the function)"))
			}
			Node::Symbol(name) if self.names.contains_key(name) => Ok(format!(".glob {}", quoted(name))),
			// a unit word is a quantity of one unit (`2 km` parses as `2 * km`)
			Node::Symbol(name) if crate::numbers::units::is_unit(name) => Ok(quantity_literal(name)),
			_ if map_entries(node).is_some() && self.classes.contains_key(MAP_WORD) => {
				self.instance(MAP_WORD, map_entries(node).expect("a map"))
			}
			Node::List(items, Bracket::Square, _) => Ok(self
				.expressions(items)?
				.iter()
				.rev()
				.fold(EMPTY_LIST.to_string(), |tail, head| format!(".cons ({head}) ({tail})"))),
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.expression(&items[0]),
			Node::List(items, Bracket::Curly, _) if !items.is_empty() => self.block(node),
			// `a; b` and `(a; b)`: the statements in order, the value the last one's
			Node::List(items, Bracket::None | Bracket::Round, Separator::Semicolon | Separator::Newline)
				if items.len() > 1 =>
			{
				self.sequence(items)
			}
			_ if effect(node).is_some() => self.effect(effect(node).expect("an effect")),
			Node::Symbol(word) if word == BREAK_KEYWORD => Err("not in W0: a break".to_string()),
			// `global y` in a function only declares: the name is main-level and assignable there
			_ if global_declaration(node)
				.is_some_and(|(name, value)| value.is_none() && self.names.contains_key(name)) =>
			{
				Ok(UNIT_TYPE.to_string())
			}
			Node::List(items, _, _) if items.len() == 2 && items[0].is_symbol(BREAK_KEYWORD) => {
				Err("not in W0: a break".to_string())
			}
			Node::List(items, _, _) if items.first().is_some_and(|class| self.classes.contains_key(&class.name())) => {
				self.construction(&items[0].name(), &items[1..])
			}
			Node::List(items, _, _) => match items.as_slice() {
				[marker, body, handler] if marker.is_symbol(TRY_MARKER) => self.binary(".tryCatch", body, handler),
				[for_word, variable, in_word, list, body]
					if for_word.is_symbol(FOR_WORD) && in_word.is_symbol(IN_WORD) =>
				{
					self.for_loop(variable, list, body)
				}
				// `count 2 in xs` parses as `count (2 in xs)`
				[count, list] if count.is_symbol(COUNT_WORD) && !self.functions.contains_key(COUNT_WORD) => {
					match list.drop_meta() {
						Node::List(words, Bracket::None, _) if words.len() == 3 && words[1].is_symbol(IN_WORD) => {
							self.tally(&words[2], Some(&words[0]), false)
						}
						_ => self.tally(list, None, false),
					}
				}
				// decision in-bool: `x in xs` is yes when x has a position in xs
				[item, in_word, list] if in_word.is_symbol(IN_WORD) => {
					Ok(truthiness(self.tally(list, Some(item), true)?))
				}
				[call, a, b]
					if EXTREMUM_WORDS.iter().any(|word| call.is_symbol(word))
						&& !self.functions.contains_key(&call.name()) =>
				{
					self.extremum(&call.name(), a, b)
				}
				// a stored error: a value until an operation that needs another value meets it (Decided #1)
				[call, message] if call.is_symbol(ERROR_WORD) => match message.drop_meta() {
					Node::Text(message) => Ok(format!(".fail {}", quoted(message))),
					Node::Char(letter) => Ok(format!(".fail {}", quoted(&letter.to_string()))),
					_ => outside_w0(node),
				},
				[call, value] if call.is_symbol(IS_ERROR_WORD) => Ok(format!(".failed ({})", self.expression(value)?)),
				[call] if self.functions.get(&call.name()).is_some_and(|parameter| parameter == UNIT_TYPE) => {
					Ok(format!(".call {} .unit", quoted(&call.name())))
				}
				// `add 1 to 2` of `to add number a to number b: …` parses as `add (1 to 2)`: two arguments
				[call, argument]
					if self.classes.contains_key(&arguments_class(&call.name()))
						&& matches!(argument.drop_meta(), Node::Key(_, Op::To, _)) =>
				{
					let Node::Key(first, _, second) = argument.drop_meta() else { unreachable!("a `to` pair") };
					let arguments = self.construction(
						&arguments_class(&call.name()),
						&[first.as_ref().clone(), second.as_ref().clone()],
					)?;
					Ok(format!(".call {} ({arguments})", quoted(&call.name())))
				}
				[call, arguments @ ..] if self.classes.contains_key(&arguments_class(&call.name())) => {
					// an argument list that does not fit is a compile error in warp: W0 rejects a call given no arguments object
					let class = arguments_class(&call.name());
					let arguments = match self.field_values(&class, arguments) {
						Ok(written) => self
							.instance(&class, written.iter().map(|(field, value)| (field.clone(), value)).collect())?,
						Err(_) => UNIT_TYPE.to_string(),
					};
					Ok(format!(".call {} ({arguments})", quoted(&call.name())))
				}
				// `f(3)` of a name holding a lambda
				[value, argument] if self.holds_value(value) => {
					let function = match value.drop_meta() {
						Node::Symbol(name) if self.lambda_names.contains(name) => format!(".glob {}", quoted(name)),
						_ => self.expression(value)?,
					};
					Ok(format!(".app ({function}) ({})", self.expression(argument)?))
				}
				[call, argument] if self.functions.contains_key(&call.name()) => {
					let argument = match argument.drop_meta() {
						Node::Key(name, Op::Assign, value)
							if self.single_parameters.get(&call.name()) == Some(&name.name()) =>
						{
							value.as_ref()
						}
						Node::Key(name, Op::Assign, _) => {
							return Err(format!("{} has no parameter {}", call.name(), name.name()));
						}
						_ => argument,
					};
					let declared = self.functions[&call.name()].clone();
					let cell = self
						.cell_classes
						.iter()
						.find(|cell| declared == format!(".cls {}", lean_strings(&[cell.to_string()])))
						.cloned();
					let argument = match place_literal(Some(&declared), argument) {
						Some(yes_or_no) => yes_or_no,
						None if cell.is_some() => self.argument(cell.as_deref(), argument)?,
						None => admitted(&declared, self.expression(argument)?),
					};
					Ok(format!(".call {} ({argument})", quoted(&call.name())))
				}
				_ => outside_w0(node),
			},
			// `i++` and `++i` are `i = i + 1`, both giving the new value
			Node::Key(left, op @ (Op::Inc | Op::Dec), right) => {
				match [left, right].into_iter().find(|side| matches!(side.drop_meta(), Node::Symbol(_))) {
					Some(target) => {
						let step = if *op == Op::Inc { Op::Add } else { Op::Sub };
						self.assignment(
							&target.name(),
							&Node::Key(target.clone(), step, Box::new(Node::Number(Number::Int(1)))),
						)
					}
					None => outside_w0(node),
				}
			}
			// `s += 2` is `s = s + 2`
			Node::Key(target, op, value) if op.is_compound_assign() => match target.drop_meta() {
				Node::Symbol(name) => self.assignment(name, &Node::Key(target.clone(), op.base_op(), value.clone())),
				_ => outside_w0(node),
			},
			Node::Key(target, Op::Assign, value) => match target.drop_meta() {
				Node::Symbol(name) => self.assignment(name, value),
				Node::Key(object, Op::Dot, field) => {
					let written = self.expression(value)?;
					Ok(format!(
						".set ({}) {} ({})",
						self.expression(object)?,
						quoted(&field.name()),
						self.admitted_in_field(&field.name(), written)?
					))
				}
				Node::Key(map, Op::Hash, index) if self.map_key(map, index).is_some() => {
					let key = self.map_key(map, index).expect("a map key");
					Ok(format!(
						".setAt ({}) ({}) ({})",
						self.expression(map)?,
						self.expression(key)?,
						self.expression(value)?
					))
				}
				// `xs#i = v` writes into the shared list xs, checked against its element type when it runs
				Node::Key(list, Op::Hash, index) if matches!(list.drop_meta(), Node::Symbol(name) if self.list_names.contains(name) && !self.cell_lists.contains(name)) => {
					Ok(format!(
						".setAt ({}) ({}) ({})",
						self.expression(list)?,
						self.expression(index)?,
						self.expression(value)?
					))
				}
				_ => outside_w0(node),
			},
			// `xs.add(v)` changes the shared list xs holds; a function local's list is a value: `xs = xs ++ [v]`
			Node::Key(list, Op::Dot, call) => match (list.drop_meta(), call.drop_meta()) {
				(Node::Symbol(name), Node::List(items, _, _))
					if items.len() == 2
						&& self.cell_lists.contains(name)
						&& crate::list_mutations::appends(&items[0].drop_meta().name(), 1) =>
				{
					let item = self.expression(&items[1])?;
					Ok(format!(
						".set (.loc {}) {} (.append ({}) (.cons ({item}) .nil))",
						quoted(name),
						quoted(CELL_FIELD),
						self.cell_value(name)
					))
				}
				(Node::Symbol(name), Node::List(items, _, _))
					if items.len() == 2 && crate::list_mutations::appends(&items[0].drop_meta().name(), 1) =>
				{
					let item = self.expression(&items[1])?;
					if !self.list_names.contains(name) {
						return outside_w0(node);
					}
					Ok(format!(".push (.glob {}) ({item})", quoted(name)))
				}
				(Node::Symbol(name), call)
					if self.list_names.contains(name) && crate::list_mutations::insert_arguments(call).is_some() =>
				{
					let (value, position) = crate::list_mutations::insert_arguments(call).expect("guarded");
					Ok(format!(
						".insertAt (.glob {}) ({}) ({})",
						quoted(name),
						self.expression(position)?,
						self.expression(value)?
					))
				}
				(_, Node::Symbol(field)) if self.is_field(field) => {
					Ok(format!(".get ({}) {}", self.expression(list)?, quoted(field)))
				}
				(_, Node::Symbol(method)) if crate::analyzer::is_counting_word(method) => self.tally(list, None, false),
				_ => outside_w0(node),
			},
			Node::Key(if_then, Op::Else, otherwise) => match if_then.drop_meta() {
				Node::Key(condition, Op::Then, then) => match condition.drop_meta() {
					Node::Key(empty, Op::If, condition) if empty.is_nothing() => {
						self.conditional(condition, then, otherwise)
					}
					_ => outside_w0(node),
				},
				_ => outside_w0(node),
			},
			Node::Key(condition, Op::Question, branches) => match branches.drop_meta() {
				Node::Key(then, Op::Colon, otherwise) => self.conditional(condition, then, otherwise),
				_ => outside_w0(node),
			},
			Node::Key(condition, Op::Then, then) => match condition.drop_meta() {
				Node::Key(empty, Op::If, condition) if empty.is_nothing() => {
					Ok(format!(".ite ({}) ({}) .unit", self.expression(condition)?, self.expression(then)?))
				}
				_ => outside_w0(node),
			},
			Node::Key(condition, Op::Do, body) => match condition.drop_meta() {
				Node::Key(empty, Op::While, condition) if empty.is_nothing() => {
					self.binary(".loop", condition, body).map(|lean| format!("{lean} {UNIT_TYPE}"))
				}
				_ => outside_w0(node),
			},
			Node::Key(left, Op::Add, right) if is_list_literal(left) || is_list_literal(right) => {
				self.binary(".append", left, right)
			}
			Node::Key(left, Op::Add, right) if !left.is_nothing() => self.binary(".add", left, right),
			Node::Key(left, Op::Sub, right) if !left.is_nothing() => self.binary(".arith .sub", left, right),
			Node::Key(left, Op::Mul, right) => self.binary(".arith .mul", left, right),
			Node::Key(left, Op::Mod, right) => self.binary(".arith .mod", left, right),
			Node::Key(left, Op::Div, right) => self.binary(".arith .div", left, right),
			Node::Key(left, Op::Pow, right) => self.binary(".arith .pow", left, right),
			Node::Key(left, Op::Lt | Op::Le, right) => self.binary(".lt", left, right),
			Node::Key(left, Op::Gt | Op::Ge, right) => self.binary(".lt", right, left),
			// `c is Color` parses as `c == Color`: a type test
			Node::Key(left, Op::Eq, right) if matches!(right.drop_meta(), Node::Symbol(class) if self.classes.contains_key(class)) => {
				Ok(format!(".isA ({}) {}", self.expression(left)?, quoted(&right.name())))
			}
			Node::Key(left, op @ (Op::Eq | Op::Ne | Op::Identical | Op::NotIdentical), right) => {
				let compared = self.binary(
					if matches!(op, Op::Identical | Op::NotIdentical) { ".eq true" } else { ".eq false" },
					left,
					right,
				)?;
				Ok(if matches!(op, Op::Ne | Op::NotIdentical) { negation(compared) } else { compared })
			}
			Node::Key(map, Op::Hash, index) if self.map_key(map, index).is_some() => {
				self.binary(".index", map, self.map_key(map, index).expect("a map key"))
			}
			Node::Key(list, Op::Hash, index) if !list.is_nothing() => self.binary(".index", list, index),
			Node::Key(empty, Op::Not, operand) if empty.is_nothing() => Ok(negation(self.expression(operand)?)),
			Node::Key(parameter, Op::Arrow, body) => self.lambda(parameter, body),
			Node::Key(value, Op::As, target) => self.conversion(value, target),
			// `a and b` is b when a is truthy, else a; `a or b` the other way round: a, free of effects, evaluated twice
			Node::Key(left, op @ (Op::And | Op::Or), right) if self.is_pure(left) => {
				let (left, right) = (self.expression(left)?, self.expression(right)?);
				Ok(if *op == Op::And {
					format!(".ite ({left}) ({right}) ({left})")
				} else {
					format!(".ite ({left}) ({left}) ({right})")
				})
			}
			// `'a'..'e'`: letters, outside W0
			Node::Key(from, Op::Range | Op::To, to) if [from, to].iter().any(|bound| bound.is_text_literal()) => {
				outside_w0(node)
			}
			Node::Key(from, Op::Range, to) => self.binary(".range", from, to),
			Node::Key(left, Op::Coalesce, right) => self.binary(".orElse", left, right),
			// `1 to 3` includes 3
			Node::Key(from, Op::To, to) => {
				Ok(format!(".range ({}) (.add ({}) (.int 1))", self.expression(from)?, self.expression(to)?))
			}
			_ => outside_w0(node),
		}
	}
}
