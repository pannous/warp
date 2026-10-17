# Test speed (warp-performance-tester, 2026-10-16)

Where the full suite's 15–18 min go and what to cut. Two full runs of ./test.sh on the same commit 4b5eab8b0, in the
perf-tester worktree through tests/queue.sh, under the usual load (~20 queued worker runs, load average 18–28):
run 1 as the gate builds today (warp-lang at opt-level 0), run 2 with warp-lang at opt-level 2 (branch gate-opt).
Logs with timestamps: data/perf/run1_o0.log, run2_o2.log; per-test times: data/perf/times_o0.txt, times_o2.txt,
both side by side: data/perf/o0_o2.txt.

| | run 1, opt-level 0 | run 2, opt-level 2 |
|---|---|---|
| queue wait before the run | 34 s | 191 s |
| build of the tests binary | 51 s | 181 s + 36 s waiting on the shared target's lock |
| second `cargo test` (waits on the target lock, builds nothing) | 18 s | 47 s |
| tests binary | 982 s | 677 s |
| summed test time | 15 645 s | 10 054 s |
| summed, without the 43 cache-dependent tests (below) | 14 424 s | 7 059 s (2.04× faster) |
| result | 6082 passed, 1 failed (web::test_guide, a main bug) | the same |

Cache-dependent tests: their time is a first build of something kept between runs, and a fresh worktree or a new
build configuration pays it. Run 1 paid the Lean build (types::test_type_model, 586 s summed) and the crates.io
components (ffi::test_cargo_crates, test_crate_errors, 537 s); run 2 paid the release runtime stub: 14 executable tests
(wasm::test_executable_*, test_build_emits_wasm, web::test_built_app, …) each waited ~175 s on `cargo build --release
-p warp-runtime` (src/cli/executables.rs build_runtime_stub), i.e. on the shared target dir's lock while other worktrees
built. The integrate worktree has them warm most of the time.

## The run is throughput-bound
- 16 test threads on 10 cores: the summed test time divided by 16 is the binary's wall clock (15 645 / 16 = 978 s of
  982 s). No single test is the critical path: cutting any test-second saves ~1/16 s; only what many tests pay counts.
- Run 1 distribution: median test 0.6 s, p90 6.3 s, p99 33 s. Tests of ≥5 s (656) are 73 % of the summed time, the
  4232 tests under 1 s 10 %.
- 85 % of the time is in-process (eval in the tests binary), 15 % in the 100 files that start the warp binary or
  another command.

## Where a test's time goes: the warp compiler
`warp compile --wasm` vs running the module (debug binary, quiet moments, user CPU s):

| program | compile | run |
|---|---|---|
| `use sound; pattern("c4 e4 g4 ~", 1s)` | 4.7 s | 0.2 s |
| `use "lib/compiler/lexer.warp"; [numbers(read(…)) × 64 files]` (one self-lexer chunk) | 3.7 s | <0.6 s |

`sample` of the first: 85 % of the compile is `modules::resolve` → `Loader::use_std_module` → `lower_module_source`:
lowering lib/sound.warp, lib/prelude.warp and the modules they use, again in every program; the rest is debug-build
overhead (memmove, Node clone/drop, drop_meta, precondition checks).

The same programs with warp-lang at other opt-levels (`--config 'profile.dev.package.warp-lang.opt-level=N'`; debug
assertions and overflow checks unchanged), user CPU s:

| program | o0 | o1 | o2 |
|---|---|---|---|
| `2+3` | 0.33 | 0.17 | 0.06 |
| `use math; 3` | 0.97 | 0.39 | 0.15 |
| `use sound; 1` | 1.79 | 0.88 | 0.34 |
| sound pattern, compile only | 3.85 | 2.22 | 0.75 |
| self-lexer chunk, whole run | 4.29 | – | 0.59 |

Alone o2 is 5–7× faster; inside the loaded suite 2× (memory bandwidth and allocator contention of 16 threads, fixed
costs). o1 gains only ~2× alone, which is why build_speed.md's 2026-10-05 trial (o1) was dropped. Building the warp
binary alone: o0 50 s wall / 61 s CPU, o1 74 s / 268 s, o2 106 s / 355 s. rustc of warp-lang at o2 peaks at 3 GB RSS,
4.2 GB in top's MEM: over the queue's 4 GB per-process cap, which killed the first o2 run (gate-opt sets 6 GB).

## Ranked cuts (gate wall clock, expected saving)
1. **warp-lang at opt-level 2 in test.sh only** (branch gate-opt, card full-suite, Ready): tests binary
   ~920 → ~450 s with warm caches, build +130 s: **~5–6 min per gate**. Workers keep o0 (their incremental builds).
2. **Prebuild the runtime stub before the run** (test.sh: `cargo build --release -p warp-runtime` once, then
   `WARP_RUNTIME_STUB`/`WARP_PAGE_STUB` for the run, or a test-side OnceLock): no test calls cargo, so no test waits
   on the shared target lock while other worktrees build. Run 2: 14 tests × 175 s = 2450 test-s, **~2.5 min on a bad
   run**, 0 on a quiet one. Same pattern for ffi::test_cargo_crates (~90 s each when cold).
3. **Run the tests binary directly** instead of a second `cargo test` (take the executable from `--no-run
   --message-format=json`): the second cargo only waits for the target lock, **18–47 s** per gate, the same for every
   targeted run in the queue.
4. **One pass over the samples instead of two**: programs::test_samples_run_cleanly (5 slices) and
   test_playground_samples (5 slices) evaluate nearly the same samples (all minus window samples vs. the menu):
   one eval per sample checking both expectations. 496 + 462 test-s at o0, ~250 at o2: **~30 s / ~15 s**.
5. **Self-hosting twins**: parser::test_self_lexer runs lexer.warp over all 1637 samples and probes three times (numbers,
   texts, spans; 26 programs of 64 sources each), test_self_parser 995 sources, test_self_serializer all: 1094 test-s
   at o0, 478 at o2. One lexer program returning all three views saves 2/3 of the lexer's; or the gate takes a fixed
   sample (every 4th source) and a nightly run all: **~40 s / ~20 s**.
6. **Lowered std modules cached per process** (code, card material): 85 % of a `use sound` compile is lowering the
   same lib/*.warp again. A cache keyed by module source and the compile flags that change lowering would cut most
   module-using tests' compile; speeds up users' runs too. Expected: a large share of the remaining test time; needs a
   design for the context the lowering reads (program_std_modules, hidden words).
7. **Queue: build outside the lock** (tests/queue.sh): every targeted run holds the machine lock through its build
   (~50–150 s): of 354 ten-second samples of the lock holder 16:37–17:40 (probes/perf/queue_sampler.sh), 121 were
   building, 227 running tests, 6 idle, i.e. **a third of the lock time is builds**; `cargo test --no-run` before
   taking the ticket (builds still serialize on the target lock) would let the queue run tests back to back. Not the
   gate's wall clock but every worker's wait (20 runs queued at 16:30, the oldest 50 min).
8. **Queue robustness**: 16:10–16:37 the queue deadlocked: a ticket renamed by hand (1- → 0-, for priority) was first
   in line while its process waited under its old name; the lock stayed free while 20 runs waited. queue.sh could find
   its own ticket by PID (`*-$$`) so a rename can't wedge it.

## Status (2026-10-16 evening)
- On main: 1 gate-opt (fa3d09c60), o2-gate (gate-no-incremental: CARGO_INCREMENTAL=0, the o2 build reused stale
  codegen units and failed to link), 7 queue-prebuild, 2+3 gate-direct (0fddc30ac, first gate 6117 passed / 0
  failed): tests binary 415 s (summed 6609 s); running it directly needs the [env] of .cargo/config.toml
  (WARP_TEST_TABLES), which test.sh exports as cargo does. 6: card cache-lowered.
- Memory: rustc of warp-lang at o2 took 6.2 GB in a cold integrate build with other builds alongside and the 6 GB cap
  killed the gate (the Integrator reran it at 8 GB). queue-prebuild caps only tests now, never rustc/sccache/linkers
  (UNCAPPED_PROGRAMS); until it lands test.sh's default cap is 8 GB (gate-direct).
- Queue slots (card test-queue-slots, on main): targeted runs go 3 at once, a full run alone. A slot holder still
  waits on cargo's build-dir lock in its second `cargo test` while other worktrees build (~90 s each of two real
  runs); running the tests binary directly (3, gate-direct) would free the slot sooner for targeted runs too.
- Card gate-skip: the one-dispatch gate's "22 failed, no panic text" was the queue's 600 s per-binary time limit
  killing the tests binary at 602 s under load; libtest prints panics only in its final failures section. Branch
  gate-skip: limit 1800 s for the full run, and a crashed or killed binary reruns its unfinished and failed tests in
  JSON (tests/crash_report.py: panics, the tests running at a second crash). Proposed to the Integrator: clippy and
  the wasm32 check during the test run phase (no cargo lock held then), cargo fix only when clippy finds lints
  (data/perf/gate_parallel_lint.sh).
- 4 and 5 decided by the user (notes/decisions.md gate-cuts): branches samples-once and twins-gate.
- **The nightly tier**: tests/nightly.sh (branch twins-gate), crontab 03:10, runs the tests the gate samples
  (`parser::test_self_`, WARP_NIGHTLY=1: every source) on origin/main in the detached worktree
  warp.worktrees.noindex/nightly, log data/nightly/<date>.log, a failure files a card. Not GitHub's "Rust CI"
  nightly: it fails every night on Linux before testing (card github-rust).

Not worth cutting: sleeps (117 calls, all ≤200 ms polling for servers), the small tests (<0.3 s, 261 test-s total).
Tests that could move to a nightly tier if the gate must shrink further: the twins (5), the sample sweeps beyond one
pass (4), types::test_type_model's Lean proofs (110 s cold, 3 s warm), ffi::test_cargo_crates (cold 90 s each).
