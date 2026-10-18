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

Then parse and resolve (quick-release, minima, `probes/cache/proxy_times.sh`; time phases with the debug build
and the numbers are 3-7× off):
- parse 4.3 → 1.6 ms for any program: building the parser's user-operator table read every standard module line through
  alias_line(code_only(..)) once per process (library_glyph_operators). Now only lines with `:=` are looked at and the
  alias check runs only on a declared operator. 0.8 ms of that scan remains (~700 `:=` lines).
- prelude: qualified names 0.35 → 0.0 ms: renamed_words renames every word in one walk over a map (it was two walks
  per prelude word over each definition).
- What is left of resolve for to_base (~2 ms): the look-ahead parses its reached prelude slices anew in each process
  (0.3 ms per call, twice), needed_definitions 0.15. Keeping parsed slices on disk would save ~0.5 ms.
- opt-level none saves only 10-20% of a full compile, nothing on cache hits; Winch refuses function references.

## Library functions compiled once per specialisation (card style-block, design 2026-10-18)
The case: a page with one `style{ … }` block (style-data branch, lib/markup.warp `style(sheet: data)`). Release, warm
caches, fresh program each run, against the same page without the style block:

| phase | plain page | style page |
|---|---|---|
| lowering, all | 3 ms | 105-110 ms (resolve ~23, program passes ~32, meaning passes ~31) |
| emit | 1.3 ms | 30-33 ms |
| wasmtime compile (function cache warm) | 10-12 ms | 23-27 ms |
| module | 58 functions, 51 types, 8 KB | 202 functions, 104 types, 50 KB |

The style block reaches 19 library functions (16 `style_*`, 3 `prelude·popped_*`), and they bring in ~125 runtime
functions. The lowered-modules cache only covers the sugar and declaration phases of a module's source
(MODULE_SOURCE_PHASES). The rest runs again in every compile over the whole spliced program: MODULE, SIGNAL and
FUNCTION passes (~100 passes), the meaning passes, analysis, emit. That costs ~5 ms per reached library function,
whatever the program is.

### Idea
Whole-program typing stays. What gets cached is each reached library function's lowered and emitted form, keyed by
its specialisation. A program re-runs the passes only for functions whose specialisation is new. Tree shaking
(with_needed_definitions, dead_functions.rs) and specialisation stay as notes/static_binding.md has them. Only their
results are reused.

Key of a specialisation:
- the definition's source hash (its slice, as prelude_slices cuts the prelude);
- the argument and result kinds the analysis gave this specialisation. Constant specialisations count too:
  `prelude·to_base·n255·base16` is its own function per literal, so its key holds the constants;
- the compile-context inputs it reads (lowered_modules' lowering_inputs and LoweredEffects already list these);
- the warp binary stamp (std_words_cache binary_stamp).

### Lowered form (step 1, the larger share: ~85 of the ~150 ms)
- Splice a cached specialisation as an opaque, already-lowered definition. Mark it with a Meta flag, the way
  lowered_prelude_definition marks a definition lowered alone. Each program pass leaves marked definitions alone.
  Today the passes are written against the whole tree, so this means auditing ~100 passes. Most rewrite only what
  they meet, and a lowered body has nothing left for them.
- The passes that read across definitions (analysis, user-function extraction, typed_library_words, effects)
  read a cached summary instead: the UserFunctionDef the analysis memo already stores per definition, plus the
  specialisation's calls (for needed_definitions and dead_functions).
- A miss lowers the definition as today, then keeps the result if nothing was said (no hint or warning) and no fresh
  name was numbered. Those are the same keep rules as lowered_modules.rs.
- Oracle: probes/cache/cold_warm_equal.sh must stay byte-identical off/cold/warm over all samples. A pass that
  doesn't skip marked definitions shows up there as a difference.

### Emitted form (step 2: ~30 ms emit + part of the wasmtime compile)
A cached body is wasm bytes with relocations, as in wasm object files (wasm-ld's R_WASM_*_LEB):
- every patchable index is written as a 5-byte padded LEB and recorded with a symbolic name: function by name
  (runtime, prelude or user function), type by key, global by name, data segment by name, and the literal
  (offset, length) by the text itself;
- final assembly lays out the module (imports, runtime, functions; dead_functions after) and patches each
  relocation from that module's tables. The string table interns the literal texts again, so they dedupe as now;
- a body's runtime dependencies (runtime_dependencies.rs) come with it, so the ~125 runtime functions it pulls in
  are known without emitting the caller. Runtime functions are the same in every program, so they can be kept
  the same way.

### What the emitter's type indices need
- Every type goes out in its own singleton rec group (type_manager.rs: `types.ty().subtype(...)`). GC types are
  canonicalised structurally, so a type key ("$Node", a user struct's fields, a signature) names the same type in
  every module. Relocation by key is therefore enough. Stable numbering isn't needed for correctness.
- Today's numbering isn't stable across programs anyway. `with_meta` inserts $MetaNode in the core prefix and shifts
  later types, and user structs and function types are appended on demand in emission order.
- For wasmtime's function cache, stable numbering does matter. Cranelift's stencil holds module type indices (while
  callee function indices sit outside the key), so the same body under shifted type indices misses. Worth doing
  with step 2: emit the core types first in a fixed order (always declare $MetaNode, or put it last), then
  library-specialisation types sorted by key, then the program's own types.

### Expected
For the style page, warm: lowering ~110 → ~25 ms (resolve and the program's own passes stay), emit ~32 → ~5 ms,
compile stays ~25 ms (better with stable types). About 250 → 100 ms wall in release. The win is about 5 ms per
reached library function, for every program that reaches them (pages, routes, sound).

### Risks
- Passes that rewrite library code based on the program (typed_library_words' `count·any`, class modules, inlining
  if it comes): their inputs have to be in the key. If one is missed, the oracle catches it as a difference;
  otherwise it's a wrong cache hit.
- Step 1 touches every pass (skip marked definitions), while step 2 is local to the emitter. Step 2 alone gives
  ~30 ms, step 1 alone ~85 ms.

### Order (supervisor, 2026-10-18; open until the user decides otherwise)
Step 2 first. Step 1 waits until the plain-library and parser cleanup has settled, because it would audit ~100
passes that those cleanups are still merging and removing.

### Found while measuring step 2: the emitter runs twice for most programs
`emit` for the style page (release, warm): extract_user_functions 6-10 ms, user functions 7-17, main 2, types and
runtime 1, finish 2. Then the whole emit_for_node again (13-30 ms). When a body calls a runtime function that
analyze_required_functions didn't foresee, emit_call writes `unreachable`, notes a Need, and emit_for_node reruns
with the needs inherited. That happens to 175 of the 218 samples (`compile --wasm`, cache off). Most frequent: print_value
139, node_list_at 134, node_list_push / node_list_as_node 112, tagged_field / elements_field 105, node_order 102,
node_add 92, list_join 82, node_slice 76, text_concat 68, m.pow (an import) 45. So about half of every program's
emit is the rerun.
- Predicting more in analyze_required_functions (print → print_value, list_text, text_of) is the cheap fix, but it's
  case by case.
- The structural fix is lazy runtime functions: emit_call takes the next index for an unforeseen runtime function and
  emits its body (with its runtime_dependencies) after the others. Today 144 `should_emit_function` gates emit runtime
  functions in a fixed order, so they'd need a name → emitter table. Imports (m.pow) still can't be appended: their
  indices come first.
- Step 2 can cut part of it: a cached body carries the needs it discovered, so seeding them up front makes the
  library functions' share of reruns disappear (main's own needs, such as print_value, remain).
