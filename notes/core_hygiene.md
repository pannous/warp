# Core hygiene: what in core/ the compiler itself does not need (card unused-core-relocation, 2026-11-01)

User's question: did we keep core, extensions and library apart? What could be an extension: everything in the core
that the compiler itself does not use. Investigation only: nothing moved; every move below needs the user's OK
through warp-interviewer.

## How it was measured (no rebuild)

- **Static: in warp.wasm?** The emitter compiles a word on its first call, so the perf.map of a build lists exactly
  the functions reachable from compiler.warp's roots (8039 names, record
  /Volumes/Build/compilers/edce5137ec4f3ac8/stage-two.record). Each `name(…) :=` / `def name` of a core file was
  looked up there.
- **Dynamic: run while the compiler compiles itself?** The macOS samples (10 ms) of all 143 stage-two records in the
  store, stack addresses mapped to perf.map: 2893 distinct functions ever on a stack. A lower bound (a cheap word can
  be missed), but 143 runs make a 0 % file a strong sign.
- Plus reading: what each file is for, and whether compiler.warp's own source uses the feature (grep).
- Scripts and raw per-file numbers: the session scratchpad (perfile.py, hits.py), not kept.

Finding behind every row: **statically nothing in the compiler proper is dead**: all ~95 % of the definitions of
parsing/, passes/, analysis/, emitter/ are in warp.wasm, because a compiler must compile any program. "Not used by
the compiler" therefore means *a language capability the compiler's own source never writes* (dynamic % low, grep 0),
or *domain vocabulary*. The runtime words are different: the runtime is carried as source and emitted per program, so
"in warp.wasm" there really means "the compiler, as a program, uses it".

## Table

def = definitions in the file, wasm = of them in warp.wasm, run = of them sampled during self-compilation.
Homes: **C** stays compiler core · **X** core/extensions/ (an optional capability with compiler code) · **L** lib/ or
lib/sugar (plain warp / rewrite rules / data) · **V** domain vocabulary in core (a missing mechanism) · **T** toolchain,
not compiler.

| file / section | lines | def/wasm/run | used by compiler? | home | risk / what blocks the move |
|---|---|---|---|---|---|
| parsing/parser.warp (notation, kernel) | ~4800 | 221/216/147 | yes | C | — |
| parser.warp shader blocks `shader{…}`/`wgsl{…}` (3621-3720) + lexer.warp shader_brace | ~110 | –/–/0 | no | V→L | needs one notation mechanism "raw block of a named language" (see injection) |
| parser.warp `\:name` entities + core/entities.tsv | 20 + 7129 | –/–/– | no (never writes one) | L (data) | table read through module_source("compiler/…"): would read lib/entities.tsv; baked-input list changes |
| parsing/injection.warp: `sql "…"`, `sh "…"` templates | 217 | 21/21/10 | no | V→X/L | SQL and shell are domains; same raw-block mechanism, the library (shell.warp, a sql module) says what a hole becomes |
| parsing/foreign_forms.warp (Ruby/Kotlin declaration forms) | 118 | 5/5/3 | no | C (notation, welcoming) | decides what the parser reads next; sugar can't do it until the rewrite engine sees modifiers |
| parsing/number_words.warp (`one plus two`) | 65 | 5/5/5 | runs on every program, compiler writes none | L (lib/english.warp rewrite rules) | rewrite rules already skip words the program binds — the pass's only special rule |
| parsing/lexer, interpolation, phrase_blocks, parse_words, positions | ~530 | high | yes | C | — |
| passes/rewrites.warp (engine) | ~2100 | 230/225/193 | yes | C | — |
| rewrites.warp `std_module_names` (42 lib names: sound, draw, PHP, R …) | 1 long line | — | — | V | the compiler should list lib/ (LIBRARY_FILES) instead; prebuild loop in compiler.warp reads it too |
| property_accessors.warp `property_modules = ["prelude","draw"]`, `["window","display","canvas"]` | ~5 of 173 | 16/16/3 | no | V | a property is declared in its library; the trigger words should come from it |
| passes: walk, scope, definitions, lambdas, uses, prelude_words, arity_overloads, class_methods, node_matches/cases, plain_enums, enum_declarations, data_items, kind_changes, main_globals, sugar, shapes, number_keys, of_lookups, index_positions, loop_heads, library_calls, prefix_calls, clauses | ~7300 | mostly 70-100 % run | yes | C | — |
| passes/int_types.warp + int_type_shows.warp (`type Color = int`) | 90 | 10/10/4 | no (0 such types in core) | X | small, self-contained; class_methods reads its shows |
| passes/class_statics.warp (`static` fields) | 58 | 6/6/1 | barely | X | small |
| passes/property_accessors.warp (getters/setters) | 173 | 16/16/3 | no | X | lib/draw's canvas properties depend on it |
| passes/code_templates.warp (quasi-quote holes) | 104 | 14/14/2 | barely | X | meta-programming capability; rewrites share code |
| passes/tries.warp (`try … else`) | 94 | 15/15/4 | yes (35 uses) | C (kernel #9) | — |
| passes/multiple_assignments.warp (`q, r = …`) | 80 | 12/10/5 | some | L/X with tuples | the tuples extension already says it is its sibling |
| analysis/ (analyzer, signatures, diagnostics, source_checks, type_tests, words, kinds, types) | ~1600 | 85-100 % | yes | C | — |
| analysis/lints.warp | 88 | 12/12/2 | runs, rarely fires | C | warnings are compiler work |
| analysis/host_words.warp (draw_shape, gpu_*, fetch_*, serve_routes, sound_samples, clipboard_text, paint …) | 93 | table | 0 (compiler imports only WASI/file) | V | **missing mechanism**: a library declaring its host imports with signatures (as c_imports already does for C) |
| analysis/builtin_signatures.warp | 250 | 21/19/14 | yes | C | reads prelude's typed lines: already the right mechanism |
| emitter/ core (emitter, functions, closures, control_flow, instances, operators, literals, quotes, output, runtime_words, encoding, byte_codes, lists, module_layout) | ~3800 | 70-100 % | yes | C | — |
| emitter/prebuilt.warp (runtime.wasm for linking) | 1086 | 28/27/11 | yes (linked runs) | C | — |
| emitter/times.warp (time values, zones) | 147 | 3/3/1 | no | X (times) | intrinsics + host imports; decision runtime-split took the runtime half out already |
| emitter/c_imports.warp + c_functions.warp (C FFI) | 350 | 23/23/16 | scans every source; compiler calls no C | X (ffi) | the scan runs on every file; native warp's C calls go through primitive, not this |
| emitter/serializer.warp | 138 | 22/20/**0** | no | L or delete | its own todo: "Is this still needed?" (card serializer-needed) |
| emitter/host.warp (host imports; paint/shader bits) | 146 | 6/5/2 | partly | C + V | the paint/shader part goes with host_words |
| extensions/primitive.warp (`@primitive`, plain MVP wasm) | 838 | 89/89/5 | **no** (only `warp build native`) | X, but load on demand | `use`d unconditionally by compiler.warp; also read by uses.warp, walk.warp read_attributes |
| extensions/tuples.warp, safe_navigation.warp | 18 | rewrite lines only | tuples no, `?.` 5 uses | **L (lib/sugar)** | they are pure rewrite rules = sugar by notes/layers.md; "extension" here only means a separate file, they apply to every program |
| commands/commands.warp, commands/tests.warp (CLI table, `warp test` listing) | 512 | 55/55/**0** | no | T | tool code inside warp.wasm; fine but not compiler (could be lib/tools or core/tools) |
| native/ (warp.warp, driver.warp, tests.warp) | 2554 | separate program | no | T | the host binary, not the compiler; folder is honest about it |
| runtime/ core words (text, show, maps, lists, nodes, fields, equality, arithmetic, conversions, case, characters, errors …) | ~2860 | 393/205/167 | yes, half of them | C | the other half are words only user programs call: harmless, compiled on first call |
| runtime/gpu_lists.warp + gpu checks in lists/nodes/kinds | 57 + ~15 | 15/5/1 | **no, but paid**: is_gpu_list is 0.8 % of self-compile samples (123 per run) | X (gpu) / L | needs list-kind hooks like bridge.warp's extended hooks, so the core list words don't test for it |
| runtime/durations.warp (sits in runtime/, counted extended) | 81 | 18/2/0 | no | runtime/extended/ | file location only; a Rust test once read it here (gone) |
| runtime/extended/ (big_ints, big_ratios, exact, exact_rounding, pow, pow_tables, times) | ~1050 | 141/80/7 | **linked but not run** | X (already split) | reachable through bridge.warp hooks, so ~80 words ride in warp.wasm unused; acceptable |
| runtime/extended/wasi.warp (files) | 278 | 45/15/14 | yes (reads its sources) | C-ish | the compiler needs files; stays |

Breaches the other way (lib/ that is really compiler):
- **lib/wgsl.warp (859 lines) and lib/rule_words.warp (420) are BAKED** into the compiler (scripts/compiler-inputs.sh
  BAKED): every GPU/shader edit makes a new compiler build. rule_words.warp's one table `rule_words = {typed_json,
  route_verb, served, wgsl_shader, gpu_packed, …}` registers the compile-time words of json, router, web, wgsl, units
  in one place: a missing mechanism (a library declaring its own compile-time words, loaded when a rule needs them).
- lib/prelude.warp, aliases.warp, math.warp baked: right (they are the language's vocabulary the analyzer reads).
- lib/compiler → ../core link: transition only, as AGENTS.md says.
- core/extensions/ is not what README says: two of three are sugar (always on), the third is a build mode loaded always.

## Verdict

Partly. The folders inside the compiler are clean — parsing/passes/analysis/emitter hold compiler code, none of it
dead, sugar mostly moved to lib/sugar, the runtime split put rationals/times/pow in runtime/extended. But "core" still
mixes three things: the compiler proper (~85 % of the lines), language capabilities the compiler's own source never
writes (times, GPU lists, @primitive, C FFI, int types, properties, statics, code templates, sql/sh templates, shader
blocks: ~2400 lines + 7129 lines of entities), and domain vocabulary tables (host words, the 42 std module names,
draw/canvas words, number words, rule words) — the pattern common.md calls a missing mechanism. core/extensions/ is
not yet the home it is meant to be: it holds sugar and an always-loaded mode.

## The most valuable moves, ranked

1. **Libraries declare what they bring** (mechanism, then deletes vocabulary from core): a lib module declares its
   host imports with their signatures (as C imports already do) and its compile-time rule words; the compiler lists
   lib/ instead of std_module_names. Removes host_words.warp, std_module_names, property_modules/canvas words, the
   rule_words table, and un-bakes wgsl.warp (no compiler rebuild for GPU work). Core change, needs the user's OK.
2. **GPU out of core**: runtime/gpu_lists.warp + the gpu checks in list words → an extension behind list-kind hooks
   (the bridge.warp pattern); shader blocks with move 3. Gains the 0.8 % is_gpu_list cost on every program, the
   compiler included.
3. **One raw-block notation for foreign languages**: `shader{…}`, `wgsl{…}`, `sql "…"`, `sh "…"` become one
   parser rule (a named language's literal with holes), the meaning in the library (wgsl.warp, shell.warp, a sql
   module). Deletes ~110 parser lines and most of injection.warp's 217.
4. **Cheap, low-risk moves**: tuples.warp + safe_navigation.warp → lib/sugar (they are rewrite lines);
   number_words.warp → lib/english.warp rules; entities.tsv → lib/ data; durations.warp → runtime/extended/;
   serializer.warp → decide on card serializer-needed (0 runs).
5. **Make extensions truly optional**: primitive.warp loaded only for a `@primitive` file; times (emitter/times.warp +
   runtime) and C FFI (c_imports) as extensions under the same switch — the compiler pays nothing for them.
