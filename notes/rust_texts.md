# Texts in the Rust emitter: `$Text` GC byte arrays (card rust-emitter)

`$Text = (array (mut i8))`, type 0 (`type_manager.text_type`). A Text/Symbol/Error node holds its `$Text` in `data`.

## Layout
- Literals: one passive data segment (`TEXT_SEGMENT`), read by `text_literal(offset, len)` (one call per literal:
  `array.new_data` inline costs several times the machine code). Memory segments (lookup tables, host texts) are active.
- Ropes: `text_append` makes `Text{data: new piece, value: text before}`; `text_flat(node)` flattens once and caches the
  flat array in the node. Appending in a loop stays linear (tests/text/test_text_building.rs appends 200 000 times).
  Readers: `emit_text_bytes` (Rust emitter), `rope_text` (wasm_reader), `text_of` (warp-runtime gc_values).
- Readers walk `(bytes, at, end)`: `at` a byte index, `end` the length (`emit_text_bounds`). utf8_next, utf8_decode,
  grapheme_end, text_grapheme_back take the array first. Text cursors and counted texts key on the array (`ref.eq`).
- Slices and builders end in `text_piece(bytes, start, length)` → `new_text_of`.

## Stages
- A (rust-texts, 2a85fcd0b): readers on arrays; some builders still write memory and end in `text_from_memory`;
  host edges keep (ptr, len) through `text_address` / `text_from_memory`.
- B: the remaining memory builders (`emit_new_text_to`, `emit_text_allocation` users: float/time/exact texts,
  int_to_decimal, list_text pieces) write `$Text`; then the text heap global and the bridges go where only texts used them.

## Traps
- `AnyRef::unwrap_struct` panics on an array in wasmtime 49: test `as_array` first (gc_traits, readers).
- An empty text has no first byte: guard array reads (`emit_is_one_character`).
- Machine code, not wasm size, is the executable budget (tests/wasm/test_executable_size.rs): compare with
  `wasmtime compile` + `wasmtime objdump --addresses` per function.
- Fuel cost per ASCII index (tests/text/test_ascii_index_costs.rs): a one-byte grapheme is its byte (emit_grapheme_node).
