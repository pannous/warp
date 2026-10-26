# GC-free "glue" from the pure compiler: does `[1,2,3] as array` give linear memory? (investigation, 2026-10-23)

Investigation only. No code changed; src/ and core were only read. Tree: /workspace/cow/trees/glue-linear (boxtree,
origin/main cdc63fbca). Probes and outputs: /workspace/cow/glue-linear-probes/ (measure.sh, runall.sh, runcimp.sh,
out/, out-cimp/, target-shape/). [M] = measured, [R] = read in source, [I] = inferred.

Compilers used (none built; per the new rule, no per-branch cargo builds):
- main-ish: /tmp/hfw-c/warp-host + compiler-self.wasm sha1 6269c5f5… (built 04:12 today, before main's later
  sugar/parser merges; none of those touch lists, `as`, the boxing or the module layout [R]). WARP_LIB = the tree's lib.
- C imports: the pure-c-imports branch (3d9802b7c, **not merged to main**): its scratch/ffi-test warp-host
  (sha1 69e8e80f…) + compiler-self.wasm (sha1 6642abef…), copied to glue-linear-probes/cimp-bin. WARP_LIB = that tree's lib.
Strip: `wasm-opt -all --closed-world --remove-unused-module-elements --remove-unused-types`; validate:
`wasm-tools validate --features mvp,mutable-global,saturating-float-to-int,sign-extension,multi-value,bulk-memory,reference-types`.
A compile takes ~1 s once warp-host is warm (first one 22-28 s) [M].

## Verdict
**The hypothesis is wrong for `as array`, but right in spirit for C glue.**
- `x = [1,2,3] as array` is refused by the pure compiler: `outside the core: the unknown name array` [M]. `x: array = …`
  is refused too ("a variable declared array") [M]. There is no `as array`, no typed array and no linear-memory array in
  the pure pipeline. `array` is only a foreign spelling of `list` in type position (words.warp:21 foreign_list_heads,
  parser.warp:3595 GENERIC_TYPE_HEADS, parser.warp:3640). `as` only knows core_type_words
  (emitter/runtime_words.warp:142-145, no `array`), emitter.warp:916-927.
- Linear arrays exist only in the frozen Rust compiler: `linear xs = int[n]` / auto-picked float arrays
  (notes/linear_arrays.md; src/lowering/collections/shared_arrays.rs:32 LINEAR_WORD, :158 picked_linear, :244
  declared_linear; src/wasm_emitter/lists/linear_arrays.rs:13-23 linear_new/get/set…, :117 emit_linear_arrays). Rust's
  "typed lists" ($IntList over (array (mut i64))) are GC arrays, not linear (notes/typed_lists.md). notes/folds.md:36:
  "the pure emitter has neither typed arrays nor threads yet".
- Pure lists are always `$list{length, items: (array anyref)}` of **boxed** `$Node` items: emitter/lists.warp:2
  list_value boxes every item, :3-6 vector_code = array.new_fixed $items + struct.new $list; boxing in
  emitter.warp:400-411 (int → $i64box+$Node, float → $f64box+$Node, text → $Node).
- What *is* already GC-free in the pure pipeline [M]: integer code, loops, functions, and raw linear-memory bytes via
  the program intrinsics `memory_byte(addr)` / `memory_set_byte(addr, v)` (emitter/runtime_words.warp:107-113, listed
  as program_intrinsics at :118; lib/memory.warp wraps them). And on the pure-c-imports branch: C imports whose
  parameters are numbers.
- **A GC-free glue program calling C already works today on the pure-c-imports branch** [M]:
  `puts(s: i32): i32 from 'c'` + three `memory_set_byte` + `puts(1024)` compiles to 832 B → 206 B stripped, valid
  non-GC (only i32.store8 + call), and wasm2c + cc + a 1-line C shim prints `hi` (23,816 B native binary).
  Under warp-host it segfaults, because the warp.c_functions line says `i32` (a number) where `ptr` is needed. A
  hand-made module with the same code and a `ptr` line runs under both warp-hosts and prints the string [M]. So a
  `cstr` parameter type is the missing piece, and on the host side it already exists: c_signatures.rs `ptr` with an i32
  wasm type is "an address in linear memory" (notes/warp_host.md "C calls").

## Measurements [M]
| probe | source | size → stripped | non-GC after strip | GC left, why | native (wasm2c+cc) |
|---|---|---|---|---|---|
| p1 | `x = [1,2,3] as array; x[1]` | refused | – | "unknown name array" | – |
| p2 | same + `print x[1]` | refused | – | same | – |
| p3 | sum loop over `… as array` | refused | – | same | – |
| p4/p4b | `[1.5,2.5] as array` (index / sum) | refused | – | same | – |
| p7 | `"hi" as array` | refused | – | same | – |
| p10 | `x: array = [1,2,3]` | refused | – | "a variable declared array" | – |
| p5 | `x = [1,2,3]; x[1]` | 1033 → 328 B | no | 15 struct.new, 2 array.new_fixed, 1 array.get: items boxed as $Node/$i64box, main returns the item as a Node | – |
| p11 | `[1,2,3] as list; x[1]` | 1033 → 328 B | no | identical to p5 | – |
| p9 | `x = int[3]; x[1] = 5; x[1]` | 1733 → 673 B | no | zero_fill list of boxed nodes (lib/sugar/lists.warp:4), 13 struct.new | – |
| p8 | `s=0; for i in 0..<4 {s=s+i}; s` | 818 → 192 B | **yes** | none | 23,672 B, prints 6 |
| p6 | `memory_set_byte` ×2 + `memory_byte` sum | 837 → 196 B | **yes** | none (4 load/store) | 23,624 B, prints 209 |
| earlier j | `print 42` | 1500 → 618 B | no | int_text builds a $text (array.new_default/set), print_line reads it | – |
| earlier k | `x=3.5; x*2` | 1806 → 824 B | no | `3.5` is an exact Ratio (struct $Ratio of $i64box), not a float | – |
| earlier m | `"ab" + "cd"` | 823 → 251 B | no | texts are (array (mut i8)) via array.new_data | – |
| f1 (branch) | `import abs from "c"; abs(-5)` | 674 → 90 B | yes | none, but no import: warp's own abs wins | prints 5 |
| f4 (branch) | `abs(x: i32): i32 from 'c'` line | 806 → 169 B | yes | same, no import | prints 7 |
| f5 (branch) | `putchar(c: i32): i32 from 'c'; putchar(104)` | 752 → 163 B | **yes, imports c.putchar** | none | 23,840 B, prints `h`; warp-host prints h, 104 |
| f6 (branch) | `import toupper from "c"; toupper(97)` | 752 → 163 B | **yes** | none | warp-host 65 |
| f7 (branch) | `puts(s: i32)…`, bytes by memory_set_byte, `puts(1024)` | 832 → 206 B | **yes** | none | 23,816 B, prints `hi`; warp-host **segfault** (line says i32, not ptr) |
| f2 (branch) | `import strlen from "c"; strlen("hello")` | 23,256 → 15,357 B | no | text param kind `node` → host_call exports 13 node builders (emitter/host.warp:14,26-62), whose code keeps the whole runtime | – |
| f3 (branch) | `puts(s: text)…; puts("hi")` | 23,248 → 15,351 B | no | same; main itself is only `array.new_data; struct.new $Node; call puts` | warp-host prints hi |
| f8 (branch) | `puts(s: cstr)…` | refused | – | "the call puts(…)": unknown C type | – |
| f9 (branch) | `"hi" as cstr` | refused | – | "unknown name cstr" | – |
| target | hand WAT: data "hi…\0" at 1024, `i32.const 1024; call puts` + `ptr` line | 161 B | yes | – | 23,688 B prints it; both warp-hosts print it |

The prelude (8 GC types, ~15 runtime globals) costs nothing in a GC-free result: wasm-opt strips all of it for p6/p8/f5/f7.
The pre-strip module never validates non-GC because module_bytes always writes gc_types first
(emitter/module_layout.warp:48) and every runtime global (emitter/host.warp:85-95, called for all of runtime.warp at
emitter.warp:1124; text globals start as ref.null $text, list ones as struct.new, module_layout.warp:5-13).

## How C imports pass strings today (pure-c-imports branch, not on main) [R]
- core/c_imports.warp (branch): `text` crosses as warp kind `node` (c_parameter_kinds, line 35) with
  c_signatures name `ptr` (c_parameter_names, line 36); warp-host copies the Node's text NUL-terminated
  (c_calls.rs native_arguments). A text *result* is a `char *` read back through new_text + text_heap.
  c_host_word at lines 244-262 builds [kinds, results, library, warp.c_functions line].
- emitter/host.warp:14 (main): any host word with a `node` parameter or result calls exported_builders(), the 13
  exports of host_builders (:26) — this alone makes f2/f3 15 KB and GC.
- emitter/c_functions.warp (branch, 26 lines): the warp.c_functions custom section and text_heap.
- So the host already accepts an i32 `ptr` as a linear-memory address; only the compiler lacks a type that emits it.

## Gaps for GC-free glue (smallest first)
1. **pure-c-imports is not on main** (branch 3d9802b7c, Claude-owned card). Everything C-related below needs it merged.
2. **cstr parameter type**: in the branch's c_imports.warp tables, `cstr: "cstr"` in c_type_words (:31-33),
   `cstr: "i32"` in c_parameter_kinds (:35), `cstr: "ptr"` in c_parameter_names (:36). ~3 lines [I]. With it f7
   runs under warp-host too [I: the hand WAT with that exact line ran, M].
3. **cstr literals** (`"hi" as cstr` or `c"hi"`): bytes + NUL placed in linear memory at compile time, value an i32
   address (int kind). Sites: emitter.warp:916-927 (`as` targets) or emitter/literals.warp:38-42 (text_value/text_bytes);
   module_layout.warp:72-77 (today one *passive* segment read by array.new_data; add an active segment or a
   memory.init at start, data_count 2); a reserved region in the layout at emitter.warp:52-57 (iovec 0, written 8,
   output 16…, WASI scratch up to 32 KB of the single page; text_heap for host-written texts) — e.g. memory_pages 2 and
   cstrs from 65536 [I]. ~30-50 lines [I].
4. **cstr results** (getenv): an i32 `char *` into C's memory is not addressable from wasm; needs the host to copy into
   linear memory (Rust/host side, frozen) or stay GC (today's `text` result). Out of scope for the first step [I].
5. **Unboxed returns/printing**: main stays unboxed only for int/float (emitter.warp:1159-1162; anything else is boxed
   to a Node). `print` always goes through shown_text → int_text/node_text → print_line over a $text
   (emitter/output.warp:17-29, :57; runtime/text.warp:55; runtime/conversions.warp:136). A GC-free print needs a
   linear int writer (digits into the output area via output_byte, then write_output: both intrinsics already GC-free,
   runtime_words.warp:104-105, output.warp:3-9) and a cstr print (fd_write straight from the cstr). ~40-60 lines [I].
6. **Arrays**: no unboxed array of any kind in the pure emitter. A linear `int[n]`/`float[n]` (Rust's linear_arrays
   design: `[count:i64][cells]` blocks from a bump heap) would be a new list representation: new kind, index/length/
   assign sites in emitter/lists.warp (:2-40), analyzer kinds (analyzer.warp:219-229 converted_kind), `as array`
   target. ~200-400 lines [I]. Not needed for C glue that only passes strings and numbers.
7. **Floats**: scalar `float` locals and results are unboxed already; but a literal like `3.5` is an exact Ratio (GC)
   unless typed float (`3.5 as float`, `x: float`), and floats in lists are $f64box. Glue code should type floats [I].
8. **Prelude types/globals**: no change needed when wasm-opt runs (stripped fully) [M]. Emitting a clean module
   without wasm-opt would need module_bytes to skip gc_types and unused runtime globals (~20 lines [I]).

## Option B: an emitter mode (`use native` pragma or a flag) [R/I]
How options reach the emitter today:
- No option object: compiler.warp takes positional args only (source, wasm, folder, lib; compiler.warp:12, 51-52) and
  calls typed_wasm_module(program, runtime_words, host_words, kinds) (compiler.warp:42; emitter.warp:1076).
- Two precedents for modes, both stored on the global `module` (class Module, emitter.warp:193):
  - pragma: `use fuel` → uses_fuel(program) (emitter.warp:258) → module.counts_fuel (:1114) → custom section
    (module_layout.warp:80). A `use native` pragma would copy this exactly: no host or CLI change.
  - env: WARP_EMIT_SURVEY → module.surveying (emitter.warp:170, :1116). A WARP_NATIVE env would work the same way,
    but warp-host's CLI flags (--compiler, --fuel) are Rust (frozen), so a `--native` flag means touching Rust.
- Sites that would branch on `module.native`:
  - text literals: literals.warp:38-42 (cstr instead of array.new_data)
  - lists/arrays: lists.warp:2-6 (refuse, or linear block)
  - boxing: emitter.warp:400-411 (refuse "a <kind> as a value in native code")
  - main result: emitter.warp:1159-1162 (refuse non-int/float)
  - print: output.warp:17-29, :57 (linear int/cstr writers)
  - host/C calls: host.warp:14 (no builders), c_imports.warp:35 (text → i32 ptr, branch)
  - module layout: module_layout.warp:48 (skip gc_types), host.warp:85-95 / emitter.warp:1124 (skip runtime globals),
    :72-77 (active cstr segment)
- Effort: the mode itself is ~10 lines (pragma + Module field); each site's native path is the same work as route A's
  steps 2-5, plus a refusal per site (~1-3 lines each, ~10 sites). It is cleaner for users ("in native code a list is
  refused" instead of silently GC) and leaves default output byte-identical, but it touches more core files at
  once. Route A (cstr type + `as cstr` + unboxed print) can be done first and later be what the mode selects; the two
  are not exclusive [I].

## Proposed minimal plan
0. Merge pure-c-imports to main (Claude-owned; its own card). No new work.
1. `cstr` parameter type: 3 table entries in core/c_imports.warp. Result: `puts(s: cstr): i32 from 'c'` +
   memory_set_byte bytes + `puts(1024)` → a 206 B GC-free module that runs under warp-host and natively via wasm2c.
   Touches core (an existing file, no new pass) → needs the user's OK and the Claude owner's.
2. cstr literals: `"hi" as cstr` → an active data segment + i32 address (emitter.warp `as`, module_layout.warp,
   layout constants). core, existing files, ~30-50 lines [I]. Then `puts("hi" as cstr)` is the one-liner glue.
3. GC-free print of ints/cstrs (output.warp + a runtime word or two), ~40-60 lines [I].
4. Optional: the `use native` mode as a pragma (emitter.warp, ~10 lines + refusals per site), new pass-list entries:
   none needed. New core/*.warp files: none needed for 1-3.
5. Later, only if needed: linear int/float arrays (a new list representation, ~200-400 lines [I]).
Tests for each, result-based only (no exit codes or internals), e.g. a host test that `puts("hi" as cstr)` prints hi,
or `is! 'memory_set_byte(100, 7); memory_byte(100)' 7`.

## Side findings
- warp-host passes an i32 argument of a C import without a `ptr` line as a raw number: `puts(1024)` segfaults the host
  (exit 139) instead of failing cleanly [M]. Worth a guard when an i32 goes to a function that expects a pointer, but
  that lives in Rust (crates/warp-runtime), so it is reported only.
- `abs`/`floor` etc. imported from c/m are replaced by warp's own words (branch commit 83b929d2a), so f1/f4 contain no
  import at all [M].
