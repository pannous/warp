# Tree shake of the core (2026-11-09)

Question: what of core/ does the compiler really use, so that what is clearly not core can go to extensions/ or lib/
(decision core-is-self: the core is what the compiler needs to compile itself) and what is tangential stays. Analysis only:
no code changed. Scratch and scripts: data/core_shake/ (git-ignored: reach.py, classify.py, classes.py, groups.json).

## Method

1. **Dynamic use (the main evidence).** scripts/core-coverage.py instrumented the compiler (every function entry and
   every branch calls a counter). The compiler was built by warp.wasm from core/ (stage 2, unoptimized so no function is
   inlined away: 6,500 functions, 66,000 points) and then run on
   - its own source: `compiler.warp` (self-compile), plus the runtime build (`-- runtime out.wasm`): together 31,800 points hit,
   - all 265 files of samples/ (41,400 points hit, for "is a feature used outside the compiler").
   Tests (test/) were not run: a feature only a test uses is class C, flagged by its
   sample hits where there are any.
2. **Static reachability (the cross-check).** reach.py splits core/**.warp into 4,719 top-level definitions and builds the
   graph "definition mentions name" (identifiers, plus the identifiers that are a whole text, because runtime words
   are named by text). Roots: compiler.warp's statements and every global initialiser. Static reach alone says
   about 95 % is reachable (names are overloaded, words are looked up by text, so it over-approximates), so it serves
   to find what is *not* reachable: dead candidates, each confirmed by a repo-wide grep of core, extensions, lib,
   test, samples, scripts, web.
3. **`warp build --trim` is not a reachability tool** (it only runs wasm-opt); no tool of the repo shakes the *source*.

A definition is **entered** when any of its functions ran in the self-compile or the runtime build. Entered does not
mean needed: a check that scans every token (number words, spacing, foreign forms) is entered and never fires. Hence
two columns below: *entered* and *ran* (the share of its code, by wasm lines, that executed).

Caveats: line counts are source lines of the definition including the comment block above it; the sum of all
definitions is 28,274 of 30,181 file lines (the rest are file headers, blank lines and `use` lines).
Branch shares are estimates (wasm lines per branch). Names are matched, not types, so same-named overloads count
together. The native harness (core/native, wasm2c-built native `warp`) could not be instrumented; it is classified by
function family, and static reach is useless there (words are dispatched by text through `answered_word`).

## Totals

(As first counted. Afterwards: D shrank to 6 lines, and 26 of its lines plus the hash words (107) and the commands group
(97) belong to A, not C; the two sound small C groups, number words (54) and foreign declarations (101), are moved.
The table below is not recounted.)

| | lines of definitions | of which compiler | of which native harness |
|---|---:|---:|---:|
| A self (stays) | 22,068 | 20,499 | 1,569 |
| B tangential (stays, flagged) | 2,380 | 1,562 | 818 |
| C feature (move) | 3,794 | 2,434 | 1,360 |
| D dead (delete) | 6 | 6 | 0 |
| **sum** | **28,274** | 24,527 | 3,747 |

- **Current core size: 30,181 lines in core/ (26,369 compiler, 3,812 native harness).**
- Moving the curated C groups (3,068 lines) and deleting D (6): **about 27,081 lines left**
  (10 % less). The 726 lines of unreached helpers (class C, last C table) are
  not counted: they go only with the feature they belong to, so the floor is about 26,355.
- Class A is 78 % of all definition lines: the core is already lean at the definition level.
  The slack is **inside** entered functions: 7,717 of the compiler's entered lines are branches the self-compile never ran (refusals,
  rare forms). They are not removable by tree shaking; the split candidates table lists the biggest.
- scripts/vitals.py (the 30 % aim) was not rerun; the numbers above are by lines of definitions, not its measure.

## Class A: self (stays)

Entered by the self-compile or the runtime build, or top-level statements that run at load. Per folder:

| lines | folder | what |
|---:|---|---|
| 7945 | `core/passes` | the lowering passes: rewrite engine, preludes, overloads, lambdas, uses, scope, enums, node matches; the compiler runs all of them on its own source |
| 4784 | `core/emitter` | wasm GC code generation: expressions, functions, closures, instances, control flow, module layout, runtime linking |
| 3346 | `core/parsing` | lexer and recursive-descent parser; the largest single file is parser.warp |
| 1979 | `core/runtime` | runtime words the compiler itself links (text, lists, maps, numbers, show, equality, WASI) |
| 1811 | `core/analysis` | analyzer, signatures, kinds, source checks and diagnostics |
| 1569 | `core/native` | native harness: wasmtime C API, run loop, paths, rebuild, promote, machine-code cache |
| 402 | `core (top)` | compiler.warp, extension.warp, node.warp: the driver and the Node enum |
| 230 | `core/runtime/extended` | WASI runtime words (files, args, environment) the self-compile uses |
| 2 | `core/commands` | the command table |

The largest files: core/parsing/parser.warp (3,004), core/passes/rewrites.warp (1,890), core/emitter/emitter.warp (1,556), core/native/driver.warp (1,230), core/passes/prelude_words.warp (1,069), core/analysis/analyzer.warp (910). The full per-file tables are in the git-ignored scratch data/core_shake/tables.md (classes.py writes it).

### Entered, but mostly cold: split candidates

Single functions where more than half of the code never ran while compiling the compiler:

| cold lines | function |
|---:|---|
| 127 of 130 | `core/emitter/runtime_words.warp` `intrinsic_of` |
| 91 of 163 | `core/parsing/parser.warp` `continued` |
| 89 of 174 | `core/emitter/emitter.warp` `key_expression` |
| 79 of 154 | `core/emitter/emitter.warp` `module_compiled` |
| 63 of 114 | `core/passes/prelude_words.warp` `with_reached` |
| 60 of 143 | `core/emitter/emitter.warp` `group_expression` |
| 55 of 73 | `core/parsing/parser.warp` `prefixed_by_symbol` |
| 51 of 84 | `core/parsing/parser.warp` `items_until` |
| 43 of 59 | `core/parsing/parser.warp` `named` |
| 43 of 74 | `core/emitter/emitter.warp` `assignment` |
| 43 of 62 | `core/compiler.warp` `compile` |
| 42 of 67 | `core/emitter/operators.warp` `unmarked_operation` |

`intrinsic_of` is a table of all intrinsics of which the compiler uses a handful; `key_expression`, `group_expression` and
`module_compiled` branch by language form. A split by form is the natural next shake, but it is a core change
(common.md "Cards": through the Interviewer).

## Class B: tangential (stays, flagged)

Not entered by self-compilation, but runtime words every program may need, general emitter machinery, declaration-only
heads (`name(args): type from runtime`, the interface between the emitter, the runtime and extensions) and error reporting.

| lines | file or group | why |
|---:|---|---|
| 552 | `native: C foreign function interface` | calling arbitrary C and guest wasm libraries with typed parameter crossings; only library programs declare foreign words, but every library host word needs it (stays: the libraries-not-core libraries (graphics, sound, sqlite) call their C library through exactly this FFI) |
| 215 | `core/emitter/prebuilt.warp` | linking of the prebuilt runtime image: raisers, import stubs, relocation; the cold half is the rebuild-without-image path |
| 140 | `core/runtime/types.warp` | type-name and type-test words (`type_of`, `is`), spelled per kind; every program may ask |
| 138 | `native: eval and parse code words` | `eval` / `parse` of source at run time, built back into Node values (stays: the only way a program reaches the compiler at run time) |
| 132 | `core/runtime/errors.warp` | one raiser per runtime error: the compile succeeds, so none fires |
| 128 | `native: shell command words` | run a shell command and capture its output: a process host word (stays: core host words are files, processes, args, env, time (roadmap_native_harness.md step 5)) |
| 102 | `core/runtime/intrinsics.warp` | bodiless heads of the intrinsics the emitter supplies (decision bodiless-imports) |
| 93 | `core/emitter/module_layout.warp` | wasm module section writers (function bodies, names, relocations, runtime exports) |
| 91 | `core/emitter/emitter.warp` | general emitter helpers: display forms, address helpers, errors of the emitter |
| 69 | `core/emitter/runtime_words.warp` | wasm_instruction, memory_instruction, type tests: general emitter machinery |
| 68 | `core/runtime/error_sentences.warp` | the human sentence of each runtime error: error reporting |
| 62 | `core/runtime/conversions.warp` | casts and checks between kinds (`as int`, bool casts) |
| 54 | `core/runtime/bridge.warp` | hooks from the core runtime to the extended runtime (decision runtime-split) |
| 52 | `core/runtime/maps.warp` | map words (remove, entries, copy of an instance): the cold half only maps of user programs use |
| 51 | `core/runtime/extended/wasi.warp` | WASI imports and file words (open, read, readdir) as declaration heads; the program-side file words |
| 46 | `core/emitter/host.warp` | host word wrappers: number and float fitting, kind tests of arguments |
| 46 | `core/emitter/operators.warp` | static type words of operands for list operations |
| 32 | `core/runtime/builders.warp` | built_* words: construct each kind of Node from code the host sends |
| 32 | `core/runtime/equality.warp` | equality of maps, captured values and identities |
| 31 | `core/emitter/closures.warp` | closure dispatcher code and captured slots |
| 29 | `core/runtime/lists.warp` | list pop/insert/sum and range sums |
| 29 | `core/runtime/text.warp` | print and C-text output words |
| 28 | `core/runtime/arithmetic.warp` | power, floor and sum edge words |
| 24 | `core/emitter/output.warp` | output file writer and spaced arithmetic display |
| 24 | `core/runtime/library_words.warp` | repeat and copy words of lib/ calls |
| 20 | `core/runtime/text_numbers.warp` | powers of ten and text-to-number |
| 19 | `core/emitter/functions.warp` | call refusals and rest calls |
| 16 | `core/runtime/ordering.warp` | float and sorted-node ordering |
| 14 | `core/runtime/text_words.warp` | slice and character offset words |
| 10 | `core/runtime/show.warp` | list and item show words |
| 8 | `core/analysis/lints.warp` | lint helpers for truthy literal and negatives |
| 8 | `core/runtime/characters.warp` | character stepping and code point |
| 7 | `core/emitter/control_flow.warp` | append-or-add statement form |
| 5 | `core/runtime/extended.warp` | joins the extended runtime (text and warnings) |
| 3 | `core/runtime/instructions.warp` | wasm instruction words (nearest, abs, shift) |
| 2 | `core/runtime/floats.warp` | rounded |

## Class C: features (candidates to move), ordered by lines saved per effort

Done on branch core-shake-small: number words (core/parsing/number_words.warp → extensions/number_words.warp, a pass
right after `definitions`) and the foreign declarations (core/parsing/foreign_forms.warp →
extensions/foreign_declarations.warp, one keyword reader per modifier word; Kotlin `enum class` joined sum_types' `enum`
reader, since two extensions may not claim one word). Both needed no new hook. The parser keeps the position helpers
other extensions use too (`starts_statement`, `name_after_blanks`, `class_keyword_after`, `block_after_name`).

Examined and dropped from C (they are core, not features):
- native hash, prime and compress words (107 lines): `sha256_hex` and `crc32_of` are used by core itself (the library
  linker's hashes, the png writer); there is no mechanism yet to answer library host words from a lib/ module.
- module dependency writing and extension commands (97 lines): core's build-time commands (`core_commands`, `trim`,
  `shell`) and `taken_before`, the machinery of source passes; the commands table of extensions already holds the rest.

Effort: small = 1, medium = 3, large = 8 (my estimate; a hook is only proposed, whether the hook table has it is for the
Interviewer). The entered/ran columns are in the git-ignored scratch data/core_shake/tables.md. All native rows need one prerequisite: library host words
answered from a lib/ module instead of driver.warp (decision libraries-not-core, roadmap_native_harness.md step 5), the
mechanism is not verified here.

| lines | group | effort | lines per effort | why and hook |
|---:|---|---|---|---|
| 285 | native: interactive console (REPL) | medium | 95 | line editor with history, arrow keys, alt-word moves for `warp` without arguments. Hook: a lib/ tool behind the commands table |
| 245 | warp test runner | medium | 82 | `warp test`: parallel slots, CPU limits, progress lines; no compile needs it. Hook: a lib/ tool run by `warp <tool>` (commands table) |
| 238 | native: png, pictures and gpu paint | medium | 79 | saving pictures, PNG and zlib writing, window paint: graphics library host words. Hook: library host words |
| 181 | native: timers and calendar | medium | 60 | every/frame timers, weekday and local time, sleep: event-loop host words. Hook: library host words |
| 174 | rule engine: glue, each and statement templates | medium | 58 | glued calls, each/indexed clauses, statement templates of sugar rules. Hook: rule registry options |
| 165 | packed node format (is! round trip) | medium | 55 | serialises a Node tree into a packed byte format for the test round trip and quoting; no compile needs it. Hook: lib/ module `pack` (words are called by name) |
| 157 | rule engine: where-conditions and holes | medium | 52 | `where` clauses, called holes and conjunct splitting of sugar rules: only extension rules that use them need it. Hook: rule registry options |
| 49 | spacing diagnostics | small | 49 | warns about `a -b` / `a - b` unevenness: a lint, no language meaning. Hook: source_checks list (a lint registry) |
| 147 | native: regex words | medium | 49 | regcomp/regexec bindings and match/replace/expand: a library's host words, not the compile loop. Hook: library host words (roadmap_native_harness.md) |
| 40 | vector swizzles and pixel rows | small | 40 | `v.xyz` swizzles and pixel chunk printing: graphics conveniences. Hook: lib/vector, lib/image |
| 36 | url and path literals | small | 36 | `https://…`, `./a/b` as text literals. Hook: lexer/parser literal hook |
| 104 | loop forms: c-style, ranged, comprehension, word-iterables | medium | 35 | the compiler source uses no c-style `for(;;)`, ranged, comprehension or word-iterable loop; only the plain for/while/each forms. Hook: parser statement hook (the extension takes the loop head, leaves for/while) |
| 101 | native: channels and signals | medium | 34 | inter-process channel files, listeners, signal listen: only programs that talk to others. Hook: library host words |
| 33 | plain-enum numbering and node-case spellings | small | 33 | `case_number`, numbered cases, spelled cases: enum conveniences. Hook: enum lowering extension |
| 88 | package roots and module search | medium | 29 | finds a module by walking folders upward and package roots; the compiler `use`s none. Hook: use-resolution hook (lib/ search path) |
| 75 | method and pipe phrases | medium | 25 | `x \|> f`, headed phrases, method phrases, with-trailing-item: reached only by library code. Hook: phrase hook after an operand |
| 189 | module signatures and library images | large | 24 | writes and reads a signature section so a compiled library can be linked later; the build of one program needs none. Hook: library writer extension |
| 56 | native: url fetch and system-value slots | medium | 19 | fetch a url, read and set window/system values: web and graphics library words. Hook: library host words (fetch is a library by decision libraries-not-core) |
| 50 | rest, optional and typed-default parameters | medium | 17 | `*rest`, `x?`, `x: int = 3`: the compiler declares plain parameters only. Hook: parameter-list hook in parser + signature pass |
| 49 | exact numbers: rationals, big literals, bit operations | medium | 16 | `as rational`, literals past 64 bits, decimal places past 18, `bitwise`: the compiler needs only int and float. Hook: extensions/big_numbers.warp (already holds the runtime side) |
| 99 | open words (slotted classes) | large | 12 | open class words with slots (`OPEN_PREFIX`): a library convention nobody in core uses. Hook: pass registry before lambdas |
| 37 | time values | medium | 12 | time kinds, quantity time specs and their tests in the emitter and runtime. Hook: extensions/time or lib/time |
| 92 | class instances: operations and inheritance | large | 12 | instance slot access, operators, inheritance, show: classes are a feature; the compiler is written with enums and defs. Hook: class lowering extension (struct + dispatch) |
| 19 | generic type application (list<int>, option) | medium | 6 | `list<int>`, `option<x>` type forms; the compiler writes plain type names. Hook: type-expression hook in parser |

### Class C: unreached helpers (not in a curated group)

Single definitions the self-compile never entered and static reach does reach. Each belongs to some feature or to an
error path; they move with their feature, or stay.

| lines | file |
|---:|---|
| 195 | `core/parsing/parser.warp` |
| 107 | `core/passes/rewrites.warp` |
| 72 | `core/compiler.warp` |
| 63 | `core/passes/prelude_words.warp` |
| 53 | `core/passes/lambdas.warp` |
| 49 | `core/passes/passes.warp` |
| 31 | `core/passes/arity_overloads.warp` |
| 29 | `core/analysis/analyzer.warp` |
| 26 | `core/passes/node_matches.warp` |
| 19 | `core/passes/sugar.warp` |
| 15 | `core/analysis/type_tests.warp` |
| 15 | `core/passes/scope.warp` |
| 9 | `core/analysis/kinds.warp` |
| 8 | `core/passes/definitions.warp` |
| 6 | `core/analysis/words.warp` |
| 6 | `core/passes/walk.warp` |
| 5 | `core/passes/data_blocks.warp` |
| 5 | `core/passes/prefix_calls.warp` |
| 3 | `core/analysis/builtin_signatures.warp` |
| 3 | `core/analysis/signatures.warp` |
| 3 | `core/passes/shapes.warp` |
| 2 | `core/passes/library.warp` |
| 1 | `core/parsing/lexer.warp` |
| 1 | `core/passes/data_items.warp` |

## Class D: dead (delete)

Each name occurs only at its definition in core, extensions, lib, test, samples, scripts and web (grepped).
The static analysis alone claimed 484 dead lines; the rest were declaration-only heads (`from runtime`), constants read by
extensions or by text, and functions called only by other cold functions. Done on branch core-shake-small (card of that name).

Correction while implementing: the first count of 32 was wrong. `std_os_input` is built by name in the emitter ("std_" +
module + "_" + member, a plain grep of the name misses it), and `word_index` / `word_index_file` are kept for the seed
compiler. Only the six lines below are dead.

| lines | file: names | why |
|---:|---|---|
| 4 | `core/passes/node_patterns.warp`: Assignment(4), Lambda(1), Call(1), Phrase(1) | four named patterns with no use (`Assignment`, `Lambda`, `Call`, `Phrase`) |
| 1 | `core/analysis/source_checks.warp`: fail_word(1) | a constant nobody reads |
| 1 | `core/passes/rewrites.warp`: use_line_start(1) | a constant nobody reads |

## Difficulties and ways forward

- Entered is not used. The number words, spacing diagnostics and foreign-forms checks run on every token and never
  fire; a dynamic measure cannot tell them from needed code. Their C class rests on reading, not on the counters.
- The native harness has no counters. Families (regex, png, hash, channels, timers, eval, C FFI) are sorted by what they do.
  Alternative: an `@primitive` coverage mode in the wasm2c build, or `warp test` runs with a sampling profiler.
- Samples and tests were not part of the hit set for class A by design: a program feature is class C even when a
  sample uses it (the loop forms). Moving it needs the extension hook to exist first; where the
  hook table lacks one, the group needs a core change (Interviewer).
- Next steps with the best yield: (1) the cheap D deletions (done), (2) the parser-side groups behind a statement/literal hook
  (foreign forms and number words done; spacing, url/paths, loop forms: about 125 lines left), (3) the native test runner and REPL as
  lib/ tools behind the commands table (about 530 lines), (4) the rule-engine groups (about 330 lines) as registry options of
  extensions/sugar, (5) a split of `intrinsic_of`/`key_expression`/`group_expression` by form.
