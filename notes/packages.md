# Packages: `use <name>` falls back to the registry

- `use name` (src/modules.rs): a local `name.wasp` in the search directories wins; else a builtin or FFI library; a name
  in the registry `packages.wasp` (repository root, compiled in with `include_str!`) is fetched by a shallow `git clone`
  into `packages/<name>` below the working directory (gitignored), once, and its `<name>.wasp` loaded like any module.
  A package without `<name>.wasp` only brings files.
- `module_directory`: compile-time text, the directory of the file it is written in ("." for the program), so a package
  reads its own files: uniscript.wasp has `read(module_directory + "/data/entities.idx")`. `read` itself is relative to
  the working directory at run time; a fetched package is below it, so the page preloads packages/uniscript/data/entities.idx.
- Concurrency: a process-wide mutex; parallel processes clone into `.<name>.fetching.<pid>` and rename, the loser deletes its copy.
- Pinned packages: `name: {repository: "git url", version: 1.2.3}` in packages.wasp is the git tag `v1.2.3` (or `1.2.3`),
  cloned once per machine into `~/.cache/warp/packages/<name>@1.2.3` and linked as `packages/<name>`; versioned `use`
  (`packages/<name>@<version>`) links the same cache. A tag never changes, so exports, worktrees and test runs never
  fetch again; raising the pinned version is the update (the stale link is replaced on the next `use`).
  uniscript is pinned: unpinned, every fresh export cloned the moving default branch (inline-tag warnings, eaten
  whitespace) and 6-7 of warp's uniscript tests failed there.
- uniscript ships its compiled entity data, `data/entities.idx`; warp only reads it and never compiles the
  `data/entities/**/*.wasp` sources. What was slow was compiling `use uniscript` itself: ~8 s per program, almost all
  of it re-parsing the C headers of m, c and SDL2 for every name the analyzer met (`ffi::get_signatures_from_headers`,
  `get_ffi_signatures`); parsed once per process now: ~0.9 s, tests/web/test_uniscript.rs 335 s → 24 s.
  A prebuilt uniscript.wasm linked into programs would need cross-module linking (shared memory for $String ptr/len,
  identical GC rec groups); not there yet.
- Package tools are prebuilt WebAssembly, never built from source into a shared target (src/package_tools.rs):
  `run_package_tool(name, args)` / `warp tool <name> args…` runs `<name>.wasm`, a wasm32-wasip1 command, in-process
  under wasmtime-wasi with the package directory preopened as `.`. One artifact for every OS, no per-platform binaries.
  It comes from `<package>/<name>.wasm`, else the GitHub release asset `<name>.wasm` of the pinned tag (v1.2.3 or 1.2.3),
  else (loudly) `cargo build --target wasm32-wasip1 --bin <name>` with CARGO_TARGET_DIR in the package's own build
  directory: `~/.cache/warp/packages/<name>@<version>.build` (pinned clone) or `packages/.build/<name>` (anything else).
  Why: on 2026-10-02 16:54 tests/web/test_uniscript.rs `the_index_matches_the_readable_entities` ran `cargo run --release
  -- check` inside the fetched ~/.cache/warp/packages/uniscript@1.0.0 without CARGO_TARGET_DIR, so cargo used
  ~/.cargo/config.toml's shared target-dir and, as the same crate uniscript 1.0.0, overwrote ~/dev/uniscript's binary
  and rlib (Sublime lost completions; its main.rs compiled against that rlib). With CARGO_TARGET_DIR set, the same
  `cargo run` would have built the package into warp's own target instead. Never run cargo inside a package directory.
  uniscript's v1.0.0 release ships uniscript.wasm (4.2 MB): the first run downloads it once per machine, no build.
  A package publishes its tool with `cargo build --release --target wasm32-wasip1 --bin <name>` as release asset <name>.wasm.
- A local checkout stands in for a fetch: `ln -s ~/dev/uniscript packages/uniscript`. Update: `git -C packages/uniscript pull`.
  For a pinned package, a clean clone of its repository in packages/<name> (what an unpinned fetch left) moves to
  packages/.replaced/ and the pin takes its place; a link outside the cache or a clone with changes wins, with a warning.
- First package: uniscript (github.com/pannous/uniscript): its uniscript.wasp and data replaced warp's lib/uniscript.wasp
  and data/uniscript/. warp's tests/web/test_uniscript.rs still tests it (through `use uniscript`).
- Versions (src/lowering/versions.rs): `use x version 1.2.3` exactly, `use x from 1.2.3` / `use x >= 1.2.3` that or later.
  The default branch's module declares its version with a top level `version 1.2.3`; if it does not satisfy the
  requirement, the best git tag (`v1.2.3` or `1.2.3`, via `git ls-remote --tags`) is cloned into packages/<name>@<version>.
  A local module named in a versioned `use` must declare a satisfying version.
- `version` is a soft keyword (like Python's `match`): only before digits or a text; `version = 2` stays a variable.
  `1.2.3` and `v1.2.3` (two dots or more) lex as version literals (`v2`, `v1.2` stay names); `version 1.10` keeps 1.10. Versions compare part by part at
  compile time (`1.9 < version 1.10`, `1.2.0 == 1.2`), otherwise they are their text.
- Command line: `warp use uniscript >= 1.0` is just the program, fetching like any other.
- Open: registry as its own repository (later); `fetch_package_version` still lists the remote tags on every versioned `use`.
