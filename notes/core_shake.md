# Tree shake of the core

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
(AGENTS.md "Principles": through the Interviewer).

## Class C: features (candidates to move), ordered by lines saved per effort

Examined and dropped from C (they are core, not features):
- native hash, prime and compress words (107 lines): `sha256_hex` and `crc32_of` are used by core itself (the library
  linker's hashes, the png writer); there is no mechanism yet to answer library host words from a lib/ module.
- module dependency writing and extension commands (97 lines): core's build-time commands (`core_commands`, `trim`,
  `shell`) and `taken_before`, the machinery of source passes; the commands table of extensions already holds the rest.

Effort: small = 1, medium = 3, large = 8 (my estimate; a hook is only proposed, whether the hook table has it is for the
Interviewer). The entered/ran columns are in the git-ignored scratch data/core_shake/tables.md. All native rows need one prerequisite: library host words
answered from a lib/ module instead of driver.warp (decision libraries-not-core), the
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
| 147 | native: regex words | medium | 49 | regcomp/regexec bindings and match/replace/expand: a library's host words, not the compile loop. Hook: library host words |
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

## Difficulties and ways forward

- Entered is not used. The number words, spacing diagnostics and foreign-forms checks run on every token and never
  fire; a dynamic measure cannot tell them from needed code. Their C class rests on reading, not on the counters.
- The native harness has no counters. Families (regex, png, hash, channels, timers, eval, C FFI) are sorted by what they do.
  Alternative: an `@primitive` coverage mode in the wasm2c build, or `warp test` runs with a sampling profiler.
- Samples and tests were not part of the hit set for class A by design: a program feature is class C even when a
  sample uses it (the loop forms). Moving it needs the extension hook to exist first; where the
  hook table lacks one, the group needs a core change (Interviewer).
- Next steps with the best yield: (1) the parser-side groups behind a statement/literal hook
  (spacing, url/paths, loop forms: about 125 lines left), (2) the native test runner and REPL as
  lib/ tools behind the commands table (about 530 lines), (3) the rule-engine groups (about 330 lines) as registry options of
  extensions/sugar, (4) a split of `intrinsic_of`/`key_expression`/`group_expression` by form.
