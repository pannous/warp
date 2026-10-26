# seed/

`warp-self.wasm` (until 2026-10-26 `compiler-self.wasm`): the seed compiler, lib/compiler/compiler.warp compiled by
itself at a fixpoint (stage N+1 == stage N, byte for byte). `warp rebuild` builds warp.wasm with it, so compiling the
compiler needs no Rust compiler (card rust-bootstrap, notes/handover/rust-bootstrap.md). A warp whose checkout has
only the old name reads seed/compiler-self.wasm (transition).

- Refresh: `scripts/slot.sh scripts/refresh-seed.sh [<tree>]` (seed → A → B (→ C), replaced only at the fixpoint;
  `--check` replaces nothing). Do it when lib/compiler changes in a way the old seed can't compile, or to pick up
  compiler fixes in the seed itself.
- Provenance of the current seed: see the commit that last changed it (`git log --follow -- seed/warp-self.wasm`).
- warp-self.wasm in data/ or beside the binary (`warp self`'s compiler) is no build of its own: warp.wasm copied there
  by scripts/promote-self.sh (`warp rebuild self`) when it passes the fixpoint test.
