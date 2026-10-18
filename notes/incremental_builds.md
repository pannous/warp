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

## A prelude word against a Rust builtin (card prelude-word, 2026-10-18)
Measure: `probes/cache/proxy_times.sh <warp> [runs] [phase pattern]`, each phase's minimum over runs of `print [1 2 N]`
(Rust list_text) and `print to_base(N, 2)` (a warp prelude word), fresh literals so the program cache misses. At load
30-140 only minima mean anything; compare CPU time (rusage of the child) for settings, interleaved runs.

| step | `print [1 2 N]` lowering | `print to_base(N, 2)` lowering |
|---|---|---|
| main before (slice 2) | 3.9 ms | 15 ms |
| per-definition analysis memo (analysis_memo::analysed_function: to_base was analysed 38× per compile) | 3.9 | 14.5 |
| C header signatures on disk (ffi/header_cache.rs: libm 20 ms cold / libc 2 ms parsed at every run meeting `abs`, `/!`) | 3.9 | 13.3 |
| library-signature indexes (library_signatures.rs, std_docs::module_index: math.warp and the prelude rescanned per word, quadratic) | 1.9 | 8.1 |

What is left of to_base's extra 6 ms: resolve 2.4 (prelude look-ahead 1.0, replaced declarations 0.6, qualified names
0.4), many whole-program passes at 0.2-0.5, emit +1.4.

The wall time of a fresh program is mostly wasmtime compiling its module: 28-31 ms for `print [1 2 N]` (123 functions,
nearly all runtime), 30-43 ms for to_base (196). A profile of `wasmtime compile` (xctrace Time Profiler, --launch, one
process per sample set; `--all-processes` gets no samples without more rights): Cranelift codegen (regalloc, egraph,
lowering) 60%, wasm→CLIF translation 15%, process start 10%. Hence:
- wasmtime's incremental compilation cache (feature incremental-cache, run/function_cache.rs: one file per function in
  the module cache) skips the codegen of every function another module compiled: a fresh `print [1 2 N]` adds 4
  functions. CPU min: to_base 88 → 75 ms, print 62 → 60. Machine code from it still loads into the reference runtime.
- parallel-compilation (was off: default-features = false): wall min to_base 111 → 70 ms with both, at load ~40.
- Cache keys of the wasm functions are stable (`Config::emit_clif` dumps of two cold runs differ only in `main`), but
  wasmtime's ~30 signature trampolines get new keys at every compile under parallel compilation (their pre-optimization
  CLIF comes out in another instruction order; serially +1 per run). So even the same module recompiled adds ~7 files and
  a new program ~30 small ones; module_cache's LRU prune at 1 GB bounds them. Dumps of cache hits show unoptimized CLIF,
  so compare only cold runs.
- opt-level none saves only 10-20% of a full compile, nothing on cache hits; Winch refuses function references.
