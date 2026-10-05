# Ahead-of-time compilation (native, wasmtime 49)

Every run compiled its module with Cranelift (`Module::new`). Survey and measurements from 2026-10-05; probes in
probes/aot/ (`measure.sh`: wasmtime CLI JIT vs .cwasm; `standalone.sh`: a standalone executable).

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
3. **Standalone executable, `warp build --exe <file>`** (implemented): `<file>.exe` is a copy of the prebuilt stub
   `warp-runtime` (crates/warp-runtime: wasmtime with `runtime`, `gc`, `gc-copying`, `std`, no Cranelift) with the
   program's machine code appended (`[stub][cwasm][u64 le length][WRPCwasm]`); at start the stub reads its own last 16
   bytes and runs what it carries. The stub is found through `WARP_RUNTIME_STUB`, else `warp-runtime` next to `warp`,
   else warp itself is the stub (warp's main also runs a program it carries: tests need no extra build, the executable
   is then warp-sized). Release stub (`cargo build --release -p warp-runtime`): **805 KB**; ackermann.exe **972 KB**
   (167 KB machine code), starts and finishes in well under 10 ms (probe build: 0.33 ms in total).
   - The program prints its value: build --exe compiles `print(<last statement>)` (pipeline::compile_printing_result;
     a declaration or a print stays), so the value is formatted by warp's own print (`[10 20 30]`, texts unquoted)
     and the stub needs no Node reader.
   - The stub provides print (WASI fd_write), libm ("m": Rust's f64 functions) and the host words sleep, random,
     random_below, clock. A program importing anything else (fetch, read, run_block, tasks, FFI libraries) is refused
     at build time with the missing imports named.
   - One copy of the run-time code: crates/warp-runtime holds the engine settings (`deterministic_config`,
     `fueled_config`), the fuel default, the host words without compiler (`link_host_words`, which warp's host.rs
     links too), fd_write and libm for any store state, and the trailer format. warp depends on it with the
     `compiler` feature (Cranelift + the component model it turns off); the browser build gets only the word names.
   - Engines turn off the component model and its concurrency support, else the .cwasm refuses to load into a
     runtime built without them.
   - macOS: appending keeps the ad-hoc signature valid enough to run, `codesign -v` reports "failed strict
     validation"; a signed release would put the machine code in a Mach-O section instead (like bun --compile).
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
3. Done (g-qV5Y, g-qV8Y): one shared gc_engine per process (a repeated run is a map lookup) and `warp build --exe`.
4. Later: link programs against one shared runtime module instead of emitting the runtime into each module.

## Test-suite time (./test.sh)
test.sh now prints `TIMING: compile N s, run N s` and the 15 slowest tests (libtest `--report-time`, enabled on stable
by RUSTC_BOOTSTRAP=1, which rebuilds nothing). Full-suite breakdown: pending the Integrator's next run.
