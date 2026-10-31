# Bisecting a regression with the stored compilers (no build)

Every built compiler stays in the store, /Volumes/Build/compilers/<hash>/, and its warp.wasm.inputs names its commit
(`<hash> commit=<sha10> built=<time>`). So the commit that broke a test case can be found without building:

1. Index the store once: `built=… commit=… <hash>` a line, one line per stored compiler.
2. For a first-parent main commit with a stored compiler: check it out in a spare worktree, copy that hash's
   warp.wasm, runtime.wasm and modules/ into its lib/, and run the case with `WARP_LIB=<worktree>/lib warp '<code>'`.
3. Binary search over those commits; then read the commits between the last good one and the first bad one.

data/full-suite/bisect.sh (ignored, in the main checkout) does this: `bisect.sh --find <expected> '<code>'`.
Its worktree is /Volumes/Worktrees/regress-bisect.

Pitfalls (2026-10-31):
- The store starts at 10-29 10:48. A case already red there can't be bisected this way: read the code instead
  (`warp lower '<code>'` shows the program after the passes, which found three causes).
- The first run of each warp.wasm compiles its machine code (~1 min a run).
- `warp '<code>'` prints the value as shown; `warp test` reads it back as data (WARP_VALUE_AS_DATA), runs each case in
  its file's folder, and may differ: `type(ø)` printed `empty` but read back as ø.
- The 10-28 full suite ran under the Rust harness at the repo root, so some "regressions" after it are harness
  differences (relative paths like "Cargo.toml", read-back of values).
