# Ahead-of-time compilation (native, wasmtime 49)

Probes in probes/aot/ (`measure.sh`: wasmtime CLI JIT vs .cwasm; `standalone.sh`: a standalone executable;
`run_speed.sh`; `shared_runtime.sh`: two-Module Linker / `warp_runtime`).

## Cranelift in dev builds
`[profile.dev.package.wasmtime] opt-level = 3` optimizes only the wasmtime crate itself; cranelift-codegen, regalloc2
and friends need opt-level 3 too (Cargo.toml), else one 8 KB module takes ~200 ms to compile instead of ~19 ms (tests 8×
slower). Compiling, not running, is what tests pay for.

## Compiled-module cache
`src/run/module_cache.rs::compiled_module(engine, bytes)` replaces `Module::new` in wasm_reader::run_main,
host::run_wasm_simple and run_raw_struct. Key: SHA-256 of the module bytes + `Engine::precompile_compatibility_hash`
(wasmtime version, target, every setting that changes machine code: the gc engine and the task engine get different
entries). Files: `~/.cache/warp/modules/<key>.cwasm` (`WARP_MODULE_CACHE=<dir>` or `off`), written to a private file
and renamed (parallel tests, parallel sessions), read with `Module::deserialize_file` (mmap); a damaged file is compiled
again; past 1 GiB the least recently used half is dropped. ~125 KB per module; a warm hit costs ~3 ms.
wasmtime's own cache (`cache` feature, zstd) does the same but is not in the offline registry and pulls more crates.
One shared gc_engine per process (task engines stay separate: epoch ticks would reach other stores).

## `warp compile --aot <file>`
Also writes `<file>.cwasm`, the machine code for this machine and wasmtime version, through the engine the program
needs (task engine when it starts tasks). `warp <file>.cwasm` runs it (Engine::detect_precompiled; a task program's
.cwasm is recognized by failing to load into the gc engine). Trust: a .cwasm is native code, loading it is like running
an executable. `warp <file>.wasm` / `.cwasm` link host, WASI and FFI imports.

## Standalone executable, `warp build <file>`
- `warp build`/`compile` write the executable `<file>` (`<file>.exe` on Windows) and the module `<file>.wasm`; `--exe`
  is accepted (redundant), `--wasm` writes only the module. A plain run leaves an executable next to the file only with
  `WARP_EXECUTABLE=1` (or true/yes/on), rebuilt only when the source is newer (P103); a program the runtime cannot
  carry gets a note. `warp run <file>` never leaves one (P105).
- The executable is a copy of the prebuilt stub `warp-runtime` (crates/warp-runtime: wasmtime with `runtime`, `gc`,
  `gc-copying`, `std`, no Cranelift; ~1 MB release) with the program's machine code appended, deflated (miniz_oxide)
  (`[stub][cwasm][u64 le length][WRPCwasm]`); at start the stub reads its own last 16 bytes and runs what it carries.
- The stub is found through `WARP_RUNTIME_STUB`, else `warp-runtime` next to `warp` (or next to the file a link to warp
  points to), else warp builds it once from its source checkout (`cargo build --release -p warp-runtime`, issue #10; a
  debug warp always takes this release stub); only without that source nothing is written (P104: never a ~120 MB copy
  of warp; a plain run notes it on stderr, build exits 1).
- Tests build the stub once per run (tests/common runtime_stub) and copy this checkout's build from deps, found by its
  dep-info (the debug stub names CARGO_MANIFEST_DIR, BUILT_FROM): the shared target/debug/warp-runtime is any
  worktree's latest build.
- The program prints its value (P77): build compiles `print(<last statement>)` (pipeline::compile_printing_result; a
  declaration or a print stays), so the value is formatted by warp's own print (`[10 20 30]`, a lone text unquoted,
  texts in a container quoted, P126) and the stub needs no Node reader. Exit code 0, 1 on a trap.
- The stub provides print (WASI fd_write), libm ("m": Rust's f64 functions), the host words sleep, random,
  random_below, clock, and foreign_call. A program importing anything else (fetch, read, run_block, tasks, FFI
  libraries, std_io: tables, json, regex, files) is refused at build time with the missing imports named; a plain run
  says so once per version of the file and of warp (~/.cache/warp/standalone_notes).
- Foreign calls (card exe-foreign): `use python`/`use js` work in an executable. It starts python3 or node from PATH
  when it runs and fails loudly naming a missing interpreter ("cannot start python3 (…); set WARP_PYTHON to an
  interpreter"). One bridge for both: warp-runtime/src/modules/foreign.rs holds the children, loop scripts and
  callbacks on serde_json::Value; warp's src/modules/foreign.rs only turns Nodes into JSON and back and calls
  components. warp-runtime/src/gc_values.rs reads a program's GC Node as JSON and builds JSON back through its
  constructor exports (new_text … exact_add); stub_calls_export_with_foreign_calls keeps those exports when the program
  imports foreign_call. Still refused in an executable, loudly: warp functions given as callbacks, components
  (`use wasm`), C libraries; big integers and ratios cross to python as floats, as in warp.
- One copy of the run-time code: crates/warp-runtime holds the engine settings (`deterministic_config`,
  `fueled_config`), the fuel default, the host words without compiler (`link_host_words`, which warp's host.rs links
  too), fd_write and libm for any store state, and the trailer format. warp depends on it with the `compiler` feature
  (Cranelift + the component model it turns off); the browser build gets only the word names.
- Engines turn off the component model and its concurrency support, else the .cwasm refuses to load into a runtime
  built without them.
- macOS (crates/warp-runtime/src/macho.rs): the stub's signature is dropped, the machine code appended inside the
  __LINKEDIT segment (its file and memory size grown to cover it), and warp signs the executable ad hoc
  (`codesign --sign - --force`): `codesign --verify --strict` passes (plain appending after the signature fails strict
  validation). A Developer ID signature for distribution is the same codesign call with an identity. The executable
  finds its program before the signature (LC_CODE_SIGNATURE), elsewhere at the end of the file.
- wasmtime's own CLI also works for import-free programs: `wasmtime compile -W gc=y,function-references=y` +
  `wasmtime run --allow-precompiled`.

## Executable size
- The stub is always the release build (std ~200 KB, wasmtime + environ ~200 KB, warp_runtime ~100 KB of .text,
  cargo bloat); a size-optimized wasmtime would cost GC speed.
- Tree shaking: warp build keeps only the exports the stub calls (standalone::stub_calls_export: main, memory, the
  `on·…` handlers, new_empty, new_int) and drops what only the others reached (dead_functions::keeping_exports).
  tests/wasm/test_executable_size.rs.
- The executable's engine (standalone::standalone_engine, shared by warp build and the stub) makes no address map:
  traps still name the functions, without wasm offsets.
- An executable is the ~1 MB stub plus 100-200 KB machine code. A sample's carried code grows with what its untyped
  parameters may be (a parameter of unknown type indexing may index a text, which counts graphemes): typed parameters
  (`arr: list`) shrink the generic paths. probes/wasm_function_sizes.py compares two builds per function.
- Next steps if needed: a stub without the system signals and values (optional packages chosen by the program's
  imports), or the shared runtime module (Recommendation 5).

## Page apps (card build-links, option A)
`warp build page.warp` of a program showing a page (serve::shows_page, what `warp page.warp` opens in a window) writes
the executable `page`: the stub carrying the page's site packed (site_files::packed, behind the same trailer as machine
code, which the stub tells apart by PACKED_SITE) and showing it in the system's web view (warp-runtime webview.rs, the
window `warp app` uses too). `page --check` prints the page text from a hidden window (tests/web/test_built_app.rs,
probe probes/build_app/hello_app.warp). The web view (wry, winit, ~170 KB) is in a stub of its own,
`warp-page-runtime` (warp-runtime's feature webview, built and shipped next to warp-runtime, WARP_PAGE_RUNTIME_STUB),
so plain executables stay small. macOS only for now (elsewhere build writes the console executable). Later:
`warp build --app`, a .app bundle (card build-app-bundle).

## Alternatives (WASM GC + exceptions are both required by warp output)
| tool | GC | exceptions (try_table) | AOT | verdict |
|---|---|---|---|---|
| wasmtime 49 | ✓ | ✓ | ✓ `precompile_module` / .cwasm, runtime-only build | use it |
| wasm2c (wabt 1.0.42) | ✗ "only supports a limited set of features" | ✓ | C source | no |
| WAMR 2.4.5 | build flag WASM_ENABLE_GC (off in the Homebrew bottle); GC in interpreter and wamrc AOT | ✗ standard exceptions (legacy EH interpreter-only) | wamrc | no, until EH |
| Wasmer 7.5 | ✗ "No backends support the required features" | ✓ (Cranelift, LLVM) | ✓ (create-exe) | no |
| WasmEdge 0.17 | ✓ interpreter | AOT try_table only from 0.18-alpha | ✓ | later |

## Recommendation
1. Done: Cranelift crates optimized in dev builds.
2. Done: the on-disk module cache for every native run, and `warp compile --aot`.
3. Done: one shared gc_engine per process and `warp build`.
4. Done: dead-function elimination of every emitted module (src/compiler/dead_functions.rs): functions no export,
   start, element segment or global reaches are dropped (`f(x):=x+1; f(41)`: 113 → 52 functions).
5. Parked (card): one shared runtime module linked to every program (each .cwasm is ~10× its .wasm because every module
   carries warp's runtime functions). It would save a few seconds of a cold suite and nothing once the module cache is
   warm, but it changes the linking model everywhere (memory, text_heap, exception tags imported from the runtime,
   identical GC rec groups, a runtime instance per store in run_main, tasks, guarded_call, run_block, host.js, a
   standalone executable carrying two modules). `probes/aot/shared_runtime.sh` proves two-Module + Linker with the import
   `"warp_runtime"` (no component model); the production INT_RUNTIME / emit split is still Later.

## Run speed (`warp run` vs `warp <file>`, P105)
`probes/aot/run_speed.sh <warp>`. With a debug build, a small program (fib(20)) takes ~70 ms fresh and ~30 ms unchanged
under `warp run`.
- `warp run` touches no machine-code path: no precompile_module (only `compile --aot`), no compile_printing_result, no
  stub. It shares the JIT and the on-disk module cache with every run, which is what makes the unchanged case fast.
- The fresh case is the front end: parse and lowering (`warp lower`) and emitting the module; wasmtime's JIT compile and
  the run add only ~8 ms. Cranelift `OptLevel::None` made no difference, so the default stays; Winch has no GC support.
  Speeding up `warp run` further is the compiler-speed card (lowering passes, the emitted runtime).
- A plain run that builds an executable (`WARP_EXECUTABLE=1`) pays for the printing variant compiled once more and its
  machine code written; unchanged files skip it (the executable is newer).
- Test-suite time: notes/test_speed.md.
