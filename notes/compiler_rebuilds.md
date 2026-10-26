# Compiler rebuilds: getting warp.wasm / warp-self.wasm without building them (card compiler-rebuild-policy)

A self build takes 5-11 minutes and has been killed for lack of memory on the box. Most builds were not needed:
the investigation of 2026-10-23 is in /workspace/cow/compiler-rebuilds.md on the box.

## Rules

- Nothing rebuilds the compilers automatically because lib/ changed: no hook, watcher or test triggers a build, and no
  user-facing binary (warp-host, a built executable) rebuilds a compiler or the runtime at run time. A stale pair
  can at most print a hint. `scripts/rebuild.sh` builds only when it is run by hand, and `scripts/self-check.sh` builds
  only with `--rebuild`.
- warp.wasm (`warp pure`) is the one build: the seed (seed/warp-self.wasm) compiles core/compiler.warp.
  warp-self.wasm (`warp self`) is no build of its own: `warp rebuild self` lets warp.wasm compile compiler.warp again
  and copies warp.wasm to warp-self.wasm only when that output is byte-identical to warp.wasm's module (the fixpoint);
  otherwise warp-self.wasm stays as it was. `scripts/promote-self.sh [--background] [<tree>]` runs it (through
  rebuild.sh self, so the store's promoted warp-self.wasm of these inputs is reused) every once in a while, in
  parallel, and never fails its caller. The gate and the normal rebuild need only warp.wasm.
- Names: until 2026-10-26 the files were compiler.wasm and compiler-self.wasm (and seed/compiler-self.wasm). Every
  lookup (warp, warp-host, rebuild.sh, the launcher) still reads a file of the old name when the new one is missing;
  installs should write the new names.
- To get them, run `scripts/rebuild.sh [pure|self|both] [<worktree>]` (pure by default; both: warp-self.wasm only from
  data/ or the store, never promoted there). It keeps the file in data/ when
  data/<compiler>.wasm.inputs names the current inputs hash. Otherwise it copies the shared store's pair of that hash.
  Only when neither has one does it build in a heavy slot and publish the result. `--check` never builds and exits 3 on a miss.
- Where warp-host finds the pair (its lookup, card pure-c-imports, not repeated in the scripts): beside the binary
  first, then lib/../data, then <prefix>/share/warp. So a pair only in data/ loses to an older one beside the binary.
  rebuild.sh therefore copies the pair from data/ beside the tree's warp-host (target/{debug,host,release}/) as well.
  It does not copy beside a lone warp binary, because before card compiler-cache-keys, warp-rust judges its own
  target/debug/ build by the lib/ path baked in it.
  A store entry is only ever a source: no lookup reads the store directly.
- `scripts/self-check.sh` checks the pair for the branch's inputs, taken from data/ or the store. On a miss it says
  "REBUILD NEEDED" and checks the existing pair. `--rebuild` builds the missing pair, in a slot.

## The inputs hash (scripts/compiler-inputs.sh <lib> [<rust fingerprint>])

16 hex digits. Paths are hashed relative to lib/, so the same inputs give the same hash in any tree. It covers:
- core/**
- the Rust fingerprint: WARP_SOURCE of the warp binary that builds, the third line of a build file's warp.build
  section. rebuild.sh reads it from the warp.wasm beside that binary, or from WARP_RUST_FINGERPRINT.
- every .warp module of lib/ outside compiler/ and data/: the std modules, sugar, aliases, lib/native, lib/crates.
  The compiler bundles them (parser.warp LIBRARY_FILES, decision built-in-lib), so a built compiler reads no lib/
  from any disk. With WARP_LIB set, a module file found there wins over its bundled copy, per module: a worktree's
  lib/ edit takes effect without a rebuild, and the next rebuild bundles it.

## The shared store

- box: /workspace/cow/compilers/<hash>/{warp.wasm,warp-self.wasm,*.inputs}
- Mac: /Volumes/Build/compilers/<hash>/…
- `main` in each store links to the hash of origin/main's last good pair. rebuild.sh moves it only after building a tree
  whose HEAD is origin/main with core unchanged.
- The directories are write-once: a published file is never overwritten.
- <store>/rebuilds.log gets one line per rebuild.sh hit or miss and per scripts/slot.sh run: time, tree, hash, result.
- Seeded 2026-10-23 with the pair of warp-host-wasm-load 4e3808b76 (compiler-self sha1 6269c5f5…).

## Why a copied pair now works anywhere

A compiler bakes the absolute path of the core it was built in (`module_directory`). Before this card it read
`use os`, sugar rules, aliases and entities from that folder, so a pair copied from another tree used that tree's
lib/, or none once the tree was removed. That was the 2026-10-22 refusal "a call of run_wasm with 2 arguments".
- core/parser.warp `run_library()` (main, ab93badf7) reads WARP_LIB first.
- warp-host now gives the compiler run WARP_LIB, the lib/ it resolved: WARP_LIB, else the lib/ beside the binary, else the
  one above the program or the current folder (`environment` in crates/warp-runtime/src/warp_host/main.rs). A
  program's own run sees the environment unchanged.
- packaging/bin/warp exports the lib/ it resolves the same way: always for warp-host, and for warp-rust when the pair lies in that lib/'s ../data.
- tests/host/test_host_library.sh checks it: a warp-self.wasm built elsewhere sees a word added to a temporary
  lib's os.warp.

What src/ still needs (card compiler-cache-keys):
- standalone.rs should export WARP_LIB to the compiler run of `warp pure` / `warp self` (as warp-host does).
- `build_path` should stop choosing the build file by the lib/ *path* recorded in it.
- `judged_build` (standalone.rs:208) and `stage_key` (self_hosted.rs:673) should hash the compiler inputs, not all of lib/.
  Today any lib/ edit invalidates the stage cache, and the next self build compiles every stage again.

warp-host's own caches are not rebuilds. Machine code is cached by the module's bytes. The compiler's kept values
(its lowered runtime words, ~25 s) are keyed by the compiler's bytes and lib/'s contents, and they are computed again
when lib/ changes. That is a cache fill on the next compile, not a build of a compiler.
