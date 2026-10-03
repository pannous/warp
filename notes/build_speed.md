# Build speed

## One shared build (rule)
All builds, including agents' scratch exports, go to the one target dir from `~/.cargo/config.toml`
(`target-dir = "/Users/me/.cargo/shared-target"`). Never set `CARGO_TARGET_DIR` per agent or per export: in the
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
4. `cargo build --timings` once to see which crates dominate (likely wasmtime/cranelift, wasmer, wasmedge).
5. Features: the three runtime backends (wasmtime, wasmer, wasmedge) are the heavy deps; builds and agents that only
   need wasmtime should not pass `--all-features`.
Not useful here: splitting crates is a large refactor for unclear gain; debug-info tuning is already done.

## Pitfall: copies share one warp artifact (2026-10-03)
Every copy of warp that builds into the shared target dir (worktrees, `git archive` exports, probes/*_export) maps to the
same warp lib artifact: cargo's metadata hash ignores the path of a root package and the fingerprint compares mtimes of
relative paths, and because the lib is also a `cdylib` cargo names the rlib without a hash (`deps/libwarp.rlib`), so every
copy overwrites and links the same file. A copy can report "Fresh warp" and run another checkout's code, both ways.
Fix, verified: in each copy's Cargo.toml set a unique pre-release `version = "0.1.1-<topic>"` AND `crate-type = ["rlib"]`
(never committed); the copy then links its own `libwarp-<hash>.rlib`, dependencies stay shared.
