# warp-host: the small native host of pure programs (card warp-host)

wasm can't run shell commands or load C libraries, so warp programs get them from a host. warp-host is that host, kept
small: it compiles a program with the warp compiler written in warp, then runs the module with WASI and the host words.
It is no warp-rust: there is no interpreter or Rust compiler in it, and it doesn't aim for parity. Only green tests
count.

    warp-host [--compiler self|full] [run] <file.warp | code> [args…]   compile and run; the run's value printed
    warp-host [--compiler self|full] compile [--aot] <file.warp> [<file.wasm>]
    warp-host [--compiler self|full] build <file.warp | file.wasm> [-o <app>]
    warp-host <file.wasm | file.wat | file.cwasm> [args…]
    warp-host help | --help | -h                                         the usage, status 0

- No command means run, as with warp: the first word after the flags decides.
  - `run`, `compile`, `help`: that command.
  - A name ending in .wasm or .wat, or a file starting with wasm's magic bytes `\0asm`: the module runs.
  - Anything else is `run`: `warp-host f.warp a b` is `warp-host run f.warp a b`, `warp-host '6*7'` is `run '6*7'`.
- File or code, as warp decides (src/main.rs program_file, src/cli/formatting.rs has_warp_extension):
  - A name with a .warp or .wasp extension is a file. When it's missing, that's the error
    `warp-host: could not read file x.warp` with status 1; it never compiles as code.
  - Any other existing file (an extensionless `#!` script) is a file too.
  - Else the word is code, a program of the current folder.
- Unlike warp, which joins all its words into the code (`warp 6 '*' 7`), warp-host's code is the first word only. The
  words after it are the program's arguments (`use os; args`), as after a file.
- The launcher packaging/bin/warp still passes `run` explicitly; both forms are the same run.

- Source: crates/warp-runtime/src/warp_host/
  - main.rs: commands, finding lib/ and the compiler, running a module
  - words.rs: std_io, and warp.wasm's host.read/fetch/warn
  - shell.rs: std_io's words of lib/shell.warp (cd, mkdir, copy, move, touch, remove …)
  - nodes.rs: GC nodes in and out
  - cache.rs: kept values and machine code
  - ffi.rs: C imports
- It reuses warp-runtime's engine settings, host words, libm and libc linking. Feature `host` (= `compiler`, parallel
  compilation, wasmtime-wasi p1); feature `wat` adds WAT text. Build: `cargo build --profile host -p warp-runtime
  --features host --bin warp-host` (target/host/warp-host, 6,499,736 B; see Size).
- Compiler: warp-self.wasm by default; `--compiler full` uses warp.wasm (built by warp-rust). It is looked for
  beside warp-host first (its own folder, a symlink to it followed), then in `<lib>/../data/` (a checkout built by
  `warp rebuild`), then in `<bin>/../share/warp/`.
  - The build puts both beside the binary (card warp-host-compiler-beside): crates/warp-runtime/build.rs, with the
    feature host, copies them from WARP_HOST_COMPILERS (a folder), else the checkout's data/, to target/<profile>/
    (a cargo warning when they are missing). Test: tests/host/test_host_compiler_beside.sh.
  - lib/ is found the way the launcher finds it: WARP_LIB, else the nearest lib/ holding compiler/ above the source or
    the current folder, else the installed one.
  - warp-self reads std modules (`use os`) from the lib/ it was built from (its module_directory), not from the lib/
    it is given. So a new std word such as os.run needs a `warp rebuild self`.
- Imports linked:
  - WASI p1, with stdio, the environment and args inherited, and preopens `/` and `.`
  - warp-runtime's host words (local_offset, clock, random, sleep_ms, exit, …), libm, and libc under `c`
  - host.std_io; for warp.wasm also host.read, fetch and fetch_within (by curl), and warn
  - any other import traps when called
- std_io:
  - os: env, args, cwd, input, stdin, kept, keep, command, exec, run, run_wasm, compile_wasm
  - file: read, write, append, exists, read_bytes, write_bytes
  - the words of lib/shell.warp (card unix-words, shell.rs), as warp-rust's src/modules/shell_adapters.rs:
    - os: cd (the process's working directory; its new one), which (ø when no PATH folder has it), open (open on
      macOS, xdg-open elsewhere)
    - file: list (lib/file.warp list_files: names, sorted), glob, mkdir (with the folders above), copy and move (into
      a folder keeping the name, else to the path), touch, remove (to the Trash: ~/.Trash on macOS, else
      ~/.local/share/Trash with a .trashinfo; never a pattern, a full folder, the root, home, or the working directory
      and the folders above it), find, size (du's bytes), info ({size, modified in ms, kind})
    - A failure names the path: `file.list: nosuch: No such file or directory (os error 2)`.
  - The file words take `~` and `~/…` as the home folder. A relative path is in the process's working directory, so
    after `cd` they follow it, as commands do.
  - warp-self's output calls them through std_io, so no `warp rebuild self` is needed for them.
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
- Cache (WARP_HOST_CACHE, else $XDG_CACHE_HOME/warp-host, else ~/.cache/warp-host), and a shipped one read after it,
  never written: WARP_HOST_SHIPPED_CACHE, else `<bin>/../share/warp/host-cache` or `<bin>/host-cache`:
  - A compiler module's machine code (`<hash>.cwasm`, ~18 MB for warp-self, keyed by its bytes and the engine's
    compatibility hash).
  - Kept values (`<hash>.kept`, keyed by the compiler's bytes, lib/'s files (paths within lib/ and contents) and the
    name with lib/'s path taken out (compiler.warp names them "… runtime words of <lib>"), so the key is the same on
    every install). compiler.warp's lowered runtime words are ~380 KB.
  - The machine code of modules run by run_wasm, keyed by their bytes, and kept in memory for the rest of the run too.
  - WARP_HOST_DEBUG=1 prints the compiler, lib/ and kept-values key of a compile.
  - Values with a big integer, a ratio or a function aren't kept. warp.wasm's runtime words hold such an Int, so
    `--compiler full` lowers them on every run (~8 s).
- Texts reach the program through its own `new_text(ptr, len)`, which copies them out of linear memory. warp-self
  exports no text_heap, so the host writes into pages past the program's memory and reuses them while the memory size
  stays the same.

## run_wasm (card warp-host-run-wasm)

`use os` (lib/os.warp):
- `run_wasm(bytes)`: a module's bytes run inside the program's warp-host, its main's value (else _start's; ø).
  - bytes: a list of bytes (read_bytes, compile_wasm) or a text holding them (read).
  - The same engine, and the same imports as a top-level module: WASI with the same stdout, std_io, C calls. A store
    and instance of its own, on a thread of its own with the full wasm stack. The args are the program's.
  - Machine code from the cache, keyed by the bytes: a second run is warm (in memory, else the .cwasm).
  - `run_wasm(bytes, ["a", "b"])`: the module's own arguments (its `args`), not the program's.
  - An uncaught Warp error of the module (a raise, a runtime error such as index out of range) is that Error, passed
    through unchanged: `try run_wasm(compile_wasm("raise \"bad\"")) catch e` gives `bad`.
  - A trap is an Error of its own, `os.run_wasm: wasm trap: wasm `unreachable` instruction executed`, catchable with try. A
    nonzero exit is an Error too, `the module exited with status 3`; exit 0 gives ø.
  - The value is copied out of the module's store (nodes.rs); a function or big integer comes back as warp-host's
    Other, as with any run.
  - An exact rational (the pure lane's `2.5`, `1/3`: an Int node holding a $Ratio of two $i64box) comes back exact,
    nodes.rs Node::Ratio: it prints as the program does (2.5, 1/3) and crosses back into the calling program as the
    same structs, made by the host of the program's structural types (card host-run-wasm-float; it had failed
    `run wasm: 0`).
- `compile_wasm(code)`: the code, a program of the current folder, compiled in-process by warp-host's compiler
  (warp-self, or warp.wasm with `--compiler full`); its bytes, or the refusal as an Error.
  - `run_wasm(compile_wasm("6*7"))` gives 42: an eval.
- Both go through std_io like os.run, so warp-self's output calls them without an emitter change. The old
  `host.run(ptr, len) -> i64` of warp-rust (src/host/native.rs) passes bytes in linear memory and gives only an i64;
  pure modules have no such call, and std_io's node in, node out fits better. warp-rust has no os.run_wasm yet.
- Timings (box, warm, profile host; the program `use os`+`use file` itself takes ~0.85 s to compile and run):
  - in-process: 200 run_wasm of a module giving 42 add ~0.05 s, ~0.25 ms each
  - the shell route, write_bytes then `run([warp-host, "t.wasm"])`: 200 add ~0.7 s, ~3.5 ms each
  - a module never run before: its compile adds a few ms (one .cwasm written)
  - `run_wasm(compile_wasm("6*7"))`: ~0.85 s in all, of which compile_wasm is the bulk, as `warp-host run '6*7'`
    (0.12 s) plus `use os`

## use wasm: other modules and components (card warp-host-wasm-load)

`use wasm` (lib/wasm.warp), served by std_io module `wasm` (warp_host/wasm_modules.rs, components.rs):

    m = load_wasm("calc.wasm")          // a handle {wasm:1 path:"calc.wasm" kind:"module"}
    wasm_call(m, "twice", [21])         // 42; m may be the path too: wasm_call("calc.wasm", "twice", [21])
    wasm_exports(m)                     // {twice:"func(i64) -> i64" divmod:"func(i64, i64) -> (i64, i64)" …}
    c = load_wasm("rust_demo.wasm")     // a component, told by its header
    wasm_call(c, "stats_of", ["hello big world"])   // {words:3 letters:13}

- What load_wasm takes:
  - A path, loaded once per run: the same file is the same instance, so its globals and memory last the run.
  - Bytes (read_bytes, compile_wasm): a fresh instance each time.
  - A handle.
- Core modules:
  - Instantiated in the program's own store with the same engine and imports as a program: WASI, host words, std_io,
    C, other modules.
  - Values go by the export's own types:
    - Int/Bool to i32/i64 (an i32 takes the signed and unsigned range), Float/Int to f32/f64.
    - Results come back as Int or Float, several as a list, none as ø.
    - A reference result that is a warp node is read out of the callee's store with its own memory (a warp module's
      texts may live in its linear memory), so `wasm_call(load_wasm(compile_wasm("[1, 2]")), "main", [])` gives the
      list.
    - A reference argument is the program's own ref, passed as it is (copied through the callee's constructors when
      it exports them).
- Failures are Errors, catchable with try: a trap (`wasm.call: calc.wasm's boom failed: wasm trap: …`), a missing
  function (it names those there are), a wrong argument count or type.
- Imports: an import whose module names a .wasm or .wat file, `(import "deps/lib.wasm" "f" …)`, or a bare name with
  name.wasm/name.wat in reach (before C libraries), is linked to that module's exports.
  - The module is instantiated in the same store, once per run, whoever imports it. Its own imports are linked the
    same way.
  - Paths are relative to the working folder, then to the importing module's folder: the old pipeline's convention
    (notes/wasm_modules.md), so `import x.wasm` programs warp-rust emits run here too.
  - A missing module or an import cycle fails the run before it starts:
    `the import module nosuch.wasm: no module nosuch.wasm (looked in the working folder)`,
    `the modules import each other in a cycle: cycle_b.wasm → cycle_a.wasm → cycle_b.wasm`.
- Components: an engine of their own with the component model on (programs' engines are core-only), a store each,
  kept for the run.
  - Values go by the WIT types, through wasmtime's canonical ABI:
    - bool, s8 to u64 (range-checked), f32/f64, char (codepoint), string
    - list and tuple (a list), record (a map `{x: 3, y: -4}`), option (ø or the value)
    - enum (its name), variant (its name or {case: value}), flags (a list of names)
    - result: the ok value; the err is an Error
  - Names: `stats_of` finds `stats-of`, and `checked_sqrt` or `text_tools.checked_sqrt` finds a function of an
    exported interface.
  - Not yet: resources (handles), and WASI p2. A component's imports trap when called; rust_demo.wasm, built for
    wasm32-wasip2, still runs its pure functions.
- Size: wasmtime's component-model feature was already compiled in (feature compiler), but nothing used its runtime.
  Measured on the box with profile host:
  - before this card: 6,510,696 B
  - with core modules and imports: 6,674,280 B (+164 KB)
  - with components: 7,351,304 B (+677 KB more, +841 KB in all, +12.9 %)
- Compiler side, not done here:
  - `m.f(x)` for a handle (a sugar or lowering to wasm_call)
  - `import x.wasm` with its exports' types read at compile time
  - Foreign lines `twice(x: i64): i64 from 'calc.wasm'`, which the pure-c-imports emitter turns into the import this
    host already links.
  - A compiled warp module exports only main, so functions of other warp modules can't be called yet.

## Locale

warp-host sets LC_CTYPE from the environment at startup (`setlocale(LC_CTYPE, "")`, main.rs with_users_ctype), as a C
program would: the C libraries a program calls read UTF-8 then (libedit's readline crashed on a UTF-8 prompt in the C
locale). Only the character type: numbers keep printing with a point.

## C calls (M2)

An import whose module isn't `host` or WASI is a C function: `name` from the library `module`.
- How the library is found:
  - `c` is the process itself (libc, macOS libSystem).
  - Other names go through c_calls.rs library_paths. Linux tries /usr/lib, /usr/local/lib, then lib<name>.so,
    .so.0 and .so.6 (libm.so.6, libsqlite3.so.0). macOS tries Homebrew, /usr/local/lib, then lib<name>.dylib.
  - Linux also finds a library by any versioned soname in WARP_LIBRARY_PATH and the system's library folders
    (version_named_sonames): libedit.so.2 and libreadline.so.8 load without their -dev packages' lib<name>.so links
    (tests/host/test_host_repl_fixes.sh).
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
  - `owned_text` (card warp-host-repl-fixes): a `char *` the caller owns, strdup's or readline's, is copied into a
    text like `text` and then given to free(); NULL is ø, freed never. Spelled:
    - in a foreign line, a result type only: `readline(prompt: text): owned_text from 'edit'`
      (core/emitter/c_imports.warp)
    - in warp.c_functions, the result field `owned_text` (c_signatures.rs CPointer::OwnedText)
    - in warp.c_calls, the result letters `ft`, "free the text", beside `t` and `ot` (ffi.rs OWNED_TEXT_LETTERS)
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
- The pure emitter doesn't emit C imports yet, so tests/host/test_host.sh checks C calls with hand-written WAT modules,
  assembled by the tool wat2wasm (`cargo build --release -p warp-runtime --features wat --bin wat2wasm`). warp-host
  runs `.wat` files itself only with the feature wat.
- Measured on the box (Linux, libsqlite3.so.0 3.x present, no dev package):
  - strlen("hello") gives 5 (exit status 5, the Int's)
  - sqrt(16) gives 4.0
  - hypot(3, 4) by dlsym gives 5.0
  - sqlite3_open(":memory:"), prepare_v2 "select 1+2", step, column_int give 3
- The macOS paths are not tested here.

## M0 (measured 2026-10-22 on the box: Linux x86_64, 8 cores; warp-host release of this branch)

- Binary size: **11,250,680 B** (11.3 MB), stripped, lto (before card warp-host-size, see Size).
  - For comparison, the warp-runtime stub (no Cranelift, no WASI) is 1,573,240 B in the same build.
  - So Cranelift, the component model and async that wasmtime-wasi p1 pulls in, and WASI add ~9.7 MB on Linux. The
    split between them is not measured.
- Engine: warp-runtime's deterministic_config, with GC and function references on. Exceptions are on too, because
  wasmtime 49 enables them whenever its `gc` feature is (config.rs:2555); warp-self.wasm has a tag section and runs.
  The wasm stack is 240 MB on a 256 MB thread.
- Timings: wall clock, one run each unless noted, release build.

  | run | time |
  |---|---|
  | `run hello.warp`, empty cache (Cranelift compile + lowering the runtime words) | 29.4 s |
  | the same, machine code cached but no kept values | 22.1 s |
  | the same, kept values but no machine code (Cranelift compiling warp-self.wasm) | 7.3 s |
  | `run hello.warp`, warm | 0.12–0.15 s (3 runs) |
  | `run '6*7'`, warm | 0.11 s |
  | `compile hello.warp hello.wasm` (1,685 B), warm | 0.12–0.14 s |
  | `hello.wasm` | 6 ms |
  | a program with `use os` and two commands (sh.warp), warm | 0.64 s |
  | `--compiler full run hello.warp` (nothing kept) | 8.4–11.1 s |
- Without Cranelift: see Size, option (a).

## The lib/ of a compile (card compiler-rebuild-policy)

warp-host resolves lib/ as WARP_LIB, else the lib/ beside the binary, else the one above the program or the current
folder. It hands that lib/ to the compiler run as WARP_LIB, so a warp-self.wasm built anywhere reads the std
modules of that lib/. How agents get a compiler pair without building it: notes/compiler_rebuilds.md.

## Tests

`tests/queue.sh tests/host/test_host.sh`: outcome checks of warp-host and of the launcher with warp-host beside it.
- WARP_HOST_BINARY names the host (default target/host/warp-host, else target/release/, else target/debug/).
- The C modules are assembled by WARP_WAT2WASM, else target/release/wat2wasm, else wabt's wat2wasm on PATH.
- The compilers come from beside WARP_LAUNCHER_RUST (default target/debug/warp, after `warp rebuild`).
- The cache is data/host-cache/. The first run computes it (~30 s); later runs take ~15 s.

After M2 (C calls, the wat crate, origin/main merged) the release binary is 12,441,720 B.

## Size (card warp-host-size, measured 2026-10-22 on the box: Linux x86_64, 8 cores)

What the 11.4 MB release build holds (`cargo bloat --release --crates`, .text 9.0 MiB, unstripped; guesswork per crate):

| part | .text |
|---|---|
| Cranelift (cranelift_codegen 2.2, assembler_x64 1.7, wasmtime_internal_cranelift 0.8, regalloc2 0.4, frontend …) | ~5.2 MiB |
| wasmtime runtime, wasmparser, environ, gimli, object, pulley_interpreter | ~2.0 MiB |
| std | 0.85 MiB |
| WASI: wasmtime_wasi 0.27, tokio 0.13, cap-std and friends | ~0.5 MiB |
| warp_host 75 KiB, warp_runtime 61 KiB | 0.14 MiB |
| the wat crate (feature wat) | 1.05 MB of the file (12.44 → 11.39 MB) |

Options, each stripped; times are wall clock of `run hello.warp` (cold = empty cache), `run '6*7'` and the prebuilt
hello.wasm, warm, 2–3 runs. Host suite 29/29 and launcher suite 22/22 pass with the chosen one.

| option | size | cold | warm hello | warm 6*7 | hello.wasm | notes |
|---|---|---|---|---|---|---|
| M2 release (with wat) | 12.44 MB | ~29 s | 0.12–0.15 s | 0.11 s | 6 ms | |
| (b) wat off by default | 11.39 MB | 32 s | 0.14–0.16 s | 0.11 s | 6–25 ms | tests assemble their WAT with wat2wasm |
| (c) opt-level "s" | 7.70 MB | 34 s | 0.13–0.16 s | 0.12–0.15 s | 7–9 ms | |
| (c) opt-level "z" | 6.43 MB | 38 s | 0.14–0.19 s | 0.13–0.16 s | 8–10 ms | Cranelift at z compiles slower |
| + parallel compilation, opt 3 | 11.47 MB | 27 s | 0.14–0.17 s | 0.11 s | 5–7 ms | warp-self's machine code 8.3 → 2.4 s |
| + parallel compilation, "s" | 7.77 MB | 25.5 s | 0.13–0.15 s | 0.10 s | 6 ms | machine code 2.6 s |
| **+ parallel compilation, "z" (profile host, the default)** | **6.50 MB** | 26 s | 0.15–0.18 s | 0.11–0.12 s | 7–8 ms | machine code 4.1 s |
| + a shipped .kept (384 KB, any machine) | 6.50 MB + 0.38 MB | **4.1 s** | 0.15 s | 0.12 s | | only the machine code is made |
| + a shipped .kept and .cwasm (18 MB, one target) | 6.50 MB + 18.5 MB | **0.15 s** | 0.15 s | 0.12 s | | |
| (a) no Cranelift, precompiled warp-self | ~2.2 MB (estimate) | — | — | — | — | can't run `run`: see below |
| (d) hand-written WASI p1 instead of wasmtime-wasi | ~6.0 MB (estimate) | same | same | same | same | not built: ~0.4–0.5 MB for ~300 lines |
| (e) wasmtime as a shared library (C API) | ~0.5–0.8 MB + 21–32 MB lib (estimate) | 2–4 s machine code + 23 s kept | ~same | ~same | 6 ms (prototype) | see below |

- Cold start: 23 s of the ~26 s is warp-self computing its kept values (the lowered runtime words), ~2–4 s is
  Cranelift compiling warp-self. A shipped `.kept` removes the first, so a fresh install's first run takes ~4 s;
  shipping the machine code too makes it warm but adds 18 MB per target, against the goal (Supervisor: ship the
  .kept only).
- Packaging (card warp-host-run-wasm): packaging/host_cache.sh <dir> fills <dir>/host-cache with the .kept files of
  one hello run in a tarball's layout (warp-host, warp-self.wasm, lib/). release.yml builds warp-host
  (`--profile host`) for each target, makes host-cache once in the compilers job and puts warp-host and host-cache in
  every tarball. deb/rpm install it in /usr/share/warp/host-cache, the wheel in <env>/share/warp/host-cache, the
  formula in pkgshare. Checked on the box with a staged tarball: nfpm's .deb, unpacked, with a fresh HOME runs hello
  in 3.7 s the first time (26.8 s without host-cache), then 0.14 s; the wheel holds share/warp/host-cache.
- Chosen default: profile host (opt-level "z", wasmtime's runtime at 3), parallel compilation, no wat. It is the
  smallest with warm times within noise of opt-level 3 (the programs' machine code is Cranelift's either way, so the
  opt-level only slows the host and Cranelift itself). `--release` still builds the 11.5 MB fast one.
- (a) Without Cranelift: warp-self could load as a precompiled `.cwasm`, but every program it compiles is a new
  module that needs machine code. Pulley (wasmtime's interpreter) gets its bytecode from Cranelift too, and Winch is
  built on Cranelift's assembler and has no GC or exceptions. So a host without Cranelift runs only modules compiled
  elsewhere. Estimate: the 1.57 MB stub plus ~0.6 MB of WASI.
- (d) WASI is ~0.5 MiB of .text; the component model stays anyway (the feature compiler turns it on). Not worth ~300
  lines of WASI now.
- (e) Wasmtime's C API as a shared library (prototype in scratch, not committed: hand-written extern blocks, 330 KB
  linked against the v49.0.1 release's libwasmtime.so):
  - Coverage: everything Warp uses is there. Config setters for GC, function references and exceptions; anyref, i31,
    structref (fields read and set, new), arrayref, exnref and tags; full reference value types, concrete struct types
    among them; host functions with any import type (`wasmtime_linker_define_func`, type from the module's import);
    WASI p1 (`wasi_config_*`, `wasmtime_linker_define_wasi`); serialize/deserialize. The prototype ran a module with
    call_ref, throw/try_table, WASI fd_write, and a host function taking a GC struct, reading its fields and answering
    an i31. Missing: choosing the collector (the release library has DRC, which keeps cycles; warp uses gc-copying)
    and gc_heap_initial_size.
  - Size: the host would be warp's own code plus std, ~0.5–0.8 MB (estimate). The library is 31.7 MB (25.5 MB
    stripped) for Linux from the release, 21.1 MB from Homebrew 50.0.0. The total is 3–4× today's 6.5 MB. A library
    built with only the needed features would be about as big as what is linked statically now.
  - Version coupling: the library has no SONAME (Linux) and compatibility version 0.0.0 under an unversioned install
    name (Homebrew), and wasmtime makes a major release monthly. The C ABI changed little from 49 to 50 (two
    callbacks took const; new setters), and the v49 prototype ran on 50's library, but nothing guarantees it, and a
    struct layout change would be undefined behaviour, not an error. `brew upgrade wasmtime` swaps the library under
    the host; the cached machine code is then refused ("Module was compiled with incompatible version '49'") and
    rebuilt. So the host would pin a version and ship the library beside itself, or be rebuilt with every upgrade.
  - Times: the same engine. The prototype compiled warp-self.wasm in 2.1 s (parallel) and loaded its .cwasm in
    2 ms, and ran hello.wasm in 6 ms. Cold start would still be dominated by the 23 s of kept values.
  - Work: nodes.rs, words.rs, ffi.rs, cache.rs and c_calls.rs's linking use the Rust API (Rooted, StructRef, Caller,
    Linker), so they would be rewritten against the C API: ~1500 lines of unsafe code (estimate).

## lib/shell.warp under warp-host (2026-10-22, branch warp-host-shell)

Work: ls, cd (its value, and commands and std_io paths after it), mkdir, touch, cp, mv, rm, cat, echo, head, tail, wc,
unique_lines, find_files, file_size, info, which, glob, list_files. tests/host/test_host.sh checks them, each in a fresh folder.
Not yet, for reasons outside warp-host:
- `pwd`/`cwd` after `cd` still give the start folder. The pure compiler computes a definition without parameters
  (`cwd := std_io(…)`, `pwd := cwd`) once, where warp-rust runs it at every read; and `f() := …` has no bare `f`.
- `cat`/`read` and the prelude's file reads go through the runtime's own WASI (core/runtime/extended/wasi.warp), not
  std_io. So they take no `~`, and a relative path stays in the start folder (WASI's preopened `.`) after `cd`.
- `ls("nosuch")` traps (`node length`): the comprehension takes list_files's Error as a list. `list_files("nosuch")`
  itself is a catchable Error.
- `grep`: the compiler refuses `matches(…)` (use regex). `sort_lines`: `sort(file_lines(…))` resolves to shell.warp's
  `sort(file:Path)` and traps with `no field name`.
- `print ls` without parentheses prints the symbol ls: the pure compiler calls no function by its bare name.

## Ahead of time (card warp-host-aot)

Adopted from warp-rust's `compile --aot` and `build` (notes/aot.md), in crates/warp-runtime/src/warp_host/aot.rs.
- `warp-host compile --aot f.warp [f.wasm]` writes the module and beside it f.cwasm, its machine code for this machine
  from the engine warp-host runs with (engine.precompile_module). `compile --aot m.wasm` writes only m.cwasm.
- `warp-host f.cwasm [args…]` runs it through Module::deserialize. Machine code is recognized by its .cwasm name, else
  by how it starts (wasmtime's ELF image, or the C table below).
  - Machine code of another wasmtime version, other engine settings or another target is refused before it runs:
    `warp-host: f.cwasm was compiled by another warp-host or for another machine (Module was compiled with incompatible
    version '48'): compile it again with `warp-host compile --aot``.
  - A file that is no machine code: `f.cwasm is no machine code of a module`.
  - Trust: a .cwasm is native code, so running one is like running an executable.
- C calls: machine code keeps no custom sections, so the module's warp.c_functions lines travel in front of it in the
  table warp build uses (c_signatures.rs with_c_functions, `WRPCcfun`). The libraries are dlopened when it starts.
- `warp-host build f.warp [-o app]` writes the standalone executable f (f.exe on Windows), or a module's with
  `build m.wasm`. It is a copy of this warp-host with the machine code appended, behind warp's executable trailer
  (standalone.rs with_machine_code: deflated, its u64 length, `WRPCwasm`, padded to 16).
  - On start warp-host reads its own end (std::env::current_exe, standalone::embedded_machine_code: the load commands
    and the last 32 bytes). When it carries a program it runs that with all its arguments, so `app --help` is the
    program's, and prints the value as `warp-host f.warp` does. It needs no compiler, lib/ or cache.
  - It is one binary: WASI, std_io (shell words, commands, run_wasm) and C calls work as in warp-host. compile_wasm
    still needs a warp-self.wasm to be found.
  - macOS: appending after the signature breaks it. Instead, the signature is dropped, the program goes inside
    __LINKEDIT (macho.rs), and the file is signed ad hoc with `codesign --sign - --force`.
    `codesign --verify --strict` passes (tested on the Mac, arm64).
  - Building again over an executable replaces its program. build never writes over its source (`build script` of an
    extensionless file needs -o).
- Tests: `tests/queue.sh tests/host/test_host_aot.sh`, with the same settings as test_host.sh. 34 checks: box 33 (no
  codesign check), Mac 34.
- Measured 2026-10-23 with hello.warp. Both machines were heavily loaded (box load ~50 on 8 cores, Mac ~60), so the
  times are upper bounds:

  | | box, Linux x86_64 | Mac, arm64 |
  |---|---|---|
  | warp-host (profile host) | 6,598,344 B | 4,042,576 B |
  | hello.cwasm (hello.wasm 1,685 B) | 19,712 B | 52,488 B |
  | executable `hello` | 6,603,328 B (+5 KB) | 4,024,336 B |
  | executable of a `use os` program | 6,666,320 B | 4,084,256 B |
  | `./hello` (executable) | 4 ms | 12.9 ms ± 3.2 |
  | `warp-host hello.cwasm` | 6 ms | 17.2 ms ± 6.8 |
  | `warp-host hello.wasm` (Cranelift each run) | 35 ms | 24.6 ms ± 11 |
  | `warp-host hello.warp`, warm cache | 333 ms | 482 ms ± 78 |
  | `warp-host hello.warp`, cold (empty cache, none shipped) | minutes (loaded box) | 43.3 s |

  The executable is warp-host plus the deflated machine code. On the Mac it came out smaller than warp-host itself:
  the linker's signature is replaced by codesign's.
- The optional precompiled warp-self.cwasm already exists as the shipped host-cache's .cwasm (18–19 MB per
  target, see Size). It is opt-in through packaging (packaging/host_cache.sh), so no second mechanism was added.

## Next

- M3 (card pure-c-imports): the pure emitter emits C imports and a warp.c_functions section, which warp-host dlopens.
- M4: libffi.
