//! Text words: text_split, list_join and the joining of items into one text, text_quoted, the decimal digits of an Int
use super::*;

/// How a type declaration of parsed code is written (Node::serialize): `type P {…}`
const TYPE_DECLARATION_WORD: &str = "type";

impl WasmGcEmitter {
	/// text_split(text, separator): the pieces between the separators as a list of texts
	pub(super) fn emit_text_split(&mut self) {
		if !self.should_emit_function("text_split") {
			return;
		}
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let (text, text_type) = (self.text_ref(), self.type_manager.text_type);
		let node_type = self.type_manager.node_type;
		let mut locals = vec![nullable];
		locals.extend([ValType::I32; 8]);
		locals[3] = text;
		locals.push(text);
		self.runtime_function("text_split", vec![node_ref, node_ref], vec![node_ref], locals, |s, f| {
			let (pieces, pointer, end, separator, separator_end, start, index, matched, offset, bytes) =
				(2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
			s.emit_codepoint_as_text(f, 0);
			s.emit_codepoint_as_text(f, 1);
			s.emit_require_kind(f, 0, Kind::Text, "not_a_text");
			s.emit_require_kind(f, 1, Kind::Text, "not_a_text");
			s.emit_text_bounds(f, (bytes, pointer, end));
			s.emit_text_bytes(f, 1);
			Self::emit_list(f, &[I::LocalTee(separator), I::ArrayLen, I::LocalTee(separator_end), I::I32Eqz]);
			s.emit_fail_if(f, "empty_separator");
			Self::emit_list(f, &[I::LocalGet(pointer), I::LocalTee(start), I::LocalSet(index)]);
			// the bytes from `start` up to the local `stop` as a text at the front of `pieces`
			let push_piece = |f: &mut Function, stop: u32| {
				f.instruction(&I::I64Const(SQUARE_LIST_KIND));
				s.emit_text_piece(f, (bytes, start, stop));
				Self::emit_list(f, &[I::LocalGet(pieces), I::StructNew(node_type), I::LocalSet(pieces)]);
			};
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			// stop when the separator no longer fits before the end
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(index), I::LocalGet(separator_end), I::I32Add, I::LocalGet(end), I::I32GtU, I::BrIf(1),
			]);
			// compare the separator with the bytes at index
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I32Const(1), I::LocalSet(matched),
				I32Const(0), I::LocalSet(offset),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(offset), I::LocalGet(separator_end), I::I32GeU, I::BrIf(1),
						I::LocalGet(bytes), I::LocalGet(index), I::LocalGet(offset), I::I32Add, I::ArrayGetU(text_type),
						I::LocalGet(separator), I::LocalGet(offset), I::ArrayGetU(text_type), I::I32Ne,
						I::If(BlockType::Empty),
							I32Const(0), I::LocalSet(matched),
							I::Br(2),
						I::End,
						I::LocalGet(offset), I32Const(1), I::I32Add, I::LocalSet(offset),
						I::Br(0),
				I::End, I::End,
				I::LocalGet(matched), I::If(BlockType::Empty),
			]);
			push_piece(f, index);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(index), I::LocalGet(separator_end), I::I32Add, I::LocalTee(index), I::LocalSet(start),
				I::Else,
					I::LocalGet(index), I32Const(1), I::I32Add, I::LocalSet(index),
				I::End,
				I::Br(0),
				I::End,
				I::End,
			]);
			push_piece(f, end);
			Self::emit_list(f, &[I::LocalGet(pieces)]);
			s.call(f, "list_reverse");
		});
	}

	/// int_to_decimal(position, value): writes the decimal digits of the i64 at `position`, returns how many bytes it wrote
	pub(super) fn emit_int_to_decimal(&mut self) {
		let locals = vec![ValType::I32, ValType::I32, ValType::I64, ValType::I32];
		self.runtime_function(
			"int_to_decimal",
			vec![ValType::I32, ValType::I64],
			vec![ValType::I32],
			locals,
			|_, f| {
				let (position, value, negative, digits, rest, offset) = (0, 1, 2, 3, 4, 5);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(value), I::I64Const(0), I::I64LtS, I::LocalTee(negative), I::If(BlockType::Empty),
						I::LocalGet(position), I32Const(MINUS_SIGN), I::I32Store8(BYTE),
						I::I64Const(0), I::LocalGet(value), I::I64Sub, I::LocalSet(value),
					I::End,
				]);
				// count the digits
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I32Const(1), I::LocalSet(digits),
					I::LocalGet(value), I::LocalSet(rest),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(rest), I::I64Const(DECIMAL_BASE), I::I64LtU, I::BrIf(1),
							I::LocalGet(rest), I::I64Const(DECIMAL_BASE), I::I64DivU, I::LocalSet(rest),
							I::LocalGet(digits), I32Const(1), I::I32Add, I::LocalSet(digits),
							I::Br(0),
					I::End, I::End,
				]);
				// write them from the last one back
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(digits), I::LocalSet(offset),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(offset), I::I32Eqz, I::BrIf(1),
							I::LocalGet(offset), I32Const(1), I::I32Sub, I::LocalSet(offset),
							I::LocalGet(position), I::LocalGet(negative), I::I32Add, I::LocalGet(offset), I::I32Add,
							I::LocalGet(value), I::I64Const(DECIMAL_BASE), I::I64RemU, I::I32WrapI64,
							I32Const(ZERO_DIGIT), I::I32Add, I::I32Store8(BYTE),
							I::LocalGet(value), I::I64Const(DECIMAL_BASE), I::I64DivU, I::LocalSet(value),
							I::Br(0),
					I::End, I::End,
					I::LocalGet(digits), I::LocalGet(negative), I::I32Add,
				]);
			},
		);
	}

	/// list_join(list, separator): the items (texts, ints, ASCII characters) as one text with the separator between them;
	/// list_text(list, separator), the text of a list (`str(xs)`), also takes nested lists, each as "[…]" ("{…}" for a
	/// map), entries as "key:value", symbols as their names and texts quoted (P126: `["a" "b"]`, `p{name:"a"}`)
	pub(super) fn emit_list_join(&mut self) {
		// list_text first: list_join calls it for a structure made at run time (emit_structure_as_literal)
		let joinings = [(LIST_TEXT, true), (LIST_JOIN, false)]
			.into_iter()
			.filter(|(name, _)| self.should_emit_function(name))
			.collect::<Vec<_>>();
		if joinings.is_empty() {
			return;
		}
		// the functions both call, once
		self.emit_text_quoted();
		self.emit_text_heap_global();
		self.emit_int_to_decimal();
		self.emit_exact_text();
		self.emit_float_text(); // after int_to_decimal, which it calls
		self.emit_uncertain_text(); // after float_text and text_concat, which it calls
		self.emit_time_text(); // after int_to_decimal and the times runtime, which it calls
		for (name, nested) in joinings {
			self.emit_joining(name, nested);
		}
	}

	/// text_quoted(text): the text in double quotes, a `"` or `\` in it escaped by a `\` (P126: `["say \"hi\""]`);
	/// one character in single quotes (chars-codepoints: `['a' "bc"]`, a one-character text is a character);
	/// byte-wise, as neither byte occurs inside a multi-byte UTF-8 character
	pub(super) fn emit_text_quoted(&mut self) {
		if !self.should_emit_function(TEXT_QUOTED) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let (text, text_type) = (self.text_ref(), self.type_manager.text_type);
		let mut locals = vec![ValType::I32; 7];
		locals[3] = text;
		locals.push(text);
		self.runtime_function(TEXT_QUOTED, vec![node_ref], vec![node_ref], locals, |s, f| {
			let (pointer, end, capacity, destination, out, byte, quote, bytes) = (1, 2, 3, 4, 5, 6, 7, 8);
			s.emit_text_bounds(f, (bytes, pointer, end));
			Self::emit_list(f, &[I32Const(CHARACTER_QUOTE_BYTE), I32Const(QUOTE_BYTE)]);
			s.emit_is_one_character(f, (bytes, pointer, end), byte);
			// at worst every byte is escaped, plus the two quotes
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Select, I::LocalSet(quote),
				I::LocalGet(end), I32Const(2), I::I32Mul, I32Const(2), I::I32Add, I::ArrayNewDefault(text_type),
				I::LocalSet(destination),
				// from the opening quote at -1 to the closing one at end; a byte written after a backslash that stays
				// only when it escapes: one write site each keeps the machine code small (executable size)
				I32Const(-1), I::LocalSet(pointer),
				I::Loop(BlockType::Empty),
					I::LocalGet(pointer), I32Const(0), I::I32GeS, I::LocalGet(pointer), I::LocalGet(end), I::I32LtS,
					I::I32And, I::LocalTee(capacity), I::If(BlockType::Result(ValType::I32)),
						I::LocalGet(bytes), I::LocalGet(pointer), I::ArrayGetU(text_type),
					I::Else,
						I::LocalGet(quote),
					I::End,
					I::LocalSet(byte),
					I::LocalGet(destination), I::LocalGet(out), I32Const(BACKSLASH_BYTE), I::ArraySet(text_type),
					I::LocalGet(out), I::LocalGet(capacity), I::LocalGet(byte), I::LocalGet(quote), I::I32Eq,
					I::LocalGet(byte), I32Const(BACKSLASH_BYTE), I::I32Eq, I::I32Or, I::I32And, I::I32Add,
					I::LocalTee(out),
					I::LocalGet(destination), I::LocalGet(out), I::LocalGet(byte), I::ArraySet(text_type),
					I32Const(1), I::I32Add, I::LocalSet(out),
					I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalTee(pointer), I::LocalGet(end), I::I32LeS,
					I::BrIf(0),
				I::End,
				I32Const(0), I::LocalSet(capacity),
			]);
			s.emit_text_piece(f, (destination, capacity, out));
		});
	}

	/// The table list_text joins an entry by: the written texts (operators::written_operator) of the operators up to
	/// the bound the program's Keys need, then `:` for any code above, in one run of bytes, and for each its offset in
	/// the run and length as two bytes; returns (run, table). A hello world carries `:` alone (web::test_bundle_budget)
	pub(super) fn allocate_operator_texts(&mut self) -> (u32, u32) {
		let bound = self.written_operator_bound() as usize;
		let texts: Vec<String> = (0..=bound)
			.map(crate::operators::written_operator)
			.chain([crate::operators::Op::Colon.as_str().to_string()])
			.collect();
		let mut table = vec![];
		let mut offset = 0;
		for text in &texts {
			table.extend([u8::try_from(offset).expect("operator texts within 255 bytes"), text.len() as u8]);
			offset += text.len();
		}
		(
			self.allocate_bytes("operator_texts", texts.concat().as_bytes()),
			self.allocate_bytes("operator_table", &table),
		)
	}

	/// The text chosen by the info bits `(shift, mask)` of the kind in `kind_local`: a chain of ifs over `choices`
	fn emit_text_of_info(
		&self,
		f: &mut Function,
		kind_local: u32,
		(shift, mask): (i64, i64),
		choices: &[(i64, (u32, u32))],
		otherwise: (u32, u32),
	) {
		let new_text = |f: &mut Function, (pointer, length): (u32, u32)| {
			Self::emit_list(f, &[I32Const(pointer as i32), I32Const(length as i32)]);
			self.call(f, "new_text");
		};
		for (info, text) in choices {
			Self::emit_list(
				f,
				&[I::LocalGet(kind_local), I::I64Const(shift), I::I64ShrU, I::I64Const(mask), I::I64And],
			);
			Self::emit_list(f, &[I::I64Const(*info), I::I64Eq, I::If(BlockType::Result(Ref(self.node_ref(false))))]);
			new_text(f, *text);
			f.instruction(&I::Else);
		}
		new_text(f, otherwise);
		for _ in choices {
			f.instruction(&I::End);
		}
	}

	pub(super) fn emit_joining(&mut self, name: &'static str, nested: bool) {
		let float_box = self.type_manager.f64_box_type;
		let exact_numbers = self.should_emit_function(crate::wasm_emitter::numbers::exact::EXACT_TEXT);
		// a nested list keeps its written brackets and separator (card print-shows): `(a; b)`, `[a, b]`, `{a; b}`
		let bracket_texts = [Bracket::Curly, Bracket::Square, Bracket::Round, Bracket::Less].map(|bracket| {
			let info = crate::wasm_emitter::bracket_info(&bracket);
			let open = self.memory_string(&bracket.opening().to_string());
			(info, open, self.memory_string(&bracket.closing().to_string()))
		});
		let no_bracket = self.memory_string("");
		let separator_texts = [Separator::Comma, Separator::Semicolon, Separator::Newline, Separator::Tab]
			.map(|separator| (crate::wasm_emitter::separator_code(&separator), self.memory_string(&separator.joint())));
		let space = self.memory_string(" ");
		// only a program whose Keys hold operators beyond `:` carries their texts (web::test_bundle_budget)
		let operator_texts =
			(self.written_operator_bound() > op_to_code(&Op::Colon)).then(|| self.allocate_operator_texts());
		let instance_texts = [self.memory_string(""), self.memory_string(Op::Colon.as_str())];
		let empty_text = self.memory_string(EMPTY_TEXT);
		let type_word = self.memory_string(TYPE_DECLARATION_WORD);
		// the unbracketed, space-separated list that line is joined as
		let type_line_kind =
			(crate::wasm_emitter::list_info(&Bracket::None, &Separator::Space) << KIND_BITS) | Kind::List as i64;
		let empty = self.memory_string("");
		let bool_texts = [self.memory_string(crate::node::NO), self.memory_string(crate::node::YES)];
		let i64_box = self.type_manager.i64_box_type;
		let own_index = self.next_func_idx; // list_text joins a nested list by calling itself
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let mut locals = vec![nullable, nullable];
		locals.extend([ValType::I32; 4]);
		locals.extend([ValType::I64, nullable, ValType::I32, ValType::I64]);
		let (text, text_type) = (self.text_ref(), self.type_manager.text_type);
		locals.extend([text, text]);
		let trap_detail = self.trap_detail_global(); // the item that cannot join, for its error to name
		let radix_texts = self.radix_text_calls();
		let text_witness = self.text_witness_table();
		let type_value_prefix = self.string_table.literal(crate::type_kinds::TYPE_VALUE_PREFIX);
		self.runtime_function(name, vec![nullable, node_ref], vec![node_ref], locals, |s, f| {
			let (cell, element) = (2, 3);
			let (bound, address, position, is_first, number, entry_value, operator_entry, list_kind) =
				(4, 5, 6, 7, 8, 9, 10, 11);
			let (joined, piece) = (12, 13);
			let is_kind = |f: &mut Function, kind: Kind| {
				s.emit_field(f, element, 0);
				Self::emit_list(f, &[I::I64Const(kind as i64), I::I64Eq]);
			};
			let masked_kind_is = |f: &mut Function, kind: Kind| {
				s.emit_field(f, element, 0);
				Self::emit_kind_test(f, kind as i64, I::I64Eq);
			};
			let next_element = |f: &mut Function| {
				s.emit_field(f, cell, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node_type)), I::LocalSet(element)]);
				s.emit_codepoint_as_text(f, element); // a character joins as its UTF-8 bytes
				if !nested {
					// code parsed at run time joins as its literal: `print "area:", parse(t)` (card print-parse); a list
					// stays no joinable item of join, as Python's ",".join([[1]])
					s.emit_structure_as_literal(f, element, &[Kind::Key, Kind::TypeDef], empty);
				}
				if nested {
					s.emit_without_code_mark(f, element);
					// a text or a character in a container is quoted (P126, chars-codepoints): `['a' "bc"]`
					s.emit_field(f, element, 0);
					Self::emit_kind_test(f, Kind::Text as i64, I::I64Eq);
					Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(element), I::RefAsNonNull]);
					s.call(f, TEXT_QUOTED);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				// a bool joins as true or false (card bool-type)
				s.emit_field(f, element, 0);
				Self::emit_list(f, &[I::I64Const(crate::type_kinds::BOOL_KIND), I::I64Eq, I::If(BlockType::Empty)]);
				s.emit_field(f, element, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefCastNonNull(HeapType::Concrete(i64_box)),
					I::StructGet { struct_type_index: i64_box, field_index: 0 }, I::I64Eqz,
					I::If(BlockType::Result(node_ref)),
						I32Const(bool_texts[0].0 as i32), I32Const(bool_texts[0].1 as i32),
				]);
				s.call(f, "new_text");
				Self::emit_list(f, &[I::Else, I32Const(bool_texts[1].0 as i32), I32Const(bool_texts[1].1 as i32)]);
				s.call(f, "new_text");
				Self::emit_list(f, &[I::End, I::LocalSet(element), I::End]);
				// an Int shown in hex joins as 0xFF (card hex-printing), in binary as 0b101 (card binary-int)
				for (radix, late) in &radix_texts {
					s.emit_field(f, element, 0);
					Self::emit_list(f, &[I::I64Const(radix.kind()), I::I64Eq, I::If(BlockType::Empty)]);
					s.emit_field(f, element, 1);
					Self::emit_list(
						f,
						&[
							I::RefCastNonNull(HeapType::Concrete(i64_box)),
							I::StructGet { struct_type_index: i64_box, field_index: 0 },
						],
					);
					Self::emit_late_call(f, late);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				// an Error joins as its message (card catch-message): `print e`
				is_kind(f, Kind::Error);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(Kind::Text as i64)]);
				s.emit_field(f, element, 1);
				Self::emit_list(
					f,
					&[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::LocalSet(element), I::End],
				);
				// an instance of a type with text() joins as that text, unquoted (card list-printable-items)
				if let Some(table) = text_witness {
					s.emit_instance_as_text(f, table, element);
				}
				// list_text: a nested list as its literal, its bracket + list_text(item, its separator) + its bracket
				if nested {
					let new_text = |f: &mut Function, (pointer, length): (u32, u32)| {
						Self::emit_list(f, &[I32Const(pointer as i32), I32Const(length as i32)]);
						s.call(f, "new_text");
					};
					// a type value as `class int` (emit_type_value_text)
					s.emit_is_type_value(f, element);
					f.instruction(&I::If(BlockType::Empty));
					s.emit_type_value_text(f, element, type_value_prefix);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
					// a symbol (a map's key) as its name
					is_kind(f, Kind::Symbol);
					Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(Kind::Text as i64)]);
					s.emit_field(f, element, 1);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::LocalSet(element),
						I::End,
					]);
					// an entry key:value as list_text([key value], ":"), ø for a missing value
					masked_kind_is(f, Kind::Key);
					Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(SQUARE_LIST_KIND)]);
					s.emit_field(f, element, 1);
					Self::emit_list(f, &[I::I64Const(SQUARE_LIST_KIND)]);
					s.emit_field(f, element, 2);
					Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Result(node_ref))]);
					s.call(f, "new_empty");
					f.instruction(&I::Else);
					s.emit_field(f, element, 2);
					f.instruction(&I::LocalSet(entry_value));
					s.emit_entry_in_braces(f, entry_value);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::End,
						I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::StructNew(node_type),
					]);
					// joined by its operator as written: `x+1`, an instance `point{x:1}` without one (P123)
					s.emit_field(f, element, 0);
					Self::emit_list(f, &[I::I64Const(KIND_BITS), I::I64ShrU, I::I64Const(OP_INFO_MASK), I::I64And]);
					if let Some((run, table)) = operator_texts {
						let last_code = I32Const(s.written_operator_bound() as i32 + 1);
						Self::emit_list(
							f,
							&[
								I::I32WrapI64,
								I::LocalTee(operator_entry),
								last_code.clone(),
								I::LocalGet(operator_entry),
								last_code,
								I::I32LtU,
								I::Select,
								I32Const(OPERATOR_ENTRY_BYTES),
								I::I32Mul,
								I32Const(table as i32),
								I::I32Add,
								I::LocalTee(operator_entry),
								I::I32Load8U(BYTE),
								I32Const(run as i32),
								I::I32Add,
								I::LocalGet(operator_entry),
								I::I32Load8U(MemArg { offset: 1, ..BYTE }),
							],
						);
						s.call(f, "new_text");
					} else {
						let [none, colon] = instance_texts;
						Self::emit_list(
							f,
							&[I::I64Const(op_to_code(&Op::None)), I::I64Eq, I::If(BlockType::Result(node_ref))],
						);
						new_text(f, none);
						f.instruction(&I::Else);
						new_text(f, colon);
						f.instruction(&I::End);
					}
					Self::emit_list(f, &[I::Call(own_index), I::LocalSet(element), I::End]);
					// a type declaration of parsed code as Node::serialize writes it, the line `type name body`
					is_kind(f, Kind::TypeDef);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::If(BlockType::Empty),
							I::I64Const(type_line_kind), I32Const(type_word.0 as i32), I32Const(type_word.1 as i32),
					]);
					s.call(f, "new_symbol");
					f.instruction(&I::I64Const(type_line_kind));
					s.emit_field(f, element, 1);
					f.instruction(&I::I64Const(type_line_kind));
					s.emit_field(f, element, 2);
					#[rustfmt::skip]
					Self::emit_list(f, &[
							I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type), I::StructNew(node_type),
							I::StructNew(node_type), I::LocalSet(element),
						I::End,
					]);
					// the bracket info sits above the kind, the separator code higher still (card keep-list)
					masked_kind_is(f, Kind::List);
					f.instruction(&I::If(BlockType::Empty));
					s.emit_field(f, element, 0);
					f.instruction(&I::LocalSet(list_kind));
					let opening = bracket_texts.map(|(info, open, _)| (info, open));
					let closing = bracket_texts.map(|(info, _, close)| (info, close));
					let bracket_bits = (KIND_BITS, BRACKET_INFO_MASK);
					let separator_bits = (SEPARATOR_SHIFT, SEPARATOR_MASK >> SEPARATOR_SHIFT);
					s.emit_text_of_info(f, list_kind, bracket_bits, &opening, no_bracket);
					f.instruction(&I::LocalGet(element));
					s.emit_text_of_info(f, list_kind, separator_bits, &separator_texts, space);
					f.instruction(&I::Call(own_index));
					s.call(f, super::super::super::text::text_builtins::TEXT_CONCAT);
					s.emit_text_of_info(f, list_kind, bracket_bits, &closing, no_bracket);
					s.call(f, super::super::super::text::text_builtins::TEXT_CONCAT);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				// a ø item reads as ø, as Node::serialize writes it: also an empty map at the top (card print-empty-map)
				is_kind(f, Kind::Empty);
				Self::emit_list(
					f,
					&[I::If(BlockType::Empty), I32Const(empty_text.0 as i32), I32Const(empty_text.1 as i32)],
				);
				s.call(f, "new_text");
				Self::emit_list(f, &[I::LocalSet(element), I::End]);
				if s.prints_uncertain() {
					is_kind(f, Kind::Uncertain);
					Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(element), I::RefAsNonNull]);
					s.call(f, super::super::super::numbers::uncertain::UNCERTAIN_TEXT);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				if s.shows_functions() {
					masked_kind_is(f, Kind::Function); // its class in the info bits
					Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(element), I::RefAsNonNull]);
					s.call(f, super::super::super::functions::closures::FUNCTION_TEXT);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				if s.should_emit_function(super::super::super::time::times::TIME_TEXT) {
					masked_kind_is(f, Kind::Time); // the form in the info bits
					Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(element), I::RefAsNonNull]);
					s.call(f, super::super::super::time::times::TIME_TEXT);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				if s.should_emit_function(super::super::super::numbers::float_text::FLOAT_TEXT) {
					is_kind(f, Kind::Float);
					f.instruction(&I::If(BlockType::Empty));
					s.emit_field(f, element, 1);
					Self::emit_list(
						f,
						&[
							I::RefCastNonNull(HeapType::Concrete(float_box)),
							I::StructGet { struct_type_index: float_box, field_index: 0 },
						],
					);
					s.call(f, super::super::super::numbers::float_text::FLOAT_TEXT);
					Self::emit_list(f, &[I::LocalSet(element), I::End]);
				}
				if exact_numbers {
					// a big integer or a ratio joins as its exact text, a fixnum by int_to_decimal below
					is_kind(f, Kind::Int);
					f.instruction(&I::If(BlockType::Empty));
					s.emit_int_value_of(f, element);
					f.instruction(&I::LocalSet(number));
					s.emit_fixnum_test(f, &[number]);
					Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(number)]);
					s.call(f, crate::wasm_emitter::numbers::exact::EXACT_TEXT);
					Self::emit_list(f, &[I::LocalSet(element), I::End, I::End]);
				}
			};
			let loop_over_cells = |f: &mut Function| {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(0), I::LocalSet(cell),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(cell), I::RefIsNull, I::BrIf(1),
				]);
				next_element(f);
			};
			let end_loop = |f: &mut Function| {
				s.emit_field(f, cell, 2);
				Self::emit_list(f, &[I::LocalSet(cell), I::Br(0), I::End, I::End]);
			};
			// append the $Text on the stack to joined at position; position += its length
			let append_bytes = |f: &mut Function| {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(piece),
					I::LocalGet(joined), I::LocalGet(position), I::LocalGet(piece), I32Const(0), I::LocalGet(piece),
					I::ArrayLen, I::ArrayCopy { array_type_index_dst: text_type, array_type_index_src: text_type },
					I::LocalGet(position), I::LocalGet(piece), I::ArrayLen, I::I32Add, I::LocalSet(position),
				]);
			};
			let append_text = |f: &mut Function, text: u32| {
				s.emit_text_bytes(f, text);
				append_bytes(f);
			};
			s.emit_empty_as_null(f, 0);
			s.emit_require_list(f);
			s.emit_codepoint_as_text(f, 1);
			s.emit_require_kind(f, 1, Kind::Text, "not_a_text");
			// pass 1: an upper bound of the bytes: every item, and a separator after it
			loop_over_cells(f);
			is_kind(f, Kind::Text);
			f.instruction(&I::If(BlockType::Result(ValType::I32)));
			s.emit_text_field(f, element, 1);
			f.instruction(&I::Else);
			is_kind(f, Kind::Int);
			Self::emit_list(f, &[I::If(BlockType::Result(ValType::I32)), I32Const(MAX_INT_BYTES), I::Else]);
			Self::emit_list(f, &[I::LocalGet(element), I::GlobalSet(trap_detail)]);
			s.call(f, super::super::super::lists::list_ops::NOT_JOINABLE);
			Self::emit_list(f, &[I32Const(0), I::End, I::End, I::LocalGet(bound), I::I32Add]);
			s.emit_text_field(f, 1, 1);
			Self::emit_list(f, &[I::I32Add, I::LocalSet(bound)]);
			end_loop(f);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(bound), I::ArrayNewDefault(text_type), I::LocalSet(joined),
				I32Const(0), I::LocalSet(position), I32Const(1), I::LocalSet(is_first),
				// the decimal digits of an Int go through memory: int_to_decimal writes there
				I32Const(MAX_INT_BYTES), I::LocalSet(bound),
			]);
			s.emit_text_allocation(f, bound, address);
			// pass 2: write
			loop_over_cells(f);
			Self::emit_list(f, &[I::LocalGet(is_first), I::I32Eqz, I::If(BlockType::Empty)]);
			append_text(f, 1);
			Self::emit_list(f, &[I::End, I32Const(0), I::LocalSet(is_first)]);
			// the bytes of a text, or of an Int's digits
			is_kind(f, Kind::Text);
			f.instruction(&I::If(BlockType::Result(s.text_ref())));
			s.emit_text_bytes(f, element);
			Self::emit_list(f, &[I::Else, I::LocalGet(address), I::LocalGet(address)]);
			s.emit_int_value_of(f, element);
			s.call(f, "int_to_decimal");
			s.call(f, crate::wasm_emitter::TEXT_FROM_MEMORY);
			f.instruction(&I::End);
			append_bytes(f);
			end_loop(f);
			Self::emit_list(f, &[I32Const(0), I::LocalSet(is_first)]);
			s.emit_text_piece(f, (joined, is_first, position));
		});
	}
}
