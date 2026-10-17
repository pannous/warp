# Self-hosting: warp's compiler written in warp
Roadmap card: self-hosted (board topic; its sub-cards are the steps below, notes/roadmaps.md).

Goal (issue #33): write the whole parser and as many components as possible in warp itself. A secondary goal that
must not bias the language design (wiki/Features.md "self hosting", wiki/goals.md).

ACTIVE again (decision self-hosting-resumed, 2026-10-17): self-hosting is the fleet's next big task. Workers finish
their current work, then switch to it. The first step is a readiness survey (session warp-self-survey): are all the
language parts there, or should macros (or a cheaper feature) come first so the warp compiler doesn't bloat? The twins
on main stay and keep being tested.
Readiness survey result: notes/self_hosting_readiness.md, which features come before the port (no macros;
forward-function, data-splice, data-pattern-sealed, parsed-kinds).

Decision self-hosting:
- Purpose: dogfooding (the compiler in warp shows what the language lacks), independence from Rust, a showcase.
- Scope: the parser first (lexer, parser, serializer); then section Next phase.
- Integration: twins, checked. Each warp part runs next to its Rust twin, and a differential test over every sample
  compares them. The Rust part goes only when its twin matches everywhere.
- Language gaps are fixed in the language as cards of this topic, never worked around in the warp code.
- Matching Rust is a check, not the goal: the twins must not copy Rust's imperfections. Where the warp part's clean
  result differs because the Rust result is a quirk or bug (an odd tree shape, a wrong kind, a lost position), the
  warp part keeps the clean result, the difference goes into the expected-deviations list the differential reads
  (notes/self_hosting_dropped.md, section Deviations, with an example each), and the Rust side gets a bug card or,
  when it is a design question, a warp-interviewer question. A twin that matches 100% gets a quick review for copied
  quirks before its Rust part is removed.

Size of the job (Rust lines, 2026-10-12): parser 7.4k, Node 2.1k, analyzer 6.6k, emitter 23k, lowering + top-level
src ~59k.

## Language support the twins rely on
- Mutable parser state in an object, char-by-char scanning, growing token lists, computed keys `{(n): 3}`,
  `data (x: 3)` with `.x` (probes/self_hosting/).
- `text[i]` is linear over a scan with look-backs: text_grapheme_offset keeps two text cursors (relies on runtime
  texts never changing in place); an ASCII step costs ~300 fuel (tests/text/test_scan_by_index.rs,
  test_ascii_index_costs.rs).
- Node API on data values (src/wasm_emitter/reflection/node_api.rs): `n.key`, `n.value`, `n.op`, `n.name`,
  `n.bracket` (a field of that name wins), `symbol(text)`, `key(left, op, right)`, `bracketed(items, "(")`; the kind
  is `type(n)`, items `n#i`, `#n`, `for x in n` (tests/node/test_node_api.rs). Missing: a list's separator is not
  kept in the wasm Node (card self-node-separator).
- Text building: text_append gives a copied text of 64+ bytes as much room again and remembers 8 rooms
  (src/wasm_emitter/text/text_rooms.rs): 400k appends to each of two texts in 70 ms
  (probes/self_hosting/text_builders.warp, tests/text/test_text_builders.rs). Lists grow in amortised O(1).
- A character inside a list serializes as `"x"` (values stay characters; Node::item_text, serializer.warp
  item_text). A `"` or `\` in a text is escaped by `\` in the runtime print (text_quoted), Node::serialize and
  serializer.warp (`quoted`). Read a message's text with Display (`to_string()`), never
  `serialize().trim_matches('"')`, which keeps the escapes.
- Deep recursion: the warp command on macOS links a 512 MB main stack (build.rs) and gives wasm all but 16 MB of it
  (warp_runtime::engine::give_main_stack_to_wasm), about 5 million frames. Linux takes `ulimit -s`; test threads,
  tasks and standalone `warp build` binaries keep the default.

## Steps
| # | Card | Step | Builds on |
|---|------|------|-----------|
| 1 | self-text-index | O(1) `text[i]` and slicing while scanning | none |
| 2 | self-node-api | Node API for data values | none |
| 3 | self-byte-buffer | Growable byte buffer and string builder | none |
| 4 | self-differential | Differential harness: warp twin vs Rust part over every sample and probe | none |
| 5 | self-lexer | Tokenizer in warp (lib/compiler/lexer.warp) | 1, 4 |
| 6 | self-parser | Parser in warp producing the same Node as src/warp_parser (lib/compiler/parser.warp) | 2, 5 |
| 7 | self-serializer | Node::serialize in warp (lib/compiler/serializer.warp) | 2, 4 |
| 8 | self-showcase | The warp-written parser as a sample: warp parses itself | 6 |
| 9 | self-next-phase | Plan the next parts: passes, analyzer, emitter (steps 10–18) | 6, 7 |

Rules
- Each step lands with its differential test: same Node, same text as the Rust twin. The harness
  (tests/common/twins.rs): `differential(part, word, rust_twin)` runs `word(read(path))` of
  lib/compiler/<part>.warp for every sample and probe, 64 sources per program (~1300 sources, a few seconds, under
  300 MB) and fails on the first source whose serialized value differs, with both texts around the first difference.
  First twin: lib/compiler/positions.warp `line_widths` (tests/parser/test_self_differential.rs). A twin's list is
  built with `warp_list`: warp's empty list is ø.
- The lexer's twins (tests/parser/test_self_lexer.rs): the Rust parser is scannerless. Its one real lexer pass is
  `comment_and_text_spans` (warp_parser/mod.rs); lexer.warp `spans` matches it on every source. The tokens after it
  get their twin from the parse tree's positioned atoms.
- A new gap becomes its own card in this topic (`todo add … --topic self-hosted`).

## Serializer twin
lib/compiler/serializer.warp `serialize(view)` writes a node as Node::serialize does
(tests/parser/test_self_serializer.rs). warp has no Node of its own for the parse of a source (no `parse` word giving
Nodes, `data` drops Meta), so both twins share a plain-data view (tests/common/twins.rs node_view): a number, yes, no
or ø as itself, every other node a list tagged by its kind (["key", op, left, right], ["list", bracket, separator,
items…], ["literal", source, node], ["attribute", name, value, node], …).
- `view_differential(part, word)`: the view goes to warp as JSON text read by `json(…)`; the same view as a warp
  literal is code to compile (a 100 KB chunk took 2.4 GB and 56 s in a debug build).
- Rust's Display shows up where serialize writes a key's left side (`name: 1`, texts bare); everything else Display
  prints is serialize again, so the twin needs no Rust-only text. A Data node without a unit description is written
  `Data(…)`. Small floats keep their exponent on both sides (`2.5e-6`).
- The twin uses `x is list`, not `type(x) == "int"` (a hint points to `type(x) == int`).

## Next phase: passes, analyzer, emitter
Order: lowering passes first, then the analyzer, then the emitter; only the bootstrap waits for all three.
- Next item: lib/math.warp and lib/units.warp become complete standalone warp files, and the parser fetches its
  lists of special functions and operators from them instead of hard-coded Rust tables (card standalone-math-units).
- Lowering passes: src/pipeline.rs runs ~150 passes, each `fn(Node) -> Node`, in SOURCE_PHASES and MEANING_PASSES. A
  pass is the smallest unit with a twin; early self-contained passes first; each twin can replace its Rust pass once
  it matches everywhere. They take node_view data, the form parser.warp's `tree` writes
  (probes/self_hosting/next_phase/view_pass.warp rebuilds a view bottom-up). A `parse` word giving real Nodes becomes a
  card once a pass needs Meta the view cannot carry.
- Pass twins (cards twin-harness, times-phrases; tests/parser/test_self_passes.rs): `pass_differential(pass, part,
  word)` (tests/common/twins.rs) takes the program from `pipeline::around_source_pass(code, pass)`, the Rust source
  passes before `pass` run as compiling runs them, hands it as view JSON to `view_of(word(node_of(…)))`, and compares
  with the view after the Rust pass. A twin is a word from node to node on real Nodes (so data(…) templates work):
  lib/compiler/walk.warp has the bridge (`node_of`, `view_of`; an error, a type or sealed data stays opaque) and the
  shared walk (`children(n, f)`, `bottom_up(n, f)`; a group is `n.bracket != ø`). Both sides drop literal and attribute
  layers, read a character as a text and an empty group as ø (card empty-group-node, open). `bridge_differential`
  checks the bridge alone. Twins so far: times_phrases.warp, number_words.warp (with its hints).
- Positions and reports (card carries-line): where the Rust pass hints, warns or makes an error, pass_differential
  gives the twin the positioned view, ["at", line, column, view] around each positioned node (twins.rs
  positioned_view); walk.warp node_of makes it `positioned(node, line, column)`, read back by `line_of(n)` /
  `column_of(n)` (ø for a node a pass made; `n.line` itself fails there). The twin says `report(n, message,
  preferred)`, and the call `reported(view)` compares [view, reports] with [view after the Rust pass, its hints and
  warnings as [line, column, message, preferred]] (pipeline::around_source_pass_reporting). Trees are compared
  without positions. First twin with reports: number_words.warp.
- The analyzer: its result is the kind of each definition and value, not a tree. It needs scopes (text-key maps work).
- The emitter: needs byte output and float bits; its twin compares run results, not bytes.

| # | Card | Step | Differential check | Builds on |
|---|------|------|--------------------|-----------|
| 10 | twin-harness | Pass twin harness | `pass_differential(k, part, word)`: Rust passes 0..k-1, the view as JSON to the warp word, compared with node_view after Rust pass k | 4 |
| 11 | times-phrases | First pass twins: number_words, times phrases (word_slices was deleted with park-phrases) | as 10 | 10 |
| 12 | carries-line | node_view keeps line:column where a pass reports, so hints and errors are compared too | diagnostics compared | 10 |
| 13 | parser-tree | Chained: parser.warp `tree`, then the warp passes in pipeline order | node_view after the same Rust passes | 6, 11 |
| 14 | inferred-kinds | Analyzer twin: the kind of each definition and top-level value, as text | the same text from src/analyzer | 10 |
| 15 | emits-wasm | Emitter core in warp: ints, floats, functions, if, while | same run value as Rust's module, per sample in that subset | 16, 17 |
| 16 | path-list | write_bytes / read_bytes for binary files | gap (probes/self_hosting/next_phase/byte_output) | |
| 17 | f64-const | float_bits / bits_float: a float's IEEE bits as an int | gap (float_bits) | |

The bootstrap (warp compiling the warp-written compiler) is planned once 13–15 cover the compiler's own subset.

## Emitter core (card emits-wasm, step 15)
lib/compiler/emitter.warp `wasm_module(program)`: the lowered program (pipeline::lower, as a node via walk.warp
node_of) gives a wasm module as a list of bytes, main returning the program's value; a program outside the core gives
the text "outside the core: <first form>". Module: type, function, export (main), code and a name section with
function and local names. No GC types, no memory: the core needs neither.
- Kinds: int = i64, float = f64, a condition = i32. A parameter takes its annotation, else the kind of its first
  call's argument; a function's result is read twice, so a recursive call knows it (the pending kind takes the kind
  it meets). `%` is Euclidean as in Rust's emitter (r + |b| for a negative truncated remainder).
- Twin (tests/parser/test_self_emitter.rs, tests/common/twins.rs emitter_differential): the module runs
  (wasm_reader::read_bytes) and its value is compared with eval(source); a float equals another number of exactly
  its value (3.5 * 2 is 7.0 here, 7 in Rust). probes/self_hosting/emitter/core/ holds one program per form, all inside
  the core; the checked share of every sample and probe must have at least a floor of programs inside.
- Dev loop: probes/self_hosting/emitter/emit_lowered.sh <file.warp> (lowered text → wasm_module → validate → run).
- Not in the core yet (outside, not dropped): int `/` (an exact rational), decimals as exact rationals (the core reads
  a decimal as f64), ints beyond 64 bits (Rust promotes), a yes/no result, a loop's value, texts, lists, every
  runtime call, among them the memo_* calls the memoization pass gives a function calling itself twice (fib), and
  library words a user function's name meets (`add` becomes add·any).
- Kinds come from emitter.warp's own small inference until lib/compiler/analyzer.warp `kinds(program)`
  (warp-self-analyzer, card inferred-kinds) lands; then the emitter reads its kinds from there.

## Runtime calls into prelude words
Runtime wasm functions (list_join …) are compiled before the user functions, so they cannot `call` a warp-written word
by index. `late_call(callee, params, results)` (src/wasm_emitter/functions/late_calls.rs) gives the callee a funcref
global `late·<callee>` that main fills with ref.func before anything runs; the runtime calls it with call_ref. The
loader loads the word through an `implied_words` hook in modules::called_names (radix.rs: `hex`, `binary`, `byte` …
imply hex_text / binary_text, lib/prelude.warp over to_base). A host calling an exported runtime function before main
would find the global null (trap): only main-run code uses it. A warp word with `%` or `//` on int pulls the
exact-rational runtime (~5 KB, card int-i64).

## Text search words in warp: measured, not moved
probes/text_search/words.warp writes trim, starts_with, ends_with and find over byte_at / byte_slice. Benchmark:
bench_find_builtin.warp vs bench_find_warp.warp (20 000 finds in a 2 KB text), `warp compile --wasm` each, then
`hyperfine -N -i 'warp bench_find_builtin.wasm' 'warp bench_find_warp.wasm'`. With `byte_at`/`t.bytes` inline, warp
find is 1.7x the builtin (0.50 s vs 0.29 s); module size equal. Left: the matches_at call per offset and fuel (card
fuel-metering). Rerun once that lands, then decide whether find moves to warp; TEXT_FIND's second caller, `in` /
contains (fields_and_keys.rs), then needs a late_call.

## Library sweep: Rust that becomes lib warp (card library-sweep, 2026-10-17)
Survey of src/wasm_emitter runtime functions and src/lowering source expansions that are plain logic. Of ~33k
emitter lines ~5k are movable; the rest is CORE (GC layout, hash tables, list cells and cursors, big-int limbs, host
imports, inline instruction selection). Every move is a card in topic self-hosted; features they wait on are cards too.
Ranked by Rust lines removed (≈ of lists/maps is lists-maps; trim/find words are card-3):

| # | Card | Rust lines | Waits on |
|---|------|-----------|----------|
| 1 | uncertain-warp + uncertain-text-warp (numbers/uncertain.rs) | ~760 | ± literals build the class |
| 2 | time-arithmetic, time-zones, time-text, time-fields, duration, civil-days-dup (time/) | ~990 | time-primitives |
| 3 | exact-text, exact-rounding, ratio-gcd, exact-pow (numbers/exact.rs) | ~670 | lib-from-runtime (print path), speed |
| 4 | list-text-warp (list_join's loop) | ~380 | kind-reflection |
| 5 | float-text-warp + float-as-exact-warp | ~365 | float-only-ops |
| 6 | shared-array-class (lowering/collections/shared_arrays.rs) | ~350 | index operators, Iterable |
| 7 | number-parsing-warp (text_as_float/int/number) | ~335 | correctly rounded scaling |
| 8 | sort-warp (merge sort + node_order) | ~225 | mutable-array |
| 9 | library-word-tables (library_words.rs tables) | ~170 | word alias declaration |
| 10 | case-table-warp, min-max-lib, type-name-warp | ~150 each | lib-constant-tables, lib-inlining, kind-reflection |
| 11 | range-type, deep-copy, paired-lists, channel-class, map-entries, slice, graphemes | 90-120 each | various (cards name them) |
| 12 | split, live-loop, try-assert, swizzle, text-set-char, contains, tree-walk-lib, parallel-map, signals | 60-80 each | |
| 13 | text-quoted, insert-at, element-children, node-codes, reverse, log-root, zero-fill, pop-clear, codepoint-count, builtin-signatures, small-dispatch | 15-55 each | |

Features the moves need (cards): time-primitives, kind-reflection, float-only-ops, mutable-array, lib-constant-tables,
lib-inlining, lib-from-runtime (late_call above is the mechanism), list-become-word. Already there: overloading by
arity, default parameters, variadic `xs...`, operator methods on classes, block parameters, `fields(p)`.
Speed is the main risk: ratio_new, contains and min/max are hot; each move measures against the builtin first (as the
text search words above).

## Speed of a warp-written compiler (user discussion, 2026-10-17)
No fundamental reason for "several times slower". Measured: warp `find` 1.7x the hand-emitted builtin (both wasm),
parser.warp tree of 100 KB ~0.6 s. The only fixed cost is wasm vs native: Cranelift optimizes less than LLVM, GC
objects cost more than Rust's stack values; typically 1.2-2x, less for allocation-heavy tree walking (a compiler);
`wasm-opt` (supports GC) narrows it. The rest is today's code generation, fixable and good for every user program:

| Cost today | Why | Fix |
|---|---|---|
| Node fields are `anyref` | a checked cast on every read | typed structs for typed values |
| Numbers boxed (`$i64box`) | an allocation per number | unboxed i64 fields, i31ref |
| Lists are cons cells | `n#i` is O(i) | wasm GC arrays |
| `text[i]` walks graphemes | ~300 fuel per ASCII step | byte indexing (`byte_at`) |
| Unbounded ints | overflow checks/masks per operation | i64 path when the type is known (card int-i64) |
| Fuel metering on | a counter per loop and call | off for trusted code (card fuel-metering) |
| Little inlining | small functions stay calls | emitter inlining or wasm-opt |

Host after self-hosting: the native `warp` stays a Rust shell (wasmtime, host words, FFI, graphics, sound) embedding
`compiler.wasm` precompiled to .cwasm; the playground loads the same file. New interface: compiler imports for
reading `use`d files, FFI signatures, the host-word list.

## Ahead-of-time executables and a host made of FFI (2026-10-17)
Exists: `warp build x.warp` (and plain `warp x.warp`) writes a standalone executable = crates/warp-runtime stub
(wasmtime runtime + GC, no Cranelift) + the program's .cwasm appended: hello world 1.77 MB, 109 KB of it machine code.
`warp compile --aot` writes the .cwasm alone. The stub provides basic host words, libm, fd_write and foreign_call
(Python/JS over JSON lines); anything else is refused: `use c; strlen("hello")` → "c.strlen used runtime"
(scratch/aot probe). So C FFI (libloading + header.rs signatures) exists in warp but not in the stub.
Idea "host from FFI + wasm": put the generic C FFI bridge into the stub; then graphics/sound/windows can be warp
modules calling SDL/raylib/CoreAudio through FFI instead of Rust crates (wgpu, cpal, winit), and the native host
shrinks to wasmtime + FFI bridge (see the self-hosted-modules direction). Hard parts: C callbacks on other threads
(audio callbacks: a wasmtime Store is single-threaded → ring buffer in shared memory), GC refs never cross to C
(copies or handles, notes/ffi_handles.md), macOS windows on the main thread.
wasmtime stays bundled in every executable: even with the program precompiled (.cwasm) it is the GC, the loader and
the trap handling. Replacing it (WAMR's wamrc AOT with its own GC; wasm2c lacks GC) is not on the table.
Shared runtime library (`warp build --shared`, libwarp-runtime.dylib): only an option, card exe-shared-runtime (Later):
no total saving, loses self-contained files, and a .cwasm only runs on the exact wasmtime that compiled it.
