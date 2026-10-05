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
3. **Standalone executable** (probe): wasmtime with `runtime`, `gc`, `gc-copying`, `std` only (no Cranelift) plus the
   .cwasm via `include_bytes!` (probes/aot/standalone_runner.rs). ackermann: **968 KB executable** (149 KB machine code),
   loads in 0.25 ms, runs in 0.33 ms in total; the warp release binary is 7 MB. Needed: warp's engines turn off the
   component model and its concurrency support (util::deterministic_config), otherwise the .cwasm refuses to load
   into a runtime built without them. Missing for real programs: the host words, WASI (print) and FFI imports, and
   printing the result Node (gc_traits); a `warp-runtime` crate holding host.rs + a small WASI fd_write would provide
   them. wasmtime-wasi pulls tokio and would grow the binary several MB.
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
3. Done (g-qV8Y): `crates/warp-runtime` is wasmtime without Cranelift (slim host + hand-rolled `fd_write`);
   `warp build --exe` copies that prebuilt stub and appends the `.cwasm` (`[stub][cwasm][u64 le len][WRPCwasm]`).
   Releases ship `warp-runtime` next to `warp` (or set `WARP_RUNTIME_STUB`). `run_block` / `host.run` / `task_*`
   error clearly in the standalone runtime. Still next: share one gc_engine per process (with the cache: a hit
   becomes a map lookup) — landed separately on main as g-qV5Y; do not reopen.
4. Later: link programs against one shared runtime module instead of emitting the runtime into each module.

## Test-suite time (./test.sh)
test.sh now prints `TIMING: compile N s, run N s` and the 15 slowest tests (libtest `--report-time`, enabled on stable
by RUSTC_BOOTSTRAP=1, which rebuilds nothing). Full-suite breakdown: pending the Integrator's next run.
