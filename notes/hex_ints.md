# Ints shown in hex

`255 as hex` is an int that behaves exactly like an int but shows as 0xFF; so does the ephemeral
fast type byte (`byte`, `u8`, `uint8`), a plain wasm integer, special logic only at FFI crossings.

- Static, not a run-time type: `x as hex` computes as `x as int` (wasm_emitter/hex.rs computed_type). Only where such a
  value becomes a Node (emit_node_instructions → `as_hex`) is it an Int marked HEX_INFO above the kind bits
  (type_kinds::HEX_KIND), so arithmetic, lists and sums never see a new kind: `x + 1` is a plain 256.
- Shows in hex: `x as hex`, `x as byte`, a variable declared byte, a variable assigned one of those (its type_node is
  `hex`, analyzer value_binding, without the byte's range: `x = 200 as uint8; x + 100` is 300).
- Text: list_join (print, `as text`, `+`, interpolation) writes a marked Int with `hex_text`: 0xFF, -0x1, 0x0,
  uppercase digits. Readers (wasm_reader, web.rs) give `Int(255).with_source_literal("0xFF")`, equal to 255.
- A declared byte loses its type word before analyze_required_functions; as_hex arrives as a discovered need, so
  emit_hex_runtime runs on either function.
- Not yet: `hex` as a declared type (`hex h = 255`), big ints beyond i64 stay decimal, `type(255 as hex)` is int.
