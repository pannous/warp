//! A map variable built by its own entries is a hash table: `m = {}` followed by `m[k] = v` with name keys (texts,
//! symbols, characters) is a `$NodeMap`, an insertion-ordered key and value array plus open-addressing slots, so an
//! entry is set and found in O(1) instead of copying and walking the cons list (field_with, map_get). The table never
//! leaves its variable: wherever the program needs the map as a value it becomes the same Node the entries would have
//! built (`{}` is ø, one entry is the entry itself, more are a curly list), a fresh copy, so value semantics hold.
//! A lookup it cannot answer (a missing key, a key no name) takes the generic way on that copy, with its errors.

use super::WasmGcEmitter;
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
const NODE_MAP_HASH: &str = "node_map_hash";
const NODE_MAP_SLOT: &str = "node_map_slot";
const NODE_MAP_REHASH: &str = "node_map_rehash";
/// The functions a program using a map variable calls; the rest come with them
pub(super) const NODE_MAP_FUNCTIONS: [&str; 4] = [NODE_MAP_NEW, NODE_MAP_SET, NODE_MAP_LOOKUP, NODE_MAP_AS_NODE];
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
		let (mut started, mut keyed, mut excluded) = (HashSet::new(), HashSet::new(), HashSet::new());
		program.visit(&mut |part| {
			let Node::Key(target, op, value) = part else { return };
			match target.drop_meta() {
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) && is_empty_map(value) => { started.insert(name.clone()); }
				Node::Symbol(name) if matches!(op, Op::Assign | Op::Define) || is_update(op) => { excluded.insert(name.clone()); }
				Node::Key(map, Op::Hash, index) if matches!(op, Op::Assign | Op::Define) || is_update(op) => {
					let Node::Symbol(name) = map.drop_meta() else { return };
					if matches!(op, Op::Assign) && self.map_key(index).is_some() { keyed.insert(name.clone()) } else { excluded.insert(name.clone()) };
				}
				_ => {}
			}
		});
		program.visit(&mut |part| {
			if matches!(part, Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == super::RAN_WITHOUT_ERROR)) {
				part.visit(&mut |inner| if let Node::Key(target, _, _) = inner {
					if let Node::Symbol(name) = target.drop_meta() { excluded.insert(name.clone()); }
				});
			}
		});
		excluded.extend(self.ctx.user_globals.keys().cloned());
		excluded.extend(self.ctx.captures.values().flatten().map(|(name, _)| name.clone()));
		started.into_iter()
			.filter(|name| keyed.contains(name) && !excluded.contains(name))
			.filter(|name| self.scope.lookup(name).is_some_and(|local| !local.is_param))
			.collect()
	}

	/// The local slot of `target` when it is a map variable held as a hash table
	pub(super) fn typed_map(&self, target: &Node) -> Option<u32> {
		let Node::Symbol(name) = target.drop_meta() else { return None };
		self.typed_maps.contains(name).then(|| self.scope.lookup(name).map(|local| local.position)).flatten()
	}

	pub(super) fn node_map_ref(&self) -> RefType {
		RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.node_map_type) }
	}

	/// `m = {}`: a new table in m's local, left on the stack
	pub(super) fn emit_typed_map_store(&mut self, func: &mut Function, slot: u32) {
		self.emit_call(func, NODE_MAP_NEW);
		func.instruction(&I::LocalTee(slot));
	}

	/// `m[key] = value`: the entry set in place
	pub(super) fn emit_typed_map_set(&mut self, func: &mut Function, slot: u32, key: &Node, value: &Node) {
		func.instruction(&I::LocalGet(slot));
		self.emit_node_instructions(func, key);
		self.emit_node_instructions(func, value);
		self.emit_call(func, NODE_MAP_SET);
	}

	/// Push the value of `m[key]` or null when the table cannot answer, inside a block the caller closes with the
	/// generic lookup: `br_on_non_null` skips it
	pub(super) fn emit_typed_map_lookup(&mut self, func: &mut Function, slot: u32, key: &Node) {
		func.instruction(&I::LocalGet(slot));
		self.emit_node_instructions(func, key);
		self.emit_call(func, NODE_MAP_LOOKUP);
		func.instruction(&I::BrOnNonNull(0));
	}

	pub(super) fn emit_typed_map_count(&self, func: &mut Function, slot: u32) {
		let map = self.type_manager.node_map_type;
		Self::emit_list(func, &[I::LocalGet(slot), I::StructGet { struct_type_index: map, field_index: COUNT }, I::I64ExtendI32U]);
	}

	pub(super) fn emit_typed_map_as_node(&mut self, func: &mut Function, slot: u32) {
		func.instruction(&I::LocalGet(slot));
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

		// twice the slots, every entry placed again
		self.runtime_function(NODE_MAP_REHASH, vec![map_ref], vec![], vec![int], |s, f| {
			let entry = 1;
			Self::emit_list(f, &[I::LocalGet(0), I::I32Const(EMPTY_SLOT)]);
			field(f, 0, SLOTS);
			Self::emit_list(f, &[I::ArrayLen, I::I32Const(1), I::I32Shl, I::ArrayNew(slots), I::StructSet { struct_type_index: map, field_index: SLOTS }]);
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
			s.call(f, NODE_MAP_REHASH);
			f.instruction(&I::End);
		});

		// node_map_lookup(map, key): the value of the entry of that name; null when there is none or the key is no name
		self.runtime_function(NODE_MAP_LOOKUP, vec![map_ref, node_ref], vec![nullable], vec![nullable, int], |s, f| {
			let (name, entry) = (2, 3);
			f.instruction(&I::LocalGet(1));
			s.call(f, super::list_ops::MAP_KEY_NAME);
			f.instruction(&I::LocalSet(name));
			s.emit_field(f, name, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Symbol as i64), I::I64Ne, I::If(BlockType::Empty), null_node.clone(), I::Return, I::End]);
			field(f, 0, SLOTS);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(name), I::RefAsNonNull]);
			s.call(f, NODE_MAP_SLOT);
			Self::emit_list(f, &[I::ArrayGet(slots), I::LocalTee(entry), I::I32Const(EMPTY_SLOT), I::I32Eq, I::If(BlockType::Empty), null_node.clone(), I::Return, I::End]);
			field(f, 0, VALUES);
			Self::emit_list(f, &[I::LocalGet(entry), I::ArrayGet(array)]);
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

fn is_update(op: &Op) -> bool {
	op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)
}

fn is_empty_map(value: &Node) -> bool {
	matches!(value.drop_meta(), Node::List(items, Bracket::Curly, _) if items.is_empty())
}
