# warp-host: wasmtime as a shared library — measured (2026-10-22, Grok box, Linux x86_64, 8 cores)

Scratch only: worktree /workspace/cow/trees/dyn-measure at origin/warp-host-size 9de3911bf (detached, removed after),
nothing committed or pushed. rustc 1.99.1, wasmtime 49.0.1 everywhere. Staged binaries, test programs and the script
(mem.py) are left in /workspace/dm/ for re-runs.

Layouts
- **A** static warp-host, profile host (opt z, wasmtime at 3, fat LTO, panic=abort, stripped). Rebuilt: 6,499,736 B.
- **B** Rust dylib: crate `wasmtime-dyn` (crate-type dylib, `pub use wasmtime::*` + wasmtime-wasi; Cranelift, GC,
  gc-copying, component-model, parallel-compilation), warp-runtime's deps renamed onto it, `RUSTFLAGS=-C prefer-dynamic`.
  rustc refuses LTO with prefer-dynamic and libstd.so is panic=unwind, so profile hostdyn = host minus LTO, panic=unwind.
  Full warp-host, works (`run hello.warp` correct, cold 25.4 s like A). Builds in 1 m 44 s from scratch.
- **C** C API: libwasmtime.so from crates.io `wasmtime-c-api-impl =49.0.1` as cdylib, features cranelift gc gc-copying
  wasi async parallel-compilation (async is needed: wasmtime-wasi turns on async in wasmtime, and then
  max_wasm_stack must be set below async_stack_size, whose setter is behind the C API's async feature), same opt
  (z, wasmtime 3, LTO, abort, strip). Needs cmake (header install step; installed in a venv). Builds in ~1 m.
  The exe is the /workspace/capi/proto prototype (hand-written externs, WASI, GC struct reading, a host function), not
  a warp-host port: it runs prebuilt .wasm only. Still no C setter for the collector or gc_heap_initial_size.

## Bytes on disk (stripped)

| | exe | shared libs | total |
|---|---:|---:|---:|
| A warp-host | 6,499,736 | — | **6.50 MB** |
| A' same, no LTO + unwind (control for B) | 10,406,384 | — | 10.41 MB |
| A warp-runtime stub (release, no Cranelift) | 1,573,368 | — | 1.57 MB |
| B warp-host | 1,243,968 | libwasmtime_dyn.so 28,054,048 + libstd-*.so 1,241,448 (strip --strip-unneeded; 2,859,784 as shipped by rustup) | **30.54 MB** |
| B warp-runtime stub | 596,968 | (same libs) | |
| C libwasmtime.so | — | 6,536,736 (6,521,296 without async) | |
| C host, prototype | 330,824 | + 6.54 MB | 6.87 MB |
| C host, real port (estimate) | ~0.5–0.8 MB | + 6.54 MB | ~7.0–7.3 MB |

libwasmtime_dyn.so is big because a Rust dylib exports every symbol: .dynstr 10.7 MB, .dynsym 1.4 MB, .rela.dyn 1.5 MB,
and no LTO / no dead-code removal (.text 8.4 MB vs 4.7 MB in A or in C's lib).

Sets of binaries (warp-rust measured on the box: target/release/warp 41.9 MB; the Mac one is 47 MB):

| set | A static each | B dylib | C C API |
|---|---:|---:|---:|
| warp-host + stub | 8.07 MB | 31.13 MB | ~7.5 MB (stub port est. ~0.3–0.5 MB; it would then load the 6.5 MB Cranelift lib instead of its 1.57 MB) |
| + warp-rust | 50.0 MB | ~63–82 MB (est.: 31.1 + warp-rust minus its ~10 MB of wasmtime = 32 MB, up to ~51 MB without LTO as A→A' grew 60 %) | ~49.5 MB (warp-rust stays static: its Rust-API use (gc_traits, Linker, Caller…) can't move to the C API cheaply) |

B also requires every binary and the dylib to come from the same rustc + same build (crate hashes in symbol names),
and it forces no-LTO + unwind on all of them.

## Memory: 10 concurrent warm runs (sum over the 10 processes, MiB, from /proc/<pid>/smaps_rollup)

`run busy.warp` (loop of 3e8, ~1.5 s: loads compiler-self's cached machine code, compiles, runs), sampled at 0.8 s
(run phase):

| scenario | sum RSS | **sum PSS** | PSS anon | PSS file |
|---|---:|---:|---:|---:|
| A, 1 process | 12.1 | 10.1 | 4.9 | 5.2 |
| A, same binary ×10 | 115.3 | **48.7** | 43.3 | 5.4 |
| A, 2 distinct copies (5+5) | 113.9 | 52.7 | 42.1 | 10.6 |
| A, 3 distinct copies | 115.5 | 59.4 | 43.6 | 15.8 |
| A, 10 distinct copies | 115.2 | 94.6 | 42.6 | 52.0 |
| B, 1 process | 31.7 | 29.6 | 5.7 | 23.9 |
| B, same exe ×10 | 310.2 | **75.1** | 51.0 | 24.1 |
| B, 2 distinct exes | 309.4 | 75.0 | 49.8 | 25.2 |
| B, 3 distinct exes | 309.6 | 76.4 | 50.2 | 26.2 |
| B, 10 distinct exes | 309.8 | 83.9 | 50.8 | 33.1 |

Prebuilt `busy.wasm` (no compiler step; the only workload the C prototype can run), sampled at 0.6 s:

| scenario | sum RSS | **sum PSS** |
|---|---:|---:|
| A same ×10 / 2 / 3 / 10 distinct | 89.0 / 88.6 / 88.1 / 88.9 | **22.9** / 27.2 / 32.0 / 69.0 |
| B same ×10 / 10 distinct | 278.8 / 278.8 | 48.6 / 55.6 |
| C proto same ×10 / 2 / 3 / 10 distinct | 89.6 / 88.6 / 88.8 / 90.5 | **21.9** / 21.1 / 21.8 / 25.0 |

Compile phase (sampled at 0.06–0.1 s of `run busy.warp`): 230–460 MiB PSS for 10 processes, almost all anon (compiler-self's
GC heap, ~30–40 MiB per process), layout differences inside the noise. hyperfine's max RSS per `run hello.warp`:
A 88.4 MiB, B 107.5 MiB.

- Yes, 10 copies of the **same** static binary already share their text via the page cache: warp-host's mapping is
  53.5 MiB RSS summed but 7.2 MiB PSS (≈ one copy). The win only appears across different binary files: each
  additional distinct static binary costs ~5.3 MiB (the whole 6.5 MB file is resident; readahead/fault-around).
- B costs ~22 MiB resident per process in the dylib (relocation + symbol tables touched), ~29 MiB PSS once, plus
  ~0.8 MiB private dirty (relro/GOT) per process. B beats A only at ≥ ~6 distinct binaries running at once.
- C costs ~5.2 MiB for the lib once and ~0.03–0.25 MiB per distinct exe. Against A it saves ~5 MiB per extra distinct
  binary: 2 binaries 27.2 → 21.1, 10 binaries 69 → 25 MiB. Against the 230–460 MiB compile-phase peak of 10 test
  runs that is 1–2 %.
- Peak system memory (MemTotal − MemAvailable delta during the run) was not measurable: other agents on the box moved it
  by hundreds of MiB between identical runs (103–1031 MiB). Not reported as a result.

## Startup (hyperfine, warm cache, -N)

| command | A | B | C proto |
|---|---:|---:|---:|
| `run hello.warp` (40 runs) | 144.6 ± 23.7 ms | 144.7 ± 13.7 ms | can't (no compiler port) |
| `hello.wasm` (200 runs) | 5.2 ± 0.6 ms | 13.8 ± 1.0 ms | 4.7 ± 0.6 ms |
| no args (usage, pure process start) | 1.8 ± 0.3 ms | 9.1 ± 0.6 ms | — |

Dynamic loader (LD_DEBUG=statistics): A 117 k cycles, 283 relocations; B 2.80 M cycles, 30,619 symbol relocations
(95 % of it relocating); C 159 k cycles, 309 relocations. So B adds ~7 ms to every process start (≈1 ms symbol
relocation, the rest page faults on the 28 MB dylib); C adds ~0.05 ms. Cranelift/run times are the same engine in all.

## Recommendation

Don't do it now. (B) the Rust dylib is strictly worse: 4.7× the bytes (30.5 vs 6.5 MB), +7 ms per process start,
more memory than static unless ≥ 6 different binaries run at once, and it forces no-LTO/unwind and lock-step rustc
builds. (C) the C API library is the only one that pays, and it pays little: ~0.5 MB on disk for host + stub, and
~5 MiB PSS per additional distinct binary running at the same time — ~1–2 % of what 10 concurrent tests use, which is
dominated by each process's private compiler GC heap (30–40 MiB). It costs ~1500 lines of unsafe C-API code, loses the
copying-collector choice, and warp-rust (the 42–47 MB binary) can't realistically use it, so the biggest wasmtime copy
stays static. For "10 tests at once", running them through the same warp-host file already shares the code; the lever
there is the anon heap (or one process running several tests), not the library layout.
