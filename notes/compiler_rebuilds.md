# Compiler rebuilds: getting warp.wasm / warp-self.wasm without building them (card compiler-rebuild-policy)

A self build takes 5-11 minutes and has been killed for lack of memory on the box. Most builds were not needed.

## Rules

- Nothing rebuilds the compilers automatically because lib/ changed: no hook, watcher or test triggers a build, and no
  user-facing binary (the native warp, a built executable) rebuilds a compiler or the runtime at run time. A stale pair
  can at most print a hint. `scripts/rebuild.sh` builds only when it is run by hand, and `scripts/self-check.sh` builds
  only with `--rebuild`.
- warp.wasm (plain `warp`'s) is the one build: the seed (warp-seed.wasm beside the binary, out of git, each
  refresh published as a GitHub pre-release seed-<date>-<sha8> by scripts/refresh-seed.sh) compiles core/compiler.warp.
  A cron job refreshes it from main every third day at 05:47 (days 1, 4, 7 … of the month), so the compiler may use newer
  features; each new seed comes with a minor release: on success an urgent card asks the Supervisor to
  cut it on that commit (log ~/Library/Logs/warp/cron/refresh-seed.log, a failure becomes an urgent card).
  warp-self.wasm (`warp self`) is no build of its own: `warp rebuild self` lets warp.wasm compile compiler.warp again
  and copies warp.wasm to warp-self.wasm when it does ("If the compiler can compile itself once more,
  we don't need the fixed point"; whether the output is byte-identical is only said); otherwise warp-self.wasm stays as
  it was. `scripts/promote-self.sh [--background] [<tree>]` runs it (through
  rebuild.sh self, so the store's promoted warp-self.wasm of these inputs is reused) every once in a while, in
  parallel, and never fails its caller. The gate and the normal rebuild need only warp.wasm.
  A passed self test is kept per baked hash (`compiler-inputs.sh --code`: seed, core/, the lib/ files the compiler
  bakes in) as <store>/self-tested/<baked hash>; a tree of that baked hash, a library change, gets warp.wasm promoted
  with no compile (rebuild.sh logs `warp-self hit-code`).
- To get them, run `scripts/rebuild.sh [pure|self|both] [<worktree>]` (pure by default; both: warp-self.wasm only from
  lib/ or the store, never promoted there). It keeps the file in lib/ when
  lib/<compiler>.wasm.inputs names the current inputs hash. Otherwise it copies the shared store's pair of that hash.
  Only when neither has one does it build and publish the result. `--check` never builds and exits 3 on a miss.
- Where the native warp finds the pair (core/native/driver.warp library_found, compiler_path): in its lib/ folder,
  --lib, else the lib/ beside the executable, the folder its links lead to, then <prefix>/share/warp.
  A store entry is only ever a source: no lookup reads the store directly.
- `scripts/self-check.sh` checks the pair for the branch's inputs, taken from lib/ or the store. On a miss it says
  "REBUILD NEEDED" and checks the existing pair. `--rebuild` builds the missing pair.

## Failed builds (card build-failed-store)

Every build of a hash happens once, whether it succeeds or fails. `warp rebuild` says each step before it runs
(`rebuild step seed|optimize|runtime|modules`, and a bare `warp rebuild` then `native`: the native warp built anew
by the new warp.wasm; rebuild.sh runs `warp rebuild pure`, the compilers alone); when one fails, rebuild.sh keeps the failure in
<store>/<hash>/<compiler>.failed/: build.log (the whole output), step, message (what is printed) and, for a step after
the seed, the half-built warp.wasm.new and runtime.wasm. It prints the header, the failing step's output (its last 60
lines) and, last, `rebuild.sh: warp build-failed at step <step>, log <path> …`, the line smoke.sh, gate.sh,
promote-self.sh and after-push.sh show. A later request for the hash (--check too) prints the same at once and exits 1.
`--force` builds it again from the failed step: the stored artifacts go back into lib/ and `warp rebuild from <step>`
runs the rest; a success removes the record. That is for a fix outside the inputs (the warp binary, memory, a full or
read-only disk): a fix of the sources changes the hash, and the earlier steps' artifacts bake those sources in, so that
build starts at the seed.

## Build records (card build-keeps)

Every build is self-contained and measured: <store>/<hash>/<compiler>.record/ (a failed build's in its .failed/record/,
self-check's stage two in <hash>/stage-two.record/) holds
- inputs.txt: hash, baked hash, tree, the commit built (`<sha>+dirty` when the tree's inputs hold uncommitted changes,
  never taken for that commit's build by vitals.py) and branch, uncommitted core/lib files, the seed (sha, kept in
  <store>/seeds/), the warp binary (sha, path), the sha of every input file; sources.tar.gz: those files themselves
- build.log: the whole output
- time: `/usr/bin/time -l` of the compile (CPU, wall, peak memory); rebuilds.log's `built` line carries its CPU
- profile.txt, sample.gz, perf.map: the function profile of the whole run (warp --profile and macOS sample every
  10 ms through scripts/profile.sh -a, from scripts/build-slot.sh WARP_BUILD_RECORD): no CPU cost, a few % wall.
So a build's time and hotspots are read from its record, never by building again; a lib-only build (a stored compiler reused, its runtime and modules made anew) is recorded too.
A compiler's first run after it lands in the store is cold and reads about 3% more instructions retired (idioms
2bc2e14adc9fac6f: 3.76e12 first, then 3.6495e12 three times within 0.003%). So one record alone is not a comparison:
to compare two compilers, run each on the same fixed source, discard one warm-up run, and compare the runs after it
(card compile-cost-drift).
Records start with build-keeps; older store entries have only their one-line .inputs.

## The inputs hash (scripts/compiler-inputs.sh <lib> [-v] [--code])

16 hex digits. Paths are hashed relative to lib/, so the same inputs give the same hash in any tree. It covers:
- the seed (WARP_SEED, its SHA-256)
- core/**, extensions/**
- every .warp module of lib/ outside compiler/ and data/: the runtime, modules and library index are built beside the
  compiler from them (the compiler reads lib/ from disk, notes/library_zero_bake.md). `--code` hashes the compiler's
  code alone (seed, core/, extensions/, the lib/ files they compile in), so a lib/-only change reuses a stored compiler.

## The shared store

- box: /workspace/cow/compilers/<hash>/{warp.wasm,warp-self.wasm,*.inputs}
- Mac: /Volumes/Build/compilers/<hash>/…
- `main` in each store links to the hash of origin/main's last good pair. rebuild.sh moves it only after building a tree
  whose HEAD is origin/main with core unchanged.
- The directories are write-once: a published file is never overwritten.
- <store>/rebuilds.log gets one line per rebuild.sh hit or miss: time, tree, hash, result.

## A compiler's library (card asciitext-compile, user OK 11-11)

A compiler compares texts by NFC through lib/unicode's module, linked at run time. Its GC types are those of the
compiler that built it, not of the lib/ beside it: warp.wasm has the seed's, the modules warp.wasm writes have
warp.wasm's emitter's. So each compiler carries its builder's runtime.wasm and modules/unicode.wasm beside it,
<compiler>.library/ (core/native/driver.warp compiler_library; scripts/lib/modules.sh companion_files), and links
the installed warp's only when it has none. `warp rebuild seed` and the seed step make <seed>.library/ once, the
seed's own (a seed is a stage that compiled itself), and hand it to warp.wasm; `warp rebuild self` to warp-self.wasm;
refresh-seed.sh from stage to stage and to the new seed. The store, the install (after-push.sh) and published builds
carry it. A failed NFC call names the folder it linked. Transitional: card unicode-module-linking takes the compiler's
NFC into core, and the folders go.

## The store in short

A compiler is built once per inputs hash, machine-wide, whether it builds or fails (ea5cb68f6b, card
  build-failed-store): `scripts/rebuild.sh` asks the store /Volumes/Build/compilers/<hash>/ first and copies a hit in
  seconds, whoever built it; while another tree builds the same hash (lock <store>/<hash>/warp.building, its pid) it
  logs `waiting-for-<pid>` and takes that result. A failed build is kept there too (<hash>/warp.failed/: build.log,
  the failing step, its half-built artifacts), and a request for that hash prints it at once: its last line names the
  step (seed, stage-two, optimize, runtime, modules) and the log. Read that log, never rebuild to see an error; a
  fixed source is a new hash and builds anew. `--force` builds the same hash again, from the failed step on, only
  after a fix outside the sources (the warp binary, memory, disk). self-check's stage two is kept the same way
  (<hash>/stage-two.passed or .failed/; `self-check.sh --rebuild` runs it again). So never copy compilers by hand,
  never rebuild "to be sure", never use --force to get past a wait, and never kill another session's build.
  Every build keeps its full record (<hash>/warp.record/: inputs and sources, build.log, CPU and memory, a function
  profile; notes/compiler_rebuilds.md "Build records"): read a build's time and profile there, never build to measure. Every
  hit, miss, wait and failure is a line of <store>/rebuilds.log.
  Only core/**, extensions/** and the seed are a compiler's code (compiler-inputs.sh --code-files); lib/ is never
  part of it. A lib/-only change compiles nothing: rebuild.sh takes the stored compiler of the same code
  (<store>/code/<code hash>) and makes lib/'s runtime, modules and index anew (`miss-rebaking-<hash>`, about a
  minute; decision lib-hash-rebake; a failed rebake is kept like a failed build, step rebake). So a library feature
  costs no compiler build, a core one does.
