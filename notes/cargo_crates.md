# `use cargo <crate>`: crates.io crates as generated components (src/modules/cargo_crates/)

`use cargo regex`, `use crate "regex"`, `use cargo regex as re`: foreign_modules::crate_module rewrites the statement
into `use wasm "<cache>/regex@1.13.1.wasm" as regex`, so everything after it is the components path
(notes/stdlib_connectors.md "Step 3"): `lib.f(x)`, resources as handles, methods, operators.

## How a wrapper is made
1. A wrapper crate `warp-crate-<crate>` in `~/.cache/warp/crates/wrappers/<crate>`. Every wrapper is a member of one
   workspace (`~/.cache/warp/crates/Cargo.toml`, `members = ["wrappers/*"]`), so a configured target-dir is used and
   a user without one gets that workspace's single target/. CARGO_TARGET_DIR is never set.
2. `cargo metadata` resolves the version (`*` for now); the dependency is then pinned with `=<version>`.
3. `RUSTC_BOOTSTRAP=1 cargo rustdoc -p <crate>@<version> -- -Z unstable-options --output-format json` on any
   toolchain, read with serde_json (no rustdoc-types crate, so a format change is less likely to break it).
4. Api::of walks the root module: `use` re-exports followed, glob re-exports expanded, no submodules. It exports
   - free functions whose parameter and result types map,
   - structs without generic parameters that an inherent fn constructs (returns `Self`/the struct, or a
     Result/Option of it): a WIT resource. Its inherent pub fns become methods (`&self`, `&mut self` through a
     RefCell, `self` through a clone) or static funcs, its pub fields getter methods, and `Display` gives `to-string`.
5. Types: str/String/Cow<str>/PathBuf and generics bounded by AsRef<str>/Into<String>/Borrow/ToString/Display →
   string; AsRef<[u8]>/&[u8]/Vec<u8> → list<u8>; primitives (usize/isize as u64/s64); Vec, slices, Option, tuples of
   identity types; Result<T, E> → result<T, string> through `e.to_string()`; plain type aliases are resolved. A
   generic with any other trait bound is tried as a text (`R: Replacer` takes `&str`).
6. The compiler has the last word: `cargo build --message-format json`; each generated function is one line, so an
   error's line names the item to drop. Up to 12 rounds. Nothing left is the loud "no public function warp can call"
   (sha2: its API is the Digest trait).
7. The component and its WIT are copied to `~/.cache/warp/crates/components-v4/<crate>@<version>.{wasm,wit}`. Raise
   GENERATION (src/modules/cargo_crates/mod.rs) when the generated wrappers change, so old builds are never reused. A failed wrapper moves to
   `failed/<crate>`, because one unresolvable dependency would break every other member of the workspace.

## Calling
- `regex.Regex("a+")`: with no `[constructor]regex`, the call takes the static `new` (components.rs `function`).
- `semver.Version.parse(t)`: a call on a component's capitalized member read is the static `[static]version.parse`
  (foreign_modules::type_read). `RegexSet` finds `regex-set` (components::kebab_case).
- `v.major` without parentheses: a read of a handle's member is a call when its resource has that method without
  arguments (components::takes_no_arguments). Records stay plain values (`s.words`).
- A result<own<r>, …> or option<own<r>> counts as a handle for compile-time tracking (components::resource_inside).
- A text where list<u8> is wanted passes its UTF-8 (`hex.encode("hi")`).
- WASI: a component that links gets no trap stubs for unknown imports (components::serve_host_imports): a stub under
  the import's exact name would shadow the newer WASI version wasmtime serves (std imports 0.2.6, wasmtime-wasi 0.2.12).

## Prebuilt downloads
- Order: the newest cached component, else the newest hosted one, else the local build. Hosted = the assets
  `<crate>.<version>.wasm` of the prerelease `crates-v4` (generation = GENERATION in src/modules/cargo_crates/mod.rs), listed by the
  release API (`api.github.com/repos/pannous/warp/releases/tags/crates-v4`); a download must match the asset's
  `digest` (SHA-256 GitHub computes) and lands in the cache as `<crate>@<version>.wasm` (no .wit: the interface is in
  the component, `wasm-tools component wit` prints it). Any failure says why and falls back to the local build.
- `WARP_BUILD_CRATES=1` never downloads (the workflow itself).
- Names: the release profile strips only DWARF (`strip = "debuginfo"`), so the core
  module keeps its name section and a trap's backtrace names the Rust functions, demangled (wasmtime features demangle,
  addr2line): about 20% more bytes (strsim 79→95 KB, regex 1.1→1.4 MB). Inlined callees (regex-automata inside
  `Regex::shortest_match_at`) don't show up as frames there.
- `WARP_DEBUG_CRATES=1`: the profile `debugging` (debug = true, nothing stripped) gives `<crate>@<version>.debug.wasm`,
  hosted as `.debug.wasm` too; with wasm_backtrace_details on (components.rs engine) traps show every inlined frame with
  file:line (`regex-automata/src/util/search.rs:426`). 10 to 45 times the size (strsim 3.7 MB, regex 13.5 MB), so it's
  opt-in, not the default of debug warps.
- .github/workflows/crates.yml builds the crates of its CRATES list and uploads them (`gh release upload --clobber`):
  on pushes to main that touch it or src/modules/cargo_crates/, weekly, by dispatch. Add a crate there.
- Release assets, not ghcr: a new ghcr package starts private and only the web UI makes it public (no API).
- A hosted version may trail crates.io until the weekly run; a raised GENERATION needs one workflow run before
  downloads work again (until then everyone builds locally).

## Offline, timing
- A cached component of the crate is used without cargo or network, the newest version first. Otherwise every cargo
  command runs `--offline` first (the local registry) and then with the network.
- First build: strsim ~10 s, regex ~60 s (rustdoc and the wasm32 release build). After that it loads at once.
- cargo missing: "use cargo needs cargo (https://rustup.rs)"; target missing: "rustup target add wasm32-wasip2".

## Not yet
- versions in the use line (`use cargo regex version 1.11`): the newest cached build, else the newest release.
- the browser: card use-cargo-playground (prebuilt components served to the page through jco).
- Iterators (`find_iter`), borrowed results (`Match<'h>`), builders returning `&mut Self`, trait methods (sha2's
  `Digest::digest`), and enums as WIT enums/variants.
