# Self-hosting: warp's compiler written in warp
Roadmap card: self-hosted (board topic; its sub-cards are the steps below, notes/roadmaps.md).

Goal (card self-hosted, issue #33): "write the whole parser and as many components as possible in the Warp language
itself". The wiki ranks it a secondary goal that must not bias the language design (wiki/Features.md "self hosting",
wiki/goals.md).

POSTPONED (user, 2026-10-13, notes/decisions.md "self-hosting-postponed"): no further steps until the language
features below are implemented cleanly (topic parser-features) and the Rust code is cleaned up (topic cleanup).
The twins on main stay and keep being tested.

Decided (user, 2026-10-12, notes/decisions.md "self-hosting"):
- Purpose: dogfooding (the compiler in warp shows what the language lacks), independence from Rust, and a showcase.
- Scope: the parser first (lexer, parser, serializer). The later parts: section Next phase (self-next-phase).
- Integration: twins, checked. Each warp part runs next to its Rust twin, and a differential test over every sample
  compares them. The Rust part goes only when its twin matches everywhere.
- Language gaps: fixed in the language as cards of this topic, never worked around in the warp code.

Size of the job (Rust lines, 2026-10-12): parser 7.4k (src/warp_parser), Node 2.1k (src/node), analyzer 6.6k, emitter
23k, lowering + top-level src ~59k. The warp written so far: lib/*.warp 1.5k, samples/objects/json_parser.warp 95.

## What warp can do today (probes/self_hosting/, 2026-10-12)
Works:
- mutable parser state in an object a function changes (parser_state.warp)
- char-by-char scanning and classification (char_scan.warp, enum_match.warp)
- growing token lists (node_values.warp)
- computed keys `{(n): 3}`
- `data (x: 3)` with `.x`

Gaps (each a card below):
- ~~`text[i]` costs O(n)~~ fixed (self-text-index): text_grapheme_offset keeps two text cursors and steps back by the
  ASCII bytes before one, so a char-by-char scan with look-backs is linear (scan_speed.warp: 35 KB in 0.14 s, was out
  of fuel; tests/text/test_scan_by_index.rs). It relies on runtime texts never changing in place. Each step still
  cost ~800 fuel (text-costs): grapheme_end now returns at once for an ASCII character before an ASCII one (but \r),
  ~300 a step, a far index ~41 a character (tests/text/test_ascii_index_costs.rs).
- ~~Gaps found by the self-parser~~ fixed: a class literal field named like an element is no markup (class-literal),
  an untyped `code`/`block` parameter is a block only when the body runs it (parameter-named). probes/self_gaps/.
  character-text (P126 follow-up): a character inside a list serializes as `"x"`, so `["x", b]` of a text b reads
  back `["x" " "]` like the list written out; values stay characters (one-letter "x" parses as one), only the printing
  changed (Node::item_text, lib/compiler/serializer.warp item_text). list-text-quotes: a `"` or `\` in a text is
  escaped by a `\` (a lone character escapes `'` and `\`), the same in the runtime print (text_quoted),
  Node::serialize and serializer.warp (`quoted`). Read a message's text with Display (`to_string()`), never
  `serialize().trim_matches('"')`, which keeps the escapes.
- ~~No Node API on data values~~ fixed (self-node-api, src/wasm_emitter/reflection/node_api.rs): `n.key`, `n.value`, `n.op`,
  `n.name`, `n.bracket` (a field of that name wins), `symbol(text)`, `key(left, op, right)`, `bracketed(items, "(")`;
  the kind is `type(n)`, items `n#i`, `#n`, `for x in n` (node_access.warp, tests/node/test_node_api.rs). Still
  missing: a list's separator (`,` `;` newline) is not kept in the wasm Node at all → self-node-separator
- ~~Bytes grow by copying~~ fixed (self-byte-buffer): lists already grew in amortised O(1) (int lists since
  3f7e212a1, `xs += [x]`, `xs.add(x)`), one text too (it grows in place at the heap top). Texts built in turn
  (`w += "x"; v += "y"`, a lexer's token and its output) were quadratic and passed the 2 GB memory cap at 100k appends.
  Now text_append, the concat of a variable's text, gives a copied text of 64+ bytes as much room again behind it and
  remembers 8 rooms (src/wasm_emitter/text/text_rooms.rs); text_concat of other texts stays small (a program without
  building pays nothing, quicksort's executable stays under its size limit). probes/self_hosting/text_builders.warp:
  400k appends to each of two texts in 70 ms; tests/text/test_text_builders.rs.

## Steps
| # | Card | Step | Builds on |
|---|------|------|-----------|
| 1 | self-text-index | O(1) `text[i]` and slicing while scanning (byte or char cursor), no quadratic lexing | none |
| 2 | self-node-api | Node API for data values: kind, key/value, head, children, bracket, build symbol/key/list | none |
| 3 | self-byte-buffer | Growable byte buffer and string builder (amortised O(1) append) | none |
| 4 | self-differential | Differential harness: warp twin vs Rust part over every sample and probe, first difference reported | none |
| 5 | self-lexer | Tokenizer in warp (lib/compiler/lexer.warp): positions, comments, numbers, texts, groups | 1, 4 |
| 6 | self-parser | Parser in warp producing the same Node as src/warp_parser | 2, 5 |
| 7 | self-serializer | Node::serialize (text emitter) in warp | 2, 4 |
| 8 | self-showcase | The warp-written parser as a sample (playground and guide): warp parses itself | 6 |
| 9 | self-next-phase | Plan the next parts: passes, analyzer, emitter (section Next phase, steps 10–18) | 6, 7 |

Rules
- Each step lands with its differential test (step 4): same Node, same text as the Rust twin.
  The harness (self-differential, tests/common/twins.rs): `differential(part, word, rust_twin)` runs
  `word(read(path))` of lib/compiler/<part>.warp for every sample and probe, 64 sources per program (~1300 sources, a
  few seconds, under 300 MB) and fails on the first source whose serialized value differs from `rust_twin(text)`, with both texts around
  the first difference. First twin: lib/compiler/positions.warp `line_widths` (columns per char, as
  warp_parser/scanning.rs counts them), tests/parser/test_self_differential.rs. A twin's list is built with
  `warp_list`: warp's empty list is ø.
- The lexer's twins (self-lexer, tests/parser/test_self_lexer.rs): the Rust parser is scannerless, it has no token
  stream to compare with. Its one real lexer pass is `comment_and_text_spans` (warp_parser/mod.rs, what `code_only`
  blanks before scanning declarations); lexer.warp `spans` matches it on every source. The tokens after it (numbers,
  names, groups) get their twin from the parse tree's positioned atoms.
- A new gap found on the way becomes its own card in this topic (todo add … --topic self-hosted).

## Next phase: passes, analyzer, emitter (card self-next-phase, 2026-10-13)
Order: the lowering passes first, then the analyzer, then the emitter. Each stage can start before the previous one
is complete; only the bootstrap waits for all three.

- Next item of this roadmap (user, 2026-10-14; card standalone-math-units): lib/math.warp and lib/units.warp become
  complete standalone warp files, and the parser fetches its lists of special functions and special operators from
  them instead of hard-coded Rust tables.

- Lowering passes first. src/pipeline.rs runs ~150 passes, each `fn(Node) -> Node`, in SOURCE_PHASES and
  MEANING_PASSES (~50k lines in src/lowering). A pass is the smallest unit with a twin: one function, one input tree,
  one output tree.
  - They can be ported one at a time, early and self-contained passes first.
  - Each twin can replace its Rust pass once it matches everywhere.
  - They need no host Node. They take node_view data, the form parser.warp's `tree` writes (warp-dispatch's advice).
    probes/self_hosting/next_phase/view_pass.warp rebuilds a view bottom-up.
  - warp-dispatch is moving semantic rewrites out of the Rust parser into passes (times phrases), which shrinks the
    parser twin and supplies easy passes.
- The analyzer second (6.6k lines). Its result is the kind of each definition and value, not a tree. It needs
  scopes: maps from names to kinds. text_key_map.warp works.
- The emitter last (23k lines). It needs byte output and float bits (gaps below), and its twin compares run results,
  not bytes: the Rust emitter's module layout is not a contract.

| # | Card | Step | Differential check | Builds on |
|---|------|------|--------------------|-----------|
| 10 | twin-harness | Pass twin harness | `pass_differential(k, part, word)`: the Rust passes 0..k-1, the view as JSON to the warp word, compared with node_view after Rust pass k, every sample and probe | 4 |
| 11 | times-phrases | First pass twins: number_words (80 lines), word_slices (210), times phrases (warp-dispatch's) | as 10 | 10 |
| 12 | carries-line | node_view keeps line:column where a pass reports, so hints and errors are compared too | passes that hint compare their diagnostics | 10 |
| 13 | parser-tree | Chained: parser.warp `tree`, then the warp passes in pipeline order | node_view after the same Rust passes, from the source text alone | 6, 11 |
| 14 | inferred-kinds | Analyzer twin: the kind of each definition and top-level value, as text | the same text from src/analyzer | 10 |
| 15 | emits-wasm | Emitter core in warp: ints, floats, functions, if, while | the warp-written module runs to the same value as Rust's, for every sample in that subset | 16, 17 |
| 16 | path-list | write_bytes / read_bytes for binary files | gap |  |
| 17 | f64-const | float_bits / bits_float: a float's IEEE bits as an int | gap |  |
| 18 | two-ints | `n & 127` on ints is logical, silently; hint toward bit_and | gap, footgun | fixed: bitwise (operator-shall) |

The bootstrap (warp compiling the warp-written compiler) is planned once 13–15 cover the compiler's own subset.

Probes (probes/self_hosting/next_phase/):
- Work: text-key maps (text_key_map), Node API rewrites (node_rewrite), a pass over the view (view_pass), fresh
  names with a `global` counter (fresh_names), LEB128 with `use math` bit_and/bit_or and `>>` (bit_ops), recursion
  10000 deep (deep_recursion).
- Gaps:
  - no binary files (byte_output);
  - no float bits (float_bits);
  - `&`/`|` on ints are logical with an `and` hint (logical_and_on_ints);
  - the call stack ran out between 10000 and 20000 frames (wasmtime's 512 KiB default). Fixed (card deep-recursion):
    the warp command on macOS links a 512 MB main stack (build.rs) and gives wasm all but 16 MB of it
    (warp_runtime::engine::give_main_stack_to_wasm), about 5 million frames of `down(n)`. Linux takes `ulimit -s`;
    test threads, tasks and standalone `warp build` binaries keep the default.
- Decided without the user (undoable): a pass twin takes node_view data, not a host Node. A `parse` word that gives
  warp code real Nodes would need a host word and a GC reader. It becomes a card once a pass needs Meta that the view
  cannot carry.

## Serializer twin (card self-serializer)
lib/compiler/serializer.warp `serialize(view)` writes a node as Node::serialize does (src/node/serialization.rs),
tests/parser/test_self_serializer.rs. warp has no Node of its own for the parse of a source (no `parse` word, and
`data` drops Meta), so both twins share a plain-data view (tests/common/twins.rs node_view, warp-dispatch's): a
number, yes, no or ø as itself, every other node a list tagged by its kind (["key", op, left, right],
["list", bracket, separator, items…], ["literal", source, node], ["attribute", name, value, node], …).
- `view_differential(part, word)`: the view of every parsed source goes to warp as JSON text read by `json(…)`. The
  same view as a warp literal is code to compile: a 100 KB chunk took 2.4 GB and 56 s in a debug build.
- Rust's Display shows up where serialize writes a key's left side (`name: 1`, texts bare); everything else Display
  prints is serialize again (Debug is serialize), so the twin needs no Rust-only text.
- Found on the way, fixed in the language: an escaped quote in a text ended it for the `//` comment check
  (escaped-quote-slashes); text literals over 64 KiB trapped, the module's memory was one page whatever its texts
  took (large-text-literals). `type(x) == "int"` is no while `type(x)` prints int (type-text-compare: by decision type-equal, now with a hint toward `type(x) == int`): the
  twin uses `x is list`.
- The entity pass (uniscript_entities::expand_entities) also ended a text at an escaped quote and turned `"\"\\:infinity"`
  into `"\∞`: text_or_comment_end now skips escapes (same card). A Data node without a unit description is written
  `Data(…)`, so its view carries that text. Small floats keep their exponent on both sides (`2.5e-6`, decision
  float-text): Node's Display writes numbers as Number does (writes-node).

## Runtime calls into prelude words (card hex-binary, late calls)
Runtime wasm functions (list_join …) are compiled before the user functions, so they cannot `call` a warp-written
word by index. `late_call(callee, params, results)` (src/wasm_emitter/functions/late_calls.rs) gives the callee a
funcref global `late·<callee>` that main fills with ref.func before anything runs; the runtime calls it with call_ref
(as witness.rs does for Comparable). The loader must load the word although the program never calls it: an
`implied_words` hook in modules::called_names (radix.rs: a named `hex`, `binary`, `byte` … implies hex_text /
binary_text). First user: lib/prelude.warp hex_text, binary_text over to_base.
- Size: a warp word with `%` or `//` on int pulls the exact-rational runtime (~5 KB): card int-i64.
- A host calling an exported runtime function before main would find the global null (trap): only main-run code uses it.

## Text search words in warp: measured, not moved (card card-3, 2026-10-16)
probes/text_search/words.warp writes trim, starts_with, ends_with and find over byte_at / byte_slice; check.warp
gives the builtins' answers. Benchmark: bench_find_builtin.warp vs bench_find_warp.warp (20 000 finds in a 2 KB
text, 40 M byte comparisons), `warp compile --wasm` each, then `hyperfine -N -i 'warp bench_find_builtin.wasm' 'warp
bench_find_warp.wasm'` (-i: the exit code is the result mod 256).
- Speed: warp find 2.0 s vs 0.30 s builtin, 6.2x slower; `t: text, offset: int` types change nothing, `fast` on the
  words nothing either. trim / starts_with / ends_with on short edges: ~2x on the time after the ~12 ms startup.
- Size: the same, ~17 KB modules either way (±1.5%); the warp words are small, the hand loops too.
- Why (WAT of matches_at): every `+` and `<` still tests the 2^62 range and branches to exact_add / exact_cmp, also
  under fast (card fast-int), and each byte_at is a call that re-resolves the text (text_argument) instead of one
  i32.load8_u on an address held outside the loop (card loop-calls). A prelude call is also never inlined (sum-list).
- fast-int landed (whole fast expressions on machine i64s): find unchanged at 6.4x. What is left is ~9 calls per
  offset at ~5 ns each, and those 5 ns are mostly fuel (card fuel-metering, notes/performance.md "Still slow").
- loop-calls: `byte_at(t, i)` and `t.bytes` of a text are inline (a kind test; another value still goes through
  text_argument / node_bytes): find 6.4x → 1.7x (0.50 s against 0.29 s). Left: the matches_at call per offset and
  fuel (card fuel-metering, decided: epochs by default). Rerun the two bench files once that lands, then decide
  whether find moves to warp. TEXT_FIND has a second caller, `in` /
  contains (fields_and_keys.rs), which needs a late_call (above) once find moves.
