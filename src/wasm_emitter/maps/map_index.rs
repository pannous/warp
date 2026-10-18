//! The index of a map held as a cons list (a parameter, a returned map, an object): a map a lookup or field set walked
//! last time and walks again gets a `$NodeMap` from each entry's name to the entry, so `m[k]` and `m[k] = v` in a loop
//! take constant time instead of walking the entries each time (quadratic). The index lives in globals, newest slot
//! first, and stays right while only field sets (the entries are kept) and fields_grow (which adds to it) change the
//! map; anything else changing a list's links or items in place forgets every index (forget_map_indexes).

use super::super::WasmGcEmitter;
use crate::type_kinds::{Kind, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub(in crate::wasm_emitter) const MAP_INDEX_ENTRY: &str = "map_index_entry";
pub(in crate::wasm_emitter) const MAP_INDEX_GROW: &str = "map_index_grow";
const MAP_INDEX_OF: &str = "map_index_of";
const MAP_INDEX_ADD: &str = "map_index_add";
/// Maps indexed at once: a loop copying one map into another (`b[k] = a[k]`) keeps both
const MAP_INDEXES: u32 = 2;
/// A map with fewer entries is walked: objects like `{x:1 y:2}` read field by field need no index
const MIN_INDEXED_ENTRIES: i32 = 16;
/// The globals of an index slot: the map (its first cell), its index, its last cell (null when it has meta entries,
/// which a new field goes in front of), and the map walked last without an index (the next walk of it builds one)
#[derive(Clone, Copy)]
enum IndexPart {
	Owner,
	Index,
	Tail,
	Seen,
}
const INDEX_PARTS: [(IndexPart, &str); 4] =
	[(IndexPart::Owner, "owner"), (IndexPart::Index, "index"), (IndexPart::Tail, "tail"), (IndexPart::Seen, "seen")];
const KEY_KIND: i64 = Kind::Key as i64;

impl WasmGcEmitter {
	/// Programs finding or setting entries of a map node
	pub(in crate::wasm_emitter) fn uses_map_lookups(&self) -> bool {
		["map_get", "map_find", "field_with"].iter().any(|name| self.should_emit_function(name))
	}

	pub(in crate::wasm_emitter) fn emit_map_index_globals(&mut self) {
		if self.map_index_global.is_some() || !self.uses_map_lookups() {
			return;
		}
		self.map_index_global = Some(self.next_global_idx);
		for slot in 0..MAP_INDEXES {
			for (part, name) in INDEX_PARTS {
				let (val_type, heap) = match part {
					IndexPart::Index => (Ref(self.node_map_ref()), HeapType::Concrete(self.type_manager.node_map_type)),
					_ => (Ref(self.node_ref(true)), HeapType::Concrete(self.type_manager.node_type)),
				};
				self.declare_named_global(val_type, ConstExpr::ref_null(heap), format!("map_index_{slot}_{name}"));
			}
		}
	}

	fn map_index_part(&self, slot: u32, part: IndexPart) -> u32 {
		self.map_index_global.expect("map index globals") + slot * INDEX_PARTS.len() as u32 + part as u32
	}

	/// The instructions forgetting every map index, before a list's links or items change in place
	pub(in crate::wasm_emitter) fn forget_map_indexes(&self) -> Vec<I<'static>> {
		let Some(_) = self.map_index_global else { return vec![] };
		let null = I::RefNull(HeapType::Concrete(self.type_manager.node_type));
		(0..MAP_INDEXES)
			.flat_map(|slot| [null.clone(), I::GlobalSet(self.map_index_part(slot, IndexPart::Owner))])
			.collect()
	}

	/// The runtime functions of the index, after node_map_set and node_map_lookup they call
	pub(in crate::wasm_emitter) fn emit_map_index(&mut self) {
		let node = self.type_manager.node_type;
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let map_ref = Ref(self.node_map_ref());
		let null_node = I::RefNull(HeapType::Concrete(node));
		let kind_of = |f: &mut Function| {
			Self::emit_list(
				f,
				&[I::StructGet { struct_type_index: node, field_index: 0 }, I::I64Const(KIND_MASK), I::I64And],
			)
		};
		let [owner, index, tail, seen] = [IndexPart::Owner, IndexPart::Index, IndexPart::Tail, IndexPart::Seen]
			.map(|part| self.map_index_part(0, part));
		let [other_owner, _, _, other_seen] = [IndexPart::Owner, IndexPart::Index, IndexPart::Tail, IndexPart::Seen]
			.map(|part| self.map_index_part(1, part));
		// slot 0 and 1 trade places, the newest first
		let swap: Vec<I> = INDEX_PARTS
			.iter()
			.flat_map(|&(part, _)| {
				let (first, second) = (self.map_index_part(0, part), self.map_index_part(1, part));
				[I::GlobalGet(first), I::GlobalGet(second), I::GlobalSet(first), I::GlobalSet(second)]
			})
			.collect();

		// map_index_add(index, item): an entry `name:value` found by its name, unless an earlier one has that name
		self.runtime_function(
			MAP_INDEX_ADD,
			vec![map_ref, Self::any_ref()],
			vec![],
			vec![nullable, nullable],
			|s, f| {
				let (table, item, entry, name) = (0, 1, 2, 3);
				let return_unless =
					|f: &mut Function| Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::Return, I::End]);
				Self::emit_list(f, &[I::LocalGet(item), I::RefTestNonNull(HeapType::Concrete(node))]);
				return_unless(f);
				Self::emit_list(
					f,
					&[I::LocalGet(item), I::RefCastNonNull(HeapType::Concrete(node)), I::LocalTee(entry)],
				);
				kind_of(f);
				Self::emit_list(f, &[I::I64Const(KEY_KIND), I::I64Eq]);
				return_unless(f);
				s.emit_field(f, entry, 1);
				Self::emit_list(f, &[I::RefTestNonNull(HeapType::Concrete(node))]);
				return_unless(f);
				s.emit_field(f, entry, 1);
				f.instruction(&I::RefCastNonNull(HeapType::Concrete(node)));
				s.call(f, super::super::lists::list_ops::MAP_KEY_NAME);
				f.instruction(&I::LocalTee(name));
				kind_of(f);
				Self::emit_list(f, &[I::I64Const(Kind::Symbol as i64), I::I64Eq]);
				return_unless(f);
				Self::emit_list(f, &[I::LocalGet(table), I::LocalGet(name), I::RefAsNonNull]);
				s.call(f, super::super::maps::map_backend::NODE_MAP_LOOKUP);
				Self::emit_list(f, &[I::RefIsNull]);
				return_unless(f);
				Self::emit_list(
					f,
					&[I::LocalGet(table), I::LocalGet(name), I::RefAsNonNull, I::LocalGet(entry), I::RefAsNonNull],
				);
				s.call(f, super::super::maps::map_backend::NODE_MAP_SET);
			},
		);

		// map_index_of(fields) -> indexed: whether slot 0 now holds the index of the map; built on the second walk of a
		// map with enough entries
		self.runtime_function(
			MAP_INDEX_OF,
			vec![node_ref],
			vec![ValType::I32],
			vec![nullable, nullable, ValType::I32, ValType::I32],
			|s, f| {
				let (fields, cell, last, meta, count) = (0, 1, 2, 3, 4);
				let return_when = |f: &mut Function, result: i32| {
					Self::emit_list(f, &[I::If(BlockType::Empty), I::I32Const(result), I::Return, I::End])
				};
				Self::emit_list(f, &[I::LocalGet(fields), I::GlobalGet(owner), I::RefEq]);
				return_when(f, 1);
				Self::emit_list(
					f,
					&[I::LocalGet(fields), I::GlobalGet(other_owner), I::RefEq, I::If(BlockType::Empty)],
				);
				Self::emit_list(f, &swap);
				Self::emit_list(f, &[I::I32Const(1), I::Return, I::End, I::LocalGet(fields)]);
				kind_of(f);
				Self::emit_list(f, &[I::I64Const(Kind::List as i64), I::I64Ne]);
				return_when(f, 0);
				// walked for the first time lately: remembered
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(fields), I::GlobalGet(seen), I::RefEq, I::LocalGet(fields), I::GlobalGet(other_seen),
					I::RefEq, I::I32Or, I::I32Eqz, I::If(BlockType::Empty),
						I::GlobalGet(seen), I::GlobalSet(other_seen),
						I::LocalGet(fields), I::GlobalSet(seen),
						I::I32Const(0), I::Return,
					I::End,
					I::LocalGet(fields), I::LocalSet(cell),
				]);
				s.emit_cell_walk(f, cell, |f| {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(count), I::I32Const(1), I::I32Add, I::LocalTee(count),
						I::I32Const(MIN_INDEXED_ENTRIES), I::I32GeU, I::BrIf(1),
					]);
				});
				Self::emit_list(f, &[I::LocalGet(count), I::I32Const(MIN_INDEXED_ENTRIES), I::I32LtU]);
				return_when(f, 0);
				// the older index moves to slot 1, the new one goes in slot 0
				for part in [IndexPart::Owner, IndexPart::Index, IndexPart::Tail] {
					Self::emit_list(
						f,
						&[I::GlobalGet(s.map_index_part(0, part)), I::GlobalSet(s.map_index_part(1, part))],
					);
				}
				Self::emit_list(f, &[I::LocalGet(fields), I::GlobalSet(owner)]);
				s.call(f, super::super::maps::map_backend::NODE_MAP_NEW);
				Self::emit_list(f, &[I::GlobalSet(index), I::LocalGet(fields), I::LocalSet(cell)]);
				s.emit_cell_walk(f, cell, |f| {
					f.instruction(&I::GlobalGet(index));
					s.emit_field(f, cell, 1);
					s.call(f, MAP_INDEX_ADD);
					s.emit_field(f, cell, 1);
					s.call(f, super::super::reflection::equality::IS_META_ENTRY);
					Self::emit_list(
						f,
						&[I::LocalGet(meta), I::I32Or, I::LocalSet(meta), I::LocalGet(cell), I::LocalSet(last)],
					);
				});
				Self::emit_list(
					f,
					&[
						I::LocalGet(meta),
						I::If(BlockType::Result(nullable)),
						null_node.clone(),
						I::Else,
						I::LocalGet(last),
						I::End,
						I::GlobalSet(tail),
						I::I32Const(1),
					],
				);
			},
		);

		// map_index_entry(fields, key) -> (entry, indexed): the entry of that name from the map's index, null when it has
		// none; not indexed (0) when the map has no index or the key is no name, so the caller walks the entries
		self.runtime_function(
			MAP_INDEX_ENTRY,
			vec![node_ref, node_ref],
			vec![nullable, ValType::I32],
			vec![nullable],
			|s, f| {
				let (fields, key, name) = (0, 1, 2);
				let unindexed = [I::If(BlockType::Empty), null_node.clone(), I::I32Const(0), I::Return, I::End];
				f.instruction(&I::LocalGet(fields));
				s.call(f, MAP_INDEX_OF);
				f.instruction(&I::I32Eqz);
				Self::emit_list(f, &unindexed);
				f.instruction(&I::LocalGet(key));
				s.call(f, super::super::lists::list_ops::MAP_KEY_NAME);
				f.instruction(&I::LocalTee(name));
				kind_of(f);
				Self::emit_list(f, &[I::I64Const(Kind::Symbol as i64), I::I64Ne]);
				Self::emit_list(f, &unindexed);
				Self::emit_list(f, &[I::GlobalGet(index), I::LocalGet(name), I::RefAsNonNull]);
				s.call(f, super::super::maps::map_backend::NODE_MAP_LOOKUP);
				f.instruction(&I::I32Const(1));
			},
		);

		// map_index_grow(fields, entry) -> appended: a new field of an indexed map goes in its index, and after its last
		// cell when it has no meta entries; else (0) fields_grow places it
		self.runtime_function(MAP_INDEX_GROW, vec![node_ref, node_ref], vec![ValType::I32], vec![nullable], |s, f| {
			let (fields, entry, made) = (0, 1, 2);
			let tail_cell = [I::GlobalGet(tail), I::RefAsNonNull];
			Self::emit_list(
				f,
				&[
					I::LocalGet(fields),
					I::GlobalGet(other_owner),
					I::RefEq,
					I::If(BlockType::Empty),
					null_node.clone(),
					I::GlobalSet(other_owner),
					I::End,
					I::LocalGet(fields),
					I::GlobalGet(owner),
					I::RefEq,
					I::I32Eqz,
					I::If(BlockType::Empty),
					I::I32Const(0),
					I::Return,
					I::End,
					I::GlobalGet(index),
					I::RefAsNonNull,
					I::LocalGet(entry),
				],
			);
			s.call(f, MAP_INDEX_ADD);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::GlobalGet(tail), I::RefIsNull, I::If(BlockType::Empty),
					I::I32Const(0), I::Return,
				I::End,
				I::LocalGet(entry),
			]);
			s.call(f, super::super::reflection::equality::IS_META_ENTRY);
			Self::emit_list(
				f,
				&[I::If(BlockType::Empty), null_node.clone(), I::GlobalSet(tail), I::I32Const(0), I::Return, I::End],
			);
			Self::emit_list(f, &tail_cell);
			Self::emit_list(
				f,
				&[
					I::StructGet { struct_type_index: node, field_index: 0 },
					I::LocalGet(entry),
					null_node.clone(),
					I::StructNew(node),
					I::LocalSet(made),
				],
			);
			Self::emit_list(f, &tail_cell);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(made), I::StructSet { struct_type_index: node, field_index: 2 },
				I::LocalGet(made), I::GlobalSet(tail),
				I::I32Const(1),
			]);
		});
	}
}
