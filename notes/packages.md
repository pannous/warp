# Packages: `use <name>` falls back to the registry

- `use name` (src/modules.rs): a local `name.warp` in the search directories wins; else a builtin library; else a
  package: an entry of `packages.warp` (repository root, compiled in with `include_str!`) or a folder of the default
  registry github.com/pannous/warp-packages; else an FFI library. A package is fetched once into `packages/<name>`
  below the working directory (gitignored) and its `<name>.warp` loaded like any module; without it, it only brings files.
- Default registry (card build-sh, user 2026-10-17; packages.rs `registered`): one folder per package, `<name>/<name>.warp`
  plus the manifest `<name>/package.warp`; versions are tags `<name>@1.2.3`. Its default branch is cloned to
  `~/.cache/warp/packages/.registry`, refreshed when older than an hour (a missing folder does not force a fetch, so a C
  library's `use sqlite3` asks at most hourly). Latest = the clone's folder, linked as packages/<name>; a pin clones the
  whole registry at tag `<name>@1.2.3` into the cache and links packages/<name> to its folder. In the browser: raw files of
  `HEAD/<name>` (a folder exists when its package.warp does, so every folder has one).
- Manifest package.warp = the keys of a long entry (repository, version, library; description, license ignored).
  `repository:` present = a proxy (user's word): the package is that repository, its versions that repository's tags,
  the proxy's `version` the default pin (a packages.warp version wins), the proxy folder's `<name>.warp` the binding of
  its `library`. uniscript is the first proxy; folder packages: colors, geometry, webgl, ciphers, life (warp-package-examples).
- packages.warp only pins or redirects: `name: 1.2.3` (registry tag), `name: "owner/repo@1.2.3"` (unquoted `owner/repo@…`
  does not parse as one path), `name: owner/repo` / `"git url"` (default branch), long `{repository, version, library}`.
  A bare `name` there is the registry's folder (it was github.com/pannous/<name> before).
- `warp fetch [names]`: fetches them (none: every entry and registry folder); web/playground/build.sh pinned runs it for
  the browser suite, which reads packages/uniscript/data/entities.idx locally (pages.yml builds a native debug warp for it).
- `module_directory`: compile-time text, the directory of the file it is written in ("." for the program), so a package
  reads its own files: uniscript.wasp has `read(module_directory + "/data/entities.idx")`. `read` itself is relative to
  the working directory at run time; a fetched package is below it, so the page preloads packages/uniscript/data/entities.idx.
- Concurrency: a process-wide mutex; parallel processes clone into `.<name>.fetching.<pid>` and rename, the loser deletes its copy.
- Pinned packages: `name: {repository: "git url", version: 1.2.3}` in packages.warp is the git tag `v1.2.3` (or `1.2.3`),
  cloned once per machine into `~/.cache/warp/packages/<name>@1.2.3` and linked as `packages/<name>` (a proxy's version
  likewise); versioned `use`
  (`packages/<name>@<version>`) links the same cache. A tag never changes, so exports, worktrees and test runs never
  fetch again; raising the pinned version is the update (the stale link is replaced on the next `use`).  uniscript is pinned: unpinned, every fresh export would clone the moving default branch.
- uniscript ships its compiled entity data, `data/entities.idx`; warp only reads it and never compiles the
  `data/entities/**/*.warp` sources. C headers are parsed once per process
  (`ffi::get_signatures_from_headers`, `get_ffi_signatures`): `use uniscript` compiles in ~0.9 s.
  A prebuilt uniscript.wasm linked into programs would need cross-module linking (shared memory for $String ptr/len,
  identical GC rec groups); not there yet.
- Package tools are prebuilt WebAssembly, never built from source into a shared target (src/modules/package_tools.rs):
  `run_package_tool(name, args)` / `warp tool <name> args…` runs `<name>.wasm`, a wasm32-wasip1 command, in-process
  under wasmtime-wasi with the package directory preopened as `.`. One artifact for every OS, no per-platform binaries.
  It comes from `<package>/<name>.wasm`, else the GitHub release asset `<name>.wasm` of the pinned tag (v1.2.3 or 1.2.3),
  else (loudly) `cargo build --target wasm32-wasip1 --bin <name>` with CARGO_TARGET_DIR in the package's own build
  directory: `~/.cache/warp/packages/<name>@<version>.build` (pinned clone) or `packages/.build/<name>` (anything else).  Never run cargo inside a package directory: with the shared target-dir, a fetched uniscript of the same crate
  version overwrites ~/dev/uniscript's binary and rlib.
  uniscript's v1.0.0 release ships uniscript.wasm (4.2 MB): the first run downloads it once per machine, no build.
  A package publishes its tool with `cargo build --release --target wasm32-wasip1 --bin <name>` as release asset <name>.wasm.
- Libraries: `library: "c/ffi/uniscript_c"` in a manifest (or entry) makes `use <name>` load the binding `<name>.warp` of
  the declaring folder (the proxy's, else the package's own) as if it lay beside the library, so its
  `import uniscript_c` calls the core module uniscript_c.wasm with the header uniscript_c.h (texts through its malloc,
  notes/wasm_modules.md). Both come from the package itself or the pinned tag's release assets, downloaded once into the
  build directory (package_tools.rs library_directory), never built. A `use` requiring another version than the pin loads
  that tag's module source instead. Browsers cannot fetch release assets (no CORS): web/playground/build.sh `libraries`
  puts them in web/playground/packages/<name>/, which the browser loader reads as page:packages/<name>; a built site ships
  every imported module as modules/<file> (web/site_modules.rs). Updating: raise `version` in the proxy's package.warp (warp-packages); the
  package's release needs the assets (uniscript: notes/publishing.md there, `make -C c/ffi wasm`).
- A local checkout stands in for a fetch: `ln -s ~/dev/uniscript packages/uniscript`. Update: `git -C packages/uniscript pull`.
  For a pinned package, a clean clone of its repository in packages/<name> (what an unpinned fetch left) moves to
  packages/.replaced/ and the pin takes its place; a link outside the cache or a clone with changes wins, with a warning.
- First package: uniscript (github.com/pannous/uniscript): its uniscript.wasp and data replaced warp's lib/uniscript.warp
  and data/uniscript/. warp's tests/web/test_uniscript.rs still tests it (through `use uniscript`).
- Versions (src/lowering/data/versions.rs): `use x version 1.2.3` exactly, `use x from 1.2.3` / `use x >= 1.2.3` that or later.
  The default branch's module declares its version with a top level `version 1.2.3`; if it does not satisfy the
  requirement, the best git tag (`v1.2.3` or `1.2.3`, via `git ls-remote --tags`) is cloned into packages/<name>@<version>.
  A local module named in a versioned `use` must declare a satisfying version.
- `version` is a soft keyword (like Python's `match`): only before digits or a text; `version = 2` stays a variable.
  `1.2.3` and `v1.2.3` (two dots or more) lex as version literals (`v2`, `v1.2` stay names); `version 1.10` keeps 1.10. Versions compare part by part at
  compile time (`1.9 < version 1.10`, `1.2.0 == 1.2`), otherwise they are their text.
- Command line: `warp use uniscript >= 1.0` is just the program, fetching like any other.
- Open: `fetch_package_version` still lists the remote tags on every versioned `use`.
