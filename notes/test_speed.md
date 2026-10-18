# Test speed

Where the full suite's time goes and what keeps the gate fast. Logs of measured runs: data/perf/.

## The run is throughput-bound
- 16 test threads on 10 cores: the summed test time divided by 16 is the binary's wall clock. No single test is the
  critical path: cutting any test-second saves ~1/16 s; only what many tests pay counts.
- Median test 0.6 s, p90 6.3 s, p99 33 s; tests of ≥5 s are ~73 % of the summed time. 85 % of the time is
  in-process eval, 15 % in the files that start the warp binary or another command.
- A test's time is mostly the warp compiler: 85 % of a `use sound` compile is `modules::resolve` →
  `lower_module_source`, lowering lib/sound.warp, lib/prelude.warp and the modules they use again in every program
  (card cache-lowered: a cache keyed by module source and the flags that change lowering).
- Cache-dependent tests pay a first build kept between runs: the Lean build (types::test_type_model, 110 s cold,
  3 s warm), crates.io components (ffi::test_cargo_crates, ~90 s each cold), the release runtime stub.

## What the gate does
- warp-lang at opt-level 2 in test.sh only (`profile.dev.package.warp-lang.opt-level`, CARGO_INCREMENTAL=0: the o2
  build reused stale codegen units and failed to link). Alone o2 compiles programs 5–7× faster than o0, inside the
  loaded suite 2×; o1 gains only ~2× alone, so it is not used. Workers keep o0 for their incremental builds. rustc of
  warp-lang at o2 takes up to 6.2 GB: the queue caps only test processes, never rustc/sccache/linkers
  (UNCAPPED_PROGRAMS); test.sh's default cap is 8 GB.
- The release runtime stub is prebuilt before the run (no test calls cargo, so none waits on the shared target lock).
- The tests binary runs directly (taken from `--no-run --message-format=json`) instead of a second `cargo test`; it
  needs the [env] of .cargo/config.toml (WARP_TEST_TABLES), which test.sh exports as cargo does.
- Each sample runs once: test_playground_samples covers the menu, test_samples_run_cleanly the rest (decision
  gate-cuts).
- The self-hosting twins check every 4th source in the gate, rotating daily so four gates sweep every source; the
  nightly tier runs all.
- The full run's time limit per test binary is 1800 s (targeted runs 600 s, WARP_TEST_TIME_LIMIT). A crashed or killed
  binary reruns its unfinished and failed tests in JSON (tests/crash_report.py: panics, the tests running at a second
  crash); libtest alone prints panics only in its final section.

## The nightly tier
tests/nightly.sh, crontab 03:10, runs the tests the gate samples (`parser::test_self_`, WARP_NIGHTLY=1: every source)
on origin/main in the detached worktree /Volumes/Worktrees/nightly, log data/nightly/<date>.log; a failure files a
card. Not GitHub's "Rust CI" nightly (card github-rust). Candidates if the gate must shrink further: the Lean proofs,
ffi::test_cargo_crates.

## The queue
Targeted runs go 3 at once, a full run alone (notes/roles.md "Enforcement"). The queue builds before taking a slot,
so slots run tests back to back. Never rename a ticket by hand: the process waits under its old name, so a renamed ticket
deadlocks the queue.

Not worth cutting: sleeps (all ≤200 ms polling for servers), the small tests (<0.3 s).
