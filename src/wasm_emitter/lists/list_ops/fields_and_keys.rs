//! Fields and keys: `x.name`, `m[key]` with a key known only at runtime, the map words
use super::*;
use crate::node::Bracket;
use crate::words::MAP_KEYS;

/// The runtime error of a missing constant field is a function `no_field_<name>`; eval reports it as `no field <name>`
pub const NO_FIELD_PREFIX: &str = "no_field_";
/// How a missing field's error begins: `no field title`
pub const NO_FIELD_MESSAGE: &str = "no field ";
/// The runtime error of a missing field whose name is known only at run time (lib/compiler/runtime.warp map_get):
/// the trap detail holds the name
pub const MISSING_FIELD: &str = "missing_field";
/// struct_body(node): the field list of an instance of a declared type, else the node itself
pub(in crate::wasm_emitter) const STRUCT_BODY: &str = "struct_body";
/// instance_field(fields, name_ptr, name_len): the value of the field whose name is that very string of the string
/// table, null when none is (then the general lookup decides)
pub(in crate::wasm_emitter) const INSTANCE_FIELD: &str = "instance_field";
/// tagged_field(node, name): the field of the tagged object `tag:{…}` the node is, null for any other node
pub(in crate::wasm_emitter) const TAGGED_FIELD: &str = "tagged_field";
/// elements_field(xs, key): the field of every element of a list that is no map, as a `[…]` list; null for any other
/// node, an empty list, or an element without the field (cards field-list-passed, field-list-read)
pub(in crate::wasm_emitter) const ELEMENTS_FIELD: &str = "elements_field";
/// The locals of a list built cell by cell: the cell just made, the first and the last
struct CellAppend {
	made: u32,
	head: u32,
	last: u32,
}
/// error_message(node): the message of an Error as a text, null for any other node: `e.message` (card catch-message)
pub const ERROR_MESSAGE: &str = "error_message";
/// The field every Error has
pub const MESSAGE_FIELD: &str = "message";
/// What a field read answers when the value has no such field or key, before the missing field's error: an Error's
/// `message`, any value's `type` (field → meta → reflection word, notes/reflection.md "Rule"; card type-non), a function
/// value's reflection words (closures.rs FUNCTION_WORDS)
const FIELD_FALLBACKS: [(&str, &str); 2] =
	[(MESSAGE_FIELD, ERROR_MESSAGE), (crate::type_tests::TYPE_WORD, crate::type_tests::NODE_TYPE_NAME)];

/// The function a field read of `name` falls back to (FIELD_FALLBACKS)
pub fn field_fallback(name: &str) -> Option<&'static str> {
	let name = crate::lowering::reflection::canonical_word(name).unwrap_or(name);
	FIELD_FALLBACKS
		.into_iter()
		.chain(crate::wasm_emitter::functions::closures::function_words())
		.find(|(field, _)| *field == name)
		.map(|(_, fallback)| fallback)
}
pub(super) const KEY_KIND: i64 = Kind::Key as i64;
pub const MAP_KEY_NAME: &str = "map_key_name";
/// node_at_key(xs, key) and node_with_key(xs, key, value): `xs[key]` read and set with a key known only at runtime
pub const NODE_AT_KEY: &str = "node_at_key";
pub const NODE_WITH_KEY: &str = "node_with_key";
/// node_with_node_at(node, index, value): the copy with a Node value at index (node_with_at takes an Int)
pub const NODE_WITH_NODE_AT: &str = "node_with_node_at";
pub(super) const IS_MAP: &str = "is_map";
pub(super) const MAP_PART: &str = "map_part";
pub(super) const MAP_COLUMN: &str = "map_column";
pub(super) const MAP_COLUMN_CELLS: &str = "map_column_cells";
/// The parts of an entry `key:value` that `map_part` reads
pub(super) const ENTRY_PART: i32 = 0;
pub(super) const KEY_PART: i32 = 1;
pub(super) const VALUE_PART: i32 = 2;
use crate::library_words::{
	COLLECTION_CONTAINS, COLLECTION_POSITION, MAP_ENTRIES, MAP_GET_OR, MAP_VALUES, MAP_WITHOUT, MAP_WORD_FUNCTIONS,
};
use crate::one_entry_maps::MAP_OF_ENTRY;

impl WasmGcEmitter {
	/// The key of `target[key]`: an index that is a symbol or a text (not a number) selects the entry of that name
	pub(in crate::wasm_emitter) fn map_is_indexed_by_key(&self, index: &Node) -> bool {
		self.map_key(index).is_some() || self.dynamic_key(index).is_some()
	}

	/// A key held as a Node (`edge[0]` of a map's value): a number indexes, a name looks up, decided at runtime
	pub(in crate::wasm_emitter) fn dynamic_key(&self, index: &Node) -> Option<Node> {
		let key = crate::warp_parser::subscript_key(index)?;
		(self.get_type(key) == Kind::Empty && !key.is_empty_node()).then(|| key.clone())
	}

	pub(in crate::wasm_emitter) fn map_key(&self, index: &Node) -> Option<Node> {
		let key = crate::warp_parser::subscript_key(index)?;
		self.is_name_key(key).then(|| key.clone())
	}

	/// A map or list the compiler knows as one: `{age:2}`, a variable holding it; a map of one entry is held as its pair
	/// at run time (card single-entry), yet has no pair parts of its own (card age-name)
	fn is_known_collection(&self, target: &Node) -> bool {
		matches!(target.drop_meta(), Node::List(_, Bracket::Curly | Bracket::Square, _))
			|| crate::analyzer::held_map_type(target, &self.scope).is_some()
			|| self.get_type(target) == Kind::List
	}

	/// A key that names an entry: a symbol, a text or a character ("x" is a codepoint)
	pub(in crate::wasm_emitter) fn is_name_key(&self, key: &Node) -> bool {
		matches!(self.get_type(key), Kind::Symbol | Kind::Text | Kind::Codepoint)
			|| matches!(key.drop_meta(), Node::Char(_))
	}

	/// Push the map a lookup searches: a struct instance `point:{x:1 y:2}` is searched by its fields, anything else as it is
	pub(super) fn emit_lookup_target(&mut self, func: &mut Function, target: &Node) {
		self.emit_node_instructions(func, target);
		if !self.ctx.type_registry.types().is_empty() {
			self.emit_call(func, STRUCT_BODY);
		}
	}

	/// Push the Node at `target#index`: the entry value for a key, else the element at the 1-based position
	pub(in crate::wasm_emitter) fn emit_indexed_node(&mut self, func: &mut Function, target: &Node, index: &Node) {
		// a map held as a hash table answers when it can; the generic lookup on its Node gives the rest and the errors
		if let (Some(slot), Some(key)) = (self.typed_map(target), self.map_key(index)) {
			func.instruction(&I::Block(BlockType::Result(Ref(self.node_ref(false)))));
			self.emit_typed_map_lookup(func, slot, &key);
			self.emit_generic_indexed_node(func, target, index);
			func.instruction(&I::End);
			return;
		}
		if self.emit_struct_field_node(func, target, index) {
			return;
		}
		self.emit_generic_indexed_node(func, target, index);
	}

	pub(super) fn emit_generic_indexed_node(&mut self, func: &mut Function, target: &Node, index: &Node) {
		match self.map_key(index) {
			Some(key) => match crate::declaration_forms::constant_field_name(&key) {
				// the value of the entry, else of the meta entry `@name`, or the runtime error naming the missing field
				Some(name) => {
					let node_ref = Ref(self.node_ref(false));
					func.instruction(&I::Block(BlockType::Result(node_ref)));
					let meta_name = format!("{}{name}", crate::node::ATTRIBUTE_MARK);
					let fallback = (!name.starts_with(crate::node::ATTRIBUTE_MARK)).then_some(meta_name);
					// the target once: a call in it runs once whichever lookup answers
					let held = self.node_scratch();
					self.emit_lookup_target(func, target);
					func.instruction(&I::LocalSet(held));
					// an instance's field by the very name the program wrote: no symbol made, no general comparison
					if !self.ctx.type_registry.types().is_empty() {
						Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull]);
						self.emit_text_literal(func, &name);
						self.emit_call(func, INSTANCE_FIELD);
						func.instruction(&I::BrOnNonNull(0));
					}
					// the entry, a field of a tagged object `Person{name:…}` (data, D4), else the meta entry `@name`
					let attempts = [(name.clone(), "map_find"), (name.clone(), TAGGED_FIELD)]
						.into_iter()
						.chain(fallback.map(|meta| (meta, "map_find")));
					for (key, finder) in attempts {
						Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull]);
						self.emit_symbol_node(func, &key);
						self.emit_call(func, finder);
						func.instruction(&I::BrOnNonNull(0));
					}
					self.emit_uncertain_field_lookup(func, held, &name);
					self.emit_time_field_lookup(func, held, &name);
					// a key's key, value, op and name, a list's bracket (node_api.rs); a map held as its one pair has none
					if let Some(part) = super::super::super::reflection::node_api::node_part(&name).filter(|part| {
						!(super::super::super::reflection::node_api::is_pair_part(*part)
							&& self.is_known_collection(target))
					}) {
						Self::emit_list(func, &[I::LocalGet(held), I32Const(part)]);
						self.emit_call(func, super::super::super::reflection::node_api::NODE_PART);
						func.instruction(&I::BrOnNonNull(0));
					}
					if let Some(fallback) = field_fallback(&name) {
						Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull]);
						self.emit_call(func, fallback);
						func.instruction(&I::BrOnNonNull(0));
					}
					// `p.xy` of a list of 2 to 4 numbers: its components (decision S1)
					if let Some(components) = crate::words::swizzle_components(&name) {
						let picks = I::I64Const(swizzle_picks(&components));
						Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, picks]);
						self.emit_call(func, SWIZZLED);
						func.instruction(&I::BrOnNonNull(0));
					}
					// a list of no shape the compiler knows: the field of each element, `names` also as `name`
					for field in
						std::iter::once(name.clone()).chain(crate::extensions::strings::english_singular(&name))
					{
						Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull]);
						self.emit_symbol_node(func, &field);
						self.emit_call(func, ELEMENTS_FIELD);
						func.instruction(&I::BrOnNonNull(0));
					}
					// a field of an Error is the Error: `(try 10/0).name` fails with divide by zero (card error-field)
					Self::emit_list(func, &[I::LocalGet(held), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty)]);
					self.emit_fail_if_error(func, held);
					func.instruction(&I::End);
					Self::emit_list(
						func,
						&[I::Call(self.func_index(&format!("{NO_FIELD_PREFIX}{name}"))), I::Unreachable, I::End],
					);
				}
				None => {
					self.emit_lookup_target(func, target);
					self.emit_node_instructions(func, &key);
					self.emit_call(func, "map_get");
				}
			},
			None => match self.dynamic_key(index) {
				Some(key) => {
					self.emit_node_instructions(func, target);
					self.emit_node_instructions(func, &key);
					self.emit_call(func, NODE_AT_KEY);
				}
				None => self.emit_list_element_node(func, target, index),
			},
		}
	}

	/// map_get(map: ref $Node, key: ref $Node) -> ref $Node: the value of the first `key:value` entry whose key equals `key`
	pub(in crate::wasm_emitter) fn emit_map_get(&mut self) {
		if !self.uses_map_lookups() {
			return;
		}
		let node = self.type_manager.node_type;
		let node_ref = Ref(self.node_ref(false));
		let nullable_node_ref = Ref(self.node_ref(true));
		let missing_fields: Vec<&'static str> = self
			.ctx
			.missing_field_names
			.iter()
			.map(|name| &*Box::leak(format!("{NO_FIELD_PREFIX}{name}").into_boxed_str()))
			.collect();
		// a missing field is a runtime error a `try` catches, thrown with the id of key_not_found
		let key_not_found =
			RUNTIME_ERRORS.iter().position(|error| *error == "key_not_found").expect("a runtime error") as i32;
		for name in missing_fields {
			self.runtime_function(name, vec![], vec![], vec![], |s, f| s.emit_error_body(f, key_not_found));
		}
		// map_find(map, key): the value of the first `key:value` entry whose key equals `key`, null when there is none or
		// `map` is no list; a text key equals the symbol of the same letters, so `p["name"]` finds `name:"Joe"`
		// map_entry_has_key(entry, key): whether the node is a `key:value` entry of that key; a text key equals the symbol of the same letters
		// map_key_name(key): the symbol a key is compared by; a text or a character becomes the symbol of the same letters,
		// so `{"A":1}`, `{A:1}`, `m["A"]` and `k="A"; m[k]` all name the same entry
		self.runtime_function(MAP_KEY_NAME, vec![node_ref], vec![node_ref], vec![nullable_node_ref], |s, f| {
			let name = 1;
			f.instruction(&I::LocalGet(0));
			s.call(f, super::super::super::text::text_builtins::TEXT_OF);
			f.instruction(&I::LocalSet(name));
			s.emit_field(f, name, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(Kind::Text as i64), I::I64Eq, I::If(BlockType::Result(node_ref)),
					I::I64Const(Kind::Symbol as i64),
			]);
			s.emit_text_bytes(f, name); // a rope (text_append) flattened, else only its last piece names the key
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::RefNull(HeapType::Concrete(node)), I::StructNew(node),
				I::Else, I::LocalGet(name), I::RefAsNonNull,
				I::End,
			]);
		});
		self.runtime_function(
			"map_entry_has_key",
			vec![node_ref, node_ref],
			vec![ValType::I32],
			vec![nullable_node_ref],
			|s, f| {
				let (key, entry_key) = (1, 2);
				let is_name = |f: &mut Function, local: u32| {
					for kind in [Kind::Text, Kind::Symbol] {
						s.emit_field(f, local, 0);
						Self::emit_kind_test(f, kind as i64, I::I64Eq);
					}
					f.instruction(&I::I32Or);
				};
				s.emit_field(f, 0, 0);
				Self::emit_kind_test(f, KEY_KIND, I::I64Eq);
				Self::emit_list(f, &[I::If(BlockType::Result(ValType::I32))]);
				s.emit_field(f, 0, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalSet(entry_key)]);
				// two names (texts or symbols) compare their letters, without making the symbols
				is_name(f, entry_key);
				is_name(f, key);
				Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty)]);
				s.emit_field(f, entry_key, 1);
				s.emit_field(f, key, 1);
				s.call(f, VALUES_EQUAL);
				Self::emit_list(f, &[I::Return, I::End, I::LocalGet(entry_key), I::RefAsNonNull]);
				s.call(f, MAP_KEY_NAME);
				f.instruction(&I::LocalGet(1));
				s.call(f, MAP_KEY_NAME);
				s.call(f, VALUES_EQUAL);
				Self::emit_list(f, &[I::Else, I::I32Const(0), I::End]);
			},
		);
		self.emit_node_map_runtime();
		self.emit_map_index();
		let locals = vec![nullable_node_ref, nullable_node_ref, nullable_node_ref, ValType::I64, ValType::I32];
		self.runtime_function("map_find", vec![node_ref, node_ref], vec![nullable_node_ref], locals, |s, f| {
			let (cell, entry, tmp, kind, indexed) = (2, 3, 4, 5, 6);
			// return the value when the entry in local `entry` is a `key:value` whose key is the wanted one
			let check_entry = |f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(entry), I::RefAsNonNull, I::LocalGet(1)]);
				s.call(f, "map_entry_has_key");
				f.instruction(&I::If(BlockType::Empty));
				s.emit_field(f, entry, 2);
				Self::emit_list(f, &[I::LocalTee(tmp), I::RefIsNull, I::If(BlockType::Result(nullable_node_ref))]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Else, I::LocalGet(tmp), I::End, I::Return, I::End]);
			};
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			// a single entry `{a:1}` is the `a:1` node itself
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(kind), I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty),
					I::LocalGet(0), I::LocalSet(entry),
			]);
			check_entry(f);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::RefNull(HeapType::Concrete(node)), I::Return,
				I::End,
				I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::LocalGet(kind),
				I::I64Const(Kind::Block as i64), I::I64Ne, I::I32And, I::If(BlockType::Empty),
					I::RefNull(HeapType::Concrete(node)), I::Return,
				I::End,
			]);
			// a map walked lately has an index
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, super::super::super::maps::map_index::MAP_INDEX_ENTRY);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(indexed),
				I::LocalSet(entry),
				I::LocalGet(indexed), I::If(BlockType::Empty),
					I::LocalGet(entry), I::RefIsNull, I::If(BlockType::Empty),
						I::RefNull(HeapType::Concrete(node)), I::Return,
					I::End,
			]);
			s.emit_field(f, entry, 2);
			Self::emit_list(f, &[I::LocalTee(tmp), I::RefIsNull, I::If(BlockType::Result(nullable_node_ref))]);
			s.call(f, "new_empty");
			Self::emit_list(
				f,
				&[I::Else, I::LocalGet(tmp), I::End, I::Return, I::End, I::LocalGet(0), I::LocalSet(cell)],
			);
			s.emit_cell_walk(f, cell, |f| {
				s.emit_field(f, cell, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalSet(entry)]);
				check_entry(f);
			});
			f.instruction(&I::RefNull(HeapType::Concrete(node)));
		});
		if self.should_emit_function(ERROR_MESSAGE) {
			self.runtime_function(ERROR_MESSAGE, vec![nullable_node_ref], vec![nullable_node_ref], vec![], |_, f| {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty),
						I::RefNull(HeapType::Concrete(node)), I::Return,
					I::End,
					I::LocalGet(0), I::StructGet { struct_type_index: node, field_index: 0 },
				]);
				Self::emit_kind_test(f, Kind::Error as i64, I::I64Ne);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::If(BlockType::Empty),
						I::RefNull(HeapType::Concrete(node)), I::Return,
					I::End,
					I::I64Const(Kind::Text as i64), I::LocalGet(0),
					I::StructGet { struct_type_index: node, field_index: 1 }, I::RefNull(HeapType::Concrete(node)),
					I::StructNew(node),
				]);
			});
		}
		if self.should_emit_function(TAGGED_FIELD) {
			let list_kind = Kind::List as i64;
			self.runtime_function(
				TAGGED_FIELD,
				vec![node_ref, node_ref],
				vec![nullable_node_ref],
				vec![nullable_node_ref],
				|s, f| {
					let value = 2;
					let kind_of = |f: &mut Function, local: u32| {
						Self::emit_list(
							f,
							&[
								I::LocalGet(local),
								I::RefAsNonNull,
								I::StructGet { struct_type_index: node, field_index: 0 },
								I::I64Const(KIND_MASK),
								I::I64And,
							],
						)
					};
					kind_of(f, 0);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Const(KEY_KIND), I::I64Ne, I::If(BlockType::Empty),
							I::RefNull(HeapType::Concrete(node)), I::Return,
						I::End,
						I::LocalGet(0), I::StructGet { struct_type_index: node, field_index: 2 }, I::LocalTee(value),
						I::RefIsNull, I::If(BlockType::Empty),
							I::RefNull(HeapType::Concrete(node)), I::Return,
						I::End,
					]);
					// the fields: a list of entries, or the one entry `{b:1}` is
					kind_of(f, value);
					Self::emit_list(f, &[I::I64Const(list_kind), I::I64Ne]);
					kind_of(f, value);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Const(KEY_KIND), I::I64Ne, I::I32And, I::If(BlockType::Empty),
							I::RefNull(HeapType::Concrete(node)), I::Return,
						I::End,
						I::LocalGet(value), I::RefAsNonNull, I::LocalGet(1),
					]);
					s.call(f, "map_find");
				},
			);
		}
		if self.should_emit_function(STRUCT_BODY) {
			let type_names: Vec<String> =
				self.ctx.type_registry.types().iter().map(|type_def| type_def.name.clone()).collect();
			let type_symbols: Vec<(u32, u32)> = type_names.iter().map(|name| self.memory_string(name)).collect();
			self.runtime_function(STRUCT_BODY, vec![node_ref], vec![node_ref], vec![nullable_node_ref], |s, f| {
				let entry = 1;
				let field = |f: &mut Function, local: u32, index: u32| {
					Self::emit_list(
						f,
						&[I::LocalGet(local), I::StructGet { struct_type_index: node, field_index: index }],
					);
				};
				let kind_of = |f: &mut Function, local: u32| {
					field(f, local, 0);
					Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And]);
				};
				// the fields of the key in `local` when it is named after a class
				let fields_when_named = |f: &mut Function, local: u32| {
					for &(pointer, length) in &type_symbols {
						field(f, local, 1);
						Self::emit_list(
							f,
							&[
								I::RefCastNonNull(HeapType::Concrete(node)),
								I::I32Const(pointer as i32),
								I::I32Const(length as i32),
							],
						);
						s.call(f, "new_symbol");
						s.call(f, VALUES_EQUAL);
						f.instruction(&I::If(BlockType::Empty));
						field(f, local, 2);
						Self::emit_list(f, &[I::RefAsNonNull, I::Return, I::End]);
					}
				};
				// `{"Todo": {…}}`, an instance as a server sends it (web_server.rs) and JSON decodes it: its one entry
				kind_of(f, 0);
				Self::emit_list(f, &[I::I64Const(Kind::List as i64), I::I64Eq, I::If(BlockType::Empty)]);
				field(f, 0, 2);
				Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Empty)]);
				field(f, 0, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefCastNullable(HeapType::Concrete(node)), I::LocalTee(entry), I::RefIsNull, I::I32Eqz,
					I::If(BlockType::Empty),
				]);
				kind_of(f, entry);
				Self::emit_list(f, &[I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty)]);
				fields_when_named(f, entry);
				Self::emit_list(f, &[I::End, I::End, I::End, I::LocalGet(0), I::Return, I::End]);
				kind_of(f, 0);
				Self::emit_list(
					f,
					&[I::I64Const(KEY_KIND), I::I64Ne, I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End],
				);
				// an instance carries its own op code (D4, type_constructor.rs): its fields, no type name compared
				field(f, 0, 0);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(8), I::I64ShrU, I::I64Const(crate::operators::op_to_code(&crate::operators::Op::None)),
					I::I64Eq, I::If(BlockType::Empty),
				]);
				field(f, 0, 2);
				Self::emit_list(f, &[I::RefAsNonNull, I::Return, I::End]);
				fields_when_named(f, 0);
				f.instruction(&I::LocalGet(0));
			});
		}
		if self.should_emit_function(INSTANCE_FIELD) {
			let text = self.type_manager.text_type;
			let nullable = Ref(self.node_ref(true));
			let text_ref = self.text_ref();
			self.runtime_function(
				INSTANCE_FIELD,
				vec![node_ref, text_ref],
				vec![nullable],
				vec![nullable, nullable, nullable],
				|s, f| {
					let (cell, entry, key) = (2, 3, 4);
					let get = |f: &mut Function, local: u32, index: u32| {
						Self::emit_list(
							f,
							&[
								I::LocalGet(local),
								I::RefAsNonNull,
								I::StructGet { struct_type_index: node, field_index: index },
							],
						)
					};
					let kind_is = |f: &mut Function, local: u32, kind: i64| {
						get(f, local, 0);
						Self::emit_kind_test(f, kind, I::I64Eq);
					};
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(0), I::LocalSet(cell),
						I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
								I::LocalGet(cell), I::RefIsNull, I::BrIf(1),
					]);
					kind_is(f, cell, Kind::List as i64);
					Self::emit_list(f, &[I::I32Eqz, I::BrIf(1)]);
					get(f, cell, 1);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::RefCastNullable(HeapType::Concrete(node)), I::LocalTee(entry), I::RefIsNull, I::I32Eqz,
						I::If(BlockType::Empty),
					]);
					kind_is(f, entry, Kind::Key as i64);
					f.instruction(&I::If(BlockType::Empty));
					get(f, entry, 1);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::RefCastNullable(HeapType::Concrete(node)), I::LocalTee(key), I::RefIsNull, I::I32Eqz,
						I::If(BlockType::Empty),
					]);
					kind_is(f, key, Kind::Symbol as i64);
					f.instruction(&I::If(BlockType::Empty));
					// the same name
					get(f, key, 1);
					Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(text)), I::LocalGet(1)]);
					s.call(f, TEXT_BYTES_EQUAL);
					f.instruction(&I::If(BlockType::Empty));
					get(f, entry, 2);
					Self::emit_list(f, &[I::Return, I::End, I::End, I::End, I::End, I::End]);
					get(f, cell, 2);
					Self::emit_list(
						f,
						&[I::LocalSet(cell), I::Br(0), I::End, I::End, I::RefNull(HeapType::Concrete(node))],
					);
				},
			);
		}
		// map_get(map, key): the value, or the runtime error `key not found`
		self.runtime_function("map_get", vec![node_ref, node_ref], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::Block(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "map_find");
			f.instruction(&I::BrOnNonNull(0));
			s.call(f, "key_not_found");
			Self::emit_list(f, &[I::Unreachable, I::End]);
		});
		// after the struct_body it calls
		if self.should_emit_function(ELEMENTS_FIELD) {
			self.emit_elements_field();
		}
		self.emit_field_with();
		self.emit_instance_copy();
		self.emit_map_words();
		self.emit_dynamic_key_access();
	}

	/// `xs[k]` and `xs[k] = v` with a key held as a Node: an Int indexes from 0, anything else names an entry
	pub(super) fn emit_dynamic_key_access(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let is_int = |s: &Self, f: &mut Function| {
			s.emit_field(f, 1, 0);
			Self::emit_list(f, &[I::I64Const(Kind::Int as i64), I::I64Eq]);
		};
		let one_based = |s: &Self, f: &mut Function| {
			f.instruction(&I::LocalGet(1));
			s.call(f, "get_int_value");
			Self::emit_list(f, &[I::I64Const(1), I::I64Add]);
		};
		if self.should_emit_function(NODE_AT_KEY) {
			self.runtime_function(NODE_AT_KEY, vec![node_ref, node_ref], vec![node_ref], vec![], |s, f| {
				is_int(s, f);
				Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(0)]);
				one_based(s, f);
				s.call(f, "node_index_at");
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, "map_get");
				f.instruction(&I::End);
			});
		}
		if self.should_emit_function(NODE_WITH_KEY) {
			self.runtime_function(NODE_WITH_KEY, vec![node_ref, node_ref, node_ref], vec![node_ref], vec![], |s, f| {
				is_int(s, f);
				Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(0)]);
				one_based(s, f);
				f.instruction(&I::LocalGet(2));
				s.call(f, NODE_WITH_NODE_AT);
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1), I::LocalGet(2)]);
				s.call(f, crate::library_words::FIELD_WITH);
				f.instruction(&I::End);
			});
		}
	}

	/// In `node_index_at` (node local 0, index local 1): a pair `k:v` is the two items k and v
	pub(super) fn emit_pair_part(&self, func: &mut Function) {
		let node = self.type_manager.node_type;
		self.emit_field(func, 0, 0);
		Self::emit_kind_test(func, KEY_KIND, I::I64Eq);
		Self::emit_list(
			func,
			&[I::If(BlockType::Empty), I::LocalGet(1), I::I64Const(1), I::I64Eq, I::If(BlockType::Empty)],
		);
		self.emit_field(func, 0, 1);
		Self::emit_list(
			func,
			&[I::RefCastNonNull(HeapType::Concrete(node)), I::Return, I::End, I::LocalGet(1), I::I64Const(2), I::I64Ne],
		);
		self.emit_fail_if(func, "index_out_of_range");
		self.emit_value_or_empty(func, 0);
		Self::emit_list(func, &[I::Return, I::End]);
	}

	/// Push the value of the pair in local `pair`, ø for `k:` without one
	pub(super) fn emit_value_or_empty(&self, func: &mut Function, pair: u32) {
		func.instruction(&I::Block(BlockType::Result(Ref(self.node_ref(false)))));
		self.emit_field(func, pair, 2);
		func.instruction(&I::BrOnNonNull(0));
		self.call(func, "new_empty");
		func.instruction(&I::End);
	}

	/// With the kind and the value of a new cell pushed: the cell made, appended after the cell in `last`, or `head` when
	/// it is the first; a list built in a loop, not a call per element (map_column_cells)
	fn emit_appended_cell(&self, func: &mut Function, cells: CellAppend) {
		let node = self.type_manager.node_type;
		let CellAppend { made, head, last } = cells;
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::RefNull(HeapType::Concrete(node)), I::StructNew(node), I::LocalSet(made),
			I::LocalGet(last), I::RefIsNull, I::If(BlockType::Empty),
				I::LocalGet(made), I::LocalSet(head),
			I::Else,
				I::LocalGet(last), I::LocalGet(made),
				I::StructSet { struct_type_index: node, field_index: 2 },
			I::End,
			I::LocalGet(made), I::LocalSet(last),
		]);
	}

	/// elements_field(xs, key) (ELEMENTS_FIELD): each element read as a field read of a value of unknown shape does, an
	/// instance by its fields, a map by its entries, a tagged object `P{name:…}` by its body
	fn emit_elements_field(&mut self) {
		let node = self.type_manager.node_type;
		let node_ref = Ref(self.node_ref(false));
		let nullable = Ref(self.node_ref(true));
		let has_struct_body = !self.ctx.type_registry.types().is_empty();
		let locals = [vec![nullable; 5], self.item_walk_locals().to_vec()].concat();
		self.runtime_function(ELEMENTS_FIELD, vec![node_ref, node_ref], vec![nullable], locals, |s, f| {
			let (element, value, head, last, made, walk) = (2, 3, 4, 5, 6, ItemWalk::at(7));
			let null = I::RefNull(HeapType::Concrete(node));
			// only a list has elements, a `{…}` map has entries; an empty list has no field to show
			s.emit_field(f, 0, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::List as i64), I::I64Ne]);
			s.emit_field(f, 0, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(crate::type_kinds::UNMARKED_KIND), I::I64And, I::I64Const(CURLY_LIST_KIND), I::I64Eq,
				I::I32Or,
			]);
			s.emit_field(f, 0, 1);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::RefIsNull, I::I32Or, I::If(BlockType::Empty),
					null.clone(), I::Return,
				I::End,
			]);
			s.emit_item_walk(f, 0, walk, |f| {
				Self::emit_list(f, &[I::LocalGet(walk.item), I::RefAsNonNull]);
				if has_struct_body {
					s.call(f, STRUCT_BODY);
				}
				Self::emit_list(f, &[I::LocalTee(element), I::RefAsNonNull, I::LocalGet(1)]);
				s.call(f, "map_find");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalTee(value), I::RefIsNull, I::If(BlockType::Empty),
						I::LocalGet(element), I::RefAsNonNull, I::LocalGet(1),
				]);
				s.call(f, TAGGED_FIELD);
				#[rustfmt::skip]
				Self::emit_list(f, &[
						I::LocalTee(value), I::RefIsNull, I::If(BlockType::Empty),
							null.clone(), I::Return,
						I::End,
					I::End,
					I::I64Const(SQUARE_LIST_KIND), I::LocalGet(value),
				]);
				s.emit_appended_cell(f, CellAppend { made, head, last });
			});
			f.instruction(&I::LocalGet(head));
		});
	}

	/// The words of maps: `keys`, `values`, the entries a `for (k, v) in m` walks, `x in xs` and `m.get(k, default)`.
	/// A map is a `{…}` list of `key:value` entries, or the one entry itself (`{a:1}`); its keys are read as texts.
	pub(super) fn emit_map_words(&mut self) {
		if !MAP_WORD_FUNCTIONS.iter().any(|name| self.should_emit_function(name)) {
			return;
		}
		let node = self.type_manager.node_type;
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let is_entry = |s: &Self, f: &mut Function, local: u32| {
			s.emit_field(f, local, 0);
			Self::emit_kind_test(f, KEY_KIND, I::I64Eq);
		};
		// is_map(node): a single entry, or a `{…}` list that starts with one
		self.runtime_function(IS_MAP, vec![node_ref], vec![ValType::I32], vec![], |s, f| {
			is_entry(s, f, 0);
			Self::emit_list(f, &[I::If(BlockType::Empty), I32Const(1), I::Return, I::End]);
			s.emit_field(f, 0, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(crate::type_kinds::UNMARKED_KIND), I::I64And, I::I64Const(CURLY_LIST_KIND), I::I64Ne,
				I::If(BlockType::Empty),
					I32Const(0), I::Return,
				I::End,
			]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefTestNonNull(HeapType::Concrete(node)), I::If(BlockType::Result(ValType::I32))]);
			s.emit_field(f, 0, 1);
			Self::emit_list(
				f,
				&[
					I::RefCastNonNull(HeapType::Concrete(node)),
					I::StructGet { struct_type_index: node, field_index: 0 },
				],
			);
			Self::emit_kind_test(f, KEY_KIND, I::I64Eq);
			Self::emit_list(f, &[I::Else, I32Const(0), I::End]);
		});
		// map_of_entry(node): a lone entry `{a:1}` as the map of it, a `{…}` list of one cell (one_entry_maps.rs); else itself
		self.runtime_function(MAP_OF_ENTRY, vec![node_ref], vec![node_ref], vec![], |s, f| {
			is_entry(s, f, 0);
			Self::emit_list(
				f,
				&[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(0), I::Return, I::End, I::LocalGet(0)],
			);
			s.emit_node_null(f);
			f.instruction(&I::I64Const(crate::type_kinds::CURLY_BRACKET_INFO));
			s.call(f, "new_list");
		});
		// map_part(entry, part): the entry itself (ENTRY_PART), its key as a text (KEY_PART) or its value (VALUE_PART)
		self.runtime_function(MAP_PART, vec![node_ref, ValType::I32], vec![node_ref], vec![nullable], |s, f| {
			let key = 2;
			is_entry(s, f, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Eqz, I::LocalGet(1), I32Const(ENTRY_PART), I::I32Eq, I::I32Or, I::If(BlockType::Empty),
					I::LocalGet(0), I::Return,
				I::End,
				I::LocalGet(1), I32Const(VALUE_PART), I::I32Eq, I::If(BlockType::Empty),
			]);
			s.emit_value_or_empty(f, 0);
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalSet(key)]);
			s.emit_field(f, key, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(Kind::Symbol as i64), I::I64Eq, I::If(BlockType::Result(node_ref)),
					I::I64Const(Kind::Text as i64),
			]);
			s.emit_field(f, key, 1);
			s.emit_field(f, key, 2);
			Self::emit_list(f, &[I::StructNew(node), I::Else, I::LocalGet(key), I::RefAsNonNull, I::End]);
		});
		// map_column_cells(cells, part): a `[…]` list of that part of every entry, appended at its last cell in a loop
		// (a call per entry exhausted the call stack near ten thousand entries)
		self.runtime_function(
			MAP_COLUMN_CELLS,
			vec![nullable, ValType::I32],
			vec![nullable],
			[vec![nullable; 3], self.item_walk_locals().to_vec()].concat(),
			|s, f| {
				let (map, part, head, last, made, walk) = (0, 1, 2, 3, 4, ItemWalk::at(5));
				// a meta entry `@name:value` is no key, value or entry of the map: list_items ends before them
				s.emit_item_walk(f, map, walk, |f| {
					Self::emit_list(f, &[I::I64Const(SQUARE_LIST_KIND), I::LocalGet(walk.item), I::RefAsNonNull]);
					f.instruction(&I::LocalGet(part));
					s.call(f, MAP_PART);
					s.emit_appended_cell(f, CellAppend { made, head, last });
				});
				f.instruction(&I::LocalGet(head));
			},
		);
		// map_column(xs, part): that part of every entry of a map, as a list; anything else unchanged
		self.runtime_function(MAP_COLUMN, vec![node_ref, ValType::I32], vec![node_ref], vec![], |s, f| {
			is_entry(s, f, 0);
			Self::emit_list(
				f,
				&[I::If(BlockType::Empty), I::I64Const(SQUARE_LIST_KIND), I::LocalGet(0), I::LocalGet(1)],
			);
			s.call(f, MAP_PART);
			Self::emit_list(
				f,
				&[I::RefNull(HeapType::Concrete(node)), I::StructNew(node), I::Return, I::End, I::LocalGet(0)],
			);
			s.call(f, IS_MAP);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Eqz, I::If(BlockType::Empty),
					I::LocalGet(0), I::Return,
				I::End,
				I::LocalGet(0), I::LocalGet(1),
			]);
			s.call(f, MAP_COLUMN_CELLS);
			f.instruction(&I::RefAsNonNull);
		});
		for (name, part) in [(MAP_ENTRIES, ENTRY_PART), (MAP_KEYS, KEY_PART), (MAP_VALUES, VALUE_PART)] {
			// keys or values of a number, a text or a list: not_a_map, no silent echo of it (card keys-value); ø and [] have
			// none. map_entries passes a list through: `for a, b in [[1, 2], [3, 4]]` unpacks with it
			self.runtime_function(name, vec![node_ref], vec![node_ref], vec![], |s, f| {
				if name != MAP_ENTRIES {
					f.instruction(&I::LocalGet(0));
					s.call(f, IS_MAP);
					s.emit_field(f, 0, 1);
					Self::emit_list(f, &[I::RefIsNull, I::I32Or, I::I32Eqz]);
					s.emit_fail_if(f, NOT_A_MAP);
				}
				Self::emit_list(f, &[I::LocalGet(0), I32Const(part)]);
				s.call(f, MAP_COLUMN);
			});
		}
		// collection_contains(xs, x) (`x in xs`, `xs.has(x)`): yes when a map has the key x or a list the element x, else
		// no (decision in-bool); collection_position(xs, x) (`xs.index_of(x)`): the 1-based position of x in a list, 1/0
		// for a map key
		for (name, positional) in [(COLLECTION_CONTAINS, false), (COLLECTION_POSITION, true)] {
			let answer_constructor = if positional { "new_int" } else { NEW_BOOL };
			self.runtime_function(
				name,
				vec![node_ref, node_ref],
				vec![node_ref],
				[vec![ValType::I64; 2], self.item_walk_locals().to_vec()].concat(),
				|s, f| {
					let (wanted, kind, position, walk) = (1, 2, 3, ItemWalk::at(4));
					let answer = |f: &mut Function, s: &Self, value: i64| {
						f.instruction(&if positional && value == 1 {
							I::LocalGet(position)
						} else {
							I::I64Const(value)
						});
						s.call(f, answer_constructor);
						f.instruction(&I::Return);
					};
					f.instruction(&I::LocalGet(0));
					s.call(f, IS_MAP);
					Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(1)]);
					s.call(f, "map_find");
					Self::emit_list(f, &[I::RefIsNull, I::I64ExtendI32U, I::I64Const(1), I::I64Xor]);
					s.call(f, answer_constructor);
					Self::emit_list(f, &[I::Return, I::End]);
					s.emit_field(f, 0, 0);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::Empty as i64), I::I64Eq,
						I::If(BlockType::Empty),
					]);
					answer(f, s, 0);
					f.instruction(&I::End);
					// a text holds a text: its 1-based byte position (in), or 1 (contains)
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(kind), I::I64Const(Kind::Text as i64), I::I64Eq, I::If(BlockType::Empty),
							I::LocalGet(0), I::LocalGet(wanted),
					]);
					s.call(f, super::super::super::text::text_builtins::TEXT_OF);
					s.call(f, super::super::super::text::text_builtins::TEXT_FIND);
					if !positional {
						Self::emit_list(f, &[I::I64Const(0), I::I64Ne, I::I64ExtendI32U]);
					}
					s.call(f, answer_constructor);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::Return,
						I::End,
						I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::LocalGet(kind),
						I::I64Const(Kind::Block as i64), I::I64Ne, I::I32And,
					]);
					s.emit_fail_if(f, "not_a_list");
					s.emit_item_walk(f, 0, walk, |f| {
						Self::emit_list(f, &[I::LocalGet(position), I::I64Const(1), I::I64Add, I::LocalSet(position)]);
						Self::emit_list(f, &[I::LocalGet(walk.item), I::LocalGet(wanted)]);
						s.call(f, VALUES_EQUAL);
						f.instruction(&I::If(BlockType::Empty));
						answer(f, s, 1);
						f.instruction(&I::End);
					});
					answer(f, s, 0);
				},
			);
		}
		// map_get_or(map, key, fallback): the value of the key, else the fallback (ø by default: `m.get(k)`)
		self.runtime_function(MAP_GET_OR, vec![node_ref, node_ref, node_ref], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::Block(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "map_find");
			Self::emit_list(f, &[I::BrOnNonNull(0), I::LocalGet(2), I::End]);
		});
		// map_without(map, key): the map without the key's entry (or, of a list, without the first element equal to the
		// key: `xs.remove(v)`), in place so every holder sees it (P200b): a later cell is unlinked, the first cell takes
		// the second's item and rest, a lone entry or cell becomes ø itself
		let set = |field_index: u32| I::StructSet { struct_type_index: node, field_index };
		self.runtime_function(
			MAP_WITHOUT,
			vec![node_ref, node_ref],
			vec![node_ref],
			vec![nullable, nullable],
			|s, f| {
				Self::emit_list(f, &s.forget_list_cursor());
				let (map, key, previous, cell) = (0, 1, 2, 3);
				let holds_key = |s: &Self, f: &mut Function, cell: u32| {
					s.emit_field(f, cell, 1);
					Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(node)), I::LocalGet(key)]);
					s.call(f, "map_entry_has_key");
					s.emit_field(f, cell, 1);
					f.instruction(&I::LocalGet(key));
					s.call(f, VALUES_EQUAL);
					f.instruction(&I::I32Or);
				};
				is_entry(s, f, map);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(map), I::LocalGet(key)]);
				s.call(f, "map_entry_has_key");
				f.instruction(&I::If(BlockType::Empty));
				s.emit_become_empty(f, map);
				Self::emit_list(f, &[I::End, I::LocalGet(map), I::Return, I::End]);
				// ø: nothing to remove
				s.emit_field(f, map, 1);
				Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Empty), I::LocalGet(map), I::Return, I::End]);
				holds_key(s, f, map);
				f.instruction(&I::If(BlockType::Empty));
				s.emit_field(f, map, 2);
				Self::emit_list(f, &[I::LocalTee(cell), I::RefIsNull, I::If(BlockType::Empty)]);
				s.emit_become_empty(f, map);
				Self::emit_list(f, &[I::Else, I::LocalGet(map)]);
				s.emit_field(f, cell, 1);
				Self::emit_list(f, &[set(1), I::LocalGet(map)]);
				s.emit_field(f, cell, 2);
				Self::emit_list(
					f,
					&[
						set(2),
						I::End,
						I::LocalGet(map),
						I::Return,
						I::End,
						I::LocalGet(map),
						I::LocalSet(previous),
						I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
					],
				);
				s.emit_field(f, previous, 2);
				Self::emit_list(f, &[I::LocalTee(cell), I::RefIsNull, I::BrIf(1)]);
				holds_key(s, f, cell);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(previous)]);
				s.emit_field(f, cell, 2);
				Self::emit_list(
					f,
					&[
						set(2),
						I::Br(2),
						I::End,
						I::LocalGet(cell),
						I::LocalSet(previous),
						I::Br(0),
						I::End,
						I::End,
						I::LocalGet(map),
					],
				);
			},
		);
		// removed_value(collection, key): what `collection.remove(key)` gives, of a map the key's value (ø if absent), of
		// a list the list itself, which map_without then changes (the static kinds don't tell a map from a list)
		self.runtime_function(
			crate::list_mutations::REMOVED_VALUE_CALL,
			vec![node_ref, node_ref],
			vec![node_ref],
			vec![],
			|s, f| {
				f.instruction(&I::LocalGet(0));
				s.call(f, IS_MAP);
				Self::emit_list(f, &[I::If(BlockType::Result(node_ref)), I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, "new_empty");
				s.call(f, MAP_GET_OR);
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End]);
			},
		);
	}
}
