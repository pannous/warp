# Incremental builds: modules lowered once (card prelude-cache-runs)

Goal (user 2026-10-18): std modules and a project's own modules are parsed, lowered and analysed once and kept on disk,
keyed by module source + warp binary + the flags that change lowering, so a program pays only for its own changed code.
Whole-program typing, specialisation and dead-code removal stay: the cache holds lowered definitions, not wasm
(notes/static_binding.md). The machine code is cached already (notes/aot.md "Compiled-module cache").

## Measuring: `WARP_PHASE_TIMES=1`
`WARP_PHASE_TIMES=1 warp x.warp` (or inline code) lists each phase's wall time on stderr (src/compiler/phase_times.rs:
`timed("phase", || …)`, `timed_as(|| name, || …)` for names made at run time). Phases nest: resolve is inside lowering;
"module source passes of lib/sound.warp" is one module's MODULE_SOURCE_PHASES, "module source pass N" the N-th pass of
SUGAR_PASSES + DECLARATION_PASSES (comments not counted). Probes: probes/cache/*.warp.

## Measured 2026-10-18 (quick-release, load average 30-80, so absolute numbers ~2x a quiet Mac)
Inline code (no program cache), median of 3, total wall time:

| program | caches on (std words, machine code) | WARP_MODULE_CACHE=off |
|---|---|---|
| `1+2` | 20-30 ms: lowering 4, parse 4, emit 1 | 150 ms: parse 110 (31 parses: every std module read for its words) |
| `to_base(255, 16)` | 40 ms: lowering 17 = resolve 8 + prelude look-ahead 5 + rest; emit 2 | 210 ms: parse 115, wasmtime 45 |
| `use sound; note(440)` | 370-490 ms, below | 740-930 ms: + parse 170-230, wasmtime compile 110-200 |

`use sound` with caches on (the case to fix): lowering ~350 ms of ~430:
- resolve uses 200-240 ms, of it lib/sound.warp's module source passes 120-170 ms (passes 60 insert_module_classes
  ~50-100, 51 reflection::lower ~20-35, 7 module_properties ~25, 53 pipes, 57 lower_unit_words, 81 music_words ~8 each),
  parse ~60 ms (13 parses; sound.warp is 33 KB, ~18 ms), the rest hidden_apart/needed definitions.
- prelude reach look-ahead 20-50 ms, emit 10-30 ms, the program passes over the spliced sound definitions ~100 ms
  (whole program: stays), wasmtime ~4 ms (cached).
So caching sound's parse + module source passes saves ~200 of ~430 ms; the ~2.5 s seen before was a debug build.
`WARP_MODULE_CACHE=off` also turns off the std-words cache (std_words_cache.rs), whose miss parses every std module.

## Plan
1. Keep `lower_module_source(parse(source))` of each standard module on disk (module_cache keep_bytes, extension
   `lowered`), key: binary stamp + source + the inputs its passes read (early classes, fast flags, the resolution of
   the module's own `use`s). Check: every sample's wasm identical with a cold and a warm cache.
2. The same for a project's file modules (key adds the file's path).
3. The prelude reach look-ahead and the parse of lib/prelude.warp.
