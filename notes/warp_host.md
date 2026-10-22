# warp-host: the small native host of pure programs (card warp-host)

wasm can't run shell commands or load C libraries, so warp programs get them from a host. warp-host is that host, kept
small: it compiles a program with the warp compiler written in warp, then runs the module with WASI and the host words.
It is no warp-rust: there is no interpreter or Rust compiler in it, and it doesn't aim for parity. Only green tests
count.

    warp-host [--compiler self|full] run <file.warp | code> [args…]   compile and run; the run's value printed
    warp-host [--compiler self|full] compile <file.warp> [<file.wasm>]
    warp-host <file.wasm> [args…]

- Source: crates/warp-runtime/src/warp_host/
  - main.rs: commands, finding lib/ and the compiler, running a module
  - words.rs: std_io, and compiler.wasm's host.read/fetch/warn
  - nodes.rs: GC nodes in and out
  - cache.rs: kept values and machine code
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

## Next

- M2: texts as host-copied warp text for C calls, and `from 'c'` declarations in lib/c.warp.
- M3 (card pure-c-imports): the pure emitter emits C imports and a warp.c_functions section, which warp-host dlopens.
- M4: libffi.
