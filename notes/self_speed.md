# Speed of the self-compiled compiler (card self-speed, 2026-10-22)

`warp self` (warp-self.wasm, emitted by emitter.warp) against `warp pure` (warp.wasm, emitted by Rust),
compiling samples/functions/fibonacci.warp. Measured with `WARP_PROFILE=<file>` (guest sampler, 10 ms epochs), 8 runs
each, summed with `probes/parser_speed/profile_top.py <files…> --top N [--callers NAME]`. Wall times were useless
(load 70–280), the profiled compiler run is the number.

| step | warp self compiler run |
|---|---|
| before | 490 ms (pure: 75 ms) |
| 1. `x is T` of a node: held kind bit against a compile-time mask | 146 ms |
| 2. case tables read once into runtime list globals | 129 ms |
| 3. held kind bit computed inline (selects), no call | 109 ms |
| 4. `==` of texts/characters without node_text | 97 ms |

## What each step was
1. ~70 % of the run: every `x is T` on a value known only at run time, and every variant match (`match n { Number(_)
   … }`, node_matches.warp lowers it to `is_type`), called runtime node_is_type(n, "<type word>"): the value's kind as
   a type-word text, compared byte by byte (types_passed_by, names_one_of). It sits under node_text, which node_equal
   and map_index call constantly. Now emitter.warp node_type_test tests `held kind bit & mask` (HeldKind places in
   kinds.warp, passing_kinds in emitter.warp); only type values (`int is number`) and `is code` still ask node_is_type.
2. ~12 %: text_upper/text_lower beyond ASCII parsed the whole case-table text into ints at every call (the lexer's
   is_letter_beyond_ascii calls upper(c) != lower(c) per non-ASCII character). Runtime globals may now hold a list
   (`global x = []`, emitter.warp runtime_global; a list global's zero is an empty list).
3. ~15 %: the held-kind bit was a runtime word called per arm of every node match; emitter.warp held_kind_bit emits it
   inline.
4. ~10 %: node_equal of two texts/characters went through node_text's match and made a text of each character
   (span_end compares each source character with `'/'`, `'#'` …); runtime equality.warp textual_equal compares them
   directly.

## What remains (not worth it now)
- The lexer walks texts by code point (code_point_width/at/text, character_end, ~4–8 %): card lexer-walks (Later).
- Statically warp-self.wasm still re-allocates text literals per use (array.new_data), boxes many ints and casts
  more (ref.cast) than Rust's output, but none of these showed as a hot spot of 10 % or more.

No compile keeps or rebuilds the runtime words any more: they come from runtime.wasm (card runtime-lowering-scope).

## Seed regression 2026-10-27 (card rebuild-pure-went)
`warp rebuild pure` is the seed compiling compiler.warp, so its time is the seed's speed. Per-hour minima in
/Volumes/Build/compilers/rebuilds.log (start/end of every build) went 105 s → 148 s → 280 s, each step at a seed swap.
The same noon tree (c4c601db90) built under load ~5, each seed's .cwasm warm (WARP_SEED=<file>, `--force`):

| seed (compiler.warp of) | rebuild |
|---|---|
| 5b3db547 (46581bc7d6) | 74 s |
| 1ec0b599 (8b4d2a2521) | 115 s |
| 18e7a6e7 (041a4b87a1) | 237 s |
| 28abccf4 (0d6920297d) | main: 87 s (warp-fixer, 265 s with 18e7a6e7 back to back) |

Cause: compiler code moved to classes (TypedCode `value.kind`, `f.kinds`, e1a8871bc9 and the emitter-kinds steps);
a field read sharing its name with a function compiled to has_field + map_get + instance_place name scans.
class-field-reads (0d6920297d) reads a known class's field by struct.get. Lesson: before publishing a seed, time
`warp rebuild pure` with it against the installed one (WARP_SEED=<new>), not only its fixpoint.

## Noise of a single timing (2026-10-28, card global-map-read)

One `warp pure build core/compiler.warp` moves by ±20 s with the load of the shared Mac: main's warp.wasm took 84 s and
a branch's 65 s back to back at load ~6, and refresh-seed refused seed 30d01c7a as 115 s vs 65 s during a load spike
(~31). A suspected 20 % cost of a `core_type_words.get` in aliased_type was that noise: 300k gets of the global map take
< 0.2 s inside the compiler (~350k aliased_type calls per self build). Compare timings only back to back, alternating,
taking each side's fastest; refresh-seed.sh does so (two alternating rounds after a warm build of each seed).
