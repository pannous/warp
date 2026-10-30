# Handover: angle-type-alias (marked floats), warp-wgsl, 2026-10-30

Card angle-type-alias (Now). Branch angle-type-alias, worktree /Volumes/Worktrees/angle-type-alias, pushed to origin.
Core change with the user's OK (decisions marks-survive-arithmetic, marks-unboxed-first; via warp-interviewer).

## What it does
- `type Angle = float` marks a float as `type Color = int` marks an int (core/passes/int_type_shows.warp
  int_type_kinds, MARKED_NUMBER_KINDS; the emitter numbers the marks the same way: module.marks).
- A marked number stays unboxed while its static type knows the mark: TypedCode.mark (analysis/types.warp), set by
  `x as T` (emitter.warp library_words branch, marked_int: the number held as T's kind, an exact 0.5 as a float),
  `a: T` parameters (functions.warp new_body → Body.marks), locals (assignment → Body.marks; two different marks on
  one local widen it to a boxed Node), globals (module.global_marks), blocks and function bodies.
- Arithmetic keeps a mark (operators.warp operation_code → marked(): the left operand's, else the right's, of the
  result's kind); grown/exact int results and boxed operands by the runtime's marks_kept / kept_mark / type_mark
  (core/runtime/arithmetic.warp). Negation keeps it.
- Stamped where boxed (emitter.warp boxed), shown by the type's show (output.warp shown_text).
- A body giving a marked number gives a Node result (functions.warp, after the analyzer's kind); a marked value given
  an unannotated parameter is passed boxed (call_values).
- lib/units.warp: `type Angle = float`, `° = 0.017453292519943295 as Angle`, show(a: Angle) prints degrees rounded to
  12 digits.

## Lessons (cost three rebuilds)
- The runtime reads only the kind maps it already knows (Kind, KindUnit, Marked): a new scalar or map of
  analysis/kinds.warp is "the unknown name …" there, which silently leaves every runtime word reaching it out of
  runtime.wasm ("no field node_add" at the next stage). Debug: warn the refusal in prebuilt.warp prebuilt_runtime.
- A new runtime word called by the emitter needs `module.runtime.has(word)` (the seed's runtime predates it).
- A decimal literal is an exact ratio Node: marking it a float type must convert first.

## State at handover
Last commits: "fix: marks through grown and exact int results (kept_mark), blocks, function bodies and unannotated
parameters …" and the user's "test: red + 1 and red / 2 commented out (user: esoteric)". A rebuild of that tip was
running; before it, test/types/test_marked_numbers.pairs.warp failed on f(a: Angle) := a * 2 (plain), g(c: Color):
Color (plain), and test/types/test_int_types.pairs.warp on rgb(1, 2, 3) and with_alpha(red, 0.5) (plain; the
last fix targets these).

## Next
1. `scripts/rebuild.sh pure /Volumes/Worktrees/angle-type-alias`, then
   `WARP_LIB=/Volumes/Worktrees/angle-type-alias/lib ~/dev/bin/warp test test/types/test_marked_numbers.pairs.warp test/types/test_int_types.pairs.warp`;
   probes: data/wgsl/m.warp style (type C = int / Angle = float programs).
2. test_int_types `'red + 0'	4294901760` may now show red: upgrade in its own commit naming
   marks-survive-arithmetic (interviewer OK'd).
3. Check samples/gpu/ocean.warp still paints (° is now `as Angle`; the WGSL translator reads `°` as a program value).
4. Smoke + self-check (scripts/self-check.sh) + gate tier 1, merge origin/main, push, after-push, `todo done`.
5. Try lines to warp-interviewer: `30° * 2`, `f(a: Angle) := a * 2; f(30°)`, `red + 1`.

## Queue after it (warp-supervisor-2)
units-ignored-by-shaders (user: shaders take a quantity's SI amount via values map `float(name)` + Quantity
float() := amount; CPU fallback: runtime float_of_node asks an instance's float() hook, quantities stay quantities;
report ocean lines raising DimensionError; no `as number` in samples) → dot-product (∘ canonical, aliases.warp
`dot { operator: "∘", aliases: ["·", "•"] }`, warp fix rewrites calls to the canonical infix generally; take
lib/math ∘ from warp-exact-decimal-big's branch shader-plain-warp 513eb3c42b) → use-keys → ocean.warp m/s and m todos.
