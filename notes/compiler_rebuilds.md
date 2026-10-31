# Compiler rebuilds: getting warp.wasm / warp-self.wasm without building them (card compiler-rebuild-policy)

A self build takes 5-11 minutes and has been killed for lack of memory on the box. Most builds were not needed:
the investigation of 2026-10-23 is in /workspace/cow/compiler-rebuilds.md on the box.

## Rules

- Nothing rebuilds the compilers automatically because lib/ changed: no hook, watcher or test triggers a build, and no
  user-facing binary (warp-host, a built executable) rebuilds a compiler or the runtime at run time. A stale pair
  can at most print a hint. `scripts/rebuild.sh` builds only when it is run by hand, and `scripts/self-check.sh` builds
  only with `--rebuild`.
- warp.wasm (plain `warp`'s) is the one build: the seed (warp-seed.wasm beside the binary, user 2026-10-26: out of git, each
  refresh published as a GitHub pre-release seed-<date>-<sha8> by scripts/refresh-seed.sh) compiles core/compiler.warp.
  warp-self.wasm (`warp self`) is no build of its own: `warp rebuild self` lets warp.wasm compile compiler.warp again
  and copies warp.wasm to warp-self.wasm when it does (user 2026-10-26: "If the compiler can compile itself once more,
  we don't need the fixed point"; whether the output is byte-identical is only said); otherwise warp-self.wasm stays as
  it was. `scripts/promote-self.sh [--background] [<tree>]` runs it (through
  rebuild.sh self, so the store's promoted warp-self.wasm of these inputs is reused) every once in a while, in
  parallel, and never fails its caller. The gate and the normal rebuild need only warp.wasm.
  A passed self test is kept per baked hash (`compiler-inputs.sh --baked`: seed, core/, the lib/ files the compiler
  bakes in) as <store>/self-tested/<baked hash>; a tree of that baked hash, a library change, gets warp.wasm promoted
  with no compile (rebuild.sh logs `warp-self hit-baked`). Before it, 20 of the Integrator's 24 self compiles of
  2026-10-30 were pushes no worker had self-checked.
- Names: until 2026-10-26 the files were compiler.wasm and compiler-self.wasm (and the seed seed/compiler-self.wasm, in git until 2026-10-26). Every
  lookup (warp, warp-host, rebuild.sh, the launcher) still reads a file of the old name when the new one is missing;
  installs should write the new names.
- To get them, run `scripts/rebuild.sh [pure|self|both] [<worktree>]` (pure by default; both: warp-self.wasm only from
  lib/ or the store, never promoted there). It keeps the file in lib/ when
  lib/<compiler>.wasm.inputs names the current inputs hash. Otherwise it copies the shared store's pair of that hash.
  Only when neither has one does it build and publish the result. `--check` never builds and exits 3 on a miss.
- Where warp-host finds the pair (its lookup, card pure-c-imports, not repeated in the scripts): beside the binary
  first, then the lib/ folder itself (user 2026-10-26: a lib/ keeps its compilers in itself, no longer in
  lib/../data), then <prefix>/share/warp. So a pair only in lib/ loses to an older one beside the binary.
  rebuild.sh therefore copies the pair from lib/ beside the tree's warp-host (target/{debug,host,release}/) as well.
  It does not copy beside a lone warp binary, because before card compiler-cache-keys, warp-rust judges its own
  target/debug/ build by the lib/ path baked in it.
  A store entry is only ever a source: no lookup reads the store directly.
- `scripts/self-check.sh` checks the pair for the branch's inputs, taken from lib/ or the store. On a miss it says
  "REBUILD NEEDED" and checks the existing pair. `--rebuild` builds the missing pair.

## Failed builds (card build-failed-store, user 2026-10-30)

Every build of a hash happens once, whether it succeeds or fails. `warp rebuild` says each step before it runs
(`rebuild step seed|stage-two|optimize|runtime|modules`); when one fails, rebuild.sh keeps the failure in
<store>/<hash>/<compiler>.failed/: build.log (the whole output), step, message (what is printed) and, for a step after
the seed, the half-built warp.wasm.new and runtime.wasm. It prints the header, the failing step's output (its last 60
lines) and, last, `rebuild.sh: warp build-failed at step <step>, log <path> …`, the line smoke.sh, gate.sh,
promote-self.sh and after-push.sh show. A later request for the hash (--check too) prints the same at once and exits 1.
`--force` builds it again from the failed step: the stored artifacts go back into lib/ and `warp rebuild from <step>`
runs the rest; a success removes the record. That is for a fix outside the inputs (the warp binary, memory, a full or
read-only disk): a fix of the sources changes the hash, and the earlier steps' artifacts bake those sources in, so that
build starts at the seed.

## Build records (card build-keeps, user 2026-10-31)

Every build is self-contained and measured: <store>/<hash>/<compiler>.record/ (a failed build's in its .failed/record/,
self-check's stage two in <hash>/stage-two.record/) holds
- inputs.txt: hash, baked hash, tree, commit and branch, uncommitted core/lib files, the seed (sha, kept in
  <store>/seeds/), the warp binary (sha, path), the sha of every input file; sources.tar.gz: those files themselves
- build.log: the whole output
- time: `/usr/bin/time -l` of the compile (CPU, wall, peak memory); rebuilds.log's `built` line carries its CPU
- profile.txt, sample.gz, perf.map: the function profile of the whole run (WARP_PROFILE=perfmap and macOS sample every
  10 ms through scripts/profile.sh -a, from scripts/build-slot.sh WARP_BUILD_RECORD): no CPU cost, a few % wall.
So a build's time and hotspots are read from its record, never by building again; a lib-only rebake is recorded too.
Records start with build-keeps (2026-10-31); older store entries have only their one-line .inputs.

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
- <store>/rebuilds.log gets one line per rebuild.sh hit or miss: time, tree, hash, result.
- Seeded 2026-10-23 with the pair of warp-host-wasm-load 4e3808b76 (compiler-self sha1 6269c5f5…).

## Why a copied pair now works anywhere

A compiler bakes the absolute path of the core it was built in (`module_directory`). Before this card it read
`use os`, sugar rules, aliases and entities from that folder, so a pair copied from another tree used that tree's
lib/, or none once the tree was removed. That was the 2026-10-22 refusal "a call of run_wasm with 2 arguments".
- core/parsing/parser.warp `run_library()` (main, ab93badf7) reads WARP_LIB first.
- warp-host now gives the compiler run WARP_LIB, the lib/ it resolved: WARP_LIB, else the lib/ beside the binary, else the
  one above the program or the current folder (`environment` in crates/warp-runtime/src/warp_host/main.rs). A
  program's own run sees the environment unchanged.
- packaging/bin/warp exports the lib/ it resolves the same way: always for warp-host, and for warp-rust when the pair lies in that lib/'s ../data.
- tests/host/test_host_library.sh checks it: a warp-self.wasm built elsewhere sees a word added to a temporary
  lib's os.warp.

What src/ still needs (card compiler-cache-keys):
- standalone.rs should export WARP_LIB to the compiler run of `warp` / `warp self` (as warp-host does).
- `build_path` should stop choosing the build file by the lib/ *path* recorded in it.
- `judged_build` (standalone.rs:208) and `stage_key` (self_hosted.rs:673) should hash the compiler inputs, not all of lib/.
  Today any lib/ edit invalidates the stage cache, and the next self build compiles every stage again.

warp-host's own caches are not rebuilds. Machine code is cached by the module's bytes. The compiler's kept values
(its lowered runtime words, ~25 s) are keyed by the compiler's bytes and lib/'s contents, and they are computed again
when lib/ changes. That is a cache fill on the next compile, not a build of a compiler.
