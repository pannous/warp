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
- Measured 2026-10-23 04:20 [M], host-from-warp merged with origin/warp-host-wasm-load 4e3808b76 (main and pure-c-imports
  included): warp-host built from the merged tree (7,429,400 B), compiler-self.wasm sha1 6269c5f5… (1,544,202 B) and
  compiler.wasm f0689f43… from warp-host-wasm-load's target/debug, wasm-tools 1.262.0 as WARP_WAT2WASM.
  - test_host.sh directly: 86 of 86, 2 min 17 s.
  - run_checks.sh in-process (compile_wasm, run_wasm): 86 of 86, 1 min 45 s. The cache checks and the run_wasm argument
    check pass with the driver's changes above; no driver fix was needed.
  - run_checks.sh --spawn: 86 of 86, 1 min 40 s.
  - Not covered by test_host.sh, still different: an uncaught raise prints `Error("bad")` on stdout from the driver,
    `warp-host: bad` on stderr from warp-host (both status 1).

## Native glue route (parked, 2026-10-23)
Question: can the thin stub's own Rust glue (crates/warp-runtime/src/warp_host, feature `host`) also become Warp, written
in a Warp subset that compiles to plain wasm (no GC, no exceptions), then wasm2c/w2c2 → C → native, linked against
wasmtime's C API? Answer: **not yet**. Only number-only programs compile to non-GC wasm today. Every text, list or
boxed number brings in GC types and instructions. The route waits for a plain-memory emitter mode for basic types
(roadmap, lib/compiler, not ours). It is parked and can be resumed from this section. Labels as above: [M] measured on the
box today, [I] inferred.

### Measurements [M]
Setup: /tmp/hfw-c/warp-host (7,429,400 B, sha1 042dc1b8e247…) with compiler-self.wasm 1,544,202 B (sha1 6269c5f5…) and
`WARP_LIB=/workspace/cow/trees/host-from-warp/lib`. Then `wasm-opt -all --closed-world --remove-unused-module-elements
--remove-unused-types` (binaryen version_124; without --closed-world the pass refuses with "RemoveUnusedTypes requires
--closed-world"). Then `wasm-tools validate --features mvp,mutable-global,saturating-float-to-int,sign-extension,multi-value,
bulk-memory,reference-types` (wasm-tools 1.262.0). Valid modules go through `wasm2c` (wabt 1.0.42; 1.0.37's build needs
libcrypto.so.1.1, which this Debian 13 box lacks) and `cc -O2` with wasm-rt-impl.c and wasm-rt-mem-impl.c, then run.
Probe sources and the script: /workspace/hfw-tools/glue-probes/{*.warp, run_a.c, measure.sh} (box, not in the repo).

| probe | source | wasm | stripped | non-GC valid? | GC instructions left in the stripped module | wasm2c → native |
|---|---|---|---|---|---|---|
| a | `6*7` | 792 | 162 | ✓ | none | 23,552 B binary, prints 42 |
| i | `1+2` | 764 | 149 | ✓ | none | 23,552 B, prints 3 |
| g | `i=0; while i<5 {i=i+1}; if i>3 {1} else {2}` | 810 | 189 | ✓ | none | 23,552 B, prints 1 |
| h | `def sq(x){x*x}; sq(7)` | 819 | 180 | ✓ | none | 23,592 B, prints 49 |
| b | `print "hello"` | 1,092 | 385 | ✗ array type | array.new_data, array.len×3, array.get_u, ref.null | – |
| j | `print 42` | 1,500 | 618 | ✗ array type | array.new_default, array.set×2, array.len×3, array.get_u | – |
| n | `x=2; if x>1 {print "big"}` | 1,111 | 401 | ✗ array type | array.new_data, array.len×3, array.get_u, ref.null×2 | – |
| m | `"ab" + "cd"` | 823 | 251 | ✗ array type | array.new_data×2, array.copy×2, array.len×5, ref.eq×2, struct.new | – |
| f | `s = "ab" + "cd"; print s` | 1,265 | 531 | ✗ array type | array.new_data×2, array.copy×2, array.len×8, ref.eq×2 | – |
| k | `x=3.5; x*2` | 1,806 | 824 | ✗ subtypes | struct.new×16, struct.get×21, ref.cast×21, ref.test×4 | – |
| l | `xs=[1,2,3]; xs[1]` | 1,034 | 328 | ✗ subtypes | struct.new×15, struct.get×3, array.new_fixed×2, array.get | – |
| c | `xs=[1,2,3]; print xs` | 21,943 | 14,896 | ✗ array type | struct.new×154, struct.get×245, ref.cast×194, array.new_data×102, ref.i31×41, … | – |
| d | `use os; os.run("echo hi")` | refused: "outside the core: a call of run with 2 arguments" | | | | |
| e | `f := (n) => n*2; print f(21)` | refused: "outside the core: the call f(…)" | | | | |

- The unstripped modules all fail non-GC validation, even `6*7`. The runtime prelude always declares $text, $items, $list,
  $Node, $i64box, $f64box, $Ratio and $MetaNode, plus 15 `(ref null $text)` globals for the text walkers. In number-only
  programs nothing uses them, so stripping removes them. The code of `6*7` is plain i64 arithmetic with an overflow check
  (f64 estimate → `unreachable`).
- wasm2c rejects GC modules outright: "unexpected type form (got 0x5e)", which is the array type, on a.wasm unstripped and
  on b stripped.
- No probe module has exceptions: try_table, throw and tag counts are 0 in all of them. EH is only in compiler-self and
  in programs that use raise/try (not probed here).
- What forces GC:
  - **Texts** are `(array (mut i8))`. Every string literal is `array.new_data`, so print, concat and int-to-text
    (`print 42`) all need GC.
  - **Lists** are `$list {length, (ref $items)}` over `(array anyref)`, with elements boxed.
  - **Non-int numbers** go through the generic number path, which uses `$Node` (a sub type), $i64box, $f64box and $Ratio
    with ref.cast and ref.test. `3.5*2` is not plain f64.
  - **Printing a list** builds Nodes (struct.new×154).
  - Closures and lambdas were not reachable: the pure compiler refuses `f := (n) => …` calls.
  - Plain ints, bools, while, if and `def` functions on ints stay GC-free.
- Imports: every probe that prints imports only `wasi_snapshot_preview1.fd_write`. wasm2c brings no WASI, so it would need
  uvwasi or a small fd_write shim. w2c2 ships a WASI implementation [I, w2c2 README].
- wasmtime C API release tarball (dev, x86_64-linux-c-api): 16,056,784 B .tar.xz [M, GitHub API]. Not downloaded or linked.

### Inventory of the native surface [M line counts, classification I]
Tree: boxtree host-from-warp at a3c6f5641 (the 86/86 state; main's warp_host has aot.rs and lacks components.rs and
wasm_modules.rs: main 484, words 239, nodes 380, shell 255, cache 128, ffi 132, aot 92 lines).

| group | files (lines) | what it does | into Warp now (on the stub)? |
|---|---|---|---|
| wasmtime setup / main | main.rs 389 (engine, Linker, Store, WASI ctx, on_own_stack, run_wasm, compiled_code, program_linker); shared engine.rs 260 | engine config, linking, instantiation, traps, the run | **stays native**: the CLI and policy part of main.rs (command, is_module, compile_file, library_for, compiler_file, installed_folders, failure/shown/refused) is already in probes/host_from_warp/driver.warp |
| os words | words.rs 250 (std_io dispatch), shell.rs 255 (list, glob, mkdir, copy, move, touch, remove, find, size, info, cd, which, open) | os.env/args/cwd/input/stdin/command/exec/run, file.read/write/append/exists/read_bytes/write_bytes, Unix commands | **partly**: file.*, most of shell.rs (readdir, mkdir, rename, unlink, filestat) and os.env/args can be Warp over WASI p1 [I]. The glob matcher, `which` (a PATH walk) and `~` expansion are pure logic. Process spawn (command/exec/run), trash and open, a real cwd (WASI has no chdir) and stdin prompts stay native or need new primitives |
| FFI | ffi.rs 148 (C imports linked by dlopen/dlsym, `warp.c_functions` custom-section reader); shared c_calls.rs 649, c_signatures.rs 223, c_structs.rs 226, libm.rs 41 | C calls and their marshalling | **split**: section parsing (uleb, the 9-field signature lines) and struct layout rules could be Warp. dlopen/dlsym and the call itself stay native. In C that means trampolines: 0–6 args × {i64,f64} × {i64,f64} = 254 shapes, 12,030 B .text (gcc -Os, measured in the main evaluation above, a scratch C file, not in the repo). f32/struct args would need libffi or more shapes |
| wasm-load words | wasm_modules.rs 446, components.rs 395 | load_wasm, wasm_call, file imports, WIT components | **mostly native**: instantiation, linking and the component model are engine calls. Handle bookkeeping, name matching (stats_of ↔ stats-of) and the value-crossing rules are logic [I] |
| kept-values cache | cache.rs 128 | cache folder, fingerprints, .cwasm machine code, os.keep/os.kept | **split**: keep/kept (a file plus encode/decode of a node) could be Warp over file words. Serializing machine code is engine work and stays native |
| other (values) | nodes.rs 388; shared gc_values.rs 448, host_words.rs 287 | GC Node ↔ host value conversion for every std_io call and the run's printed value | goes away only if the words move into Warp. As long as native words answer Nodes it stays native (or C via exported builder functions, as gc_values' Builders already does) |
| total | warp_host 2,399 lines / 102,782 B; plus shared modules used: c_calls, c_signatures, engine, gc_values, host_words, libm, standalone, system_signals | | |

### What the native glue route would need
1. **Plain-memory emitter mode for basic types** (roadmap item; lib/compiler, owned by the Claude workers). Ints, floats,
   bools, texts as (ptr, len) in linear memory, and fixed records and arrays in memory. No $Node, no boxes, no GC type
   declarations in the prelude unless used. Gate: probes b, j, m, n, f, k and l validate non-GC.
2. **Exception-free mode** (status returns instead of try_table/throw), the "later option" in Decisions above. The glue
   must report failures, and wasm2c only supports legacy EH.
3. **The ABI to wasmtime's C API.** A wasm32 module cannot hold native pointers. Either (a) wasm32 plus a native C shim:
   the glue imports `wasmtime.*` and `c.*` functions that take i64 handles and linear-memory offsets, and a generated C
   adapter maps them to wasmtime_* calls (memory base + offset for buffers). Or (b) memory64, which wasm2c supports and
   which still does not make host pointers [I]. (a) is the realistic one. The shim must also stay native for: wasmtime
   host callbacks (`wasmtime_func_callback_t` into wasm2c functions, one adapter per callback signature), the dlsym call
   trampolines, setjmp/trap handling, and reading the program's GC Nodes (the C API has no struct.get; use exported
   builder and reader functions as gc_values.rs does) [I].
4. **WASI for the glue itself**: uvwasi with wasm2c, or w2c2's own WASI.
5. Compare against the Rust stub on size, startup and test_host.sh.

### Resume steps
1. Re-measure: `WARP_LIB=<tree>/lib /workspace/hfw-tools/glue-probes/measure.sh /tmp/glue-out` (tools in
   /workspace/hfw-tools: wasm-tools-1.262.0, binaryen-version_124, wabt-1.0.42). Add probes for raise/try and for closures
   once the pure compiler accepts them.
2. When the plain-memory mode lands: check that b/j/m/n/f become non-GC valid, then wasm2c + cc them with an fd_write shim.
3. Write one glue word as a spike: os.env in the Warp subset, then plain wasm, wasm2c, and C linked with the wasmtime
   C API (libwasmtime.a from the c-api release). Compare its output with warp-host's.
4. Choose wasm2c or w2c2 (WASI included, MVP-oriented). Then write the C shim generator (callback adapters plus the 254
   dlsym shapes).
5. Move the other words in the order of the inventory: file/shell over WASI first, then cache keep/kept, FFI section
   parsing, and the wasm-load bookkeeping. The engine setup, callbacks and trampolines stay native C.

### Related open items (recorded here in case board cards get lost)
- **C-import follow-up for the driver**: once pure-c-imports (origin/pure-c-imports 836b8e9a4, not on main yet) merges,
  re-run the driver's checks with programs that declare C imports (`strlen(s: text): i64 from "c"`). Check that
  driver.warp's in-process run_wasm links them as warp-host does (ffi.rs). The driver itself needs no C imports.
- **stderr-word gap**: the driver prints an uncaught raise as `Error("bad")` on stdout. warp-host prints `warp-host: bad`
  on stderr. Both exit with status 1. There is no stderr word (os.stderr / print to fd 2) to close it, and test_host.sh
  does not check it.
- **pure-exit-miscompile**: in compiler-self (2026-10-22), `try 42 catch e { exit(1) }`, and an if with an inline exit()
  branch, yield ø when nothing fails. Repro: `use os; c(code) := try compile_wasm(code) catch e { exit(1) }; count(c("6*7"))`
  gives 0. The driver works around it by marking failures as texts and exiting in functions of its own.
- **Exception-free / plain-memory emitter mode**: a later option, not planned. It is the prerequisite of this route
  (items 1–2 above) and would also open WAMR and wasm2c for whole programs.

### Proposed Later card
- Title: host-glue-native: warp-host's remaining Rust glue in Warp, via plain wasm → wasm2c → C against wasmtime's C API
- Description: Parked until lib/compiler has a plain-memory (no GC, no EH) emitter mode for basic types. Today only number-only programs
  (`6*7`, while/if, int functions) strip to valid non-GC wasm and run through wasm2c. Texts, lists and floats need GC
  (notes/host_from_warp_eval.md "Native glue route (parked)", measure.sh). Then do the spike with os.env through a
  C shim to the wasmtime C API, and move the file/shell/cache/FFI-parsing words. Engine setup, callbacks and dlsym
  trampolines stay native.
