# Performance: what the benchmarks found (2026-10-05)

Benchmarks: `probes/night/bench_lists.sh [n]` (lists, closures, maps, text building), run with the release CLI
(`WARP=…/release/warp`). Keep n unknown at compile time when timing one idiom by hand (`n=2000000+random_below(1)`),
otherwise constant evaluation may hide the loop. Profile the generated module apart from warp:
`warp compile x.wasp`, then `wasmtime run -W gc=y,function-references=y,exceptions=y,tail-call=y --profile=guest,out.json
--invoke main x.wasm` (Firefox profiler); `/usr/bin/sample <pid>` on the debug CLI shows the host's side.

## Fixed
| idiom | before | after | cause |
|---|---|---|---|
| map of 4000 text keys, write loop | 11.7 s | 0.49 s | DRC collector + empty GC heap + key comparison |
| same, write then read | 16.1 s | 0.59 s | map_entry_has_key made two symbols per entry |
| `t += "x"` 200000 times | out of memory | 0.09 s | every append copied the text into fresh linear memory |
| `"\(i)"` 200000 times | 0.40 s | 0.10 s | GC heap growth |
| `count(xs.filter(…))`, 2 million | 0.83 s | 0.10 s | the typed list became 2 million nodes just to count |
| map of 2 million keys, write and read | call stack exhausted | 2.7 s | cons-list map: quadratic, recursive copy |

- **GC collector**: wasmtime's DRC collector (warp's only enabled one) ran this program about 3× slower than the
  copying collector, which also collects cycles. Cargo.toml enables `gc-copying` and `Collector::Auto` picks it.
- **Initial GC heap** (util.rs GC_HEAP_INITIAL_BYTES = 1 GB, reserved and committed lazily): the copying collector
  grows its heap too little, so a growing live set is copied again at every collection (400000 map entries: 6 s with
  64 MB, 0.3 s with 1 GB; the wasmtime CLI behaves the same with `-O gc-heap-initial-size`).
- **wasmtime's own build** (Cargo.toml `[profile.*.package.wasmtime] opt-level = 3`): release is size-optimized
  (`opt-level = "z"`, for the web build) and dev unoptimized; both ran the host side of GC-heavy programs slowly
  (2.5× and about 10× the CLI). The web build has no wasmtime, its size is unchanged.
- **Map variables are hash tables** (wasm_emitter/map_backend.rs): `m = {}` followed by `m[k] = v` with name keys is
  a `$NodeMap` (insertion-ordered keys and values, open-addressing slots, FNV-1a of the key's letters). It becomes
  the cons-list Node wherever the map is a value (a fresh copy, so value semantics hold); a lookup it cannot answer
  takes the generic way on that copy, with the same errors.
- **Texts grow in place** (text_concat): a left text ending where the heap starts gets right's bytes after it; a
  right text made just now (a character's bytes) already follows left, so nothing is copied. The left text's own
  bytes never change, so other values sharing them stay right.

## Measured as fine
Closure calls cost what definition calls cost: 50 million calls of `add=(a,b)=>a+b`, of a capturing `x=>x*k` and of a
`def` take 0.3 s each (6 ns per call), through `apply(f, x)` 0.42 s. No boxing showed on these paths, so the per-site
closure typing has nothing to win here yet. Loops over typed lists, map, append: 2 million items in under 0.1 s.

## Still slow
A map that is not a hash table variable (a literal start `{a:1}`, a parameter, a global, one updated by `+=` or with
dynamic keys) is still the immutable cons list: n keys cost n². Each use of a hash-table map as a whole value
converts it (O(n)); a loop that prints or passes the map each round pays that every round.
