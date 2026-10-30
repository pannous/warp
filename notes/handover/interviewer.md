# Interviewer handover (2026-10-30 night)

Read notes/agents/common.md + notes/agents/interviewer.md first; they beat this file.
Supervisor is warp-supervisor-2. Feature workers: warp-hello-rss-peak, warp-wgsl (cap of two).

## Open with the user
- notes/open_decisions.md "To confirm": ~48 entries decided on the user's behalf (newest first: f32-growing,
  marked-floats-shape, method-clash-from-source, fast-int-readings, library-host-words → replaced by
  library-direct-refs, …). Ask: confirm all, or name the ones to undo or discuss.
- Still "ask later": round(2.5) = 3, sample-spellings, charged-values-no-pass, the event-handlers prose discussion
  (three questions in Parked).
- Two defaults of mine inside user decisions, to confirm when it fits: marks-survive-arithmetic (two different marks →
  the left operand's), units-ignored-by-shaders ("ignores the unit" = the base SI amount, 20 km → 20000).

## Try lines to run once installed, then show the user
- library-direct-refs: `warp samples/gpu/gpu_million.warp` (2.2 s for 1M; floats-takes should make it ms).
- floats-takes ([f32], packed f32 in linear memory, landing now): gpu_million's time.
- cast-checks / nat-and-int (608d7caadb): `(3037000500*3037000500) as int` → too big for int; `as! int` →
  -9223372036709301616; `type(3)` → nat; `x: int? = 3; type(x)` → int; `5 is nat` → yes.
- file-use (1d3fb08976): `use math\nuse units\nx = 6 m\nprint(x)\nprint(m.text())` from a file → 6m, 1m.
- module-export-arity (31dc737e14): `warp data/visualizer/b5.warp` paints, then prints I.
- bare-overloaded-word: `use sound; x = sound` → a Recording.
- unused-imports-pass (550dee2568): `warp build hello.warp && wasmtime -W gc=y,function-references=y,exceptions=y
  hello.full.wasm` prints hi.
- shader-port (236453f400): `WARP=warp bash test/gpu/test_gpu_matches_cpu.sh` 8 ✓ (all failed on the old install).
- marked floats (angle-type-alias, building): `30° * 2` → 60°, `f(a: Angle) := a * 2; f(30°)` → 60°, `red + 1`.
- list-display-slowness: show of a 10k float list in ms.
- heap-per-run 64 MB: hello ~101 MiB (`/usr/bin/time -l`, divide bytes by 1048576).

## Workers waiting on nothing from me
- warp-wgsl queue: angle-type-alias, units (a)+(b) + float() hook for the CPU fallback (user core OK), units in
  ocean.warp (m/s, m, °/px, s/ms), dot-product (∘ canonical, warp fix rewrites from the aliases entry).
- warp-hello-rss-peak: rebake-tables-only (user OK), bisect of the self-compile slowdown, let-inside-mark /
  node-attributes, heap 64 MB landing.
- Unassigned on the board: as-cast-warning, f32-gpu, xs-map-gpu, list-display-slowness step 2 (parse_code),
  module-parsed-tables (Next), runtime-host-words-out, native-libraries done.

## Today's user decisions (notes/decisions.md)
relative-paths-cwd, project-root-paths (•/ @/ ://), source-relative-prefix (^/), source-dir-words ($dir $file),
unused-imports-pass, library-direct-refs, gpu-bound-floats ([f32], f32), cast-checks, checked-forced-conversions,
degree-constant, units-ignored-by-shaders, marks-survive-arithmetic, marks-unboxed-first, heap-per-run (64 MB),
rebake-tables-only, dot-glyphs (∘ canonical). Lesson in notes/taste.md: familiar conventions first.
