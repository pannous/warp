# warp compiles parser.warp: refusal roots and owners

Goal (user 2026-10-19): the self-hosted pipeline compiles lib/compiler/parser.warp. One owner per root cause:
claim a line here (commit straight to main) BEFORE you start, mark it done with the commit, add new roots you find.
Survey: WARP_EMIT_SURVEY over parser.warp with its use'd modules (warp-emitter-structures, 2026-10-19 evening:
28 functions refused, down from 124).

## Done
- field write on a parameter (`p.at = p.at + 1`): emit-enum-match d717c74deb, also pure-lane 4696852d1
- `def f(x): T := e` registers as a definition: emit-enum-match d717c74deb, runtime-declared ae7663fc80
- `def f(x): list|map|any := e` result types in the Rust parser: emit-enum-match d717c74deb
- bracket_of, separator_of, bracketed, separated as runtime words, n.op / n.bracket fields: emit-enum-match d717c74deb
- node into a variable declared list/int: runtime-declared 3a700ae
- match with node patterns: pure-lane 54dabdd10 (node_matches.warp)
- bare `return` (`{ return ø }`) and `continue` in while loops: pure-lane ec8cd7ade (a for whose body continues stays
  with Rust: its step must not be skipped)

## Open
| root | owner |
|---|---|
| lower/upper (case tables), ran_without_error, contains alias | warp-self-emitter (pure-builtins) |
| `x is symbol/list/any` type tests, prelude defs on demand (is_digit) | warp-self-emitter |
| match literal and enum-symbol arms | warp-parsing |
| enum lowering (`enum Op {…}`; Op.Colon wrong result) | warp-cache |
| class constructor `Point(2, 3)`, the file's own globals, try, `===` | warp-self-analyzer |
| int(x), positioned, `op as Op` (keyed), an instance inside a map, a node into a typed Parser field, a write by a run-time key, pending kinds in recursive functions, `branch` called with a missing argument | warp-emitter-structures |
| `!` words in parser.warp (parser-reads-2), then the pure-lane pass count | warp-is-tests |
