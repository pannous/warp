# Speed of the self-compiled compiler (card self-speed, 2026-10-22)

`warp self` (compiler-self.wasm, emitted by emitter.warp) against `warp pure` (compiler.wasm, emitted by Rust),
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
- `new_symbol` under `kept`, ~20 % of the remaining run: the host rebuilds the kept runtime words' tree node by node
  through the exported builders (runtime builders.warp copies each symbol byte by byte out of linear memory). Pure
  spends the same absolute time (~0.13 s over 8 runs), so it is no gap between the two compilers; a real fix is
  host-side (keeping the lowered runtime as wasm data instead of a rebuilt tree).
- The lexer walks texts by code point (code_point_width/at/text, character_end, ~4–8 %): card lexer-walks (Later).
- Statically compiler-self.wasm still re-allocates text literals per use (array.new_data), boxes many ints and casts
  more (ref.cast) than Rust's output, but none of these showed as a hot spot of 10 % or more.

Related: a cold module cache (no kept runtime words) makes `warp self` trap in lowered_program, on main too (cards
self-traps / self-cold-trap).
