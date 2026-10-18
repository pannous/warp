# Lists in the Rust emitter: vectors instead of cons cells (card rust-emitter)

Target (notes/self_hosting.md "Value layout"): a List/Block node's `data` is a `$list = (struct (mut i32 length)
(mut (ref $items)))`, `$items = (array (mut anyref))`, its `value` null; `xs#i` is O(1). Today a list is cons cells:
`data` the first item, `value` the rest (a cell, a last single item, or null), meta entries `@name:value` at the tail.

A vector is told from a cons cell by its node's `data`: a struct whose first field is an i32 (a cons cell's first item
is a $Node, first field the i64 kind): `warp_runtime::gc_values::vector_items`.

## Stages
- A (rust-lists): every reader outside the emitter reads both layouts through `gc_values::items_of` /
  `vector_items`: wasm_reader (already did), warp-runtime gc_values (foreign calls of executables), host tasks
  (`read_value`), foreign-call arguments (native.rs `function_arguments`).
- B: the emitter, in small branches through one runtime function `list_items(node) -> $list`: a vector's own, else
  a fresh vector of the cons cells' items (not cached in the node: readers not yet moved still walk the cells).
  B1..Bn move the readers to it (a cons list costs one O(n) copy per read meanwhile, as walking it did); the last step
  turns the builders and in-place changes (list_extend's splice, list_become) to vectors, then list_items is O(1) and
  the cons branch goes. Central walkers first (`emit_cell_walk`, `emit_list_walk` with the list cursors, `emit_collect_cells`
  / `emit_rebuild_cells`), then the ~30 functions walking field 2 by hand (list_changes, fields_and_keys maps,
  object_copies, equality, ordering, text_words joins, library_ops reverse/slice, closures' argument lists), and the
  builders: `new_list` and the literal chains (list_emitter `emit_list_structure`), raw `StructNew` cells (~25 sites).
  The existing `$NodeList` (list_dispatch/list_runtime.rs: length, items, kind) is the base to grow `$list` from.
  Open points: meta entries at the tail (`IS_META_ENTRY`), improper tails (a last single item), maps as curly lists of
  Keys (`node_is_map`, `IS_MAP`), the empty list's cell (null first item).

### B1 (branch rust-lists-b): list_items and emit_item_walk
`list_items` (list_dispatch/list_runtime.rs `emit_list_items`, the old node_list_of grown) is emitted always, right after
is_meta_entry in emit_constructors, so every runtime function can call it. It counts, then fills (no push): the items
up to a meta entry or a cell without an item; ø and null none; a non-list the one item. `emit_item_walk(list,
ItemWalk, body)` (list_ops.rs) loops over its array, the item in a local; `emit_items_length` is `#items`.
Moved: node_count, index_out_of_range_in, list_items_check, collection_contains/position, map_column_cells,
elements_field, list_reverse, node_slice (no list_reverse any more: cells built from the end), list_sorted_by (sorts
list_items' own array; ranks copied from it, then from the keys' items), list_text/list_join, swizzled (picks index
the array, no local per item), list_to_ints, node_map_of. A rebuilt list (reverse, slice, sort) takes the list's kind
for every cell, not each cell's own; meta entries no longer join, reverse or count (they never should).
Still on cells: map_find and map_index (they must see meta entries: `x.name` falls back to the entry `@name`),
deep_copy's memo (internal), emit_list_walk (indexing: a copy per `xs#i` would make loops quadratic, it moves with the
builders), values_equal/unordered entries, list_changes (in-place changes), copy_spine, node_api relisted.

### Layout decisions for the flip (undoable defaults)
- A vector list node: kind as now, `data` the `$NodeList` (length, items, kind), `value` its meta entries (a cons list
  of `@name:value` entries) or null. Readers of items never see meta entries; map_find reads `value` for `@name`.
- `list_items` returns a vector's own `$NodeList` (shared): callers that change it in place (a typed Node list
  variable's start, list_dispatch/list_abi) take NODE_RUNTIME.copy of it.
- Mutations change a cons list into a vector in place first (same node, so every holder sees it), then the vector.
- The browser reader gets a payload tag for a `$NodeList` (reflection/exports.rs PAYLOAD_TAGS, reader.js
  readPayload, src/web/mod.rs node_from_tree), as "floats" has: stage C must land before any builder makes vectors.
- C: hosts that build lists (`Builders::list`, tasks `build`, web/playground host.js `new_list` via reduceRight) and
  the browser readers through the reflection exports (reader.js `readNode` chain, host-tasks.js `readTaskValue`,
  src/web/mod.rs `list_node`): a `reflect_item(list, i)` / length export instead of walking `reflect_value`.

## Inventory (2026-10-18)
~60 walker sites in ~30 functions, ~35 builder sites, 7 cons-only readers outside the emitter (stage A takes the native
ones; the JS ones read through the emitter's exports, stage C).
