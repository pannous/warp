//! A map variable built by its own entries is a hash table: `m = {}` followed by `m[k] = v` with name keys (texts,
//! symbols, characters) is a `$NodeMap`, an insertion-ordered key and value array plus open-addressing slots, so an
//! entry is set and found in O(1) instead of copying and walking the cons list (field_with, map_get). The table never
//! leaves its variable: a map is a reference (P200b), so a variable used as a whole value (`n = m`, `[m]`, an argument)
//! stays the Node every holder shares; only a bare final `m` becomes the same Node the entries would have built
//! (`{}` is ø, one entry is the entry itself, more are a curly list).
//! A lookup it cannot answer (a missing key, a key no name) takes the generic way on that copy, with its errors.

use super::list_dispatch::Slot;
use super::WasmGcEmitter;
use crate::library_words::{COLLECTION_CONTAINS, COLLECTION_POSITION};
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::type_kinds::{Kind, CURLY_LIST_KIND, KIND_MASK};
use std::collections::HashSet;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub(super) const NODE_MAP_NEW: &str = "node_map_new";
pub(super) const NODE_MAP_SET: &str = "node_map_set";
pub(super) const NODE_MAP_LOOKUP: &str = "node_map_lookup";
pub(super) const NODE_MAP_AS_NODE: &str = "node_map_as_node";
pub(super) const NODE_MAP_OF: &str = "node_map_of";
/// The hash-table copy of a map parameter, `m·map = m` (analyzer::indexed_parameter_copies)
pub const MAP_COPY_SUFFIX: &str = "·map";
const NODE_MAP_HASH: &str = "node_map_hash";
const NODE_MAP_SLOT: &str = "node_map_slot";
const NODE_MAP_RESLOT: &str = "node_map_reslot";
const NODE_MAP_REMOVE: &str = "node_map_remove";
/// The functions a program using a map variable calls; the rest come with them
pub(super) const NODE_MAP_FUNCTIONS: [&str; 6] = [NODE_MAP_NEW, NODE_MAP_SET, NODE_MAP_LOOKUP, NODE_MAP_AS_NODE, NODE_MAP_OF, NODE_MAP_REMOVE];
/// Words that read a map without holding on to it: a map variable given to them stays a hash table
const READING_WORDS: [&str; 16] = [crate::analyzer::REMOVED_VALUE_CALL, "count", "len", "size", "length", "print", "put", "string", "text", "type", "has", "contains",
	crate::library_words::MAP_KEYS, crate::library_words::MAP_VALUES, COLLECTION_CONTAINS, COLLECTION_POSITION];
/// Entries and slots of a new table; both double when full (slots when half full)
const FIRST_ENTRIES: i32 = 8;
const FIRST_SLOTS: i32 = 16;
const EMPTY_SLOT: i32 = -1;
const FNV_OFFSET: i32 = 0x811c9dc5_u32 as i32;
const FNV_PRIME: i32 = 16777619;
/// $NodeMap fields
const COUNT: u32 = 0;
const KEYS: u32 = 1;
const VALUES: u32 = 2;
const SLOTS: u32 = 3;

impl WasmGcEmitter {
	/// The map variables of `program` held as hash tables: locals assigned only `{}` and entries with name keys
	pub(super) fn find_typed_maps(&self, program: &Node) -> HashSet<String> {
		let held_as_nodes = self.names_held_as_nodes(program);
		self.map_candidates(program).into_iter()
			.filter(|name| !held_as_nodes.contains(name))
			.filter(|name| self.scope.lookup(name).is_some_and(|local| !local.is_param))
			.collect()
	}

	/// The globals held as hash tables: those main and every function use only as find_typed_maps admits a local, so a
	/// map a function fills (`global m; m[k] = v`) is set and found in O(1) too
	pub(super) fn find_typed_map_globals(&self, program: &Node, globals: &std::collections::HashMap<String, crate::local::Local>) -> HashSet<String> {
		let bodies = self.ctx.user_functions.values().map(|function| *function.body.clone());
		let whole = Node::List(std::iter::once(program.clone()).chain(bodies).collect(), Bracket::None, crate::node::Separator::Newline);
		let kept_as_nodes = self.names_kept_as_nodes(&whole);
		self.map_candidates(&whole).into_iter().filter(|name| globals.contains_key(name) && !kept_as_nodes.contains(name)).collect()
	}

	/// The variables of `program` started as a map and only set, read and tested by name keys
	fn map_candidates(&self, program: &Node) -> HashSet<String> {
		let (mut started, mut keyed, mut excluded) = (HashSet::new(), HashSet::new(), HashSet::new());
		program.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			match target.drop_meta() {
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) && is_map_start(name, value) => { started.insert(name.clone()); }
				// `m = field_with(m, "k", v)`, the lowered `m["k"] = v` of a parameter
				Node::Symbol(name) if *op == Op::Assign && entry_update(name, value).is_some_and(|(key, _)| self.map_key(&key).is_some() && !is_meta_key(&key)) => { keyed.insert(name.clone()); }
				// `m = map_without(m, k)`, the lowered `m.remove(k)`
				Node::Symbol(name) if *op == Op::Assign && entry_removal(name, value).is_some_and(|key| self.is_name_key(key)) => {}
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) || is_update(op) => { excluded.insert(name.clone()); }
				Node::Key(map, Op::Hash, index) if matches!(op, Op::Assign | Op::Define) || is_update(op) => {
					let Node::Symbol(name) = map.drop_meta() else { return };
					if matches!(op, Op::Assign) && self.map_key(index).is_some() && !is_meta_key(index) { keyed.insert(name.clone()) } else { excluded.insert(name.clone()) };
				}
				_ => {}
			}
		});
		// a map is a reference (P200b): one used whole (`n = m`, `[m]`, an argument) is the one Node every holder shares
		let mut used_whole = vec![];
		super::struct_backend::used_whole(&without_reads(super::struct_backend::without_final_variable(program)), &Default::default(), &mut used_whole);
		excluded.extend(used_whole);
		started.into_iter().filter(|name| keyed.contains(name) && !excluded.contains(name)).collect()
	}

	/// The variables a typed backend (hash table, struct) never holds: globals, captured ones and those assigned in a
	/// block that may stop at an error (`RAN_WITHOUT_ERROR`)
	pub(super) fn names_held_as_nodes(&self, program: &Node) -> HashSet<String> {
		let mut names = self.names_kept_as_nodes(program);
		names.extend(self.ctx.user_globals.keys().cloned());
		names
	}

	/// The variables no typed backend holds, globals aside: captured ones and those a block that may stop assigns
	fn names_kept_as_nodes(&self, program: &Node) -> HashSet<String> {
		let mut names = HashSet::new();
		program.visit(&mut |part| {
			if matches!(part, Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == super::RAN_WITHOUT_ERROR)) {
				part.visit(&mut |inner| if let Node::Key(target, _, _) = inner {
					if let Node::Symbol(name) = target.drop_meta() { names.insert(name.clone()); }
				});
			}
		});
		names.extend(self.ctx.captures.values().flatten().map(|(name, _)| name.clone()));
		names
	}

	/// Where `target` lives when it is a map variable held as a hash table: a local of the body, else a typed global
	pub(super) fn typed_map(&self, target: &Node) -> Option<Slot> {
		let Node::Symbol(name) = target.drop_meta() else { return None };
		match self.scope.lookup(name) {
			Some(local) => self.typed_maps.contains(name).then_some(Slot::Local(local.position)),
			None => self.typed_map_globals.get(name).map(|&global| Slot::Global(global)),
		}
	}

	/// A null-initialized mutable global holding a hash table
	pub(super) fn declare_typed_map_global(&mut self) -> u32 {
		self.declare_global_of(Ref(self.node_map_ref()), ConstExpr::ref_null(HeapType::Concrete(self.type_manager.node_map_type)))
	}

	/// `collection_contains(m, key)` and `collection_position(m, key)` (`key in m`, `m has key`) of a hash table with a
	/// name key: 1 when it holds the key, else 0; false for any other call
	pub(super) fn emit_typed_map_membership(&mut self, func: &mut Function, items: &[Node]) -> bool {
		let [word, map, key] = items else { return false };
		let is_membership = matches!(word.drop_meta(), Node::Symbol(word) if word == COLLECTION_CONTAINS || word == COLLECTION_POSITION);
		let Some(slot) = self.typed_map(map).filter(|_| is_membership && self.is_name_key(key)) else { return false };
		func.instruction(&slot.get());
		self.emit_node_instructions(func, key);
		self.emit_call(func, NODE_MAP_LOOKUP);
		Self::emit_list(func, &[I::RefIsNull, I::I32Eqz, I::I64ExtendI32U]);
		self.emit_call(func, "new_int");
		true
	}

	pub(super) fn node_map_ref(&self) -> RefType {
		RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.node_map_type) }
	}

	/// `m = {}`, `m = {a:1}`, `m·map = m`: a table in m's slot, left on the stack; `m = field_with(m, k, v)` sets the
	/// entry in place, `m = map_without(m, k)` removes it
	pub(super) fn emit_typed_map_store(&mut self, func: &mut Function, name: &str, slot: Slot, value: &Node) {
		if let Some((key, entry_value)) = entry_update(name, value) {
			let key = self.map_key(&key).expect("find_typed_maps admits only name keys");
			self.emit_typed_map_set(func, slot, &key, &entry_value);
			func.instruction(&slot.get());
		} else if let Some(key) = entry_removal(name, value) {
			func.instruction(&slot.get());
			self.emit_node_instructions(func, key);
			self.emit_call(func, NODE_MAP_REMOVE);
			func.instruction(&slot.get());
		} else if is_empty_map(value) {
			self.emit_call(func, NODE_MAP_NEW);
		} else {
			self.emit_node_instructions(func, value);
			self.emit_call(func, NODE_MAP_OF);
		}
		slot.tee(func);
	}

	/// `m[key] = value`: the entry set in place
	pub(super) fn emit_typed_map_set(&mut self, func: &mut Function, slot: Slot, key: &Node, value: &Node) {
		func.instruction(&slot.get());
		self.emit_node_instructions(func, key);
		self.emit_node_instructions(func, value);
		self.emit_call(func, NODE_MAP_SET);
	}

	/// Push the value of `m[key]` or null when the table cannot answer, inside a block the caller closes with the
	/// generic lookup: `br_on_non_null` skips it
	pub(super) fn emit_typed_map_lookup(&mut self, func: &mut Function, slot: Slot, key: &Node) {
		func.instruction(&slot.get());
		self.emit_node_instructions(func, key);
		self.emit_call(func, NODE_MAP_LOOKUP);
		func.instruction(&I::BrOnNonNull(0));
	}

	pub(super) fn emit_typed_map_count(&self, func: &mut Function, slot: Slot) {
		let map = self.type_manager.node_map_type;
		Self::emit_list(func, &[slot.get(), I::StructGet { struct_type_index: map, field_index: COUNT }, I::I64ExtendI32U]);
	}

	pub(super) fn emit_typed_map_as_node(&mut self, func: &mut Function, slot: Slot) {
		func.instruction(&slot.get());
		self.emit_call(func, NODE_MAP_AS_NODE);
	}

	/// The runtime functions of the hash tables, after map_get (they compare keys as map_key_name makes them)
	pub(super) fn emit_node_map_runtime(&mut self) {
		if !NODE_MAP_FUNCTIONS.iter().any(|name| self.should_emit_function(name)) {
			return;
		}
		let (node, string) = (self.type_manager.node_type, self.type_manager.string_type);
		let (map, array, slots) = (self.type_manager.node_map_type, self.type_manager.node_array_type, self.type_manager.limbs_type);
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let map_ref = Ref(self.node_map_ref());
		let slots_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(slots) });
		let array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let int = ValType::I32;
		let field = |f: &mut Function, local: u32, index: u32| Self::emit_list(f, &[I::LocalGet(local), I::StructGet { struct_type_index: map, field_index: index }]);
		let null_node = I::RefNull(HeapType::Concrete(node));

		self.runtime_function(NODE_MAP_NEW, vec![], vec![map_ref], vec![], |_, f| {
			Self::emit_list(f, &[I::I32Const(0)]);
			for _ in [KEYS, VALUES] {
				Self::emit_list(f, &[null_node.clone(), I::I32Const(FIRST_ENTRIES), I::ArrayNew(array)]);
			}
			Self::emit_list(f, &[I::I32Const(EMPTY_SLOT), I::I32Const(FIRST_SLOTS), I::ArrayNew(slots), I::StructNew(map)]);
		});

		// FNV-1a of the letters of a name
		self.runtime_function(NODE_MAP_HASH, vec![node_ref], vec![int], vec![int, int, int], |s, f| {
			let (pointer, length, hash) = (1, 2, 3);
			for (index, local) in [(0, pointer), (1, length)] {
				s.emit_field(f, 0, 1);
				Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(string)), I::StructGet { struct_type_index: string, field_index: index }, I::LocalSet(local)]);
			}
			Self::emit_list(f, &[I::I32Const(FNV_OFFSET), I::LocalSet(hash), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(length), I::I32Eqz, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(hash), I::LocalGet(pointer), I::I32Load8U(MemArg { offset: 0, align: 0, memory_index: 0 }), I::I32Xor]);
			Self::emit_list(f, &[I::I32Const(FNV_PRIME), I::I32Mul, I::LocalSet(hash)]);
			Self::emit_list(f, &[I::LocalGet(pointer), I::I32Const(1), I::I32Add, I::LocalSet(pointer)]);
			Self::emit_list(f, &[I::LocalGet(length), I::I32Const(1), I::I32Sub, I::LocalSet(length), I::Br(0), I::End, I::End, I::LocalGet(hash)]);
		});

		// the slot of a name: the one holding its entry, else the empty one where it would go
		self.runtime_function(NODE_MAP_SLOT, vec![map_ref, node_ref], vec![int], vec![slots_ref, int, int, int], |s, f| {
			let (table, mask, slot, entry) = (2, 3, 4, 5);
			field(f, 0, SLOTS);
			Self::emit_list(f, &[I::LocalTee(table), I::ArrayLen, I::I32Const(1), I::I32Sub, I::LocalSet(mask), I::LocalGet(1)]);
			s.call(f, NODE_MAP_HASH);
			Self::emit_list(f, &[I::LocalGet(mask), I::I32And, I::LocalSet(slot), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(table), I::LocalGet(slot), I::ArrayGet(slots), I::LocalTee(entry), I::I32Const(EMPTY_SLOT), I::I32Eq]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(slot), I::Return, I::End]);
			field(f, 0, KEYS);
			Self::emit_list(f, &[I::LocalGet(entry), I::ArrayGet(array), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 1 }]);
			s.emit_field(f, 1, 1);
			s.call(f, super::VALUES_EQUAL);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(slot), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(slot), I::I32Const(1), I::I32Add, I::LocalGet(mask), I::I32And, I::LocalSet(slot), I::Br(0), I::End, I::Unreachable]);
		});

		// node_map_reslot(map, slots): that many slots, every entry placed again
		self.runtime_function(NODE_MAP_RESLOT, vec![map_ref, int], vec![], vec![int], |s, f| {
			let entry = 2;
			Self::emit_list(f, &[I::LocalGet(0), I::I32Const(EMPTY_SLOT), I::LocalGet(1), I::ArrayNew(slots), I::StructSet { struct_type_index: map, field_index: SLOTS }]);
			Self::emit_list(f, &[I::I32Const(0), I::LocalSet(entry), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(entry)]);
			field(f, 0, COUNT);
			Self::emit_list(f, &[I::I32GeU, I::BrIf(1)]);
			field(f, 0, SLOTS);
			Self::emit_list(f, &[I::LocalGet(0)]);
			field(f, 0, KEYS);
			Self::emit_list(f, &[I::LocalGet(entry), I::ArrayGet(array), I::RefAsNonNull]);
			s.call(f, NODE_MAP_SLOT);
			Self::emit_list(f, &[I::LocalGet(entry), I::ArraySet(slots)]);
			Self::emit_list(f, &[I::LocalGet(entry), I::I32Const(1), I::I32Add, I::LocalSet(entry), I::Br(0), I::End, I::End]);
		});

		// node_map_set(map, key, value): the entry of that name set, or added after the others
		self.runtime_function(NODE_MAP_SET, vec![map_ref, node_ref, node_ref], vec![], vec![nullable, int, int, int, array_ref], |s, f| {
			let (name, slot, entry, count, grown) = (3, 4, 5, 6, 7);
			f.instruction(&I::LocalGet(1));
			s.call(f, super::list_ops::MAP_KEY_NAME);
			f.instruction(&I::LocalSet(name));
			s.emit_require_kind(f, name, Kind::Symbol, "not_a_text");
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(name), I::RefAsNonNull]);
			s.call(f, NODE_MAP_SLOT);
			f.instruction(&I::LocalSet(slot));
			let slots_of_map = |f: &mut Function| field(f, 0, SLOTS);
			slots_of_map(f);
			Self::emit_list(f, &[I::LocalGet(slot), I::ArrayGet(slots), I::LocalTee(entry), I::I32Const(EMPTY_SLOT), I::I32Ne, I::If(BlockType::Empty)]);
			field(f, 0, VALUES);
			Self::emit_list(f, &[I::LocalGet(entry), I::LocalGet(2), I::ArraySet(array), I::Return, I::End]);
			// a new entry: room for it, then key, value and slot
			field(f, 0, COUNT);
			f.instruction(&I::LocalTee(count));
			field(f, 0, KEYS);
			Self::emit_list(f, &[I::ArrayLen, I::I32Eq, I::If(BlockType::Empty)]);
			for part in [KEYS, VALUES] {
				Self::emit_list(f, &[null_node.clone(), I::LocalGet(count), I::I32Const(1), I::I32Shl, I::ArrayNew(array), I::LocalTee(grown), I::I32Const(0)]);
				field(f, 0, part);
				Self::emit_list(f, &[I::I32Const(0), I::LocalGet(count), I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array }]);
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(grown), I::RefAsNonNull, I::StructSet { struct_type_index: map, field_index: part }]);
			}
			f.instruction(&I::End);
			for (part, value) in [(KEYS, name), (VALUES, 2)] {
				field(f, 0, part);
				Self::emit_list(f, &[I::LocalGet(count), I::LocalGet(value), I::ArraySet(array)]);
			}
			slots_of_map(f);
			Self::emit_list(f, &[I::LocalGet(slot), I::LocalGet(count), I::ArraySet(slots)]);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(count), I::I32Const(1), I::I32Add, I::LocalTee(count), I::StructSet { struct_type_index: map, field_index: COUNT }]);
			Self::emit_list(f, &[I::LocalGet(count), I::I32Const(1), I::I32Shl]);
			slots_of_map(f);
			Self::emit_list(f, &[I::ArrayLen, I::I32GtU, I::If(BlockType::Empty), I::LocalGet(0)]);
			slots_of_map(f);
			Self::emit_list(f, &[I::ArrayLen, I::I32Const(1), I::I32Shl]);
			s.call(f, NODE_MAP_RESLOT);
			f.instruction(&I::End);
		});

		// the entry of the name of key local 1 into `entry`, the name into `name`; else `missing`, returned
		let find_entry = |s: &mut Self, f: &mut Function, name: u32, entry: u32, missing: &[I<'static>]| {
			let return_missing = [&[I::If(BlockType::Empty)], missing, &[I::Return, I::End]].concat();
			f.instruction(&I::LocalGet(1));
			s.call(f, super::list_ops::MAP_KEY_NAME);
			f.instruction(&I::LocalSet(name));
			s.emit_field(f, name, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Symbol as i64), I::I64Ne]);
			Self::emit_list(f, &return_missing);
			field(f, 0, SLOTS);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(name), I::RefAsNonNull]);
			s.call(f, NODE_MAP_SLOT);
			Self::emit_list(f, &[I::ArrayGet(slots), I::LocalTee(entry), I::I32Const(EMPTY_SLOT), I::I32Eq]);
			Self::emit_list(f, &return_missing);
		};

		// node_map_remove(map, key): the entry of that name gone, the later ones moved up a place; the slots placed again
		self.runtime_function(NODE_MAP_REMOVE, vec![map_ref, node_ref], vec![], vec![nullable, int, int], |s, f| {
			let (name, entry, count) = (2, 3, 4);
			find_entry(s, f, name, entry, &[]);
			field(f, 0, COUNT);
			Self::emit_list(f, &[I::I32Const(1), I::I32Sub, I::LocalSet(count)]);
			for part in [KEYS, VALUES] {
				field(f, 0, part);
				f.instruction(&I::LocalGet(entry));
				field(f, 0, part);
				Self::emit_list(f, &[I::LocalGet(entry), I::I32Const(1), I::I32Add, I::LocalGet(count), I::LocalGet(entry), I::I32Sub]);
				Self::emit_list(f, &[I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array }]);
				field(f, 0, part);
				Self::emit_list(f, &[I::LocalGet(count), null_node.clone(), I::ArraySet(array)]);
			}
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(count), I::StructSet { struct_type_index: map, field_index: COUNT }, I::LocalGet(0)]);
			field(f, 0, SLOTS);
			f.instruction(&I::ArrayLen);
			s.call(f, NODE_MAP_RESLOT);
		});

		// node_map_lookup(map, key): the value of the entry of that name; null when there is none or the key is no name
		self.runtime_function(NODE_MAP_LOOKUP, vec![map_ref, node_ref], vec![nullable], vec![nullable, int], |s, f| {
			let (name, entry) = (2, 3);
			find_entry(s, f, name, entry, &[null_node.clone()]);
			field(f, 0, VALUES);
			Self::emit_list(f, &[I::LocalGet(entry), I::ArrayGet(array)]);
		});

		// node_map_of(node): the table of a map Node: ø none, an entry `k:v` one, a curly list its entries (a missing
		// value as ø); anything else is no map
		self.runtime_function(NODE_MAP_OF, vec![node_ref], vec![map_ref], vec![map_ref, nullable, nullable, ValType::I64], |s, f| {
			let (table, cell, entry, kind) = (1, 2, 3, 4);
			let add_entry = |s: &Self, f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(table), I::LocalGet(entry), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 1 }, I::RefCastNonNull(HeapType::Concrete(node))]);
				Self::emit_list(f, &[I::LocalGet(entry), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 2 }, I::RefIsNull, I::If(BlockType::Result(node_ref))]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Else, I::LocalGet(entry), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 2 }, I::RefAsNonNull, I::End]);
				s.call(f, NODE_MAP_SET);
			};
			let kind_is = |f: &mut Function, local: u32, expected: i64| {
				Self::emit_list(f, &[I::LocalGet(local), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 0 }, I::I64Const(KIND_MASK), I::I64And, I::I64Const(expected), I::I64Eq]);
			};
			s.call(f, NODE_MAP_NEW);
			f.instruction(&I::LocalSet(table));
			Self::emit_list(f, &[I::LocalGet(0), I::StructGet { struct_type_index: node, field_index: 0 }, I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::Empty as i64), I::I64Eq]);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(table), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::Key as i64), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(0), I::LocalSet(entry)]);
			add_entry(s, f);
			Self::emit_list(f, &[I::LocalGet(table), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::If(BlockType::Empty)]);
			s.call(f, "not_an_object");
			Self::emit_list(f, &[I::End, I::LocalGet(0), I::LocalSet(cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(cell), I::RefIsNull, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(cell), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 1 }, I::RefCastNullable(HeapType::Concrete(node)), I::LocalTee(entry), I::RefIsNull]);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			kind_is(f, entry, Kind::Key as i64);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			s.call(f, "not_an_object");
			f.instruction(&I::End);
			add_entry(s, f);
			f.instruction(&I::End);
			Self::emit_list(f, &[I::LocalGet(cell), I::RefAsNonNull, I::StructGet { struct_type_index: node, field_index: 2 }, I::LocalSet(cell), I::Br(0), I::End, I::End, I::LocalGet(table)]);
		});

		// node_map_as_node(map): ø, the one entry, or the curly list of the entries in their order
		let colon = crate::operators::op_to_code(&Op::Colon);
		self.runtime_function(NODE_MAP_AS_NODE, vec![map_ref], vec![node_ref], vec![int, nullable], |s, f| {
			let (entry, rest) = (1, 2);
			let entry_node = |s: &Self, f: &mut Function| {
				for part in [KEYS, VALUES] {
					field(f, 0, part);
					Self::emit_list(f, &[I::LocalGet(entry), I::ArrayGet(array), I::RefAsNonNull]);
				}
				f.instruction(&I::I64Const(colon));
				s.call(f, "new_key");
			};
			field(f, 0, COUNT);
			Self::emit_list(f, &[I::LocalTee(entry), I::I32Eqz, I::If(BlockType::Empty)]);
			s.call(f, "new_empty");
			Self::emit_list(f, &[I::Return, I::End, I::LocalGet(entry), I::I32Const(1), I::I32Eq, I::If(BlockType::Empty), I::I32Const(0), I::LocalSet(entry)]);
			entry_node(s, f);
			Self::emit_list(f, &[I::Return, I::End, I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(entry), I::I32Eqz, I::BrIf(1), I::LocalGet(entry), I::I32Const(1), I::I32Sub, I::LocalSet(entry), I::I64Const(CURLY_LIST_KIND)]);
			entry_node(s, f);
			Self::emit_list(f, &[I::LocalGet(rest), I::StructNew(node), I::LocalSet(rest), I::Br(0), I::End, I::End, I::LocalGet(rest), I::RefAsNonNull]);
		});
	}
}

/// A meta entry `@name` (meta_entries.rs) is no field: a map with one keeps the generic way
fn is_meta_key(index: &Node) -> bool {
	let key = crate::warp_parser::subscript_key(index).unwrap_or(index);
	matches!(key.drop_meta(), Node::Text(name) | Node::Symbol(name) if name.starts_with(crate::node::ATTRIBUTE_MARK))
}

pub(super) fn is_update(op: &Op) -> bool {
	op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)
}

/// `field_with(m, key, value)` updating the variable m itself: the key (as an index, `key + 1` like `m[key]`) and value
pub(super) fn entry_update(name: &str, value: &Node) -> Option<(Node, Node)> {
	let Node::List(items, _, _) = value.drop_meta() else { return None };
	let [_, _, key, entry_value] = items.as_slice() else { return None };
	crate::library_words::is_field_update_of(name, value).then(|| (Node::Key(Box::new(key.clone()), Op::Add, Box::new(crate::node::int(1))), entry_value.clone()))
}

/// `map_without(m, key)` removing an entry of the variable m itself: the key
pub(super) fn entry_removal<'a>(name: &str, value: &'a Node) -> Option<&'a Node> {
	let Node::List(items, _, _) = value.drop_meta() else { return None };
	let [word, map, key] = items.as_slice() else { return None };
	(word.name() == crate::library_words::MAP_WITHOUT && matches!(map.drop_meta(), Node::Symbol(map) if map == name)).then_some(key)
}

/// What a map variable held as a table may start from: `{}`, a map literal of name keys, or the parameter it copies
fn is_map_start(name: &str, value: &Node) -> bool {
	let is_entry = |item: &Node| matches!(item.drop_meta(), Node::Key(key, Op::Colon, _) if matches!(key.drop_meta(), Node::Symbol(key) | Node::Text(key) if !key.starts_with(crate::node::ATTRIBUTE_MARK)));
	match value.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.iter().all(is_entry),
		Node::Symbol(copied) => name.strip_suffix(MAP_COPY_SUFFIX) == Some(copied.as_str()),
		_ => false,
	}
}

fn is_empty_map(value: &Node) -> bool {
	matches!(value.drop_meta(), Node::List(items, Bracket::Curly, _) if items.is_empty())
}

/// The program without the variables only read by a reading word or a comparison (`count(m)`, `m.count`, `m == n`):
/// what is left of a variable is a use that may share it
fn without_reads(node: Node) -> Node {
	let is_variable = |node: &Node| matches!(node.drop_meta(), Node::Symbol(_));
	let is_reading_word = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if READING_WORDS.contains(&word.as_str()));
	let read = |node: Node| if is_variable(&node) { Node::Empty } else { without_reads(node) };
	match node {
		Node::List(items, bracket, separator) if items.first().is_some_and(is_reading_word) => {
			let mut items = items.into_iter();
			let word = items.next().expect("guarded");
			Node::List(std::iter::once(word).chain(items.map(read)).collect(), bracket, separator)
		}
		// `m same n` asks for the one Node
		Node::Key(left, op, right) if op.is_comparison() && !matches!(op, Op::Identical | Op::NotIdentical) => Node::Key(Box::new(read(*left)), op, Box::new(read(*right))),
		Node::Key(object, Op::Dot, member) if is_reading_word(&member) => Node::Key(Box::new(read(*object)), Op::Dot, member),
		other => other.map_children(without_reads),
	}
}
