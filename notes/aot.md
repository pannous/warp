# Ahead-of-time compilation (native, wasmtime 49)

Every run compiled its module with Cranelift (`Module::new`). Survey and measurements from 2026-10-05; probes in
probes/aot/ (`measure.sh`: wasmtime CLI JIT vs .cwasm; `standalone.sh`: a standalone executable; `shared_runtime.sh`: two-Module Linker / `warp_runtime`).

## Finding 0: tests compiled with an unoptimized Cranelift (fixed, on main)
`[profile.dev.package.wasmtime] opt-level = 3` optimized only the wasmtime crate itself; cranelift-codegen, regalloc2 and
friends ran at opt-level 0. One 8 KB module took ~200 ms to compile (the wasmtime CLI: ~8 ms).

| functions:: (249 tests, 1 thread) | wall | Cranelift share | per module |
|---|---|---|---|
| before | 122.4 s | 117.7 s (581 modules) | 200 ms |
| Cranelift crates at opt-level 3 | 14.8 s | 11.2 s | 19 ms |
| + module cache, cold | 13.5 s | 9.9 s | 17 ms |
| + module cache, warm | 5.2 s | 1.9 s (deserialize) | 3.2 ms |

Programs themselves ran 0.1–0.2 s in total: compiling, not running, is what tests pay for.

## Options
1. **Compiled-module cache** (implemented): `src/run/module_cache.rs::compiled_module(engine, bytes)` replaces
   `Module::new` in wasm_reader::run_main, host::run_wasm_simple and run_raw_struct. Key: SHA-256 of the module bytes +
   `Engine::precompile_compatibility_hash` (wasmtime version, target, every setting that changes machine code: the gc
   engine and the task engine get different entries). Files: `~/.cache/warp/modules/<key>.cwasm`
   (`WARP_MODULE_CACHE=<dir>` or `off`), written to a private file and renamed (parallel tests, parallel sessions),
   read with `Module::deserialize_file` (mmap); a damaged file is compiled again; past 1 GiB the least recently used
   half is dropped. ~125 KB per module (547 modules: 68 MB).
   wasmtime's own cache (`cache` feature, wasmtime-internal-cache, zstd) does the same but is not in the offline registry
   and pulls more crates; ours is 100 lines.
   An in-process map of Modules would save the remaining ~3 ms, but a Module belongs to its Engine and every run makes
   its own engine; sharing the gc_engine is possible (task engines must stay separate: epoch ticks would reach other
   stores).
2. **`warp compile --aot <file>`** (implemented): also writes `<file>.cwasm`, the machine code for this machine and
   wasmtime version, through the engine the program needs (task engine when it starts tasks). `warp <file>.cwasm` runs
   it (Engine::detect_precompiled; a task program's .cwasm is recognized by failing to load into the gc engine).
   Trust: a .cwasm is native code, loading it is like running an executable.
   `warp <file>.wasm` / `.cwasm` now link host, WASI and FFI imports (a printing program failed with an unknown
   `wasi_snapshot_preview1::fd_write` before).
   CLI, debug warp, ackermann: 41 ms without cache, 12 ms with the cache or the .cwasm.
3. **Standalone executable, `warp <file>`** (implemented; P103: running a file leaves `<file>` next to it, `<file>.exe`
   on Windows, rebuilt only when the source is newer; a program the runtime cannot carry gets a note; `warp build`/`compile`
   only make it, `--wasm` writes only the module): the executable is a copy of the prebuilt stub
   `warp-runtime` (crates/warp-runtime: wasmtime with `runtime`, `gc`, `gc-copying`, `std`, no Cranelift) with the
   program's machine code appended (`[stub][cwasm][u64 le length][WRPCwasm]`); at start the stub reads its own last 16
   bytes and runs what it carries. The stub is found through `WARP_RUNTIME_STUB`, else `warp-runtime` next to `warp` (or next to
   the file a link to warp points to), else warp builds it once from its source checkout (`cargo build --release -p
   warp-runtime`, issue #10: building an executable just works; a debug warp always takes this release stub, its own
   neighbour is a debug stub, card g_gFs8); only without that source nothing is
   written (P104: never a ~120 MB copy of warp; a plain run notes it on stderr, build exits 1).
   Tests build the stub once per run (tests/common runtime_stub) and copy this checkout's build from deps, found
   by its dep-info (the debug stub names CARGO_MANIFEST_DIR, BUILT_FROM): the shared target/debug/warp-runtime is
   any worktree's latest build (card stub-race: 12 empty outputs in a gate). `warp run <file>` runs without leaving an
   executable (P105). Release stub (`cargo build --release -p warp-runtime`): **805 KB**; ackermann.exe **972 KB**
   (167 KB machine code), starts and finishes in well under 10 ms (probe build: 0.33 ms in total).
   - The program prints its value: build compiles `print(<last statement>)` (pipeline::compile_printing_result;
     a declaration or a print stays), so the value is formatted by warp's own print (`[10 20 30]`, a lone text unquoted, texts in a container quoted, P126)
     and the stub needs no Node reader.
     Decided by the user (P77): an executable shows its prints, then its value as `print` shows it (texts without
     quotes); exit code 0, 1 on a trap.
   - The stub provides print (WASI fd_write), libm ("m": Rust's f64 functions) and the host words sleep, random,
     random_below, clock. A program importing anything else (fetch, read, run_block, tasks, FFI libraries) is refused
     at build time with the missing imports named. A plain run says so once per version of the file and of warp
     (~/.cache/warp/standalone_notes remembers the failure; card standalone-std-io): std_io (tables, json, regex,
     files) would need the Node reader in the stub.
   - One copy of the run-time code: crates/warp-runtime holds the engine settings (`deterministic_config`,
     `fueled_config`), the fuel default, the host words without compiler (`link_host_words`, which warp's host.rs
     links too), fd_write and libm for any store state, and the trailer format. warp depends on it with the
     `compiler` feature (Cranelift + the component model it turns off); the browser build gets only the word names.
   - Engines turn off the component model and its concurrency support, else the .cwasm refuses to load into a
     runtime built without them.
   - macOS (2026-10-05, crates/warp-runtime/src/macho.rs): the stub's signature is dropped, the machine code appended
     inside the __LINKEDIT segment (its file and memory size grown to cover it), and warp signs the executable ad hoc
     (`codesign --sign - --force`): `codesign --verify --strict` passes. Plain appending after the signature ran but
     failed strict validation. A Developer ID signature for distribution is the same codesign call with an identity.
     The executable finds its program before the signature (LC_CODE_SIGNATURE), elsewhere at the end of the file.
4. **wasmtime's own CLI**: `wasmtime compile -W gc=y,function-references=y` + `wasmtime run --allow-precompiled`
   works for import-free programs (11–12 ms per process).

| program | wasm | cwasm | wasmtime compile | run .wasm (JIT) | run .cwasm |
|---|---|---|---|---|---|
| ackermann | 10.7 KB | 133 KB | 19 ms | 20 ms | 12 ms |
| binary_tree | 21.1 KB | 187 KB | 22 ms | 23 ms | 12 ms |
| calculator | 20.1 KB | 170 KB | 19 ms | 26 ms | 18 ms |
(wasmtime 49.0.2 CLI, best of 10 process runs; most of the 11 ms is process start.)

The .cwasm is ~10× the .wasm: every module carries warp's runtime functions (texts, lists, maps …), so each compiled
module repeats their machine code. A shared runtime module linked to the programs would shrink both.

## Executable size (card g_gFs8, 2026-10-09)
User: "quicksort: enormous file size of simple algorithms should be reduced with tree shaking". The 21 MB were a debug
stub (a debug warp built the stub in its own profile; that stub is 5.6 MB today). Now:
- the stub is always the release build (~890 KB; std ~200 KB, wasmtime + environ ~200 KB, warp_runtime ~100 KB of
  .text, cargo bloat); a size-optimized wasmtime would cost GC speed (see the release profile note);
- tree shaking: warp build keeps only the exports the stub calls (standalone::stub_calls_export: main, memory, the
  `on·…` handlers, new_empty, new_int) and drops what only the others reached (dead_functions::keeping_exports);
- the executable's engine (standalone::standalone_engine, shared by warp build and the stub) makes no address map:
  traps still name the functions, without wasm offsets;
- the carried machine code is deflated (miniz_oxide, already in std's tree: the stub grew 32 bytes); it is mostly page
  padding and tables.

| executable (release stub) | before | after | machine code (raw) |
|---|---|---|---|
| samples/quicksort.warp | 1131568 | 927296 | 215856 → 179752 |
| samples/game_of_life.warp | 1115744 | 919616 | 200032 → 163680 |
| `6*7` | 1045520 | 906816 | |
The rest is the stub. Start time unchanged (~19 ms per run with process start). Next steps if needed: a stub without
the system signals and values (optional packages chosen by the program's imports), or the shared runtime module
(Recommendation 5).

## Alternatives (WASM GC + exceptions are both required by warp output)
| tool | GC | exceptions (try_table) | AOT | verdict |
|---|---|---|---|---|
| wasmtime 49 | ✓ | ✓ | ✓ `precompile_module` / .cwasm, runtime-only build | use it |
| wasm2c (wabt 1.0.42) | ✗ "only supports a limited set of features" (checked) | ✓ | C source | no |
| WAMR 2.4.5 | build flag WASM_ENABLE_GC (off in the Homebrew bottle: "invalid type flag", checked); GC in interpreter and wamrc AOT | ✗ standard exceptions (lists them as unsupported; legacy EH interpreter-only) | wamrc | no, until EH |
| Wasmer 7.5 | ✗ "No backends support the required features" (checked) | ✓ (Cranelift, LLVM) | ✓ (create-exe) | no |
| WasmEdge 0.17 | ✓ interpreter | AOT try_table only from 0.18-alpha | ✓ | later |

## Recommendation
1. Done: Cranelift crates optimized in dev builds (largest win, tests 8×).
2. Done (branch aot): the on-disk module cache for every native run, and `warp compile --aot`.
3. Done (g-qV5Y, g-qV8Y): one shared gc_engine per process (a repeated run is a map lookup) and `warp build`.
4. Done (2026-10-05): dead-function elimination of every emitted module (src/dead_functions.rs): functions no export,
   start, element segment or global reaches are dropped. `f(x):=x+1; f(41)`: 113 → 52 functions, 7.6 → 5.0 KB wasm,
   cwasm 131 → 92 KB, single-threaded Cranelift 17.0 → 14.7 ms; ackermann 10.9 → 8.4 KB, cwasm 149 → 128 KB;
   functions:: (289 tests, one thread, cache off) 17.1 → 13.4 s.
5. Parked (card): one shared runtime module linked to every program. Would bring f to ~7 ms and ~55 KB, about −3 s of a
   35 s cold suite and nothing once the module cache is warm, but it changes the linking model everywhere (memory,
   text_heap, exception tags imported from the runtime, identical GC rec groups, a runtime instance per store in
   run_main, tasks, guarded_call, run_block, host.js, a standalone executable carrying two modules).
   Probe (g-qV-s first slice): `probes/aot/shared_runtime.sh` proves two-Module + Linker with import
   `"warp_runtime"` (no component model); production INT_RUNTIME / emit split is still Later.

## Test-suite time (./test.sh)
test.sh prints `TIMING: compile N s, run N s` and the 15 slowest tests (libtest `--report-time`, enabled on stable by
RUSTC_BOOTSTRAP=1, which rebuilds nothing). Integrator run 2026-10-05 (main 006b141a, 2174 tests, module cache and
optimized Cranelift in): **compile 12 s, run 35 s**. The ~7 minutes the Integrator saw before are spent outside
test.sh's two cargo commands (queue waiting, merging, the wasm32 browser-test build).
The run is bounded by its slowest tests, not by Cranelift any more: test_law_proved_by_lean 17.8 s,
every_sample_runs_without_a_compiler_error 15.6 s, test_upper_and_lower 14.9 s, the uniscript tests 6–11.6 s each
(8 of the 15 slowest), upper_and_lower_map_latin_greek_and_cyrillic 9.0 s, upper_and_lower_cover_unicode_scripts
8.2 s, use_requires_a_version_of_a_package 8.1 s, read_loads_a_file_as_bytes 7.4 s.

## Run speed (`warp run` vs `warp <file>`, P105, 2026-10-06)
`probes/aot/run_speed.sh <warp>`, debug build on a loaded M-series Mac, medians of 9, a small program (fib(20)):
| | fresh program | unchanged program |
|---|---|---|
| `warp run <file>` | 73 ms | 28 ms |
| `warp <file>` (runs, then builds the executable) | 159 ms | 29 ms |
| `warp compile --wasm` (front end and emitter only) | 65 ms | |

- `warp run` touches no machine-code path: no precompile_module (only `compile --aot`), no compile_printing_result, no
  stub. It shares the JIT and the on-disk module cache with every run, which is what makes the unchanged case 28 ms.
- The fresh case is the front end: parse and lowering ~23 ms (`warp lower`), emitting the module ~40 ms; wasmtime's
  JIT compile and the run add only ~8 ms. Cranelift `OptLevel::None` measured no difference (73 vs 75 ms fresh, the
  50M-iteration loop 148 vs 150 ms), so the default stays; Winch has no GC support. Speeding up `warp run` further is
  the compiler-speed card (lowering passes, the emitted runtime).
- A plain run of a fresh file costs ~85 ms more: the printing variant compiled once more and its machine code
  written into the executable; unchanged files skip it (the executable is newer).

