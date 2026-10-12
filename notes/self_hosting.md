# Self-hosting: warp's compiler written in warp
Roadmap card: self-hosted (board topic; its sub-cards are the steps below, notes/roadmaps.md).

Goal (card self-hosted, issue #33): "write the whole parser and as many components as possible in the Warp language
itself". The wiki ranks it a secondary goal that must not bias the language design (wiki/Features.md "self hosting",
wiki/goals.md).

Decided (user, 2026-10-12, notes/decisions.md "self-hosting"):
- Purpose: dogfooding (the compiler in warp shows what the language lacks), independence from Rust, and a showcase.
- Scope: the parser first (lexer, parser, serializer). The later parts get planned once that works (self-next-phase).
- Integration: twins, checked. Each warp part runs next to its Rust twin, and a differential test over every sample
  compares them. The Rust part goes only when its twin matches everywhere.
- Language gaps: fixed in the language as cards of this topic, never worked around in the warp code.

Size of the job (Rust lines, 2026-10-12): parser 7.4k (src/warp_parser), Node 2.1k (src/node), analyzer 6.6k, emitter
23k, lowering + top-level src ~59k. The warp written so far: lib/*.warp 1.5k, samples/json_parser.warp 95.

## What warp can do today (probes/self_hosting/, 2026-10-12)
Works:
- mutable parser state in an object a function changes (parser_state.warp)
- char-by-char scanning and classification (char_scan.warp, enum_match.warp)
- growing token lists (node_values.warp)
- computed keys `{(n): 3}`
- `data (x: 3)` with `.x`

Gaps (each a card below):
- `text[i]` costs O(n): lexing 8 KB char by char takes 0.69 s and 16 KB takes 2.6 s, and the parser's own 35 KB file
  runs out of fuel (index_cost.warp, scan_speed.warp). → self-text-index
- ~~No Node API on data values~~ fixed (self-node-api, src/wasm_emitter/node_api.rs): `n.key`, `n.value`, `n.op`,
  `n.name`, `n.bracket` (a field of that name wins), `symbol(text)`, `key(left, op, right)`, `bracketed(items, "(")`;
  the kind is `type(n)`, items `n#i`, `#n`, `for x in n` (node_access.warp, tests/node/test_node_api.rs). Still
  missing: a list's separator (`,` `;` newline) is not kept in the wasm Node at all → self-node-separator
- Bytes grow by copying (`bytes = bytes + [b]`, lib/memory.warp). → self-byte-buffer

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
| 9 | self-next-phase | Plan the next parts once the parser twin matches (lowering passes as plugins, emitter, bootstrap) | 6, 7 |

Rules
- Each step lands with its differential test (step 4): same Node, same text as the Rust twin.
- A new gap found on the way becomes its own card in this topic (todo add … --topic self-hosted).
