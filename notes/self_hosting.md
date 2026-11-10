# Self-hosting: warp's compiler written in warp

## Value layout
Typed values inside, a boxed $Node only at the edges. Where emitter.warp knows a value's kind it stays plain:

| kind | wasm value | |
|---|---|---|
| int | i64 | |
| float | f64 | |
| yes/no | i32 | |
| character | i32 code point | a one-character literal, as warp holds `"a"` |
| text | `(ref null $text)`, `$text = (array (mut i8))` | UTF-8; literals from one passive data segment (array.new_data) |
| list | `(ref null $list)`, `$list = (struct (mut i32 length) (mut (ref $items)))`, `$items = (array (mut (ref null $Node)))` | a vector: `xs#i` O(1); items boxed |
| any kind (a value of mixed branches, a list item, ø) | anyref holding a `$Node` | |

Boxing (`boxed` in emitter.warp) makes a `$Node(kind i64, data anyref, value ref null $Node)` (CLAUDE.md
"Serialization"): an int or yes/no in an `$i64box`, a float in an `$f64box`, a character as an i31, a text with data =
its `$text`, a list with data = its `$list`. A value is boxed only where its kind is not known statically: a list item,
a value of branches of different kinds, an `any` parameter, and main's result unless it is a number. Texts are GC
arrays; linear memory holds one only at the host/FFI edge.
Readers of the layout (each place a module's result or a value crossing the boundary is read):
- web/playground/values.js reads a module's values in the browser through its exports (shown_at, item_at …);
  core/native/driver.warp reads them natively through the same exports.
- Host imports and FFI take $String (ptr, len) texts in linear memory. emitter.warp imports
  only WASI's fd_write, and only for a program that says `print` (imports shift function indices, so it is decided
  before compiling): one page of memory, exported as "memory", where the runtime word print_line copies a text's bytes
  a chunk at a time (iovec at 0, written count at 8, bytes from 16).
Runtime words are warp: core/runtime.warp (an index of `use "runtime/<part>.warp"` lines, one idea per part
in core/runtime/), written in idiomatic warp (for loops and all sugar) and lowered by
the same pipeline as any program before emitter.warp sees them (`wasm_module(program, runtime)`).
emitter.warp handles only lowered forms, never hand-lowered source. The words use a
handful of intrinsics, one or two instructions each and callable only from runtime words: texts (new_text, copy_text,
byte_count, byte_at, set_byte), lists (new_list, copy_items, adopt_items), numbers (int_quotient, int_remainder,
truncating; code_of, char_of), boxed values (kind_code, marked_kind, unboxed_int/float/text/char, is_null), output
(output_byte, write_output) and fail (a trap). Each word is compiled on its first call like the program's own
functions: text_join and list_join (`+`), list_append (`+=` on a list, in place, so aliases see it), text_count (`#`
on a text: code points), text_char (`t#i`), text_order (text comparisons), print_line, int_text, char_text, node_text
(print and `+` of a value known only at run time).

## Runtime calls into prelude words
A word whose code can only be written once the module's closures and classes are known (the closure dispatchers, the
instance words of `late_words`) is made at its first call with a placeholder body and filled in afterwards
(`made_late`, `late_code` in core/emitter/closures.warp); behind runtime.wasm such a word is runtime.wasm's, written
there once.
