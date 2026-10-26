# seed/

`compiler-self.wasm`: the seed compiler, lib/compiler/compiler.warp compiled by itself at a fixpoint (stage N+1 ==
stage N, byte for byte). It is what the self lane starts from when no compiler-self.wasm has been built yet, so
compiling the compiler needs no Rust compiler (card rust-bootstrap, notes/handover/rust-bootstrap.md).

- Refresh: `scripts/slot.sh scripts/refresh-seed.sh [<tree>]` (seed → A → B (→ C), replaced only at the fixpoint;
  `--check` replaces nothing). Do it when lib/compiler changes in a way the old seed can't compile, or to pick up
  compiler fixes in the seed itself.
- Provenance of the current seed: see the commit that last changed it (`git log -- seed/compiler-self.wasm`).
