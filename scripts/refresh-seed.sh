#!/bin/bash
# Refresh the seed compiler (warp-seed.wasm beside the warp binary, card rust-bootstrap): the seed compiles core/compiler.warp
# (stage A), stage A compiles it again (stage B), and only when B compiles it to the same bytes as itself (stage C == B,
# the fixpoint; or A == B at once) does B replace the seed. The seed is what `warp rebuild` builds warp.wasm with; no
# Rust-compiled stage takes part. (scripts/promote-self.sh is the light check of warp.wasm alone: N+1 == N.)
# usage: scripts/refresh-seed.sh [--check] [<tree>, default this checkout]   (--check: run the stages, replace nothing)
# A new seed replaces warp-seed.wasm beside the binary (WARP_SEED names another) and is published as the asset of a
# GitHub pre-release seed-<date>-<sha8>, where the release workflow and fresh installs fetch it.
# WARP_BINARY: the warp host to run with (default `warp` on PATH). A stage takes ~17-20 min on the Mac: run it
# in the background. The bytes depend on where lib/ lies (compiler.warp keeps its folder), so
# the fixpoint holds per tree, and a seed refreshed in another tree differs in that path alone.
set -u
check=
[ "${1-}" = --check ] && { check=1; shift; }
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
warp=${WARP_BINARY:-warp}
seed=${WARP_SEED:-$(dirname "$(command -v "$warp")")/warp-seed.wasm}
data=$tree/data
built=$tree/core/compiler.wasm  # where `warp self build --wasm core/compiler.warp` writes
[ -f "$seed" ] || { echo "refresh-seed: no seed at $seed" >&2; exit 2; }
# a self build of the compiler runs for minutes: an exported short limit (600 s) must not cut it
export WARP_LIB=$tree/lib WARP_TIME_LIMIT=${WARP_SEED_TIME_LIMIT:-7200} WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_SELF_HOSTED WARP_EMITTER
scratch=$(mktemp -d "${TMPDIR:-/tmp}/refresh-seed.XXXXXX") || exit 2
mkdir -p "$data"
kept=
[ -f "$data/warp-self.wasm" ] && { kept=$scratch/kept.wasm; cp "$data/warp-self.wasm" "$kept"; }
restore() {
  if [ -n "$kept" ]; then cp "$kept" "$data/warp-self.wasm"; else rm -f "$data/warp-self.wasm"; fi
  rm -rf "$scratch"
}
trap restore EXIT
# stage <compiler> <output>: compiler.warp compiled by <compiler>
stage() {
  cp "$1" "$data/warp-self.wasm" || exit 1
  rm -f "$built"
  local start=$SECONDS
  (cd "$tree" && "$warp" self build --wasm core/compiler.warp) 2>&1 | tail -5
  [ -f "$built" ] || { echo "refresh-seed: $(basename "$2" .wasm) FAILED: compiled by $(basename "$1") wrote no module" >&2; exit 1; }
  mv "$built" "$2"
  echo "refresh-seed: $(basename "$2" .wasm) $(wc -c <"$2" | tr -d ' ') B in $((SECONDS - start)) s, sha256 $(shasum -a 256 "$2" | cut -c1-16)"
}
stage "$seed" "$scratch/A.wasm"
stage "$scratch/A.wasm" "$scratch/B.wasm"
if cmp -s "$scratch/A.wasm" "$scratch/B.wasm"; then
  fixpoint=$scratch/B.wasm
else
  stage "$scratch/B.wasm" "$scratch/C.wasm"
  cmp -s "$scratch/B.wasm" "$scratch/C.wasm" || { echo "refresh-seed: NO FIXPOINT: stage C differs from stage B; the seed stays" >&2; exit 1; }
  fixpoint=$scratch/C.wasm
fi
echo "refresh-seed: FIXPOINT $(wc -c <"$fixpoint" | tr -d ' ') B sha256 $(shasum -a 256 "$fixpoint" | cut -d' ' -f1)"
if [ -n "$check" ]; then
  cmp -s "$fixpoint" "$seed" && echo "refresh-seed: the seed is the fixpoint" || echo "refresh-seed: the seed differs from the fixpoint (--check: kept)"
  exit 0
fi
cp "$fixpoint" "$seed" && echo "refresh-seed: replaced $seed"
release=seed-$(date +%Y-%m-%d)-$(shasum -a 256 "$seed" | cut -c1-8)
gh release create "$release" "$seed" --repo pannous/warp --prerelease --title "$release" \
  --notes "The seed compiler: core/compiler.warp of $(git -C "$tree" rev-parse --short HEAD) compiled by itself at a fixpoint." &&
  echo "refresh-seed: published $release"
# its machine code beside it, for the engine runs use (card precompiled-compilers): no build JIT-compiles the seed
(cd "$tree" && "$warp" rebuild seed)
