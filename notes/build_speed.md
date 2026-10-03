# Build speed

## One shared build (rule)
All builds, including agents' scratch exports, go to the one target dir from `~/.cargo/config.toml`
(`target-dir = "/Users/me/.cargo/shared-target"`). Never set `CARGO_TARGET_DIR` per agent or per export: in the
2026-10-02 fixer round 13 per-agent dirs (`/opt/cargo/warp-<topic>`) took 19–25 GB each (~280 GB) and each recompiled
every dependency. The shared dir's cargo lock also queues concurrent builds, which keeps the CPU load down.
Agents run targeted tests only (`CARGO_BUILD_JOBS=2 cargo --offline test --all-features --test <file> -- --test-threads=2`);
the supervisor runs the one full suite before merging.

## Already in place (~/.cargo/config.toml)
- `rustc-wrapper = "sccache"`: caches compiled crates across checkouts/exports at different paths.
- `[profile.dev] debug = "line-tables-only"`: config profiles override Cargo.toml's `debug = true`.

## Worth doing, by expected gain
1. Fewer test binaries: `tests/` has ~197 files = ~197 test binaries, each linked separately (ld at 500–700 MB each
   was what swamped the machine). Grouping them as modules of a few test crates (`tests/suite/main.rs` + `mod x;`)
   links a handful of binaries instead. Biggest win; moves test files, so only with the user's go.
2. `cargo check --tests` for fast feedback while editing (no codegen, no linking); build/test only to run.
3. Linker: mold is Linux-only; on macOS Apple's ld-prime (Xcode 15+) is already fast. `ld64.lld` is installed
   (~/.swiftly/bin) and can be tried with `-C link-arg=-fuse-ld=lld` in `[target.aarch64-apple-darwin] rustflags`;
   measure before adopting.
4. `cargo build --timings` once to see which crates dominate (likely wasmtime/cranelift, wasmer, wasmedge).
5. Features: the three runtime backends (wasmtime, wasmer, wasmedge) are the heavy deps; builds and agents that only
   need wasmtime should not pass `--all-features`.
Not useful here: splitting crates is a large refactor for unclear gain; debug-info tuning is already done.
