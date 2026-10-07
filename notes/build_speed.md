# Build speed

## One shared build (rule)
All builds, including agents' scratch exports, go to the one target dir from `~/.cargo/config.toml`
(`target-dir = "/Users/me/.cargo/shared-target.noindex"`). Never set `CARGO_TARGET_DIR` per agent or per export: in the
2026-10-02 fixer round 13 per-agent dirs (`/opt/cargo/warp-<topic>`) took 19–25 GB each (~280 GB) and each recompiled
every dependency. The shared dir's cargo lock also queues concurrent builds, which keeps the CPU load down.
Agents run targeted tests only (`CARGO_BUILD_JOBS=2 cargo --offline test --all-features --test tests <file_stem>:: -- --test-threads=2`);
the supervisor runs the one full suite before merging.

## Already in place (~/.cargo/config.toml)
- `rustc-wrapper = "sccache"`: caches compiled crates across checkouts/exports at different paths.
- `[profile.dev] debug = "line-tables-only"`: config profiles override Cargo.toml's `debug = true`.

## Worth doing, by expected gain
1. DONE 2026-10-03: one test crate. `tests/` had ~197 files = ~197 test binaries, each linked separately (ld at
   500–700 MB each swamped the machine). Now `autotests = false` + `tests/main.rs` declaring every file as a module:
   the full suite went from ~25 min to 128 s. Run one file with `--test tests <file_stem>::`.
2. `cargo check --tests` for fast feedback while editing (no codegen, no linking); build/test only to run.
3. Linker: mold is Linux-only; on macOS Apple's ld-prime (Xcode 15+) is already fast. `ld64.lld` is installed
   (~/.swiftly/bin) and can be tried with `-C link-arg=-fuse-ld=lld` in `[target.aarch64-apple-darwin] rustflags`;
   measure before adopting.
4. `cargo build --timings` once to see which crates dominate (likely wasmtime/cranelift).
5. Features: wasmtime is the only runtime; besides `native` only `optimizer` and `ffi` exist (the ~20 empty C++
   flags are gone, 2026-10-03), so `--all-features` builds the default program plus the optimizer and FFI tests.
6. DONE 2026-10-03: the warp binary no longer re-declares every module (`mod x;` in main.rs compiled the crate twice).
Not useful here: splitting crates is a large refactor for unclear gain; debug-info tuning is already done.

## Pitfall: copies share one warp artifact (2026-10-03)
Every copy of warp that builds into the shared target dir (worktrees, `git archive` exports, probes/*_export) maps to the
same warp lib artifact: cargo's metadata hash ignores the path of a root package and the fingerprint compares mtimes of
relative paths, so a copy can report "Fresh warp" and run another checkout's code, both ways.
Fix, verified: in each copy's Cargo.toml set a unique pre-release `version = "0.1.1-<topic>"` (never committed); the
version is part of the hash, so the copy links its own `deps/libwarp-<hash>.rlib` and test binary, dependencies stay shared.
Verified 2026-10-04 (`cargo build --lib --message-format=json`, `cargo test --no-run -v` in two worktrees): the same
version gives both copies `libwarp-ac4e8b88….rlib` (the second one "fresh", i.e. the first one's code), different
versions give different hashes for the lib and the tests binary.
The `crate-type = ["rlib"]` half of the old tweak is gone: Cargo.toml no longer lists a `cdylib` (the playground build
asks for it, web/playground/build.sh), which was what made cargo name the rlib without a hash. Still shared by every
copy: the uplifted `debug/libwarp.rlib` (nothing links it) and the binary `debug/warp`. The remaining shared artifact
that probes actually run is the bin: `scripts/own-warp.sh` builds offline and keeps a private copy under `scratch/warp`.

## Spotlight (2026-10-03)
The shared target dir is `~/.cargo/shared-target.noindex` (renamed from shared-target): the `.noindex` suffix keeps
Spotlight's mdworker processes out of ~90 GB of build output (16 of them were busy during a full-suite run).
Worker copies under probes/ are deleted once their branch is merged; keep one work copy per worker, no extra exports.

## Shared target sweep (card shared-cargo, 2026-10-07)
The shared target dir filled the disk twice (183 GB on 2026-10-03, 664 GB on 2026-10-07). The cause is mostly not the
per-branch dependency builds: macOS keeps every link's object files (`*.rcgu.o`, split-debuginfo=unpacked) next to the
binary for its debug info, and with several agents linking warp's test binary that is ~30 GB an hour (327 GB of
439 GB in debug/deps). Each worktree's version suffix adds its own warp/test artifacts on top.

`~/dev/bin/prune-cargo-target.sh` (Python; cron hourly at :17, log ~/.companion/logs/prune-cargo-target.log;
`--dry-run` prints what each rule would free) deletes, never anything written in the last hour (a running link's
inputs):
1. `*.rcgu.o` older than 1 hour: a binary runs without them; only Rust backtraces lose file:line
2. artifacts of a warp worktree that no longer exists (its `*.d` dep-info names the worktree's sources)
3. warp's own artifacts (tests-, warp-, libwarp-, warp_runtime-…) untouched for 12 hours
4. any artifact group (name-hash) not read (atime) for 2 days: old dependency versions
5. incremental sessions older than 12 hours, then the oldest above a 20 GB cap
Below 50 GB free afterwards it shows a notification and messages the supervisor session (warp-96) through
`claude -p` + SendMessage, at most every 6 hours. First run: 384.6 GB freed in 5 s, 512 GB free; a build afterwards
rebuilds only what rule 4 took (4 min for the test binary with an empty cache).

Decided (user, P180, 2026-10-07): split-debuginfo for `[profile.dev]` stays "unpacked" (the default, keeps file:line
in backtraces) and the hourly sweep above removes the kept object files; "off" was the recommendation. Measured 2026-10-07 on the test
binary (touch src/lib.rs, `cargo test --no-run --test tests`, cargo's own time): unpacked 6.1 s, 32 object files kept
per build (the default, swept hourly); off 6.1 s, no object files, Rust backtraces keep function names but lose
file:line; packed 47.9 s (dsymutil, a 287 MB .dSYM per test binary), file:line kept.

## Test-suite wall clock (card suite-wall, 2026-10-05)
test.sh prints `TIMING: compile N s, run N s; tests summed N s (N s per thread)` and keeps every test's seconds in
data/test_times.txt. When the per-thread sum is close to the run's wall clock, the threads were busy to the end: the
run is bound by total test time, not by one slow test, and only cutting what many tests pay helps.
- 2026-10-05 before: run 36 s. After the late-binding fix: run 22 s, tests summed 273 s (17 s per thread); the 180
  tests of 0.2 s or more were 234 s of it. web::test_uniscript alone was 64 s (every conversion compiled
  `use uniscript` again: now one program per test), upper/lower walked all code points per module (now once).
- Where a program's time goes (debug build, `use uniscript; uniscript("…")`, ~0.4 s): lowering 0.2 s (parse of the
  package 40 ms, late binding 115 → 30 ms, ~23 passes of ~6 ms that each rebuild the tree), emission 70 ms, run 90 ms
  (Cranelift on a module-cache miss). Small programs: ~5 ms lowering, ~2 ms emission, ~18 ms Cranelift on a miss.
- Tried and dropped: Cranelift's incremental cache (Config::enable_incremental_compilation, per-function): functions::
  + uniscript got slower, 32 → 42 s. Building warp itself at opt-level 1 for tests: the slow tests ran 1.75× faster,
  but an incremental test build grew from 7–12 s to 16–39 s, about what the run saves and a loss for every worker's
  targeted run.
- Left as they are: tests that sleep or overlap tasks on purpose (control::test_threads, test_job_lists), Lean proofs,
  the two-million-key map (runtime GC work).

## P91: one analysis per program state (src/analysis_memo.rs, branch shared-analysis, 2026-10-05)
- extract_user_functions (and every EffectReport::of, which calls it) of a fresh Context is remembered per thread by a
  fingerprint of the whole tree, positions included (LineInfo hashed; other Rust data makes a tree unremembered), the
  last 256 states. A hit restores exactly the fields the analysis writes (user_functions, ffi_imports, field_kinds,
  enclosing_functions, parameter_conflicts, closure_targets, closure_variable_targets). Not remembered: an analysis
  that changed required_functions or said something (diagnostic::said(): warnings, asks, hints, errors with fixes),
  so a repeat says it again. `WARP_ANALYSIS_CACHE=off` for A/B.
- Who analyses (uniscript program, 185 calls): late_binding functions_in per statement 133 calls 24 ms, law
  function_definition 39 calls 8 ms, whole-program analyses of lambdas/effects/type_tests/analyzer 2–10 ms each (each
  after a pass that changed the tree: those miss legitimately).
- Measured (debug, `warp lower 'use uniscript; uniscript("<:fracture A b c >")'`, 3 runs): lowering 165 → 143 ms
  (−13 %), analysis 78 → 56 ms; 66 of 185 analyses are hits. Typical test programs (functions:: operators::
  welcoming::, 1 thread, A/B twice): 12.9/9.6 s off vs 12.3/9.6 s on, within noise: their analyses are ~1 ms after the
  one-off header parse of the first.
- Next gains: functions_in needs only the definitions a statement makes, not the full inference (a cheaper extractor
  would remove most of late_binding's 24 ms), and whole-program analyses after passes that changed only far parts.
- late-binding-definitions (follow-up): `late_binding::functions_in` (also folding's) uses `analyzer::defined_functions`
  (extraction and closure registration, no parameter or return kind inference): its callers read names, parameters and
  bodies only. Checked by computing both on 1780 tests and the uniscript program: identical names, parameters, bodies.
  `warp lower` of the uniscript program: median 151 → 144 ms (12 runs each).
