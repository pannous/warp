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
- Success-driven: the measure is the existing test suite and samples passing through the warp-written compiler
  (program results, errors, output), not agreement with Rust's intermediate trees. Card warp-pipeline-suite runs the
  is!/samples suite through the warp pipeline and reports the pass count; that count is the progress number. A warp
  stage becomes the default when the suite passes with it. 80–90% passing is fine at the start: breadth over the
  long tail, the failing rest is a ranked list, not a blocker.
- The Rust compiler stays as fallback and reference implementation, and gets an intermediate cleanup phase now
  (notes/rust_cleanup.md): every worker finishes its current card, applies the rewrite's structural decisions to the
  Rust side of its stage, then self-hosting resumes and ports the cleaned design.
- Twins (a warp part next to its Rust original, compared over every source) are scaffolding only: kept where a warp
  stage hands its result to a Rust stage, because the Rust side needs a compatible shape, and dropped as soon as the
  stages on both sides are warp. No new intermediate twin is written where the suite can already judge.
- Language gaps are fixed in the language as cards of this topic, never worked around in the warp code.
- No hand-lowering (notes/agents/common.md "Never degrade the input"): the compiler is written with warp's full
  features, aiming to be more elegant than the Rust one. Runtime words are ordinary lib/*.warp code through the
  normal pipeline, so a warp stage only ever sees lowered forms; a form it can't handle yet goes to its Rust twin.
- Matching Rust is a check, not the goal: the twins must not copy Rust's imperfections. Where the warp part's clean
  result differs because the Rust result is a quirk or bug (an odd tree shape, a wrong kind, a lost position), the
  warp part keeps the clean result, the difference goes into the expected-deviations list the differential reads
  (notes/self_hosting_dropped.md, the component's "deviations" section, with an example each), and the Rust side gets a bug card or,
  when it is a design question, a warp-interviewer question. A twin that matches 100% gets a quick review for copied
  quirks before its Rust part is removed.
- The same holds for structurally unsound decisions, not only bugs: special cases for single words, passes that undo
  another pass's work, two mechanisms for one concept, tables duplicated across stages, pass order that exists only to
  work around an earlier pass. The warp compiler may restructure (merge or split passes, change an internal tree
  shape, replace special cases by one general rule). Where it does, the twin compares at the next level where both
  agree (the tree after a later pass, or the program's result) instead of forcing the Rust structure. Each
  restructuring is a short entry in Deviations; ones that change the language go to warp-interviewer.

Size of the job (Rust lines, 2026-10-12): parser 7.4k, Node 2.1k, analyzer 6.6k, emitter 23k, lowering + top-level
src ~59k.

## Parser restructuring (review 2026-10-17, topic self-hosted)
parser.warp leaves the Rust parser's structure where it is unsound. Order: parser-tokens and parser-desugar-pass first,
then parser-scope-pass and parser-application with warp-self-passes, then parser-typed-fields, parser-context,
parser-keyword-table. Target shapes:
- parser-tokens: lexer.warp yields tokens, each with "blank before" and "newline before" bits; the parser reads tokens
  and asks no character lookahead (operand_after, operand_follows, expression_ends, ends_line,
  only_blanks_before_newline, line_blanks go).
- parser-desugar-pass: the tree keeps what is written: `xs[i]` an index, `xs[a:b]` a slice, `loop {…}`, `/!`, `/!=`,
  `twice {…}`, `is in`, `1e3`, `2x`, `x⁻¹`, `global int n`. One sugar pass in passes.warp rewrites them, one table row
  each; the twin compares after that pass.
  Done (lib/compiler/sugar.warp, `tree` = `sugared(written_tree(…))`): `xs[i]`, slices, `/!`, `div`, `/!=`, `is in`,
  `loop {…}`, count words (`twice {…}`, with the scope: `counted`); `1e3`, `2x`, `x⁻¹`, `global int n`
  already leave the parser as one number or one key, with no written form worth keeping.
- parser-scope-pass + parser-application: the parser writes word phrases `[f x]` without knowing names; a pass that
  knows the scope makes calls of them. applies/word_begins_item and trailing_block go.
  Done (parser-application, part 1): the head of `for`, `if` and `while` is one context in both parsers (Rust
  Context::in_header = in_for_header or equals_compares; parser.warp p.header): a `{` there is the body, never an
  argument, so the repair that split the body off a condition's right spine (Rust split_trailing_block, warp
  trailing_block) is gone. Left: applies' power windows (OPERAND_POWERS minus ARGUMENT_POWER, ≤ 60, the branch
  power) and word_begins_item still decide application in both twins alike.
  Done (parser-scope-pass): parser.warp keeps no names (program_functions, its line scan and p.bound are gone); it
  writes `twice {…}` as written and sugar.warp `counted(tree, scope_of_view(written))` makes it `2 times {…}` unless
  the program binds the word (scope.warp, one scope for the whole program: a parameter `once` keeps the word anywhere).
- parser-typed-fields: `x: int` in a class body stays a key; the type pass reads it as a field type.
  Done: parser.warp keeps the class body as written, the sugar pass (lib/compiler/sugar.warp typed_fields) types it.
- parser-context: one context value (header, iterable, branch, spaces, indent, grouped) passed down, no saving and
  restoring by hand.
- parser-keyword-table: one table, word → shape, for the word forms (print, times, count words, function references,
  div as tag, unit words, membership, global/export, type words), in the format of library-word-tables.

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

## Target pipeline shape (approved 2026-10-17, proposal in notes/self_hosting_dropped.md "Simplified shape")
parser (clean tree, parser restructuring above) → desugar table pass → scope pass (calls of word phrases, bindings
per scope) → one word-phrase pass (`a of b`, `<selector> in|from b`, member_phrases, times_phrases, number_words; a
table word → rewrite in the library-word-tables format, shared with parser-keyword-table) → structural passes
(test_blocks, enums, comprehensions, go_blocks, references, lambdas, pipes, quasi_quote, tree_walks) → analyzer
checks (match_exhaustive, switch unit names, index_positions, check_unit_names: analysis, warp-self-analyzer's side).
- Passes the target eliminates are not ported: repair passes such as grouped_receivers (the parser's `f()` = `(f)`)
  and of_lookups' repairs (`type of x` as a declaration head, `sum (of xs)` grouping). The twins already written stay
  until the target passes replace them.
- New ports go toward the target: the word-phrase pass and the scope pass (with warp-parsing: parser-scope-pass,
  parser-keyword-table); broadcasting minimal; the structural keepers of the SUGAR_PASSES triage.

## Back-end focus (user 2026-10-18 night, decisions self-hosting-back-end, compiler-typed, compiler-high-level)
Goal: WARP_SELF_HOSTED runs parser → passes → analyzer → emitter → runtime all in warp; the suite count per stage
shows the progress. Every compiler function typed (generic ones generically), written with classes, enums and match.
| owner | part |
|---|---|
| warp-self-analyzer | analyzer.warp `kinds(program)` complete; the switch hands the analysis to it instead of src/analyzer |
| warp-list-result-type | the builtins' typed lines as the analyzer's signature table (analyzer.warp reads them); types on every lib/compiler function |
| warp-self-emitter | emitter.warp beyond the core (int `/`, exact rationals, big ints, `#`, `==`, slices, errors, memo calls); the switch hands emission to it |
| warp-self-passes | runtime.warp: the text, list and number runtime functions of src/wasm_emitter in warp |
| warp-clicking-guide (markup) | maps, classes/objects and equality: their layout in emitter.warp and their runtime words (coordinate emitter.warp hunks with warp-self-emitter) |
| warp-parsing | twins for the source and meaning passes still without one, in target-shape order |
| warp-cache | the end-to-end switch (each stage compiled once, speed), then caching emitted specialisations |
| warp-steward | suite count per stage: front end only, + analyzer, + emitter |

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
function and local names.
- Values: see "Value layout" below.
- Kinds: int = i64, float = f64, a condition = i32. A parameter takes its annotation, else the kind of its first
  call's argument; a function's result is read twice, so a recursive call knows it (the pending kind takes the kind
  it meets). `%` is Euclidean as in Rust's emitter (r + |b| for a negative truncated remainder).
- Twin (tests/parser/test_self_emitter.rs, tests/common/twins.rs emitter_differential): the module runs
  (wasm_reader::read_bytes) and its value is compared with eval(source); a float equals another number of exactly
  its value (3.5 * 2 is 7.0 here, 7 in Rust). probes/self_hosting/emitter/core/ holds one program per form, all inside
  the core; the checked share of every sample and probe must have at least a floor of programs inside.
- Dev loop: probes/self_hosting/emitter/emit_lowered.sh <file.warp> (lowered text → wasm_module → validate → run).
- Int `/` gives an exact rational (runtime word exact_ratio): an Int `$Node` holding a `$Ratio` of two `$i64box`, as
  the Rust reader reads it; exact again with ints and ratios (exact_add/sub/mul/div/order), shown as Rust shows it
  (exact_text: 3.5, 1/3); `%` on one and a ratio with a float go through floats. `#`, `x#i` and `x#i = v` on a value
  known only at run time call node_length / node_at / node_set_at.
- Not in the core yet (outside, not dropped): decimals as exact rationals (the core reads a decimal as f64, so
  `1/3 + 0.5` is a float where Rust gives 5/6), ints beyond 64 bits (Rust promotes; a ratio's parts overflow
  silently), a yes/no variable, a loop's value, slices, error values (a run-time error traps, without
  Rust's message: the twin counts a failure on both sides as the same outcome), every other runtime call, among them the memo_* calls the memoization pass gives a function calling itself twice (fib), and
  library words a user function's name meets (`add` becomes add·any).
- Kinds come from emitter.warp's own small inference until lib/compiler/analyzer.warp `kinds(program)`
  (warp-self-analyzer, card inferred-kinds) lands; then the emitter reads its kinds from there.

## Value layout (card emitter-gc; the Rust emitter follows: card rust-emitter)
Typed values inside, a boxed $Node only at the edges. Where emitter.warp knows a value's kind it stays plain:

| kind | wasm value | |
|---|---|---|
| int | i64 | |
| float | f64 | |
| yes/no | i32 | |
| character | i32 code point | a one-character literal, as warp holds `"a"` |
| text | `(ref null $text)`, `$text = (array (mut i8))` | UTF-8; literals from one passive data segment (array.new_data) |
| list | `(ref null $list)`, `$list = (struct (mut i32 length) (mut (ref $items)))`, `$items = (array (mut anyref))` | a vector: `xs#i` O(1); items boxed |
| any kind (a value of mixed branches, a list item, ø) | anyref holding a `$Node` | |

Boxing (`boxed` in emitter.warp) makes a `$Node(kind i64, data anyref, value ref null $Node)` (CLAUDE.md
"Serialization"): an int or yes/no in an `$i64box`, a float in an `$f64box`, a character as an i31, a text with data =
its `$text`, a list with data = its `$list`. A value is boxed only where its kind is not known statically: a list item,
a value of branches of different kinds, an `any` parameter, and main's result unless it is a number. No linear memory
but WASI's: texts are GC arrays, so no allocator and no (ptr, len) `$String`.
Readers of the layout (each place a module's result or a value crossing the boundary is read):
- src/run/wasm_reader.rs (native runs, the twins): reads a Text whose data is an `(array i8)`, a List whose data is a
  `$list` (told from a cons cell by its first field: the i32 length, where a cons cell's first item is a $Node), and
  needs no memory export. Done.
- src/gc_traits (GcString::from_val): already reads an `(array i8)` text.
- web/playground/reader.js reads through the reflection exports each Rust-emitted module carries
  (src/wasm_emitter/reflection/exports.rs). emitter.warp modules do not run in the browser yet; when they do, they
  carry the same exports, with tags for a byte-array text and a vector list.
- Host imports, FFI and WIT (src/host, src/ffi): take $String (ptr, len) texts in linear memory. emitter.warp imports
  only WASI's fd_write, and only for a program that says `print` (imports shift function indices, so it is decided
  before compiling): one page of memory, exported as "memory", where the runtime word print_line copies a text's bytes
  a chunk at a time (iovec at 0, written count at 8, bytes from 16). That is the one edge helper; nothing else
  touches memory. Done (card emitter-prints); the emitter twin compares stdout too (host::with_captured_stdout).
Runtime words are warp: lib/compiler/runtime.warp, written in idiomatic warp (for loops and all sugar) and lowered by
the same pipeline as any program before emitter.warp sees them (`wasm_module(program, runtime)`; the twin harness
lowers it with pipeline::lower). emitter.warp handles only lowered forms, never hand-lowered source. The words use a
handful of intrinsics, one or two instructions each and callable only from runtime words: texts (new_text, copy_text,
byte_count, byte_at, set_byte), lists (new_list, copy_items, adopt_items), numbers (int_quotient, int_remainder,
truncating; code_of, char_of), boxed values (kind_code, marked_kind, unboxed_int/float/text/char, is_null), output
(output_byte, write_output) and fail (a trap). Each word is compiled on its first call like the program's own
functions: text_join and list_join (`+`), list_append (`+=` on a list, in place, so aliases see it), text_count (`#`
on a text: code points), text_char (`t#i`), text_order (text comparisons), print_line, int_text, char_text, node_text
(print and `+` of a value known only at run time; a float is not printed yet).

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
Load order (lib-from-runtime, landed with zero-fill-lib): code generation that calls a prelude word the program never
named, through `compiler_word(word)` (a call it writes, `n times [x]` → filled_list) or a late_call whose callee is
missing, notes the word; pipeline::loading_compiler_words then compiles once more with resolve loading it (hints muted).
The second compile is a safety net, not the plan: with it alone 13 of 216 samples compiled twice (~7% of compile time),
so each move also adds an implied_words scan of the forms that need its word (filled_lists.rs: none twice).
Measured cost (2026-10-17, quick-release, thread CPU of the lowering; cards cache-lowered, parse-once): a small program
4 ms, with one function of its own 11 ms, with one prelude word 22 ms (the prelude parse 8 ms, the reach look-ahead
4 ms, the word itself as an own function). So a scan must not load words a program cannot need, and a runtime function
that nearly every program carries stays Rust until lowered prelude words are cached across runs: exact_text (any int
arithmetic links ratio_new, so its printer) stays, `as decimal`'s decimal_text moved (card exact-text-warp).
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
| Lists are cons cells | `n#i` walks cells (cursors help in order) | wasm GC arrays: emitter.warp's vectors (Value layout) |
| `text[i]` walks graphemes | ~300 fuel per ASCII step | byte indexing (`byte_at`) |
| Unbounded ints | overflow checks/masks per operation | i64 path when the type is known (card int-i64) |
| Fuel metering on | a counter per loop and call | off for trusted code (card fuel-metering) |
| Little inlining | small functions stay calls | emitter inlining or wasm-opt |

Host after self-hosting: the native `warp` stays a Rust shell (wasmtime, host words, FFI, graphics, sound) embedding
`compiler.wasm` precompiled to .cwasm; the playground loads the same file. New interface: compiler imports for
reading `use`d files, FFI signatures, the host-word list.

Measured (2026-10-17, probes/self_hosting/emitter/bench/list_reads.warp: 20 million `xs#i` at scattered places of a
1000-item mixed list; `warp compile --wasm` for Rust's module, emit_lowered.sh for emitter.warp's, hyperfine): Rust's
cons cells 151 ms, emitter.warp's vector 80 ms (1.9x). Reads in order: 141 ms vs 66 ms (2.2x). Rust's list cursors
keep its reads far from O(i) here; the gain is the plain array.get.

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

## The suite through the warp pipeline (card warp-pipeline-suite)
`scripts/self-hosted-suite.sh [<pushed tip>]` runs the whole suite on the cloud lane with WARP_SELF_HOSTED=1 and
prints "self-hosted: N passed / M" plus the failures grouped by cause (an assertion's left side, digits as #), the most
frequent first; `--local <filters>` runs it here through tests/queue.sh, `--rank <log>` ranks any test log,
`--tests <cause regex> <log…>` lists the failed tests of a cause. `--stage <name>=<VAR>` turns a later stage's switch on too (run `<tip>+<name>`, compared with the
last run of the same stages): the per-stage counts of "Back-end focus". A cloud run appends its count to
data/self_hosting/suite_counts.txt (warp-steward reruns after each batch touching lib/compiler or the pipeline, and
nightly). The failure groups are cards suite-* in topic self-hosted-2, each naming its stage owner.
- src/compiler/self_hosted.rs: with the switch on, `lawful_program` parses with parser.warp (passes.warp `stage`
  [0, source]) and `run_program_passes` hands every source pass with a warp twin (TWINNED_PASSES, the order of
  passes.warp `passes`) to it ([i, view]); the passes without a twin, the analyzer and the emitter stay Rust.
- Each stage call crosses src/compiler/node_views.rs: the plain view as JSON in, the view back to a node
  (node_of_handled_view). Data nodes (times, quantities) travel as handles ["data", "#i"]. Lost on the way: source
  positions and literal layers (a plain view has none), so position-checking tests fail by design.
- The stage program compiles once per worker thread (its compile has thread-local state the outer compile keeps);
  the twin tests parser::test_self_* are skipped (they compare warp with Rust themselves).
- Since card End-end (branch self-switch): three stage programs, each its own warp program (a `Stage` static with
  its worker pool): passes.warp `stage`, analyzer.warp `kinds` (self_hosted::analysis, JSON; separate because
  analyzer.warp's names clash with parser.warp's, card local-variable-named) and emitter.warp `wasm_module`
  (wasm_emitter::emit_module asks it first; "outside the core: …" falls back to the Rust emitter, other failures are
  errors). Stage modules and the emitter's lowered runtime.warp view are kept in the module cache, keyed by the
  binary, lib/ and the WARP_ settings: a warm self-hosted run takes ~0.3 s. A cold cache compiles the stage programs
  with the Rust pipeline (that compile shows a Rust `emit` phase).
- Emit refusals ranked 2026-10-18 (probes/self_hosting/refusals.sh, every sample and probe with the switch on, each
  refusal once per program; raw list data/self_hosting/refusals.txt): 1471 programs, 585 inside the core (or no emit).
  | programs | family (refusal texts) | owner |
  |---|---|---|
  | 57 | `view·opaque`: a class/type or error node kept whole | warp-clicking-guide (classes), warp-self-emitter (errors) |
  | 47+4 | `#` on int, `^` on int | warp-self-emitter |
  | 41+10+9+5 | operator `:` on int / char and text / char and int / char and list (ranges, slices, pairs) | warp-self-emitter (slices) |
  | 33+8 | int division, an exact rational; floor_quotient | warp-self-emitter |
  | 27+27+18+4 | untyped values: the length of a node, a node where an int/list is needed, an assignment to an element of a node | warp-self-emitter with warp-self-analyzer's kinds |
  | 52 | `use` / `import` statements still in the lowered program (Rust skips declarations) | warp-cache |
  | 20 | closure_new (lambdas, closures) | warp-cache |
  | 16+14+14+13 | host calls: system_value, clock, std_io, foreign_call | warp-cache |
  | 14 | `break` | warp-cache |
  | 12+8 | a yes/no variable; `global` | warp-cache |
  | 12+6+5+4+8 | maps: map_entries, map_of_entry, field_with, key, collection_contains | warp-clicking-guide |
  | 26+12+6+6+5+5+4+8 | runtime words not yet in runtime.warp: join, zero_fill, times·text, text_form, len, round (in: emitter.warp library_words, card program-join); left: random_below (host), as_code (code values) | warp-self-passes |
  | 10 | `try` | warp-self-emitter (errors) |
  | 11+14+6+5 | sound/signals/tasks: midi_hertz, signal_new/listen, task_spawn_values, linear_new | later |
- The cloud run is the pushed tip plus a throwaway commit turning the switch on in .cargo/config.toml [env] (with
  warp-lang at opt-level 2, as test.sh), in three jobs at once (TEST_GROUPS: self-hosted, the whole suite outlasts the
  lane's 2400 s limit), branches self-hosted/<tip>-<n>, deleted after the run.
- First count (2026-10-17, aa7a536a, data/self-hosted/aa7a536a.txt): 4529 passed / 6421 (70.5%). Top families:
  parser.warp's `Unexpected character` 344 (π @ > . $ & = * ⊕ : σ, a space before a trailing block), `undefined
  variable` 184 (try 52, T# 22: forms parser.warp reads otherwise), a bare `#` left side 76 (a wrong number), `cannot
  extract a numeric value from ø` 46, database `may be ø` hints 50, `not a number` 33, hints pointing into
  lib/compiler/parser.warp (its own warnings leak into the program's output) 41.
