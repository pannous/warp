# lib/ as precompiled wasm modules: runtime.wasm + on-demand modules loaded by warp-host

Investigation and design, 2026-10-23 10:00–10:15 CEST. No lib/compiler or src/ edits, nothing committed.
Base: worktree `/workspace/cow/trees/lib-wasm-modules` = `origin/warp-host-wasm-load` `4e3808b76` (not yet in main; main is `7b437156b`).
Probes: `/workspace/cow/scratch/lwm/` (x1 GC/tag link, x2 startup, x3 sqlite3 naming, x4 WASI from a module; `w2w` = a tiny wat→wasm built offline with the `wat` 1.259 crate).
warp-host used: `/workspace/cow/trees/warp-host-wasm-load/target/host/warp-host` (built 03:27 CEST from 4e3808b76, no `wat` feature).
Compiler used for real compiles: `trees/integrate-host/data/compiler-self.wasm` (1,520,527 B) with its lib/.
**Measured** = I ran it; **inferred** = worked out from the code or extrapolated.

## Hard requirement (user, 2026-10-23)

Shipped native binaries (warp-host, and executables from `warp-host build`) carry a **fixed, versioned** runtime.wasm and module
.wasm files, found **beside the binary**. Neither the compiler nor runtime.wasm nor any module.wasm is ever rebuilt automatically:
no staleness checks against lib/ at run time. A lib/ change takes effect only after a deliberate developer step
(`warp-host build-modules` / `scripts/build-modules.sh`). The design below is built around that.

## TL;DR

* **Feasible today with no host changes for the core mechanism (measured).** Two hand-written modules using the compiler's own type
  prelude (`$text` `$items` `$list` `$Node` … as compiler-self emits them), linked by warp-host's import-by-filename: a GC ref crosses
  with **no copy** (`ref.eq` holds both ways), the runtime mutates the program's `$list` in place, a `$Node` built in the runtime passes
  the program's `ref.test (ref $Node)`, a throw of the runtime's **imported** tag is caught by the program's `try_table`, the payload
  goes through an imported `trap_detail` global, and a mutable global written in the runtime is read in the program. Score 111111/111111.
* **Type identity is structural and strict (measured).** Changing one type's mutability or wrapping the runtime's types in one `(rec …)`
  breaks linking. Tags are **not** structural: a program that defines its own `$warp_error` does not catch the runtime's throw.
  So: one frozen type prelude of singleton rec groups for all modules, and exactly one tag + `trap_detail`, owned by runtime.wasm.
* **Startup (measured, box, warm .cwasm):** +0.37 ms per small module (13 KB, 200 functions), 20 modules 10.0 ms vs 2.5 ms with none.
  A 1.5 MB module (compiler-self.wasm as a stand-in for a big runtime): **3,112 ms cold** (Cranelift, no .cwasm) vs **7.2 ms warm**,
  and its .cwasm is 18.1 MB. ⇒ shipped `.cwasm` beside the binary is mandatory for runtime.wasm; that fits the fixed-artifact requirement.
* **Cross-module call cost (measured):** 10⁸ calls of a tiny imported function 118–120 ms vs 98 ms in-module (~0.2 ns per call). Negligible,
  except that a call into a module can never be inlined.
* **"A compiled warp module exports only main" is outdated for compiler-self output (measured):** `hello.wasm` exports `greet`,
  compiler-self.wasm exports 1,970 functions (module_layout.warp:59-67 exports the program's functions). The copy-on-cross is only
  needed for modules whose texts live in linear memory (warp-rust's emitter, `text_heap`); compiler-self's texts are GC `array (mut i8)`.
* **Gaps found (measured):** (a) a type mismatch on a module import is reported as *"the C library runtime of the import runtime.text_join
  is not found"*: ffi.rs falls through to C for any import the linker can't type-match; (b) the main program is instantiated with
  `folder: None`, so bare-name imports resolve only against the **working folder**, not the program's folder or the binary's;
  (c) **bare names collide with C libraries**: a `sqlite3.wasm` that imports C `"sqlite3"` resolves to itself ("cycle: sqlite3.wasm →
  sqlite3.wasm"), and a stray `sqlite3.wat` in the working folder hijacked the import. ⇒ a separate namespace `warp:<name>` for warp modules.
* **The real design constraint is not linking but ABI:** the pure emitter specialises a function by the kinds of its first call
  (emitter.warp:8-10) and compiles runtime words on first call. A precompiled module needs **fixed exported signatures**: typed words
  export their typed form, untyped words a boxed (`anyref`/`$Node`) form, and the caller converts. Hot generic words lose specialisation
  and inlining, so they stay in source (compile-time) until typed.
* **Compiler readiness (measured):** with today's compiler-self (integrate-host, and sc-sugar-preposition-heads `33c994c6b`)
  `use time`, `use json`, `use matrix` compile; `use sound` (si_amount), `use markup` (unknown name option / pair test) and
  `use sqlite3` (main lacks pure-c-imports, `2caa11295` is not in main) are refused "outside the core". So M2 should be **sqlite3**
  (after pure-c-imports lands) or a small pure module, not sound.

---

## 1. Inventory of lib/ (at 4e3808b76)

Columns: bytes, lines; kind = **pure** Warp, **host** (calls host words: std_io/std_pure/sound_samples/paint/…), **FFI** (`… from 'lib'`
C declarations), **CT** (compile-time: rewrite rules, operator/alias declarations, typed builtin lines, read by the compiler, not run).
"defs" = lines with `:=`. Verdict: **R** = core runtime.wasm, **M** = good on-demand module, **I** = keep inline/source (too thin, or must be
compiled into the caller), **C** = stays compiler-side.

### Core runtime (lib/compiler/runtime/, 30 files, 135,042 B, ~399 definitions; plus auto-used type modules)

| file(s) | what | kind | deps | verdict |
|---|---|---|---|---|
| compiler/runtime.warp + runtime/*.warp (instructions, intrinsics, walks, text, text_words, characters, case, show, floats, arithmetic, equality, ordering, types, conversions, text_numbers, lists, library_words, maps, fields, nodes, errors, builders) | the text/list/number/map runtime of every program (card warp-runtime; runtime/README "core") | pure over intrinsics; wasi.warp uses fd_write | case_tables | **R** (runtime.wasm) |
| runtime/ exact, exact_rounding, pow, pow_tables, durations, times, wasi (README "extended") | exact rationals, big ints, float `^`, durations | pure | — | **R-ext**: second module `runtime-ext.wasm`, or in runtime.wasm (types declared lazily today) |
| case_tables.warp | 33,348 B, 14 L: Unicode case ranges as texts (generated) | pure data | — | **R** (data of runtime/case.warp) |
| list.warp | 5,432 B, 77 L, 50 defs: unique, zip, enumerate… "every program, no use"; 11 glyph operator lines | pure + CT (operators baked, compiler-rebuilds.md) | — | defs **R** (typed ones), operators **C** |
| text.warp | 5,207 B, 85 L, 34 defs: repeat, pad_left… | pure | list | **R** |
| map.warp | 1,960 B, 27 L, 14 defs: merge, invert… | pure | — | **R** |
| time.warp | 7,658 B, 165 L, 23 defs: calendar over clock() | host (clock) | prelude civil_from_days | **R** (auto-used type module, prelude_words.warp:181) |
| prelude.warp | 27,993 B, 467 L, 91 defs; 29 `from warp` typed builtin lines, 35 operator lines, 2 rewrites | pure + CT; host paint/random/sleep_ms | — | defs **R**; typed lines/operators/rewrites **C** (they *are* the signature table) |

### Thin host-word wrappers (each word is one `std_io`/`std_pure` call)

| file | size | what | verdict |
|---|---|---|---|
| os.warp | 2,608 B, 13 defs | env, args, run, cwd… → std_io "os" | **I** (or in runtime.wasm): a module call that just calls the host doubles the hop |
| file.warp | 1,238 B | write, read_bytes… → std_io "file" | **I** |
| net.warp | 537 B | post → std_io "net" | **I** |
| json.warp | 975 B, 4 defs | parse_json/to_json → std_pure "json" (serde_json natively) | **I**: "json.wasm" buys nothing; a pure-Warp JSON parser would be **M** later |
| regex.warp | 736 B | matches (infix operator, CT) → std_pure "regex" (Rust regex) | operator **C**, words **I** |
| hash.warp | 300 B | sha256, crc32 → std_pure | **I** |
| wasm.warp | 1,152 B | load_wasm, wasm_call, wasm_exports → std_io "wasm" | **I** |
| random.warp | 793 B, 4 defs | choice, shuffle, sample over random_below | **I**/R (tiny) |

### Pure Warp, sizeable: the on-demand candidates

| file | size | what | host words | deps | today's compiler-self | verdict |
|---|---|---|---|---|---|---|
| sound.warp | 40,359 B, 812 L, 388 defs | synth: tone, chords, melody, WAV; played by host `sound_samples` | sound_samples, random, std_io | units (si_amount) | **refused** (si_amount) | **M** `sound.wasm`, best payoff, but after the compiler covers it |
| markup.warp | 25,088 B, 519 L, 60 defs | to_html of markup values (web-ssr) | — | — | **refused** | **M** `markup.wasm` |
| draw.warp | 7,916 B, 177 L | canvas, colors, shapes | draw_shape/show/shader, paint, save_picture, text_coverage | math | not tried | **M** `graphics.wasm` (with compute_toys) |
| compute_toys.warp | 5,396 B, 118 L, 24 operator lines | compute.toys WGSL → fragment shader | — | regex | not tried | **M** (graphics) |
| units.warp | 8,138 B, 193 L, 10 CT lines | run-time quantities; also the unit definitions the compiler reads | — | — | — | split: definitions **C**, run-time words **M** `units.wasm` |
| uncertain.warp | 5,452 B, 108 L | ± classes with operator methods | — | — | — | **M** (classes: needs exported constructors/methods) |
| router.warp | 5,471 B, 117 L | route patterns | page_path, response_header | — | — | **M** `web.wasm` (with wagi, i18n) |
| wagi.warp | 3,660 B, 87 L | CGI-style answers for `--wagi` | response_header | router | — | **M** (web) |
| i18n.warp | 2,820 B, 68 L | translations, plural rules | — | — | — | **M** (web) or I |
| shell.warp | 3,610 B, 61 L, 26 defs | ls/cat/cd as words | clock, std_io | os file list regex text | — | **M** `shell.wasm` (but mostly host calls) |
| agent.warp | 2,017 B | `agent "prompt"` (Anthropic API) | — | net json os | — | **M** (small) |
| case_table.warp | 2,816 B | upper/lower in warp, twin of the Rust builtin | — | case_tables | — | **R** once it replaces the builtin |
| extrema.warp | 1,668 B | min/max in warp (hot) | — | — | — | **I** (hot, wants specialisation/inlining) |
| fast_math.warp | 4,001 B | rebinds sin/cos/exp/pow to polynomials | — | — | — | **C/I**: a rebind is compile-time and only pays inlined |
| math.warp | 11,088 B, 253 L, 109 defs, 27 libm `from 'm'` lines, 65 operator/circumfix lines | math words, libm | libm (C) | — | — | defs **M** `math.wasm` (or R), libm lines and operators **C** |
| matrix.warp | 1,010 B, 15 defs | list-of-rows matrices | random | — | compiles (24,671 B program) | **M** (tiny; with math) |
| collections.warp | 1,644 B, 9 alias lines | Stack/Queue classes, foreign names as aliases | — | — | — | classes **M** later, aliases **C** |
| memory.warp | 434 B | reads the program's own linear memory | memory_byte | — | — | **I** (must run in the caller: it reads the *caller's* memory) |
| wasmtime.warp | 1,066 B | libwasmtime C API | FFI (via headers) | — | — | **M** (small) |
| PHP.warp, R.warp | 193 B, 187 B | foreign spellings (array_map := map) | — | — | — | **C/I** (aliases) |
| extra/netbase.warp | 849 B | netbase graph DB client | — | os json | — | **M** (package, not core) |

### FFI-backed (lib/native, lib/c.warp)

| file | size | what | verdict |
|---|---|---|---|
| native/sqlite3.warp | 3,667 B, 76 L: 26 C declarations + column_value, rows, run_query | **M** `sqlite3.wasm`: the warp words move to the module, the C imports go with them (measured: a module importing C `sqlite3` gave SQLite 3046001 on the box) |
| native/SDL2.warp | 1,216 B, 22 L, declarations only | **C**: nothing to precompile; the C import *is* the module. A `graphics.wasm` wrapper over it later |
| native/raylib.warp | 2,336 B, 54 L, declarations only | **C** (same); graphics.wasm later |
| native/z.warp | 394 B, declarations only | **C** |
| c.warp | 2,557 B, libc declarations | **C** |

python / crates: not in lib/ at this tip. Python is the host's foreign_call (crates/warp-runtime/src/foreign.rs, foreign_python.py, branch
ffi-python-adapter); crates are host-side adapters (crate-adapter tree). A python.wasm would be a one-hop wrapper: **I**.

### Stays compiler-side (C), and why

* **lib/sugar/*.warp** (8 files, 4.8 KB) and every `rewrite` line (channels.warp 10, english.warp 2, prelude.warp 2): they are **rewrite
  rules applied to the program's syntax tree before emission**. There is no run-time function to export; a precompiled module cannot change
  how the caller's source parses or lowers.
* **aliases.warp, operator/infix/circumfix lines** (math 65, prelude 35, compute_toys 24, list 11, units 10, collections 9, regex 1) and
  **entities.tsv**: parser tables (precedence, spellings). Parse-time by definition.
* **typed builtin lines** (`… from warp`, prelude 29) and **C/libm declarations** (`from 'm'`, lib/native): these *are* signatures, the
  compile-time view of something implemented elsewhere. Under this design they become the model for the module signature tables.
* **web.webidl** (122,733 B): browser bindings, compile-time declarations.
* **fast_math, extrema, memory**: semantics that only exist inlined in the caller (rebinding, specialisation, the caller's own memory).

Inference on size: a rich 6-line program carried 276 functions in 27,462 B, all but a few of them runtime words. A runtime.wasm with every core
word in its fixed exported forms is likely **200–400 KB** (inferred), i.e. ~0.4–0.8 s cold Cranelift and a ~3–5 MB .cwasm, by the 1.5 MB → 3.1 s / 18 MB ratio.

---

## 2. Linking design

### 2.1 Shared types: one frozen type prelude ("warp ABI types")

Every module (programs, runtime.wasm, sound.wasm …) starts its type section with the same prelude, emitted by the same code
(module_layout.warp's type list), each type its own **singleton rec group**, exactly as compiler-self emits today:

```wat
(type $text  (array (mut i8)))
(type $items (array (mut anyref)))
(type $list  (struct (field $length (mut i32)) (field $items (mut (ref $items)))))
(type $Node  (sub (struct (field $kind i64) (field $data anyref) (field $value (ref null $Node)))))
(type $i64box (struct (field $value i64)))   (type $f64box (struct (field $value f64)))
(type $Ratio (struct (field $numerator anyref) (field $denominator anyref)))
(type $MetaNode (sub final $Node (struct … $line i64 $column i64 $comment (ref null $Node))))
```

wasmtime canonicalises rec groups structurally across modules in one engine, so these are the same types everywhere (measured, x1).
Singleton groups are the robust choice: adding a new type later does not change the identity of existing ones; changing one existing
type's shape is an ABI break (measured: mutability or a `(rec …)` wrapper breaks the link). Lazily declared extended types
(`$Limbs`, `$BigInt`, classes) follow the same rule: one canonical definition, owned by the module that exports functions over them.
Rule: **the type prelude is append-only**; its hash is part of every module's ABI hash.

### 2.2 Exceptions, error detail, globals, memory

* **Tag:** runtime.wasm defines and exports `warp_error` `(param i32)`; every program and module **imports** it
  (`(import "warp:runtime@<abi>" "warp_error" (tag …))`). Tags are nominal per instance (measured: a program's own tag misses the
  runtime's throw). Today each program defines its own tag (`tag $warp_error`): with modules it must import it.
* **trap_detail:** runtime.wasm owns `(global $trap_detail (mut anyref))` and exports it; programs import it and **re-export** it as
  `trap_detail` so warp-host's failure reader (which reads the main instance's export) keeps working unchanged. Same for `try_depth` and
  `failed_in` (they are per-run error state, read by `raise`/try).
* **Runtime-private globals** (`counted_text`, `walked_*`, `other_*`, `lower_table`, `upper_table`: the text walk caches) move into
  runtime.wasm and are not exported.
* **Memory:** every module keeps its own linear memory, used only as WASI scratch (fd_write). wasmtime-wasi reads the *calling*
  instance's `memory` export (measured, x4: runtime.wasm printed a program's GC text through its own memory). No shared memory needed,
  no multi-memory. Values never live in linear memory in compiler-self output, so **no copy** across modules. The copy path in
  wasm_modules.rs stays for warp-rust-emitted modules (`text_heap`).

### 2.3 Names

* **Import module names:** `warp:<name>@<abi8>`, e.g. `warp:runtime@3fa91c0e`, `warp:sound@7d21b9aa`. The `warp:` prefix separates warp
  modules from C libraries (`sqlite3`, `m`, `c`), WASI and `host`, fixing the measured collision; the ABI tag in the import name makes
  a mismatched module **unlinkable by construction** and lets several ABI versions sit beside one binary.
* **Export names:** the word itself for its canonical form (`text_join`, `play`, `rows`); a fixed specialisation as
  `<word>:<kinds>` (`max:int,int`, `round:float`), kinds as in host_words.warp (`int float i32 text list node`). Runtime state as plain
  names (`warp_error`, `trap_detail`). Plus the existing `memory`.
* **Custom sections in every module.wasm:**
  * `warp.module`: `name`, `abi` (16 hex of sha256 over the type-prelude hash + the sorted signature lines), `content` (sha256 of the
    code), `uses` (other warp modules it imports, with their abi), `built` (timestamp, git tip of lib/).
  * `warp.signatures`: one line per export, tab-separated like warp.c_functions:
    `word  export-name  param-kinds  result-kinds  flags` (flags: `infix`, `method`, `default:<n>` for defaulted parameters).
* **In programs:** `warp.links`: `warp:runtime@3fa91c0e` … the modules the program needs, so warp-host can say what is missing
  before instantiating, and `warp-host build` knows what to bundle.

### 2.4 ABI: fixed signatures across a module boundary

The pure emitter specialises a function by its first call's kinds and compiles runtime words on first call. Across a precompiled
boundary that cannot happen. Rules:
1. A module exports a word only in **fixed** forms: its annotated kinds when it is typed (`tone(frequency: float, seconds: float,
   wave: text): list`), else the boxed form (all `anyref` in, `anyref` out).
2. The caller converts at the call (box an i64 into `$Node`, unbox the result), as it already does for host words with `node` kinds.
3. Hot generic words (min/max, list's unique/zip on ints) stay **source-included** until they are typed; a module may also export
   a few chosen specialisations (`max:int,int`).
4. Classes (uncertain, collections) export constructor + methods + their struct type in the module's part of the type prelude. Later.

### 2.5 What the compiler needs (M3, new files only)

* New `lib/compiler/module_imports.warp`: given `use sound` and a signature table for `sound` (`<modules>/sound.sig`, the
  `warp.signatures` section as text, written by the build step, so the compiler does not parse wasm), it turns each word into a **host
  word of the module `warp:sound@<abi>`**: exactly what c_imports.warp does for C functions (`with_c_functions(host, functions)`,
  "each becomes a host word of the module its library names, so the emitter imports `library.name` and calls it like any host word").
  The emitter's import machinery is reused; it needs the kinds `text`/`list` besides `int float i32 node`.
* Behind an explicit switch (`--link-modules` / `WARP_LINK_MODULES=1`) so nothing changes for anyone else until M3 is proven.
* Runtime words: in `emitter/runtime_words.warp`, a word found in runtime.sig is emitted as an import instead of compiled. This is the one
  hook inside an existing emitter file: one `if` calling into the new file. The tag/trap_detail/try_depth become imports.
* With signatures read from .sig files, the compiler no longer needs the **baked prelude/math/list/regex texts** for those modules'
  run-time words (compiler-rebuilds.md §1): a lib/ edit inside a module changes only that module's .wasm/.sig, never the compiler. The
  CT parts (operators, rewrites, typed builtin lines) stay compiler-side and still need the run_library() fix from compiler-rebuilds.md.

### 2.6 What warp-host needs (new file `warp_host/warp_modules.rs`, small hooks in wasm_modules.rs/ffi.rs)

* **Lookup** of `warp:<name>@<abi>`, in this order, nothing else, **never lib/**:
  1. `WARP_MODULES=<dir>` (developer's explicit choice, e.g. a fresh `build-modules` output);
  2. `<dir of the running binary>/modules/` (warp-host's, or a built executable's);
  3. `<bin>/../share/warp/modules/` (installed layout, as compiler-self.wasm is found today).
  File: `<name>@<abi>.wasm` with `<name>@<abi>.cwasm` beside; a `modules.json5` manifest per folder lists name, abi, content hash, built tip.
* **No staleness checks, no rebuilds.** warp-host never compares a module with lib/, never compiles lib/. A missing or different-ABI
  module is an error naming both sides:
  `warp-host: this program needs warp:sound@7d21b9aa; beside warp-host is sound@a90c11f2 (built 2026-10-23 from 7b437156b): recompile the program with this warp-host's modules, or install the modules it was built for`.
* **Machine code:** load the shipped `.cwasm` (Module::deserialize, as `warp-host f.cwasm` does). If it was built for another
  wasmtime/CPU, fall back to compiling the **shipped .wasm** into the user cache (cache::module, keyed by bytes + engine hash, as today):
  that is machine code of the same fixed module, not a lib rebuild. `WARP_MODULES_STRICT=1` refuses instead (for CI and release checks).
* **Instance per run:** one instance per module per run (today's `by_path`), runtime.wasm first; programs and modules import from it.
* **Errors:** (a) fix the misleading "C library … not found" for a `warp:` import the linker couldn't type-match: report the import's
  expected and found types; (b) pass the program's folder into `instantiate` (today `None`) for plain path imports.
* **`warp-host build`:** reads the program's `warp.links`, writes the executable and copies the listed `<name>@<abi>.{wasm,cwasm}` into
  `<exe-dir>/modules/` (beside, per the requirement); an `--embed` option to append them to the executable can come later.
* **Developer step:** `warp-host build-modules [--lib <lib>] [-o <dir>]` (or `scripts/build-modules.sh` first): compiles
  lib/compiler/runtime.warp → runtime.wasm and each module in a list (`lib/modules.list`) with compiler-self, writes .sig, .cwasm and
  modules.json5. Releases run it once and ship the folder. It is the **only** way a lib/ change reaches a module.

---

## 3. Feasibility checks (real outputs)

All on the box, warp-host 4e3808b76, release `host` profile. Files under `/workspace/cow/scratch/lwm/`.

**x1: two modules, shared GC types and tag, linked by warp-host's import-by-filename** (`program.wasm` imports `"runtime"` → `runtime.wasm` beside it).
Each check adds a digit: 1 same ref back (`ref.eq`), 10 runtime's text_join result read by the program, 100 runtime pushes into the program's list in
place, 1000 runtime-made `$Node` passes the program's `ref.test (ref $Node)`, 10000 runtime throw caught by program via imported tag with
`trap_detail` `ref.eq` the thrown text, 100000 runtime's mutable global read by the program.
```
$ warp-host program.wasm
111111
```
Negative controls:
```
A: program's $text declared (array i8) instead of (array (mut i8)):
warp-host: the C library runtime of the import runtime.text_join is not found (tried …/libruntime.so, … libruntime.so.6)
B: runtime's types wrapped in one (rec …), program unchanged:      same message
C: run from /tmp (program and runtime.wasm in x1/):                same message (bare name resolved against the working folder only)
D: import "lib/runtime.wasm", run from x1: 111111; from /tmp: warp-host: the import module lib/runtime.wasm: no module lib/runtime.wasm (looked in the working folder)
E: program defines its own tag $warp_error instead of importing it:
warp-host: raise          (the runtime's exception escapes the program's try_table)
```
**x3: bare names vs C libraries**
```
sqlite3.wasm (imports C "sqlite3"), program imports "sqlite3":
warp-host: the import module sqlite3: the modules import each other in a cycle: sqlite3.wasm → sqlite3.wasm
modules/sqlite3.warp.wasm by path, run with a sqlite3.wat in the working folder:
warp-host: the import module sqlite3: sqlite3.wat: this warp-host reads no WAT text: …
same, clean folder:
3046001          (sqlite3_libversion_number via the module's C import)
```
**x4: WASI from an imported module**: runtime.wasm's print_line (own memory, fd_write) printing the program's GC text:
```
printed by runtime.wasm through WASI, from the program's GC text
```
**x2: startup cost of N modules** (each 12,877 B, 200 functions, same type prelude; program calls one export of each; cold = empty
WARP_HOST_CACHE, median of 3; warm = .cwasm cached, median of 10). The box has 8 cores and other agents' load, so read as ±1 ms.
```
p0.wasm   cold    3.0 ms | warm   2.5 ms (min 2.2)
p1.wasm   cold   13.4 ms | warm   3.1 ms (min 2.8)
p2.wasm   cold   22.3 ms | warm   3.6 ms (min 3.3)
p5.wasm   cold   48.2 ms | warm   4.9 ms (min 4.8)
p10.wasm  cold   85.1 ms | warm   6.3 ms (min 5.6)
p20.wasm  cold  173.6 ms | warm  10.0 ms (min 9.2)
pbig.wasm (imports new_int from compiler-self.wasm, 1,520,527 B, 7,464 functions)
          cold 3112.0 ms | warm   7.2 ms (min 6.5) | .cwasm 18,117,744 B
```
**Cross-module call cost**: 10⁸ calls of `inc(i64)`: in-module 98 / 98 ms, imported from another module 120 / 118 ms.

**Real compiles with compiler-self** (integrate-host): `hello.warp` (a function + print) 1,294 B, exports `main memory greet _start`;
a 6-line program with sort/try/split 27,462 B, 276 functions, its own `tag $warp_error`, 18 globals; `use time` 1,886 B, `use json`
23,754 B, `use matrix` 24,671 B compile; `use sound`, `use markup`, `use sqlite3` refused "outside the core" (also with
sc-sugar-preposition-heads `33c994c6b`'s compiler-self). First compile after a cold cache took 21.8 s, later ones 0.8–1.1 s (keep cache).

---

## 4. Staged plan (cards)

Topic **lib-wasm-modules** (Soon, under ffi-libraries-roadmap). Cards in order; estimates are agent-days, inferred.

| card | what | touch points (new files first) | est. | conflict risk |
|---|---|---|---|---|
| **M0 lwm-host-namespace** | warp-host: `warp:<name>@<abi>` lookup (WARP_MODULES, `<bin>/modules`, `<bin>/../share/warp/modules`), shipped .cwasm, strict mode, clear ABI-mismatch and type-mismatch errors, program folder passed to instantiate. Tests: the x1/x3 WAT pairs as tests/ fixtures (new). | new `crates/warp-runtime/src/warp_host/warp_modules.rs`; 2–3 line hooks in wasm_modules.rs `import_file` and ffi.rs (skip `warp:` imports) | 1 | low: warp-host is ours; Rust is frozen except for the pure pipeline's needs, and this is one (needs Supervisor OK) |
| **M1 lwm-runtime** | `build-modules` step (script first) compiling runtime.warp → runtime@abi.wasm with every core word in fixed forms + `warp.signatures`/`warp.module` sections; .sig and .cwasm; measure size/cold/warm. Runtime owns tag, trap_detail, try_depth, failed_in. | new `scripts/build-modules.sh`, new `lib/compiler/module_exports.warp` (an "export everything with fixed kinds" mode of the compiler driver), `lib/modules.list` | 3 | **medium**: needs an emitter mode to export runtime words and to define-and-export the tag; keep it a new file plus one switch in compiler.warp; coordinate with the emitter owners |
| **M2 lwm-sqlite3** | first on-demand module: lib/native/sqlite3.warp's warp words (column_value, rows, run_query) → sqlite3@abi.wasm importing C `sqlite3`; a hand-linked program calls `warp:sqlite3@…`.`rows`. Blocked on pure-c-imports (`2caa11295`) reaching main. sound.wasm follows once compiler-self compiles sound.warp (si_amount). | build-modules list; no compiler change (the program side can be hand-written WAT or wasm_call first) | 1–2 | low |
| **M3 lwm-compiler-imports** | compiler emits imports: `use sound` + `<modules>/sound.sig` → host words of `warp:sound@abi` (c_imports.warp's path), boxing at the boundary; runtime words from runtime.sig become imports; tag/trap_detail imported and trap_detail re-exported; `warp.links` section. Behind `--link-modules`. Test: the suite count with the switch on vs off. | new `lib/compiler/module_imports.warp`; one hook each in `emitter/runtime_words.warp` (import instead of compile), `emitter/module_layout.warp` (import tag/globals, write warp.links), host_words kinds `text`/`list` | 4–6 | **high**: emitter files are owned by the Claude workers (AGENTS: stay out of lib/compiler emitter); do it as new files + minimal hooks, merged by warp-integrator, after announcing to warp-supervisor |
| **M4 lwm-build-bundle** | `warp-host build` copies the linked modules beside the executable (`<exe-dir>/modules/`); release.yml/packaging ships `share/warp/modules/` built once by build-modules | warp_host/aot.rs, packaging/ | 1 | low |
| **M5 lwm-more-modules** | graphics (draw + compute_toys over host words, later raylib/SDL2), markup, web (router/wagi/i18n), units run-time, uncertain (classes), math defs | build-modules list | per module 0.5–1 | low, once M3 works |

Not doing: thin std_io/std_pure wrappers as modules (json, os, file, net, regex, hash, wasm stay inline), sugar/rewrites/operators/aliases
as modules (compile-time by nature), fast_math/extrema/memory (only meaningful inlined).

Risks: (1) the type prelude freezes: any change to `$Node`/`$list` shape is an ABI break for all shipped modules (append-only rule, abi hash
catches it); (2) losing specialisation/inlining on hot runtime words could cost more than the program-size saving: M1 must measure a
text- and list-heavy benchmark both ways before M3 switches anything on; (3) .cwasm size (~12× the wasm) for shipping; (4) a .cwasm only
runs on the wasmtime that made it (warp_host.md): the fallback compiles the shipped .wasm, never lib/.
