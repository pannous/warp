# Where warp.wasm's bytes go (card wasm-bytes-profile)

Measured on the installed warp.wasm of 2026-11-10 (2,501,558 bytes). How: `wasm-objdump -h` for the sections,
`wasm-tools print` for the code, and a short parser of the code and name sections (function sizes by name, data
segment sizes; kept in data/wasm-bytes-profile/, ignored). Machine code and memory of the same module:
notes/compile_speed.md "The compiler's machine code".

## Sections

| bytes | section |
|--|--|
| 2,044,216 | code: 6,695 functions |
| 173,938 | name (wasm-opt without -g drops it; traps and profiles read it) |
| 142,693 | data: 4,936 segments, 132,737 bytes of text |
| 108,042 | export: 5,694 exports, nearly every function |
| 18,491 | global: 1,587 globals |
| 7,213 | function, 5,200 type, 1,729 elem, 586 import |

## Top 30 functions (bytes of code)

| bytes | function | where |
|--|--|--|
| 197,800 | `main` | the program's globals: 1,189 constant tables built in code |
| 25,057 | `intrinsic_of` | core/emitter/runtime_words.warp: a match over the intrinsic names |
| 18,581 | `class_word·construct·Module` | class Module (67 fields, its field names built one by one) |
| 13,713 | `group_expression` | core/emitter/emitter.warp |
| 10,300 | `key_expression` | core/emitter/emitter.warp |
| 8,767 | `class_word·construct·Extension` | class Extension |
| 8,677 | `methods_lifted` | extensions/class_methods/methods.warp |
| 6,901 | `class_word·construct·Parser` | class Parser |
| 6,846 | `linked_bytes` | core/emitter/prebuilt.warp |
| 6,366 | `unmarked_operation` | core/emitter/operators.warp |
| 6,066 | `continued` | core/parsing/parser.warp |
| 5,925 | `module_compiled` | core/emitter/emitter.warp |
| 5,854 | `node_operation` | core/emitter/operators.warp |
| 5,378 | `mvp_condition` | extensions/primitive.warp |
| 4,904 | `implicit_modules` | extensions/implicit_uses.warp |
| 4,865 | `class_word·construct·Body` | class Body |
| 4,853 | `with_reached` | core/passes/prelude_words.warp |
| 4,626 | `assignment` | core/emitter/emitter.warp |
| 4,548 | `prepended_bytes` | core/emitter/prebuilt.warp |
| 4,439 | `gather` | core/emitter/module_layout.warp |
| 4,402 | `class_word·construct·ModuleSignature` | class ModuleSignature |
| 4,391 | `items_until` | core/parsing/parser.warp |
| 4,155 | `mvp_field` | extensions/primitive.warp |
| 3,993 | `class_word·set·Module` | class Module |
| 3,894 | `class_word·construct·PrimitiveFunction` | class PrimitiveFunction |
| 3,682 | `mvp_conditional` | extensions/primitive.warp |
| 3,573 | `logical_code` | core/emitter/operators.warp |
| 3,568 | `primitive_bytes` | extensions/primitive.warp |
| 3,448 | `class_word·construct·Function` | class Function |
| 3,426 | `mvp_assigned` | extensions/primitive.warp |

By source file: core/parsing/parser.warp 133 KB, core/emitter/emitter.warp 93, core/passes/rewrites.warp 89,
extensions/primitive.warp 70, extensions/rules/rule_words.warp 63, core/passes/prelude_words.warp 51,
extensions/class_methods/methods.warp 51, core/emitter/prebuilt.warp 48 (names matched to their definitions; a
name defined in two files counts once).

## Where the size comes from, and the cards

1. **Literal nodes built inline** (card literal-nodes-inline): 28,826 sites build a text or symbol node from a data
   segment, about 16 bytes each (`i64.const kind; i32.const 0; i32.const len; array.new_data; ref.null; struct.new`):
   ≈ 460 KB, a fifth of the code. Each site also inlines a GC allocation in Cranelift's machine code (94,734 struct.new
   and array.new sites in all). A match over many texts, `intrinsic_of`, is the same pattern. The biggest lever.
2. **Constant tables as code** (card globals-constant-init): `main` builds the compiler's tables (rule_words,
   core_host_words, library_words, node_spellings, Op texts, intrinsics, gc_names, …) instruction by instruction:
   197,800 bytes. As packed data unpacked at start, the way web.webidl became lib data.
3. **Per-class words** (card each-wasm): 61 classes, each with its own construct, set, place, entries, field_at and
   descriptor: ≈ 190 KB (construct alone 99.6 KB). A generic constructor reading the descriptor, field names as one text.
4. **Every function exported** (card wasm-export-coverage): the 108 KB export section, and it keeps 2,747 functions that no
   entry point reaches (only 54 KB of code: small accessors and closures) from wasm-opt's dead code removal.
   Overlap: warp-cleanup step 4 removes dead source code; the unreached list can feed it, not a second removal card.
5. **Baked data files with their comments** (card baked-data-files): diagnostics, builtins, warnings, prelude core,
   primitive prelude, math builtins and hints are baked whole, comments too (~27 KB together).

Not worth a card: the six enums' `·same` words (634 bytes each, 3.8 KB), the name section (kept for traps and
profiles), the Unicode case tables as decimal text (17 KB, already data).
