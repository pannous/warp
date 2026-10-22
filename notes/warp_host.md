# warp-host: the small native host of pure programs (card warp-host)

wasm can't run shell commands or load C libraries, so warp programs get them from a host. warp-host is that host, kept
small: it compiles a program with the warp compiler written in warp, then runs the module with WASI and the host words.
It is no warp-rust: there is no interpreter or Rust compiler in it, and it doesn't aim for parity. Only green tests
count.

    warp-host [--compiler self|full] run <file.warp | code> [args…]   compile and run; the run's value printed
    warp-host [--compiler self|full] compile <file.warp> [<file.wasm>]
    warp-host <file.wasm | file.wat> [args…]

- Source: crates/warp-runtime/src/warp_host/
  - main.rs: commands, finding lib/ and the compiler, running a module
  - words.rs: std_io, and compiler.wasm's host.read/fetch/warn
  - nodes.rs: GC nodes in and out
  - cache.rs: kept values and machine code
  - ffi.rs: C imports
- It reuses warp-runtime's engine settings, host words, libm and libc linking. Feature `host` (= `compiler` + wasmtime-wasi
  p1). Build: `cargo build --release -p warp-runtime --features host --bin warp-host`.
- Compiler: compiler-self.wasm by default; `--compiler full` uses compiler.wasm (built by warp-rust). It is looked for in
  `<lib>/../data/` (a checkout built by `warp rebuild`), then in `<bin>/../share/warp/`, then beside warp-host.
  - lib/ is found the way the launcher finds it: WARP_LIB, else the nearest lib/ holding compiler/ above the source or
    the current folder, else the installed one.
  - compiler-self reads std modules (`use os`) from the lib/ it was built from (its module_directory), not from the lib/
    it is given. So a new std word such as os.run needs a `warp rebuild self`.
- Imports linked:
  - WASI p1, with stdio, the environment and args inherited, and preopens `/` and `.`
  - warp-runtime's host words (local_offset, clock, random, sleep_ms, exit, …), libm, and libc under `c`
  - host.std_io; for compiler.wasm also host.read, fetch and fetch_within (by curl), and warn
  - any other import traps when called
- std_io:
  - os: env, args, cwd, input, stdin, kept, keep, command, exec, run
  - file: read, write, append, exists, read_bytes, write_bytes
  - A failure answers the program an Error with the same text as warp-rust's std_adapters.rs, e.g. `os.command: sh -c
    exit 3 failed (exit status: 3): …`.
  - `run(argv)` (lib/os.warp) gives `{status, out, err}` with trailing newlines dropped. A failing command is no error
    (killed by a signal: status 128 + signal). warp-rust has no os.run yet.
  - There's no sandbox: warp-host is a trusted native runner.
  - `exec(…)` is refused by the compiler at present ("outside the core: an error value").
- Output: a run's value is printed and decides the status. ø prints nothing; an Int is the status; an Error prints
  `Error("…")` and exits 1. A compile's failure is its only stdout line `Error("…")`, status 1, which is what the
  launcher's `refused` expects. A runtime trap prints `warp-host: <message>` on stderr (the failing runtime function's
  name and trap_detail), status 1; WARP_HOST_DEBUG=1 adds the wasm backtrace.
- Cache (WARP_HOST_CACHE, else $XDG_CACHE_HOME/warp-host, else ~/.cache/warp-host):
  - A compiler module's machine code (`<hash>.cwasm`, ~18 MB for compiler-self, keyed by its bytes and the engine's
    compatibility hash).
  - Kept values (`<hash>.kept`, keyed by the compiler's bytes, lib/'s file fingerprint and the name). compiler.warp's
    lowered runtime words are ~380 KB.
  - Values with a big integer, a ratio or a function aren't kept. compiler.wasm's runtime words hold such an Int, so
    `--compiler full` lowers them on every run (~8 s).
- Texts reach the program through its own `new_text(ptr, len)`, which copies them out of linear memory. compiler-self
  exports no text_heap, so the host writes into pages past the program's memory and reuses them while the memory size
  stays the same.

## C calls (M2)

An import whose module isn't `host` or WASI is a C function: `name` from the library `module`.
- How the library is found:
  - `c` is the process itself (libc, macOS libSystem).
  - Other names go through c_calls.rs library_paths. Linux tries /usr/lib, /usr/local/lib, then lib<name>.so,
    .so.0 and .so.6 (libm.so.6, libsqlite3.so.0). macOS tries Homebrew, /usr/local/lib, then lib<name>.dylib.
  - The library is loaded with dlopen and the function found with dlsym.
- How each parameter and the result cross:
  - The module's `warp.c_functions` custom section gives one c_signatures.rs CFunction line per function, with nine
    tab-separated fields: module, library, name, params, ret, result, out pointee, layouts, failure. Example:
    `sqlite3 sqlite3 sqlite3_prepare_v2 handle,ptr,i32,out,out i32 handle sqlite3_stmt` (tabs between).
  - Without a line there, the import's own type decides:
    - numbers stay numbers
    - a reference parameter is a text
    - a reference result is a `char *` read back as a text
  - An import that warp-host's own words already define with the same type keeps that definition (libm's sqrt, the
    hand-linked libc). An import described in the section always goes to dlsym.
- Texts: a `ptr` parameter the import declares as a reference takes a warp text node. The host copies it NUL-terminated
  for the call (c_calls.rs native_arguments), and ø passes NULL. An i32 `ptr` is still an address in linear memory.
- Results:
  - Handles cross as ids into the run's CHandles.
  - A `text` result is built with the module's new_text and needs its text_heap; pure modules don't export text_heap
    yet (M3).
- c_calls.rs marshals the call (link_c_function_as, with the import's wasm type).
- A missing library or symbol fails the run before it starts, status 1:
  - `warp-host: the C library nosuchlib of the import nosuchlib.nosuch_function is not found (tried …)`
  - `warp-host: the C library c has no function no_such_c_function`
- lib/c.warp has the declarations as `strlen(s: text): i64 from 'c'` lines, for M3 (card pure-c-imports, the pure
  emitter, Claude's).
  - Their types are the warp.c_functions names: i32 i64 f32 f64 text handle out void.
  - warp-rust's `warp parse lib/c.warp` reads them. Like lib/math.warp, `abs` and `sqrt` parse as the operators ‖ and √.
- The pure emitter doesn't emit C imports yet, so tests/host/test_host.sh checks C calls with hand-written WAT modules.
  warp-host runs `.wat` files (the wat crate).
- Measured on the box (Linux, libsqlite3.so.0 3.x present, no dev package):
  - strlen("hello") gives 5 (exit status 5, the Int's)
  - sqrt(16) gives 4.0
  - hypot(3, 4) by dlsym gives 5.0
  - sqlite3_open(":memory:"), prepare_v2 "select 1+2", step, column_int give 3
- The macOS paths are not tested here.

## M0 (measured 2026-10-22 on the box: Linux x86_64, 8 cores; warp-host release of this branch)

- Binary size: **11,250,680 B** (11.3 MB), stripped, lto.
  - For comparison, the warp-runtime stub (no Cranelift, no WASI) is 1,573,240 B in the same build.
  - So Cranelift, the component model and async that wasmtime-wasi p1 pulls in, and WASI add ~9.7 MB on Linux. The
    split between them is not measured.
- Engine: warp-runtime's deterministic_config, with GC and function references on. Exceptions are on too, because
  wasmtime 49 enables them whenever its `gc` feature is (config.rs:2555); compiler-self.wasm has a tag section and runs.
  The wasm stack is 240 MB on a 256 MB thread.
- Timings: wall clock, one run each unless noted, release build.

  | run | time |
  |---|---|
  | `run hello.warp`, empty cache (Cranelift compile + lowering the runtime words) | 29.4 s |
  | the same, machine code cached but no kept values | 22.1 s |
  | the same, kept values but no machine code (Cranelift compiling compiler-self.wasm) | 7.3 s |
  | `run hello.warp`, warm | 0.12–0.15 s (3 runs) |
  | `run '6*7'`, warm | 0.11 s |
  | `compile hello.warp hello.wasm` (1,685 B), warm | 0.12–0.14 s |
  | `hello.wasm` | 6 ms |
  | a program with `use os` and two commands (sh.warp), warm | 0.64 s |
  | `--compiler full run hello.warp` (nothing kept) | 8.4–11.1 s |
- Without Cranelift (not built, inferred): warp-host could ship compiler-self as a `.cwasm` for its target. That would
  drop Cranelift and bring the binary toward the stub's ~1.6 MB plus WASI.
  - wasmtime-wasi p1 still requires the component model and async features, so that part stays unless WASI p1 is
    hand-linked like output.rs's fd_write.
  - Programs would need precompiling too, which can't happen without a compiler. So that build could only run programs
    compiled elsewhere. Not worth it while 11 MB is fine.

## Tests

`tests/queue.sh tests/host/test_host.sh`: outcome checks of warp-host and of the launcher with warp-host beside it.
- WARP_HOST_BINARY names the host (default target/release/warp-host, else target/debug/).
- The compilers come from beside WARP_LAUNCHER_RUST (default target/debug/warp, after `warp rebuild`).
- The cache is scratch/host-cache/. The first run computes it (~30 s); later runs take ~15 s.

After M2 (C calls, the wat crate, origin/main merged) the release binary is 12,441,720 B.

## Next

- M3 (card pure-c-imports): the pure emitter emits C imports and a warp.c_functions section, which warp-host dlopens.
- M4: libffi.
