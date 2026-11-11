# Shake of the extensions

Question: which parts of extensions/ are really libraries (code a program runs), so they can move to lib/ and load
only on demand, and which are language (what the compiler understands). The same analysis as notes/core_shake.md,
for extensions/. Analysis only: no code changed. Scratch and scripts: data/extension_shake/ (git-ignored:
outline.sh, users.sh, unreferenced.py, bytes_per_file.py and their outputs).

Not covered: the two data files already moving to lib/ with load on demand (cards webidl-on-demand, entities-on-demand:
extensions/js_globals/web.webidl, extensions/entities.tsv). Their .warp readers (js_globals.warp, entities.warp) are
counted below as language.

## Method

1. **Rows.** For each extension, its Extension row in extensions/extensions.warp (core/extension.warp says what a row
   holds): the hooks it registers (readers, passes, lints, implicit uses, runtime root, commands).
2. **Users.** users.sh: which files `use` each extension file. Fourteen `use` lines in core/ name extension files directly,
   not through the hook table (see "Side findings").
3. **Where its code ends up.** The function names of each file matched against the name sections of the installed
   warp.wasm (the compiler, 2,025 KB of function code) and runtime.wasm (the prebuilt runtime words, 116 KB of
   code) in bytes_per_file.py. Name matching is an estimate, because names are overloaded.
   - Extension code in warp.wasm: about 517 KB, a quarter of the compiler's code.
   - big_numbers/, times/runtime and gpu_lists/runtime are not in warp.wasm. They are in runtime.wasm: big numbers
     18.5 KB, times 3.7 KB, GPU lists 3.0 KB.
   - extensions/sugar/ and extensions/rules/modules/ compile to 0 bytes: they are rewrite lines, which the compiler
     reads as text.
4. **Dead.** unreferenced.py lists the top-level names that nothing else mentions in core, extensions, lib, web,
   scripts, test or samples.

### How code loads today: the fact that decides every class

- **Compile-time code cannot load on demand.** The compiler is one wasm module, so every pass, reader and rule word
  is in warp.wasm for every compile. Only rewrite *rules* load on demand: sugar/ (rules for every program) and
  rules/modules/ (the rules of a lib module, taken when a program uses that module).
- **Runtime words are in runtime.wasm**, which every run loads whole. A program compiled behind it is a few KB
  (`print x + 4`: 5 KB).
- runtime.wasm cannot call a lib module, because modules link *behind* it. The core runtime reaches the extensions'
  runtime words through the bridge hooks (core/runtime/bridge.warp: extended_sum, extended_text,
  extended_list_at …), so those bodies must sit inside runtime.wasm.

So moving something to lib/ with load on demand needs one of two core mechanisms that do not exist yet:

- **(M1) module rule words.** A lib module brings its own compile-time words, which the compiler runs through the
  host. Core: computed() in rewrites.warp, a host word in the
  driver, and its twin in the playground.
- **(M2) bridge slots.** A bridge hook becomes a slot that a module linked behind runtime.wasm fills when it is
  instantiated, for example a funcref global or table entry the module's start sets. The emitter links the module
  when a compile-time trigger shows the program needs it.

Without M1 or M2, the only move possible today is tool code (class D).

## Totals

| class | files' lines | lines that move/go | blocked on |
|---|---:|---:|---|
| A language (stays) | 10,543 | 0 | – |
| B split (hook stays, bulk moves) | 3,265 | 2,104 | M1 (compile-time domain words), M2 (runtime) |
| C library (all moves) | 2 | 2 | – |
| D tool | 954 | 435 | a dispatch line in core/native/warp.warp (precedent: `warp fix`) |
| **sum** | **14,764** | **2,541** (incl. ≈30 dead lines since deleted) | |

- **extensions/ now: 14,764 lines of .warp.**
  - The total includes 762 lines of sugar/ and 82 of rules/modules/: rules read as text, no compiled code.
  - Data files not counted: entities.tsv (7,129 lines) and web.webidl (3,503), both in flight.
- **After moving B's bulk, C and D's movable part: about 12,223 lines (−17 %).**
- **Moves possible without a core change: about 435 lines (−3 %).** That is D's `warp test`.
- The rest waits on M1 (about 1,050 lines) or M2 (about 1,055 lines).
- **Candid summary:** extensions/ is mostly language. Its library-shaped bulk comes in two kinds:
  - runtime words (about 1,050 lines). They are already out of warp.wasm but sit in the runtime.wasm every run loads.
  - library-bound compile-time words (about 1,050 lines). They are in warp.wasm for every compile.

  Neither kind can load on demand without a core mechanism.

## Class A: language (stays)

Grouped by what the row registers. Group sums are approximate (several small files are shared between groups); the class total, 10,543, is exact.

| lines | group | files |
|---:|---|---|
| 3,153 | passes (rewrite the program between core passes) | class_methods/ 1,199, conversions 512, index_positions 214, protocols 187, property_accessors 187, of_lookups 169, function_clauses 145, block_inlining 114, code_templates 102, multiple_assignments 78, iteration_inlining 69, number_words 67, operator_folds 58, class_extensions 61, key_bindings 55, object_functions 48, last_results 42 … |
| 1,669 | parser readers (atoms, numbers, suffixes, keywords, types, infix/prefix) | declared_operators 277, phrase_definitions 207, sum_types 162, shader_blocks 144, glyph_atoms 103, record/alias/attributes/pipe/circumfix/superscripts/percent/degrees/durations/time_literals/radix/float_suffixes/times_loops/event_phrases/foreign_declarations, entities 104 (its table in flight) |
| 949 | primitive mode (`@primitive`; builds the native warp, so the toolchain needs it) | primitive 851, primitive/prelude 98 (text, read only for @primitive files) |
| 820 | C imports and wasm modules (foreign lines, `use x.wasm`) | c_imports/imports 484, wasm_modules 302, section 27, root 7 |
| 790 | rules/rule_words.warp, the part that stays: reflection (`f.params`, help, dir), lints and small sugar words, the rule-word table | rules/rule_words.warp (of 1,510) |
| 762 | sugar/ (rewrite lines, read as text, 0 compiled bytes) | 28 files |
| 517 | emitter parts the core names directly | time_values 188 (time intrinsics, time_of), quotes 129, separate_module 187 (the linker: compiler.warp's linked_bytes, see "Side findings") … |
| 505 | checks and lints | effects 174, match_patterns 171, text_lints 91, int_misfits 84 … |
| 300 | `use js` (foreign_globals.warp 218, webidl.warp 82; its .webidl data in flight) | js_globals/ |
| 269 | int types | int_types/ |
| 227 | typed templates (`sql "…"`, `sh "…"`: injection-safe lowering, a text pass) | typed_templates |
| 225 | implicit uses (the on-demand trigger itself) | implicit_uses |
| 203 | inline HTML (`<div>` read as markup: a source pass; lib/fix reuses it) | inline_html/markup |
| 94 | GPU-list packing (rewrites how a list is made) | rules/gpu_packing |
| 82 | lib modules' compile-time rules (already on demand, with their module) | rules/modules/ |
| ≈160 | small rows and roots: go_tasks, test_lines, dir_hints, from_imports, local_shadows, block_definitions, parameter_values, collection_type_tests, match_phrases, enum_calls, c_enums, scoped_words, optional_unwraps, runtime.warp's list hooks, gpu_lists.warp row, big_numbers.warp root, commands roots … | |

## Class B: split (a hook stays, the bulk moves)

Ordered by lines moved per effort. Every row needs M1 or M2 first.

| moves | of | file | what moves where | trigger (on demand) | why |
|---:|---:|---|---|---|---|
| 689 | 1,043 | big_numbers/big_ints 317, big_ratios 108, pow 258, pow_tables 6 | a lib module big_numbers behind runtime.wasm (M2). exact.warp and exact_rounding.warp (305) stay in the runtime, because every int `/` makes a ratio. | a literal past 64 bits or 18 decimal places, an int `^`, a float `^`, `as rational` (all visible to the emitter) | 16 % of runtime.wasm's code, carried by every run, reached only by big literals and powers |
| ≈720 | 1,510 | rules/rule_words.warp domain words: signals 209, tasks/events/frames/pages 158, units 110, storage 91, typed JSON 82, web routes 37, laws 33 | each lib module's own rule words (M1): lib/signals, tasks, frames, markup, units, storage, json, web, laws | the module's use, which already brings its rules (rules/modules/<m>.warp) | the rules already load with their module; their words do not. |
| 214 | 214 | times/runtime 135, times/durations 79 | lib/time (M2). The bridge bodies in core/runtime/extended.warp stay. | time_values.warp's makes_time(program): `instant_at`/`time_of` in the lowered program | time values are a library domain; the compile-time trigger already exists |
| 152 | 152 | gpu_lists/runtime | a lib GPU module (M2). extensions/runtime.warp's list hooks and the 9-line row stay. | the row's library_words (gpu_zeros, gpu_list_of …) or a `[f32]` type | only GPU programs make GPU lists |
| 120 | 125 | fetch_signals | lib/net (M1). Check first whether plain rewrite rules in rules/modules/net.warp can do it: then no core change is needed. | the module's use (`fetch`) | a pass for one library word |
| 80 | 85 | components | lib/markup (M1). core/passes/passes.warp and prelude_words.warp `use` it directly, so those lines go too. | lib/markup's use | per-instance state of markup components |
| 79 | 79 | rules/shader_holes | lib/shaders (M1) | paint, shader_of (rules/modules/prelude, shaders) | the shader translator itself is already lib/shaders (e799249b58) |
| 30 | 34 | dev_values | lib/storage or the dev build (M1) | `warp build --site --dev` | only dev builds |
| 20 | 23 | page_values | lib/markup (M1) | `$who` and lib/markup | page fields are markup's |

## Class C: library (all of it moves)

| lines | file | where | trigger | why |
|---:|---|---|---|---|
| 2 | uncertain/operators.warp | lib/uncertain.warp, which already declares `infix operator ±`. What moves is the alias `+-` and the power [140, 141]. | implicit-operators (decision): `±` loads uncertain | the row bakes one library's operator into the compiler; verify that the declared operator with an alias covers it, then drop the row |

There is nothing else. No extension is a library brought by `use`: whatever programs *run* reaches the runtime through
the bridge (class B, M2), and whatever reads a library's words runs at compile time (B, M1).

## Class D: tool

| lines | file | where | how it would load | why |
|---:|---|---|---|---|
| 291 + 144 | commands/tests.warp + serializer.warp | a lib program lib/test/command.warp, compiled and run by the native warp exactly as `warp fix` runs lib/fix/command.warp (core/native/warp.warp:596). serializer folds into lib/repl.warp's "the value as warp source" writer, or stays beside it there. | on `warp test` | judging test cases is a tool. It parses with `parsed`, which a program reaches through parse_code. This answers card serializer-needed: the only user is `warp test`. |
| 257 + 78 | commands/build.warp, site.warp | stay | – | they call compile() and linked_bytes in process; the native half (wasm2c, cc) could move to core/native, but that grows the core |
| 184 | commands.warp, commands/commands.warp | stay | – | help, version, REPL, the stage views: the command row itself |

Risk: `warp test` is the Integrator's suite. As a lib program it must be compiled once and cached, as lib/fix is,
or every run pays for a compile.

## Side findings

- **The core bypasses the hook table.** These extension files are `use`d by core files directly, by path:
  - c_imports: compiler.warp, emitter.warp, prelude_words.warp
  - primitive: compiler.warp
  - quotes and time_values: emitter.warp
  - code_templates and components: passes.warp, and components also in prelude_words.warp
  - class_methods and int_types: definitions.warp, and int_types/classes also in arity_overloads.warp
  - uncertain/operators: emitter.warp, through module_source

  Any of these leaving extensions/ needs a core edit. They are "extensions" by folder, not by hook. A row field for
  each of them (a definition pass, emitter intrinsics) would make extensions/ removable row by row.
- **commands/separate_module.warp (187) is the linker, not a tool.** compiler.warp calls its linked_bytes for every
  compile that links modules apart (only_apart) and for every module `warp rebuild` writes (module_written_behind).
  It belongs beside core/emitter/prebuilt.warp. This is a placement fix that grows core by 187 lines (Interviewer).
- **time_values.warp (188) is emitter code without a row**: time intrinsics and time_of. When times become library
  values (dates-times-extraction), it goes with B's times row.
- extensions/rules/rule_words.warp keeps growing (1,510 lines); M1 is the only thing that stops that growth.

## Recommended order

1. D `warp test` (435): lib/test/command.warp plus one dispatch line in core/native/warp.warp (a small, generic core
   change on the `warp fix` precedent). Closes card serializer-needed.
2. C uncertain row (2): verify, then drop.
3. M2 bridge slots (a core decision through the Interviewer) unlocks big numbers (689), times (214) and GPU lists
   (152). That takes about 9 % of runtime.wasm's code out of every run.
4. M1 module rule words (a core decision) unlocks about 1,050 lines of
   compile-time domain words: rule_words' domains, fetch_signals, components, shader_holes, dev_values, page_values.
   Try plain rules for fetch_signals and page_values first.

Risks:
- **Compile speed.** M1 makes a host round-trip per rule word call, and signals rules run on every program using
  lib/signals.
- **Self-compilation.** Nothing in B, C or D is needed by the compiler's own build, which already leaves big_numbers
  out (COMPILER_BUILD_EXTENSIONS). separate_module is needed by `warp rebuild`.
- **Playground.** M1 and M2 each need their twin in web/playground (a host word for M1, linking an extra module
  behind runtime.wasm for M2).
