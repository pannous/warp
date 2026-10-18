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
- B (rust-texts-builders): float, time, duration and big-int texts and list_join's Int digits write a `$Text`
  (`int_to_decimal(bytes, position, value)`, `emit_text_prefix` / `emit_text_piece` at the end); constant texts are
  literals through one `constant_text(offset, len)` call (`emit_constant_text`, `emit_text_node`), the empty text is
  `emit_empty_text`. `new_text` / `new_symbol` / `text_from_memory` stay for hosts only (exported builders, WASI
  reads, FFI); `text_address` stays for print's fd_write and C strings.
- Next: hand `$Text` to the host for print (drops `text_address` from every printing program), host reads and FFI.

## Traps
- `AnyRef::unwrap_struct` panics on an array in wasmtime 49: test `as_array` first (gc_traits, readers).
- An empty text has no first byte: guard array reads (`emit_is_one_character`).
- Machine code, not wasm size, is the executable budget (tests/wasm/test_executable_size.rs): compare with
  `wasmtime compile` + `wasmtime objdump --addresses` per function.
- A constant text inline (`text_literal` + `new_text_of`) costs ~30 B of machine code per site more than one call:
  list_text has ~20 such sites (quicksort budget). Measure the carried program: probes/rust_texts/carried.py
  (decompresses an exe's machine code) + probes/rust_texts/fsizes.py (bytes per function).
- Runtime functions emitted inside emit_constructors (the text foundation) run before the constructors: one calling
  `new_text_of` belongs after it (constructors.rs, as `constant_text`).
- Fuel cost per ASCII index (tests/text/test_ascii_index_costs.rs): a one-byte grapheme is its byte (emit_grapheme_node).
