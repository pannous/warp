# Core extensions roadmap (decision core-is-self, card core-extensions-roadmap)

The core is what the compiler needs to compile itself; the rest moves to extensions/ feature by feature, registered in
one static table of hooks the core knows ahead of time.

## GPU lists (card gpu-lists-extraction; mapped 2026-11-03 on main, no code changed yet)

GPU lists `[f32]`, `[i32]`, `[u32]` (decision gpu-bound-floats, landed in 31ab8ce82e): four-byte cells in linear
memory from their creation, a square List $Node marked KindUnit.GpuList whose Int is (address << 32) + count. The
compiler never makes one for itself, so all of it can leave the core.

### Where it is (core/ about 100 lines, the rest already outside)

| place | lines | what | kind of coupling |
|---|---|---|---|
| core/runtime/gpu_lists.warp | 76 | the words: gpu_zeros, gpu_list_of, gpu_put, gpu_list_at/set/count/items/place/cell, refusals | whole file: moves |
| core/runtime.warp:20, core/runtime/lists.warp:8, nodes.warp:8 | 3 | `use "gpu_lists.warp"` | goes with the file |
| core/runtime/lists.warp:34, 54, 85 | 3 | item_at, count, set_item: `if is_gpu_list(n) { return gpu_list_… }` | hook: list access dispatch |
| core/runtime/lists.warp:177-182 | 6 | unlisted_items: a GPU list refuses (a range's items are made) | hook: list access dispatch |
| core/runtime/nodes.warp:89 | 1 | list_shown: a GPU list's items | hook: list access dispatch |
| core/analysis/kinds.warp:22-26 | 5 | KindUnit.GpuList (1 << 52), KindUnit.GpuCell (1 << 57), GpuCell {F32 I32 U32} | mark bits: reserved by the core, named by the extension |
| core/emitter/emitter.warp:1130 | 1 | GpuCell in constant_tables | goes with the table |
| core/emitter/host.warp:76-82 | 7 | exports gpu_list_place, gpu_list_cell (the C side reads the cells in place) | hook: an extension's exported runtime words |
| core/emitter/runtime_words.warp:166-172 | 7 | memory_f32, memory_set_f32, memory_i32, memory_u32, memory_set_i32 intrinsics | stays: generic linear-memory loads/stores (host cells use the i64/f64 siblings) |
| core/emitter/runtime_words.warp:192 | 1 | library_words gpu_zeros, gpu_put, gpu_list_of | hook: an extension's library words |
| core/emitter/runtime_words.warp:267 | 1 | comment: a GPU list's cells are no $list | goes with the dispatch hook |

Not GPU lists, though they read so: the `f32` type words (parser NUMBER_TYPE_WORDS, analysis/words, runtime/types,
arity_overloads, rewrites numeric_parameter_types) and c_imports' f32 are C floats and type names, they stay. The
driver's gpu_paints/gpu_painted (core/native/driver.warp:144-145, 983, 1194-1222) is GPU painting, a separate
extraction (paint), not GPU lists. Already outside the core: lib/wgsl.warp's GPU lists part (775-864, 90 lines:
gpu_bound, gpu_packed, gpu_creating, GPU_CELLS), lib/rule_words.warp gpu_lists_packed/gpu_list_made (280-292 and the
table entries 691-694), lib/prelude.warp's rules (268-276: the program rule, gpu_compute's argument, `xs: [f32|i32|u32]
= …`), lib/native/warp_node.h warp_cells (126-140, the C side).

### Hooks an extension needs from the core

1. **List access dispatch**: item_at, count, set_item, unlisted_items and list_shown ask a registered list kind first,
   chosen by a mark bit on the List node. Ranges (KindUnit.Range, runtime/lists.warp) are the second kind with the
   same need, so this is one hook for both: a table from mark bit to {at, count, set, items}.
2. **Reserved mark bits**: the core reserves a few KindUnit bits for extensions' list kinds; the extension names its
   own (GpuList, GpuCell) and its cell table.
3. **Runtime words an extension adds**: its runtime file prebuilt with the runtime (runtime.wasm) or as its own module,
   its exported words (gpu_list_place, gpu_list_cell) declared by the extension, not in core/emitter/host.warp.
4. **Library words**: the extension's entries of library_words (gpu_zeros, gpu_put, gpu_list_of) from its table.
5. **Rules**: lib/prelude.warp's GPU list rules move with lib/wgsl.warp's part and rule_words' words into the extension;
   the core keeps the rule mechanism only.

Size: core/ drops by about 100 lines (gpu_lists.warp 76 plus about 25 lines of dispatch and declarations); hook 1
also takes the range dispatch out of the list words.
