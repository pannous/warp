# A minimal native host for pure-pipeline wasm: shell exec and FFI (investigation, 2026-10-22)

Investigation only: nothing was built, committed or pushed. The base is origin/main **e5ca6591b** (fetched 19:44
CEST), read in a throwaway boxtree. Measurements use the box's debug warp (bf03df01c) and a compiler.wasm /
compiler-self.wasm built at 19:41–19:48 CEST from another agent's tree (main-check/data, only read). Runs used
`WARP_LIB=<origin/main lib>`. **[M]** marks a fact I measured or read this session, **[I]** an inference.

## TL;DR
- Most of the host already exists. `crates/warp-runtime` (the stub of `warp build` executables) is a small
  wasmtime host with no compiler. It has dlopen and dlsym (libloading), a C-call wrapper for every signature, C
  handles, structs-as-maps, Node readback and building, libc and libm, the host words, and WASI `fd_write`. Built on
  its own on the Mac (arm64), with no Cranelift, it is **1,220,672 B** [M]. Its cdylib `libwarp_runtime.dylib`
  (feature `library`, which includes Cranelift) is **6,233,856 B** [M]. The full Rust `~/dev/bin/warp` is **47,035,728 B** [M].
- The pure pipeline emits **no C imports** [M]. `import f from "c"`, `use m` and foreign lines are dropped as
  statements (emitter.warp:251-252). A call then either compiles to warp's own word (sqrt, abs, floor) or is refused
  with "outside the core: the call strlen(…)". **On pure, FFI test files pass 10 of 66 (test_ffi.warp), 0 of 15
  (test_ffi.pairs), and 0 of 12, 0 of 6, 0 of 5 … in the others** [M]. The 10 that pass make no C call.
- Recommendation: **option (a), built on warp-runtime.** Add one more binary to that crate, `warp-host` (≈ 6–7 MB [I]).
  It runs compiler-self.wasm, which needs only WASI p1 plus `host.std_io` and `host.local_offset` [M], and then the
  module it writes. Unknown imports are linked from a custom section `warp.c_functions`, in the CFunction line
  format that already exists (c_signatures.rs:164). Keep the current register-based caller and add libffi later for
  variadics and callbacks.

## 1. What the Rust side already offers

### How warp source declares native functions today
- `import strlen from "c"`, `import (sin,floor,fabs) from 'm'`, `use m`, `use sqlite3`, `use raylib`: the
  signatures come from the library's **C headers**, parsed at compile time (src/ffi/header.rs 517 lines,
  macros.rs 202, struct_layouts.rs 200, header_cache.rs 98).
- Foreign lines: `erf(x: float): float from 'm'` (src/ffi/mod.rs:126 `foreign_line`). lib/math.warp:77-79 declares
  libm this way.
- Hand-linked libc (warp's own variants: strcmp and strncmp take (ptr,len) pairs): src/ffi/mod.rs:19-32,
  crates/warp-runtime/src/c_signatures.rs:8 `HAND_LINKED_LIBC`, c_calls.rs:46 `link_libc_functions`.
- `extern`, `native`, `C` and `inline` are only skipped as foreign modifiers (core/foreign_forms.warp:9).
  `extern int abs(int x)` is no declaration: on pure it gives "the unknown name x" [M].

### What reaches the wasm (Rust emitter)
- Plain wasm imports: **module = the library as the program names it** (`c`, `m`, `sqlite3`, `zlib`), **field = the
  C symbol**, and a func type derived from the C types (src/wasm_emitter/import_manager.rs:85 `emit_ffi_imports`).
  No custom section carries the C types. The runtime reads the **headers again** at link time
  (src/ffi/link.rs:168 `link_module_libraries`, :129 `link_dynamic_library`).
- Executables (`warp build`) carry the descriptors in front of their machine code instead: magic `WRPCcfun`, then a
  length, then one tab-separated `CFunction::line` per function (c_signatures.rs:128 struct, :164 `line`, :204
  `with_c_functions`). warp-runtime links them without headers (c_calls.rs `link_carried_function`,
  standalone.rs:187 `run_module`).

### How a call crosses (crates/warp-runtime/src/c_calls.rs, notes/ffi_handles.md)
- `link_c_function` (:214) is one `Linker::func_new` per function. `native_arguments` (:324) fills **8 integer and 8
  float registers**. `call_native_function` (:531) transmutes the pointer to
  `extern "C" fn(u64×8, f64×8) -> u64|f64`. **No libffi** (it is not in the cargo registry either [M]).
- Types: i32, i64, f32, f64. A `char*` parameter is an i32 offset into linear memory (`Ptr` → `memory.data_ptr()+off`,
  NUL-terminated copy made by the emitter). A `char*` result is copied into a warp text built through the module's
  exported `new_text` (`text_node` :307). A struct pointer becomes a **handle id** (CHandles :492, 0 = NULL). A `T**`
  is an out-pointer that becomes the result. `const uint8_t*, size_t` is a text or list (anyref). A struct with a
  known layout crosses as a map.
- dlopen: `get_or_load_library` (:120) uses the process itself for `c`. Otherwise it tries `library_paths` (:137):
  /opt/homebrew/lib, /usr/local/lib, `lib<name>.dylib` on macOS; /usr/lib, /usr/local/lib, `lib<name>.so`,
  `lib<name>.so.6` on Linux.
- Limits [I from the code]: at most 8 int and 8 float args. **No variadics**: on Apple arm64, variadic arguments go on
  the stack, so `printf` through this caller is wrong. Small structs by value pass only as i32 (raylib Color). No
  callbacks: a function pointer crosses as `void*`. Library calls that are not thread-safe (sdl, raylib) take a mutex.
- A window opener is refused under WARP_NO_WINDOW or CI (src/ffi/link.rs:8).
- Bonus: warp can already drive libwasmtime through this FFI (lib/wasmtime.warp, tests/ffi/test_wasmtime_ffi.warp).

### Shell
- `use os`: `exec(command)` runs an argv without a shell. `command(line)` runs `sh -c line` (lib/os.warp:12, :20 →
  `std_io("os", …)` → src/modules/std_adapters.rs:244-249 → `executed` :418). stdin is null. On status 0 the result
  is **stdout** (trailing newline trimmed [M]). On any other status it is an **Error** with the status and stderr:
  `Error("os.command: sh -c exit 3 failed (exit status: 3): ")` [M]. No exit code or stderr reaches a successful
  call. `sandbox::check_process` gates it.
- On pure: `command("echo hi | tr a-z A-Z")` → `"HI"` [M]. `exec("echo hi")` and `exec(["echo","hi"])` are refused
  ("outside the core") [M].
- The `host.run` import (src/host/native.rs:342) is unrelated to shell. It takes (ptr,len) of wasm bytes, runs them
  in a fresh plain Linker with no imports, and returns main's i64 (-1 on failure).

### Host imports of the compiler modules [M, parsed import sections]
| module | imports |
|---|---|
| compiler.wasm (1,759,355 B, built by Rust; sections include tag=13, so it uses exceptions) | host.fetch, fetch_within, read, warn, run, std_io; wasi fd_write. Custom sections warp.std_modules = `file,os`, warp.user_functions, name, warp.meta, warp.build |
| **compiler-self.wasm** (1,505,984 B) | **host.std_io, host.local_offset**; WASI p1 args_sizes_get, args_get, environ_sizes_get, environ_get, fd_prestat_get, fd_prestat_dir_name, path_open, fd_read, fd_close, fd_write |
| user module `use os; command("echo hi")` (23,213 B) | host.std_io only. Exports memory, trap_detail, main, _start, new_empty/int/bool/float/codepoint/text/symbol/key/list/type, error_of, returned_error |
| `print "hi"` (1,085 B) | wasi fd_write |
| `import abs from "c"; abs(-3)` / `import floor from 'm'` | **no imports** (warp's own abs/floor) |

The linear-memory host words (fetch, read, warn: `(ptr,len) -> (ptr,len)`, the result written at the exported
`text_heap`) are at src/host/native.rs:248-283. `std_io(module, member, args)` takes and gives `$Node` anyrefs
(native.rs:846 `std_call` → `std_adapters::call`). notes/standalone.md: compiler-self.wasm runs under plain
`wasmtime run -W gc=y,function-references=y,exceptions=y`. Under plain WASI, `kept`/`keep` keep nothing, so every
run is cold (it lowers runtime.warp anew each time).

## 2. The pure pipeline and FFI
- Imports in the emitter: `module.imports = if "print" in reached then [fd_write_import] else []`. After that only
  the host words of core/host_words.warp that the program reaches (emitter.warp:1100-1103; type encoding as
  raw bytes, :47). There is **no path for a library import**. `is_import` statements are dropped
  (emitter.warp:251-252: "the Rust pipeline brought the module in already").
- builtin_signatures.warp:153 reads lib/math.warp's `from 'm'` lines only for result kinds. library_calls.warp:13
  knows the libraries `warp`, `emitter` and `'m'` for typed lines.
- Pure results via `warp-rust test <one-file folder>` (pure_tests.rs; note that `warp test <file.warp>` takes the
  **Rust** path) [M]:

| file | pure failures |
|---|---|
| test_ffi.warp | 56 of 66 (the 10 passes are sqrt, abs, floor computed by warp, or bare import lines) |
| test_ffi.pairs.warp | 15 of 15 |
| test_libc_results 12/12, test_ffi_handles 6/6, test_ffi_signatures 15/17, test_ffi_float_results 3/3, test_libm_linking 1/1, test_foreign_lines 1/1, test_ffi_text_results.pairs 5/5, test_libm_header_functions.pairs 5/5, test_ffi_gaps.pairs 12/12, test_wasi.pairs 4/4, test_host_words 1/1 | all fail |

  The refusals across test_ffi, test_ffi.pairs and test_libc_results are mostly "the call X(…)": fabs 11, strcmp 9,
  atoi 8, ceil 7, strlen 6, fmin 5, fmax 5, sin 4, atol 3, …. Others are "unknown name rand / SDL_Quit".
  `floor(3.7)` gives Int 3 on pure, where the test expects 3.0.
- By contrast, through the Rust path (`warp test tests/ffi/test_ffi.warp`), 1 of 66 fails (the SDL2 header is
  missing on the box) [M].

## 3. Runtime embedded today
- wasmtime **49.0.1** (Cargo.toml:102), with features anyhow, cranelift, gc, gc-copying, runtime, wat, demangle,
  addr2line, incremental-cache and parallel-compilation, plus wasmtime-wasi 49.0.1. libloading 0.9 is optional, in
  feature `native`. The `native` feature also pulls ureq, wgpu, winit, cpal, midir, tiny_http, tungstenite,
  wit-component, rustyline and more (Cargo.toml:51).
- warp-runtime crate: wasmtime 49.0.1 with `anyhow, call-hook, gc, gc-copying, runtime, std` and **no Cranelift**.
  Its feature `compiler` adds `wasmtime/cranelift`. Other dependencies: libloading 0.9, miniz_oxide, serde_json, and
  libc on unix. Engine: `deterministic_config` (engine.rs:21) with wasm_gc, function_references, a 1 GiB GC heap
  reservation, and stack sizing.
- Sizes [M]:

| binary | bytes |
|---|---|
| ~/dev/bin/warp (Mac, Rust, full) | 47,035,728 |
| target/release/warp (box Linux x86-64, strip, LTO) | 41,937,416 |
| target/debug/warp (box) | 253,017,128 |
| warp-runtime release, Mac arm64, no Cranelift | 1,220,672 |
| warp-runtime quick-release, Mac | 1,859,520 |
| libwarp_runtime.dylib release (Cranelift + C ABI), Mac | 6,233,856 |
| Homebrew libwasmtime.dylib 50.0.0 / libwasmtime.a | 21,146,736 / 31,080,088 |
| Homebrew libffi.8.dylib 3.8.0 | 120,576 |

  wasmtime's own docs give a release C-API without a compiler at 2.1 MB, down to about 700 KB (docs.wasmtime.dev
  examples-minimal). They say "no effort has yet been put into minimizing the code size of Cranelift".
- On the Mac [M]: arm64, macOS 27.0.1. Homebrew has libffi 3.8.0, raylib 6.2, SDL2, sqlite 3.54, and wasmtime 50.0.0
  (its headers have `wasmtime_structref_field` and `wasmtime_anyref_as_struct`). The SDK has `usr/include/ffi/ffi.h`.
  The wasmtime CLI is 49.0.2.

## 4. Design options

Constraints from the modules themselves [M]:
- Both compilers and user programs are **WasmGC**: `$Node {kind i64, data anyref, value ref null $Node}`.
- compiler.wasm uses **exceptions** (tag section).
- This rules out **wasm3 and wasmi**, which have no GC. **WAMR** is also unsuitable: its own docs mark GC "not fully
  compliant… nested structs and arrays are not fully supported" and exceptions are legacy and interpreter-only.
  A 2026-03 WAMR commit even "prevent[s] usage of anyref in struct fields", and `$Node.data` is exactly that.
  wasmtime is the only practical engine.

### (a) A small Rust binary on warp-runtime (recommended)
- What: a new `[[bin]] warp-host` in crates/warp-runtime with `features = ["compiler"]`.
  - main: `warp-host <module.wasm|.cwasm> [-- args]`, plus `warp-host run f.warp [-- args]`, which runs the compiler
    and then the program in one process.
  - It links WASI p1 (wasmtime-wasi's p1 adapter is the least code; a hand-written set of 10 calls is the smaller
    option), host.std_io for a small set (os.kept/keep/args/env/cwd/exec/command/input/stdin), and host.local_offset.
  - The existing pieces: host_words (sleep, random, clock, exit, signals…), libm, libc, output, c_calls.
  - Unknown imports resolve generically: for every import whose module is not host or WASI, look up its descriptor
    in the `warp.c_functions` custom section, or fall back to the wasm type. Then dlopen the library and dlsym the
    field, and call it through `link_c_function`.
- Size [I]: about 6–7 MB. libwarp_runtime.dylib, the same crate with Cranelift, is 6.2 MB measured. WASI p1 adds
  some, and a hand-written p1 adds little.
- Code [I]: about 400–700 new Rust lines, mostly main, a WASI p1 subset or the adapter, std_io dispatch, kept/keep
  as a file cache, and the section reader. Everything else is reused.
- Pros: reuses tested marshalling (handles, structs, texts, failures) and one engine config. It is portable wherever
  warp-runtime builds (macOS and Linux today). It keeps warp-rust (src/) out of the path.
- Cons: it is Rust, so it needs Supervisor's OK. Cranelift dominates the size. Each run compiles the 1.5 MB
  compiler, so the host needs a .cwasm cache: `Module::serialize`/`deserialize` keyed by sha256, about 50 lines; the
  main crate has src/run/module_cache.rs.

### (b) A C host (wasmtime C API + libffi)
- What: about 800–1200 lines of C [I]: WASI through `wasmtime_linker_define_wasi`, std_io, and a Node reader through
  wasmtime_structref_field (C API ≥ the 2026-04 commit, present in Homebrew 50.0.0 [M]). Import resolution with
  dlopen/dlsym/ffi_call.
- Size: it links libwasmtime (21 MB dylib, or a static .a with gc-sections to an unmeasured size) plus libffi
  (120 KB).
- Pros: no Rust, and libffi from day one (variadics, closures for callbacks).
- Cons: a second implementation of every marshalling rule (texts via new_text, handles, structs, failures,
  exit/trap_detail). The C API's GC surface is new. Not smaller, since Cranelift sits inside libwasmtime either way.
  It is a new language and build in the repo (cc, pkg-config for Homebrew vs distro paths).

### (c) Extend only the existing warp-rust host
- What: no new binary. The pure emitter emits `c.strlen` and similar imports, and warp-rust's run path already links
  them (wasm_reader.rs:181/219 → link_module_libraries reads the headers).
- Size: stays 47 MB with the frozen compiler inside, which fails "very small".
- Pros: the **fastest way to turn FFI tests green**. It may need no Rust at all if the emitter emits exactly the
  header-derived types: strlen (i32)->i64 with the text as a NUL-terminated i32 offset; fmin (f64,f64)->f64.
- Cons: it does nothing for size, and the launcher keeps depending on warp-rust.

### Memory and string marshalling (proposed for (a), compatible with today's Rust)
- Numbers: i32, i64, f32 and f64 as wasm. A float goes to the float registers, in order.
- Text parameter: either (1) today's `ptr` convention, an i32 offset of a **NUL-terminated copy** written by the
  emitter into linear memory, or (2) **new `text` param kind = anyref $Node text**, which the host reads with
  gc_values and copies to a CString for the call. (2) is simpler for the pure emitter, which then needs no NUL-copy
  code, and safer: no stale pointer after memory.grow. It is one new ParamType in c_signatures.rs/c_calls.rs.
- Text result: copied at once into a new warp text through the exported `new_text` (existing `text_node`). NULL → ø.
- Pointers: an opaque C pointer is a handle id (i32, 0 = NULL, per run) as today. A raw pointer into linear memory is
  valid only during the call (base+offset, which can move on grow). Out-pointers stay as today.
- Signature source for pure:
  - (i) explicit foreign lines in warp, e.g. a new lib/c.warp with `strlen(s: text): int from 'c'`, emitted as
    import + `warp.c_functions` line.
  - (ii) a compile-time host word that answers a CFunction line from the headers. That needs the header parser in
    the host, which would move about 1000 Rust lines from src/ffi into warp-runtime.
  - (iii) a port of the header parser to warp.
  - Recommendation: (i) now, (iii) later. (ii) only if Supervisor accepts more Rust.
- Fallback without a descriptor: from the wasm type alone (i32 → int, i64 → long/size_t, f64 → double, anyref → text).
  That is enough for libm and strlen-style calls.

### Shell exec API shape
- Keep `os.exec(argv)` and `os.command(line)` as they are (stdout text, Error on failure), served by the host's std_io.
- Add one structured word for programs that need the exit code, e.g. `os.run(argv)` → `{status, out, err}`. The name
  and shape are open. Pipes and stdin input come later.

### How the launcher calls the host
- packaging/bin/warp `pure()`: `"$HOST" run "$source" -- args` (one process), or the two steps:
  `"$HOST" compiler-self.wasm -- src out.wasm folder lib` then `"$HOST" out.wasm -- args`.
- `find_host` mirrors `find_rust`: WARP_HOST, else `warp-host` beside the script. `WARP_PIPELINE=rust` keeps warp-rust.
- The refusal and value-status logic stays as it is. The host must print the value the way warp-rust does (Int →
  exit status, Error → stdout line plus exit 1) or the launcher's `refused` check changes. gc_values `json_of`
  output differs from warp serialization (open question).

### Portability
- macOS arm64 and Linux x86-64 and aarch64: the 8+8 register caller is right for non-variadic SysV and AAPCS64
  calls. Apple arm64 variadics need libffi (`ffi_prep_cif_var`).
- libffi is in the macOS SDK (ffi.h, and the system libffi in the dyld cache) and in Homebrew 3.8.0 [M]. On Linux
  it is libffi.so.8 (assumed present, not checked here).
- The Rust `libffi` crate is not in the offline registry [M]. Adding it needs a network fetch, or a dozen lines of
  `extern "C"` declarations against the system libffi.
- Library search: extend `library_paths` with `DYLD_FALLBACK_LIBRARY_PATH`/`LD_LIBRARY_PATH` and versioned sonames
  (`libSDL2-2.0.so.0`) [I].
- Windows: not covered (library_paths has only `<name>.dll`).

## 5. Recommended plan

### Why Rust qualifies
The host is not the old compiler. It is the runtime that the pure pipeline cannot run without: wasm itself cannot
dlopen or exec. Today that runtime is the 47 MB warp-rust carrying the frozen compiler. warp-runtime is already the
runtime layer: it is separate from src/, compiles without the compiler, and is shared by `warp build`. This work
only adds to it.

### Files
- crates/warp-runtime/Cargo.toml: `[[bin]] warp-host` (required-features `compiler`).
- crates/warp-runtime/src/host_main.rs (new): arguments, engine, cwasm cache, run compiler then program, print the
  value, exit status.
- crates/warp-runtime/src/wasi_p1.rs (new): the 10 calls compiler-self.wasm imports, or the wasmtime-wasi p1 adapter
  behind a feature.
- crates/warp-runtime/src/std_io.rs (new): the os subset; kept/keep as files under ~/.cache/warp/kept/<sha>.
- crates/warp-runtime/src/c_imports.rs (new): read the `warp.c_functions` section → CFunction → existing
  `link_carried_function`. Fallback from the wasm type.
- crates/warp-runtime/src/c_calls.rs (small change): an optional `text` ParamType (anyref → CString). Later, the
  libffi path for variadics and callbacks.
- packaging/bin/warp + notes/pure_wrapper.md: WARP_HOST, routing to warp-host.
- lib/c.warp (new, warp): foreign lines for common libc. Not Claude-owned.
- **Claude-owned (hand over, do not edit):** core/emitter.warp to emit imports for `import f from lib` and
  foreign lines, plus the `warp.c_functions` section. The import table already takes `[module, name, type]`.

### Milestones
- **M0, measure (no code in the repo):**
  - Build warp-runtime with `--features compiler` once and record its size.
  - Time compiler-self.wasm under the Homebrew wasmtime CLI 49.0.2 on the Mac: cold, and warm with its cache.
  - Confirm the exceptions flag in `deterministic_config` (engine.rs:21 sets no wasm_exceptions; check whether
    wasmtime 49 enables it by default for GC).
- **M1, warp-host runs pure programs:**
  - WASI p1, std_io (args, env, kept/keep, command, exec), local_offset, cwasm cache.
  - `warp-host run hello.warp` matches `warp-rust pure hello.warp`.
  - No FFI yet. Launcher under `WARP_HOST=1`.
- **M2, FFI from descriptors, with no emitter dependency:**
  - Test by hand-writing a .wat/.wasm (or warp `use wasm`) with `(import "c" "strlen")`, `(import "m" "sqrt")` and a
    `warp.c_functions` section.
  - Run it through warp-host. Covers libc strlen/atoi, libm sqrt/hypot, sqlite3 handles.
- **M3, the pure emitter emits C imports (Claude card):**
  - `import f from "c"/'m'` and foreign lines → imports + section.
  - Then the tests/ffi pairs go green on both warp-rust (c) and warp-host (a).
  - Count the green tests: today 10 of 66, and 0 of 15 for test_ffi.pairs.
- **M4, libffi:**
  - Variadics (printf on Apple arm64), more than 8 arguments, small structs by value.
  - **Callbacks**: an `ffi_closure` trampoline that calls a wasm funcref through the store (qsort, SDL event filters).
- **M5, packaging:** release archives and the brew formula ship warp-host next to or instead of warp-rust, after
  decision launcher-not-yet lifts.

### Test strategy (outcome tests only)
- tests/host/test_host.sh, run as `tests/queue.sh tests/host/test_host.sh`, same style as
  tests/launcher/test_launcher.sh: expected stdout and exit status.
  - `warp-host run` of `print "hi"` → `hi`.
  - `use os; command("echo hi")` → `"hi"`.
  - `command("exit 3")` → Error and status 1.
  - The M2 .wasm calling `c.strlen("hello")` → 5 and `m.sqrt(16.0)` → 4.0.
  - sqlite3 `select 1+2` → 3 (skipped where libsqlite3 is missing).
  - A missing library or symbol → a loud error naming both.
- After M3, the existing tests/ffi/*.pairs.warp and test_*.warp run through `warp test` (pure), plus a sweep lane
  through warp-host. Success is the pass count, not Rust parity.

### Open questions for Supervisor
1. Is Rust acceptable here (a new bin in crates/warp-runtime) as an "urgent pure-pipeline need"? Or does it have to
   be C (option b)?
2. Is crates/warp-runtime/src/standalone.rs (the `warp build` stub) inside the Claude-owned "standalone" area, or
   only src/compiler/standalone.rs?
3. Which compiler does the host run: compiler-self.wasm (WASI p1, 2 host imports, needs kept/keep for warm runs) or
   compiler.wasm (needs host.read/fetch/warn/run/std_io file+os)? I recommend compiler-self.
4. Text parameter convention: the existing i32 NUL-terminated pointer in linear memory, or the new anyref `text`
   kind that the host copies? I recommend anyref.
5. Signature source for pure: foreign lines in warp (lib/c.warp) now and a warp port of the header parser later, or
   move the Rust header parser into the host?
6. Exit codes from the shell: add `os.run(argv)` → `{status, out, err}`? What name and shape?
7. Dependencies: is libffi OK for M4 (system or Homebrew; the crate needs a network fetch)? Is wasmtime-wasi OK, or a
   hand-written WASI p1 subset to stay small?
8. Size budget: is about 6–7 MB with Cranelift "very small" enough? Or should we evaluate Winch, or ship
   compiler-self as a .cwasm (the user's programs still need a compiler)?
9. Printing the value: the host has to match warp-rust's value and exit-status contract (the launcher's `refused`
   and `is_value_status` depend on it). Is gc_values' JSON-style printing acceptable?
