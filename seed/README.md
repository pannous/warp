# seed/

`compiler-self.wasm`: the seed compiler, lib/compiler/compiler.warp compiled by itself at a fixpoint (stage N+1 ==
stage N, byte for byte). It is what the self lane starts from when no compiler-self.wasm has been built yet, so
compiling the compiler needs no Rust compiler (card rust-bootstrap, notes/handover/rust-bootstrap.md).

- Refresh: `scripts/slot.sh scripts/refresh-seed.sh [<tree>]` (seed → A → B (→ C), replaced only at the fixpoint;
  `--check` replaces nothing). Do it when lib/compiler changes in a way the old seed can't compile, or to pick up
  compiler fixes in the seed itself.
- Provenance of the current seed: see the commit that last changed it (`git log -- seed/compiler-self.wasm`).
- Current seed: sha256 f1bda08846a0534523904b74bb2600f15beb103249da73cd1036ea4732e2e888 (1519065 B), refreshed on
  branch seed-refresh (lib/ of 3dd48ce45 plus the padded fix) in /Volumes/Worktrees/seed-refresh: old seed 3197e7ff → A
  91febb7b → B f1bda088 → C f1bda088 (C == B, the fixpoint). The old seed could not compile commands.warp's `" " * n`
  (char * int).
