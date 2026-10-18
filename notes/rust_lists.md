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
- C: hosts that build lists (`Builders::list`, tasks `build`, web/playground host.js `new_list` via reduceRight) and
  the browser readers through the reflection exports (reader.js `readNode` chain, host-tasks.js `readTaskValue`,
  src/web/mod.rs `list_node`): a `reflect_item(list, i)` / length export instead of walking `reflect_value`.

## Inventory (2026-10-18)
~60 walker sites in ~30 functions, ~35 builder sites, 7 cons-only readers outside the emitter (stage A takes the native
ones; the JS ones read through the emitter's exports, stage C).
