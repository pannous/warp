# host-from-warp: can warp-host be built ahead of time from Warp? (evaluation, 2026-10-23, Linux box x86_64)

Labels: [M] = measured this session on the box; [N] = from repo notes, measured by others; [I] = inferred or estimated.
Scratch (ignored, uncommitted): /workspace/cow/trees/host-from-warp/scratch/hfw (box) (tools/, mods/, ffi/). The worktree is boxtree
`host-from-warp` on origin/warp-host-run-wasm 1aec4ee9f. Nothing was committed. src/ and lib/compiler were not touched.

## Verdict
- **Fully native host with no Rust and no wasmtime: not feasible now.** No wasm→C or wasm→native tool takes our real modules.
  They all use WasmGC, and compiler-self also uses standard EH (try_table). The run path (run_wasm, `warp run` of new code)
  needs an engine whatever happens.
- **Feasible now: the host's logic in Warp on a fixed native engine shim (approach c).** host.warp is compiled by the pure
  compiler and embedded as .cwasm in a prebuilt stub (wasmtime + Cranelift + WASI + primitive host words). It reuses the
  `warp build` stub+cwasm mechanism. Gain: the policy (~1.2k lines of warp_host/*.rs) moves to Warp, and Warp developers
  never rebuild Rust. Size does not change: Cranelift is ~5.2 MiB of the 6.5 MB [N].
- Full AOT becomes feasible only after one of these: (1) WAMR ships standard EH plus struct/array const-expr init;
  (2) our emitter lowers try_table/throw to something WAMR or wasm2c accept; (3) our own C backend, with a GC and EH in C
  (weeks of work [I]).

## Current state (facts)
- warp-host [N/M]: crates/warp-runtime/src/warp_host/{main 375, words 229, nodes 349, cache 128, ffi 122} = 1,203 lines [M].
  It reuses warp-runtime: 5,610 lines of .rs in total, including c_calls 567, c_signatures 223, gc_values 448 and engine 260 [M].
  Built on wasmtime 49.0.1 (gc, gc-copying, Cranelift, parallel compilation), wasmtime-wasi p1 and libloading [M Cargo.toml].
  Binary /tmp/tarball/warp-host: 6,509,288 B, sha256 58bf08f5… [M]. No host build exists in /workspace/cow/warp/target, so
  I measured no build time.
- What warp-host gives modules [N]: WASI p1 with preopens / and ., host.std_io (os.env/args/cwd/command/run/run_wasm/
  compile_wasm, file.*), libm, libc under `c`, and C imports by dlopen/dlsym typed by the `warp.c_functions` section.
- compiler-self.wasm (1,506,183 B, sha256 ee96036e…) [M]: 7,455 functions, 7,476 types, one tag, 8 try_table, 5,441 throw,
  no call_ref. Imports: host.std_io, host.local_offset and 10 WASI calls. It needs gc, exceptions, reference-types and
  bulk-memory (wasm-tools validate with each feature removed in turn).
- hello.wasm (1,085 B) needs gc but no EH. The FFI sample strlen.wasm (202 B, the WAT from tests/host/test_host.sh) needs
  gc plus a C import [M]. With the tarball's compiler-self, `strlen(s: text): i64 from "c"` is refused with "outside the core:
  the call strlen(…)" [M]: pure modules with C imports wait for the card pure-c-imports. Correction: an earlier
  "os.run is refused" result was wrong. A local os.warp probe file shadowed `use os`. os.run, os.command, run_wasm,
  compile_wasm and the file words all compile and run with the tarball's compiler-self [M].
- **New [M]:** compiler-self runs as a plain WASI module. I merged a 2-function null stub for host.std_io and
  host.local_offset into it (wasm-merge v133, `--disable-custom-descriptors --disable-compact-imports`) and ran it under
  stock `wasmtime 50.0.0 run`. Its hello.wasm is byte-identical to warp-host's: 3 min 01 s wall, 58.6 s user, no kept
  values, box load ~40. So compiling needs no custom host. std_io only serves the kept-values cache and the os words.

## Routes tried on real modules [M unless noted]
| route | version | hello | strlen (FFI) | compiler-self | notes |
|---|---|---|---|---|---|
| wasm2c | wabt 1.0.42 | ✗ | ✗ | ✗ | "only supports a limited set of features", even with --enable-gc. Legacy EH only, no GC |
| w2c2 | upstream README | – | – | – | [I] no GC/EH mentioned; MVP-oriented; not built |
| WAMR wamrc AOT | 2.4.5 --enable-gc | ✓ 4.5 KB .aot, runs "hi" | ✓ compiles (C import unlinked) | ✗ "invalid section id" (tag). With EH stripped: no .aot after 14.6 min, cause unknown | upstream: standard EH unimplemented, legacy EH interpreter-only |
| WAMR iwasm-gc-eh interp | 2.4.5, 583,264 B | ✓ 6 ms | needs native registration | ✗ "unsupported opcode 1f" (try_table). With EH stripped: "array as a field in struct object is not supported in constant init expr" | default GC heap too small: alloc test aborts without --gc-heap-size |
| wasmtime compile → .cwasm | 50.0.0 | ✓ 15 KB .cwasm | – | ✓ (warp-host does this, ~18 MB .cwasm [N]) | wasmtime-min 3,290,864 B is built without gc. Our warp-runtime stub (no Cranelift) is ~1.57 MB [N] |
| WasmEdge interp | 0.18.1, libwasmedge 63.9 MB | ✓ 19 ms | – | loads and validates (GC+EH), stops only at the missing import host.std_io | |
| WasmEdge AOT (LLVM) | 0.18.1 | – | – | compiles the merged module to a 6.26 MB .so in 2 min 02 s (41 s user). Run inconclusive: OOM-killed at 1.74 GB RSS, 42 s in, with 1.8 GB free on the box | retry on an idle box |
| Binaryen GC lowering | v133 | – | – | no GC→linear-memory pass. Only --strip-eh, which breaks warp errors | v133 emits exact refs and compact imports by default, which WAMR and wasmtime 50 reject unless disabled |
| wasmi | upstream | – | – | – | [I] no GC, function-references or EH (planned for 3.0) |
| wasm3 | not tried | – | – | – | [I] no GC; unmaintained |

## Interpreters vs Cranelift for the run path [M]
fib(32) wall time: warp-host 0.233 s; iwasm-gc-eh 2.33 s (~10×); WasmEdge interp 3.39 s (~15×).
Pulley needs Cranelift to make its bytecode, and Winch has no GC [N]. So of the engines checked, only wasmtime+Cranelift and
WasmEdge (64 MB, C++/LLVM) cover GC+EH today. A smaller interpreter would be WAMR (~0.6 MB), but only once it has try_table.

## FFI and minimal C runtime [M]
- Generated trampolines: a switch over 0–6 args × {i64, f64} × {i64, f64} results gives 254 call shapes in 12,030 B of .text
  (gcc 14 -Os). A test calling hypot through dlsym and strlen ran correctly; the stripped test binary is 22,688 B.
- libffi: libffi.so.8 is 47,576 B on the box, without headers. f32 and struct arguments would need libffi or more cases [I].
- A fixed C shim (dlopen/dlsym/trampolines, exec, file I/O primitives) would be ~1–2k lines [I]. It does not remove the
  engine, which is the 6 MB.

## Approaches
| approach | missing | effort | blockers / outside our areas |
|---|---|---|---|
| (a) host.warp → wasm → wasm2c/wamrc + C runtime | GC+EH in the tool; still needs an engine for run_wasm | large; blocked | upstream WAMR/wabt; or lib/compiler emitter EH lowering (Wrapper's or the compiler owner's call) |
| (b) Warp emits C or LLVM IR | a whole backend: GC (Boehm or own), EH (setjmp), FFI | weeks–months [I] | lib/compiler (not ours); still needs an engine for `run` |
| (c) host.warp → .cwasm inside a prebuilt Rust stub that exposes primitives | stub variant with Cranelift + std_io + run_wasm; host.warp; a compiler-self that accepts os/file words | days [I] | crates/warp-runtime stub feature (allowed, Rust), compiler-self rebuild with os words |
| (c') host.warp on a WasmEdge-based C shim | measure the AOT run; a C shim for std_io and FFI | ~1 week [I] | 64 MB library; C++ toolchain to build it |

## Bootstrap plan (for c)
0. Stage0 is today's Rust warp-host.
1. Write probes/host_from_warp/driver.warp, the warp-host CLI logic (dispatch, file-or-code decision, lib lookup, output and
   status rules from notes/warp_host.md) using os.compile_wasm, os.run_wasm and file.*.
2. Run it with stage0: `warp-host driver.warp run hello.warp`.
3. Add a stub feature `host-stub`: warp-runtime + Cranelift + WASI + std_io + run_wasm. Its main runs the carried driver.cwasm.
4. Stage1 = stub + driver.cwasm (the `warp build` packing). Stage1 compiles driver.warp again, and the bytes must match stage0's.
5. Move the remaining Rust policy (cache keys, output formatting) into driver.warp step by step. The stub keeps only primitives.
- First milestone: driver.warp under stage0 passes the run/compile/help rows of tests/host/test_host.sh, with no Rust change.
  Measured blocker: the tarball's compiler-self refuses os words, so it first needs a `warp rebuild self` that has them.
- Gains: Rust frozen to a small stub that can be prebuilt per target; host behaviour editable in Warp.
  Size, startup and portability barely change [I], since Cranelift dominates and the warm start is 0.12–0.15 s [N].
- Risks: two compilers in play (stub-carried and fresh), cwasm tied to the wasmtime version, a driver bug bricks the host
  (keep stage0 as fallback). True no-Rust needs a WasmEdge/WAMR swap, which stays blocked until standard EH.

## Open questions
- Warp Wrapper: since compiler-self runs under plain `wasmtime run` with a null host stub, could the wrapper skip the custom
  host for compiling and keep warp-host only for os/FFI words and kept values?
- Warp Wrapper: should the emitter offer an EH-free mode (status flag returns instead of 8 try_table and 5,441 throw)? That
  would open WAMR (0.6 MB interp, AOT) and wasm2c-legacy-EH routes. It is an emitter change in lib/compiler.
- Supervisor: is ~6.5 MB with Cranelift acceptable long-term? Answered yes (Decisions below).
- (Cancelled 2026-10-23 by the user decision below: the WasmEdge AOT retry.)

## Decisions (2026-10-23)
- warp-host stays the thin native stub: wasmtime + Cranelift, WASI, the os/FFI/wasm-load words (std_io, C calls by
  dlopen, run_wasm, compile_wasm) and the kept-values cache. Everything else (the CLI and the policy around compiling
  and running) moves to Warp. probes/host_from_warp/driver.warp is the first step.
- An exception-free emitter mode (status-flag returns instead of try_table/throw) is a later option, not planned now.
- User decision 2026-10-23: a ~6.5 MB host with wasmtime+Cranelift is acceptable long-term; a fully native
  no-Rust/no-wasmtime host is not a goal; the WasmEdge/wasm2c/WAMR routes are dropped. The WasmEdge AOT rerun is
  cancelled, and the measurements above stay as a record.

## Driver: warp-host's CLI in Warp (probes/host_from_warp/, 2026-10-23)
- driver.warp (157 lines) handles run, compile, help, --help/-h, --compiler/--fuel/other options, the no-command forms,
  a module by name or magic bytes, the missing-.warp error, refusals as the only `Error("…")` line, and exit statuses.
  It is built on os.compile_wasm, os.run_wasm, os.run and the file words.
- run_checks.sh <warp-host> [--spawn] runs tests/host/test_host.sh, unchanged, against a shim warp-host that runs
  `<warp-host> driver.wasm "$@"`.
- Measured on the box with /tmp/tarball/warp-host (6,509,288 B, sha256 58bf08f5…) and its compiler-self (ee96036e…):
  - in-process (compile_wasm, run_wasm): 31 of 46 pass, 2 min 07 s. All 15 failures are one warp-host bug (below).
    They are 4 command/run, 3 argument, 1 launcher run and 7 run_wasm checks.
  - `--spawn` (WARP_DRIVER_SPAWN: compiles and runs by warp-host processes through os.run): 46 of 46 pass, 47 s.
- Found while doing this, both in warp-host (Rust, crates/, not changed here):
  - A list of more than ~20,000 items from a host word fails with "memory cap: growing a memory to 4096 MB passes the
    cap of 2048 MB", then "list join: 0". Measured: `use os; use file; print count(read_bytes(args#1))` gives 20000 for
    a 20,000-byte file and fails for 24,000 B. compile_wasm gives its module as such a list, so in-process compiling
    fails for every module over ~20 KB, and every program with `use os` is ~23–26 KB. run_wasm(read(file)) (a text)
    works at any size.
  - run_wasm takes no argument vector: a module run in-process sees its runner's args. The check
    "inline code gets the words after it" needs os.run_wasm(bytes, args) even after the list fix.
- compiler-self (2026-10-22) miscompile, worked around in the driver: `try X catch e { …; exit(1) }` (and an if
  with an inline exit() branch) gives ø when nothing fails. Repro: `use os; c(code) := try compile_wasm(code) catch
  e { exit(1) }; count(c("6*7"))` gives 0. The driver marks failures as texts and exits in functions of its own.
- Not covered by the driver yet: `--compiler full` (compile_wasm uses the compiler warp-host started with), stderr
  (no word: the driver's failures go to stdout), and in-process compiles of a .warp file as code of the current folder.
  The driver itself needs no C imports. Programs' C imports unblock with pure-c-imports.
- Follow-up, 2026-10-23 02:35 (host-from-warp merged with origin/warp-host-wasm-load 5bc01c62f, which has the list fix
  00c3c9b5c and os.run_wasm(bytes, args)):
  - driver.warp now passes the program's arguments (run_wasm(module, program_args)). An uncaught Error of the program
    (no "os.run_wasm: " prefix) prints as `Error("…")`, status 1; a trap or a module that could not run prints as
    `warp-host: …`, status 1; "the module exited with status N" exits N.
  - Cache checks: run_wasm caches the program's own machine code, which warp-host never does for the program it runs,
    so the fresh cache held 2 .cwasm. The driver lists the cache folder before the run and removes the oldest .cwasm the
    run added afterwards (the program's: run_wasm compiles it before anything it loads). A run_wasm without caching
    would make that unneeded.
  - Not measured yet [I]: no compiler-self built from warp-host-wasm-load's lib/ existed by 02:33 (waited ~30 min).
    Older ones refuse the new words: /tmp/tarball's gives "outside the core: a call of run_wasm with 2 arguments",
    the one in rmain/data (origin/main 11aa8cbbf, 02:21) gives "outside the core: the call compile_wasm(…)" even with
    this lib/. So driver.wasm does not compile yet and run_checks.sh could not run.
  - test_host.sh (64 checks now) with warp-host built from 5bc01c62f (7,382,248 B) and /tmp/tarball's compilers: 51 of
    64 pass [M]. All 13 failures are that compiler-self's refusals of the branch's new words (run_wasm with arguments,
    `use wasm`, components), not the host.
