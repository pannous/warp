# Build speed

## One shared build (rule)
All builds, including agents' worktrees and exports, go to the one target dir from `~/.cargo/config.toml`
(`target-dir`). Never set `CARGO_TARGET_DIR` per agent or per export: a per-agent dir takes ~20 GB and recompiles every
dependency. The shared dir's cargo lock also queues concurrent builds. The `.noindex` suffix of the dir keeps Spotlight
out of it.

## In place
- `rustc-wrapper = "sccache"` (~/.cargo/config.toml): caches compiled crates across checkouts at different paths.
- `[profile.dev] debug = "line-tables-only"` in the config (overrides Cargo.toml's `debug = true`).
- One test crate: `autotests = false` + `tests/main.rs` declaring every folder as a module; run one file with
  `--test tests <file_stem>::`.
- Features: besides `native` only `ffi` exists, so `--all-features` builds the default program plus the FFI tests.
- P180 split-debuginfo stays "unpacked" (keeps file:line in backtraces); the hourly sweep removes the object files.

## Tips
- `cargo check --tests` for fast feedback while editing (no codegen, no linking).
- `ld64.lld` (~/.swiftly/bin) could be tried with `-C link-arg=-fuse-ld=lld`; measure before adopting.
- `cargo build --timings` shows which crates dominate.
- Building warp at opt-level 1 for tests was tried and dropped: slow tests ran 1.75× faster, but incremental test
  builds doubled. Cranelift's incremental cache made tests slower.

## Pitfall: copies share one warp artifact
Every copy of warp building into the shared target dir (worktrees, exports) maps to the same warp lib artifact: cargo's
metadata hash ignores the path of a root package and the fingerprint compares mtimes of relative paths, so a copy can
report "Fresh warp" and run another checkout's code. Fix: a unique pre-release `version = "0.1.1-<topic>"` in each
copy's Cargo.toml, never committed (`scripts/version-tweak.sh`, notes/agents/common.md). Still shared by every copy:
the binary `debug/warp`; `scripts/own-warp.sh` keeps a private copy under `scratch/warp`.

## Shared target sweep
macOS keeps every link's object files (`*.rcgu.o`) next to the binary for its debug info (~30 GB an hour with several
agents). `~/dev/bin/prune-cargo-target.sh` (cron hourly at :17, log ~/.companion/logs/prune-cargo-target.log,
`--dry-run`) deletes, never anything written in the last hour:
1. `*.rcgu.o` older than 1 hour (only Rust backtraces lose file:line)
2. artifacts of a warp worktree that no longer exists (its `*.d` dep-info names the worktree's sources)
3. warp's own artifacts (tests-, warp-, libwarp-, warp_runtime-…) untouched for 12 hours
4. any artifact group (name-hash) not read (atime) for 2 days: old dependency versions
5. incremental sessions older than 12 hours, then the oldest above a 20 GB cap
Below 50 GB free afterwards it notifies and messages the supervisor, at most every 6 hours.

## Test-suite wall clock
test.sh prints `TIMING: compile N s, run N s; tests summed N s (N s per thread)` and keeps every test's seconds in
data/test_times.txt. When the per-thread sum is close to the run's wall clock, the run is bound by total test time, not
by one slow test. More: notes/test_speed.md.

## P91: one analysis per program state (src/compiler/analysis_memo.rs)
extract_user_functions (and every EffectReport::of) of a fresh Context is remembered per thread by a fingerprint of the
whole tree, positions included, the last 256 states. A hit restores exactly the fields the analysis writes. Not
remembered: an analysis that changed required_functions or said something (diagnostic::said()), so a repeat says it
again. `WARP_ANALYSIS_CACHE=off` for A/B. `late_binding::functions_in` uses the cheaper `analyzer::defined_functions`
(no parameter or return kind inference).
