//! Casts: as int/float/text/char/bool/number/exact, list and run-time text casts

use super::*;

impl WasmGcEmitter {
	/// Text converts only if it is a number literal (truncated for int); anything else is a runtime error, never a plausible 0
	/// `'7' as int` is 7; `'x' as int` is no number (invalid_number, which `try` catches), hinted toward `codepoint('x')`
	pub(super) fn emit_character_cast(&mut self, func: &mut Function, character: char, target_type: &Node) {
		if !character.is_ascii_digit() {
			let (target, codepoint) = (target_type.name(), crate::library_words::CODEPOINT);
			crate::normalize::hint(&format!("'{character}' as {target}"), &format!("{codepoint}('{character}') as {target}"), "a character that is no digit is no number; codepoint gives its code point");
		}
		self.emit_text_cast(func, &character.to_string(), target_type);
	}

	/// `reverse(xs)`, `codepoint(c)`: a call of a library word the program does not define itself
	pub(super) fn is_library_word_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		crate::analyzer::call_name(items, bracket, separator)
			.is_some_and(|name| library_ops::LIBRARY_FUNCTIONS.iter().any(|(word, _)| *word == name) && !self.ctx.user_functions.contains_key(name))
	}

	pub(super) fn emit_text_cast(&mut self, func: &mut Function, text: &str, target_type: &Node) {
		match crate::wasp_parser::number_in_text(text) {
			Some(number) => self.emit_cast(func, &Node::Number(number), target_type),
			None => self.emit_runtime_error(func, "invalid_number"),
		}
	}

	/// An f64 has no exact value yet: `x as exact` of a runtime float is refused instead of rounded silently
	pub(super) fn emit_inexact_to_exact(&mut self, func: &mut Function, value: &Node) {
		let message = format!("{} is an IEEE float, `as exact` of a float is not supported yet: keep it exact from the start", value.serialize());
		self.emit_type_error(func, message);
	}

	/// `v as T` as a raw Int: `as float` has no exact value, `as exact` keeps it
	/// A cast that makes no sense: a list as a number or character, a number as a list
	pub(super) fn cast_refusal(&self, value: &Node, target: &Node) -> Option<String> {
		let target_name = target.name().to_lowercase();
		let kind = self.get_type(value);
		let text_like = matches!(target_name.as_str(), "string" | "str" | "text");
		let refused = match kind {
			Kind::List => target_name != "list" && !text_like,
			Kind::Int | Kind::Float => target_name == "list",
			_ => false,
		};
		refused.then(|| format!("cannot cast {kind} to {target_name}: {} as {target_name}", value.serialize()))
	}

	pub(super) fn emit_numeric_cast(&mut self, func: &mut Function, located: &Node, value: &Node, target: &Node) {
		if let Some(refusal) = self.cast_refusal(value, target) {
			self.emit_type_error(func, refusal);
			return;
		}
		let exact_value = !matches!(value.drop_meta(), Node::Text(_) | Node::Char(_)) && !self.get_type(value).is_float();
		match crate::type_kinds::canonical_type_name(&target.name().to_lowercase()) {
			"float" => {
				let message = format!("{} is a float where an exact Int is expected: `as float` promotes, `as int` truncates", located.serialize());
				self.emit_type_error(func, message);
			}
			"exact" if exact_value => self.emit_numeric_value(func, value),
			// a character held unboxed is its code point
			"char" | "character" => match value.drop_meta() {
				Node::Char(character) => {
					func.instruction(&I::I64Const(*character as i64));
				}
				_ => self.emit_numeric_value(func, value),
			},
			_ if crate::analyzer::builtin_type_kind(&target.name()) == Some(Kind::Int) => {
				self.emit_cast(func, value, target);
				self.emit_call(func, "get_int_value");
			}
			_ => self.emit_not_a_number(func, located, located.drop_meta()),
		}
	}

	/// `text as list` is its characters, a list stays as it is; the characters of a text known only at runtime need a runtime splitter
	pub(super) fn emit_list_cast(&mut self, func: &mut Function, value: &Node) {
		match value {
			Node::Text(text) => {
				let characters = text.chars().map(Node::Char).collect();
				self.emit_node_instructions(func, &Node::List(characters, Bracket::Square, Separator::Space));
			}
			Node::Char(_) => self.emit_node_instructions(func, &Node::List(vec![value.clone()], Bracket::Square, Separator::Space)),
			_ if self.get_type(value) == Kind::List => self.emit_node_instructions(func, value),
			_ if matches!(self.get_type(value), Kind::Text | Kind::Codepoint) => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, "text_chars");
			}
			_ => {
				let kind = self.get_type(value);
				self.emit_type_error(func, format!("cannot cast {kind} to list: {} as list", value.serialize()));
			}
		}
	}

	/// `x as string` for a variable: an Int, a Text or a character is the join of the one-element list;
	/// anything else has no runtime text yet
	pub(super) fn emit_runtime_text_cast(&mut self, func: &mut Function, value: &Node) {
		let kind = self.get_type(value);
		let node = match kind {
			Kind::Int | Kind::Float | Kind::Text | Kind::Codepoint => joined_text(std::slice::from_ref(value), ""),
			// user decision #35: an int list joins to "[1 2]", like its literal; a general runtime serializer comes later
			// a list known only at run time (an element, a parsed value) too: "[1 2]", nested lists as their literals;
			// an instance as its literal "point{x:1 y:2}" (P123), an entry "a:1"
			// and a value of a kind known only at run time (`p.dist / 2` of an any-typed field, card text-arithmetic)
			Kind::List | Kind::Empty | Kind::Key | Kind::Data => {
				self.emit_dynamic_text(func, value);
				return;
			}
			// card try-raise: an Error (a caught one, `catch e`) reads as its message
			Kind::Error => {
				self.emit_error_message(func, value);
				return;
			}
			_ => {
				self.emit_type_error(func, format!("cannot cast {kind} to string: `{} as string` has no runtime text yet", value.serialize()));
				return;
			}
		};
		self.emit_node_instructions(func, &node);
	}

	/// The message of an Error as a Text: error_of builds an Error as a Text is built, its $String in the data field
	fn emit_error_message(&mut self, func: &mut Function, error: &Node) {
		let (held, node_type) = (self.node_scratch(), self.type_manager.node_type);
		self.emit_node_instructions(func, error);
		Self::emit_list(func, &[
			I::LocalSet(held), I::I64Const(Kind::Text as i64),
			I::LocalGet(held), I::StructGet { struct_type_index: node_type, field_index: 1 },
			I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type),
		]);
	}

	/// The text of a Node of unknown kind, as list_text writes the one item of a list: "[1 2]" for a list, "{a:1 b:2}" for
	/// a map, "{a:1}" for an entry (the one-entry map), the text itself for a text; gives the local still holding the node
	pub(super) fn emit_dynamic_text(&mut self, func: &mut Function, value: &Node) -> u32 {
		let (held, node_type) = (self.node_scratch(), self.type_manager.node_type);
		self.emit_node_instructions(func, value);
		func.instruction(&I::LocalSet(held));
		// a text is itself and a character its text; only texts inside a container are quoted (P126)
		let kind_is = |kind: Kind| [I::LocalGet(held), I::StructGet { struct_type_index: node_type, field_index: 0 }, I::I64Const(crate::type_kinds::KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq];
		Self::emit_list(func, &kind_is(Kind::Text));
		Self::emit_list(func, &kind_is(Kind::Codepoint));
		Self::emit_list(func, &[I::I32Or, I::If(BlockType::Result(Ref(self.node_ref(false)))), I::LocalGet(held), I::RefAsNonNull]);
		self.emit_call(func, text_builtins::TEXT_OF);
		func.instruction(&I::Else);
		func.instruction(&I::I64Const(crate::type_kinds::SQUARE_LIST_KIND));
		self.emit_entry_in_braces(func, held);
		Self::emit_list(func, &[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type)]);
		let (pointer, length) = self.allocate_string("");
		Self::emit_list(func, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
		self.emit_call(func, "new_text");
		self.emit_call(func, library_ops::LIST_TEXT);
		func.instruction(&I::End);
		held
	}

	/// Emit type cast: value as type
	/// Handles conversions between int, float, string
	/// Optimizes literal conversions at compile time
	pub(super) fn emit_cast(&mut self, func: &mut Function, value: &Node, target_type: &Node) {
		let type_name = match target_type.drop_meta() {
			Node::Symbol(s) => s.to_lowercase(),
			Node::Text(s) => s.to_lowercase(),
			_ => {
				// Unknown type, emit as-is
				self.emit_node_instructions(func, value);
				return;
			}
		};

		let value = value.drop_meta();
		// `cube 3 as int`, the body of `def g() -> int { cube 3 }`: an unknown word is an error as without the cast
		let unknown_word = match value {
			Node::List(items, bracket, separator) => self.unknown_word_error(items, bracket, separator),
			_ => None,
		};
		if let Some(refusal) = unknown_word.or_else(|| self.cast_refusal(value, target_type)) {
			self.emit_type_error(func, refusal);
			return;
		}

		match crate::type_kinds::canonical_type_name(&type_name) {
			"list" => self.emit_list_cast(func, value),
			"i64" | "int64" if !self.get_type(value).is_float() && !matches!(value, Node::Text(_) | Node::Char(_)) => {
				self.emit_wrapping_int(func, value);
				self.emit_call(func, "new_int");
			}
			"int" | "integer" | "i32" | "i64" | "long" => self.emit_cast_to_int(func, value, target_type),
			"exact" => self.emit_cast_to_exact(func, value, target_type),
			"float" | "f32" => self.emit_cast_to_float(func, value, target_type),
			"string" | "str" | "text" => self.emit_cast_to_text(func, value),
			"char" | "character" => self.emit_cast_to_char(func, value),
			"bool" | "boolean" => self.emit_cast_to_bool(func, value),
			"number" | "num" => self.emit_cast_to_number(func, value),
			_ => {
				// Unknown type, emit as key node for dynamic dispatch
				self.emit_node_instructions(func, value);
				self.emit_node_instructions(func, target_type);
				func.instruction(&I::I64Const(op_to_code(&Op::As)));
				self.emit_call(func, "new_key");
			}
		}
	}

	/// An Int node of a value known at compile time
	pub(super) fn emit_int_node(&mut self, func: &mut Function, value: i64) {
		func.instruction(&I::I64Const(value));
		self.emit_call(func, "new_int");
	}

	pub(super) fn emit_float_node(&mut self, func: &mut Function, value: f64) {
		func.instruction(&I::F64Const(value.into()));
		self.emit_call(func, "new_float");
	}

	/// `x as int`: a float truncated, a text parsed, a ratio truncated
	pub(super) fn emit_cast_to_int(&mut self, func: &mut Function, value: &Node, target_type: &Node) {
		match value {
			Node::Number(Number::Float(f)) => self.emit_int_node(func, *f as i64),
			Node::Text(s) => self.emit_text_cast(func, s, target_type),
			// a character is a one-character text: its digit, else invalid_number; its code point is codepoint(c)
			Node::Char(c) => self.emit_character_cast(func, *c, target_type),
			_ if self.get_type(value).is_float() => {
				self.emit_float_value(func, value);
				self.emit_truncating_cast(func);
				self.emit_call(func, "new_int");
			}
			// a text, or a value known only at run time (a list element), parses its digits
			_ if matches!(self.get_type(value), Kind::Text | Kind::Codepoint | Kind::Empty) => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, list_ops::TEXT_AS_INT);
				self.emit_call(func, "new_int");
			}
			// Already int or coercible; a ratio is truncated
			_ => {
				self.emit_int_value_truncated(func, value);
				self.emit_call(func, "new_int");
			}
		}
	}

	/// The i64 of an Int value; a ratio (`x/2` is exact) is truncated
	pub(super) fn emit_int_value_truncated(&mut self, func: &mut Function, value: &Node) {
		self.emit_numeric_value(func, value);
		if self.int_runtime() && !big_int::is_fixnum_range(self.int_range(value)) {
			self.emit_call(func, "exact_trunc");
		}
	}

	/// `x as exact`: decimal literals are ratios already (exact.rs), a float becomes the ratio it is
	pub(super) fn emit_cast_to_exact(&mut self, func: &mut Function, value: &Node, target_type: &Node) {
		match value {
			Node::Text(s) => self.emit_text_cast(func, s, target_type),
			Node::Char(_) => self.emit_cast(func, value, &Node::Symbol("int".into())),
			_ if self.get_type(value).is_float() => self.emit_inexact_to_exact(func, value),
			_ => {
				self.emit_numeric_value(func, value);
				self.emit_call(func, "new_int");
			}
		}
	}

	/// `x as float`
	pub(super) fn emit_cast_to_float(&mut self, func: &mut Function, value: &Node, target_type: &Node) {
		match value {
			Node::Text(s) => self.emit_text_cast(func, s, target_type),
			Node::Char(c) => self.emit_character_cast(func, *c, target_type),
			Node::Number(Number::Int(n)) => self.emit_float_node(func, *n as f64),
			// a text, or a value known only at run time, parses its digits
			_ if matches!(self.get_type(value), Kind::Text | Kind::Empty) => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, list_ops::TEXT_AS_FLOAT);
				self.emit_call(func, "new_float");
			}
			// `codepoint(c) as float`: a library word's number result (a text one parses above)
			Node::List(items, bracket, separator) if self.is_library_word_call(items, bracket, separator) => {
				self.emit_node_as_f64(func, value);
				self.emit_call(func, "new_float");
			}
			_ => {
				self.emit_float_value(func, value);
				self.emit_call(func, "new_float");
			}
		}
	}

	/// `x as text`, `str(x)`
	pub(super) fn emit_cast_to_text(&mut self, func: &mut Function, value: &Node) {
		match value {
			Node::Number(n) => self.emit_string_call(func, &n.to_string(), "new_text"),
			Node::Char(c) => self.emit_string_call(func, &c.to_string(), "new_text"),
			Node::Text(s) => self.emit_string_call(func, s, "new_text"),
			// a list of numbers and texts reads as it prints, its texts quoted (P126)
			Node::List(..) if is_plain_data(value) => self.emit_runtime_text_cast(func, value),
			// data and names are their source text; a number expression (`str(1+2)`, `str(f(1))` of a float f) is the
			// text of its value
			_ if !self.mentions_variable(value) && !self.mentions_call(value) && !matches!(self.get_type(value), Kind::Int | Kind::Float) => {
				self.emit_string_call(func, &value.serialize(), "new_text");
			}
			_ => self.emit_runtime_text_cast(func, value),
		}
	}

	/// `x as char`: a digit 0…9 is its character, another number its code point
	pub(super) fn emit_cast_to_char(&mut self, func: &mut Function, value: &Node) {
		match value {
			Node::Number(Number::Int(n)) => {
				let c = if (0..=9).contains(n) { char::from_digit(*n as u32, 10) } else { char::from_u32(*n as u32) };
				func.instruction(&I32Const(c.unwrap_or('?') as i32));
			}
			Node::Char(c) => {
				func.instruction(&I32Const(*c as i32));
			}
			_ => {
				self.emit_numeric_value(func, value);
				func.instruction(&I::I32WrapI64);
			}
		}
		self.emit_call(func, "new_codepoint");
	}

	/// `x as bool`: a literal's truth known at compile time, else non-zero is true
	pub(super) fn emit_cast_to_bool(&mut self, func: &mut Function, value: &Node) {
		let literal_truth = match value {
			Node::Text(s) => Some(!FALSY_TEXTS.contains(&s.to_lowercase().as_str())),
			Node::Char(c) => Some(!matches!(*c, '0' | 'ø')),
			Node::Number(Number::Int(n)) => Some(*n != 0),
			Node::Number(Number::Float(f)) => Some(*f != 0.0),
			Node::True => Some(true),
			Node::False => Some(false),
			_ => None,
		};
		match literal_truth {
			Some(truth) => self.emit_int_node(func, truth as i64),
			None => {
				self.emit_numeric_value(func, value);
				func.instruction(&I::I64Eqz);
				func.instruction(&I::I64ExtendI32U);
				func.instruction(&I::I64Const(1));
				func.instruction(&I::I64Xor);
				self.emit_call(func, "new_int");
			}
		}
	}

	/// `x as number`: a text parses as an Int, else as a float (0 when neither), a character is its digit or code point
	pub(super) fn emit_cast_to_number(&mut self, func: &mut Function, value: &Node) {
		match value {
			Node::Text(s) => match (s.parse::<i64>(), s.parse::<f64>()) {
				(Ok(n), _) => self.emit_int_node(func, n),
				(_, Ok(f)) => self.emit_float_node(func, f),
				_ => self.emit_int_node(func, 0),
			},
			Node::Char(c) => self.emit_int_node(func, c.to_digit(10).map_or(*c as i64, |digit| digit as i64)),
			_ => self.emit_node_instructions(func, value),
		}
	}

	/// Extract numeric value from a block { expr } or plain expr
	pub(super) fn emit_block_value(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::List(items, Bracket::Curly, _) if items.len() == 1 => {
				// Block with single item: { expr } -> extract expr
				self.emit_numeric_value(func, &items[0]);
			}
			_ => self.emit_numeric_value(func, node),
		}
	}

	/// Emit the numeric value of a node onto the stack (as i64)
	/// `print x`, `puti x`: an output builtin (not shadowed by a user function), whose value is the printed node
	pub(super) fn is_output_call(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, _]
			if matches!(word.drop_meta(), Node::Symbol(name) if OUTPUT_CALLS.contains(&name.as_str()) && !self.ctx.user_functions.contains_key(name))))
	}
}

/// A list whose items are numbers, texts, characters or such lists
fn is_plain_data(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) => true,
		Node::List(items, Bracket::Square, _) => items.iter().all(is_plain_data),
		_ => false,
	}
}
