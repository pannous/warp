#!/bin/bash
# Refresh the seed compiler (warp-seed.wasm beside the warp binary, card rust-bootstrap): the seed compiles core/compiler.warp
# (stage A), each stage compiles it again with the one before (B, C; WARP_SEED_STAGES names them), and the last stage
# that compiled itself once more (the next stage built) replaces the seed; no fixpoint is needed (user 2026-10-26), one
# ends the stages early. A feature the compiler uses on itself (a runtime word, a bundled lib/) arrives a stage after
# the stage that brings it, so such a change asks for more stages. The seed is what `warp rebuild` builds warp.wasm
# with; no Rust-compiled stage takes part. (scripts/promote-self.sh is the light check of warp.wasm alone.)
# usage: scripts/refresh-seed.sh [--check] [<tree>, default this checkout]   (--check: run the stages, replace nothing)
# A new seed replaces warp-seed.wasm beside the binary (WARP_SEED names another) and is published as the asset of a
# GitHub pre-release seed-<date>-<sha8>, where the release workflow and fresh installs fetch it.
# WARP_BINARY: the warp host to run with (default `warp` on PATH). A stage takes ~17-20 min on the Mac: run it
# in the background. The bytes depend on where lib/ lies (compiler.warp keeps its folder), so
# a seed refreshed in another tree differs in that path alone.
set -u
STAGES=${WARP_SEED_STAGES:-A B C}
# how much longer than with the old seed `warp rebuild pure` may take with the new one, in percent (noise of a run)
SLOWER_PERCENT=${WARP_SEED_SLOWER_PERCENT:-115}
check=
[ "${1-}" = --check ] && { check=1; shift; }
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
warp=${WARP_BINARY:-warp}
seed=${WARP_SEED:-$(dirname "$(command -v "$warp")")/warp-seed.wasm}
compilers=$tree/lib
built=$tree/core/compiler.wasm  # where `warp self build --wasm core/compiler.warp` writes
[ -f "$seed" ] || { echo "refresh-seed: no seed at $seed" >&2; exit 2; }
# a self build of the compiler runs for minutes: an exported short limit (600 s) must not cut it
export WARP_LIB=$tree/lib WARP_TIME_LIMIT=${WARP_SEED_TIME_LIMIT:-7200} WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_SELF_HOSTED WARP_EMITTER
scratch=$(mktemp -d "${TMPDIR:-/tmp}/refresh-seed.XXXXXX") || exit 2
kept=
[ -f "$compilers/warp-self.wasm" ] && { kept=$scratch/kept.wasm; cp "$compilers/warp-self.wasm" "$kept"; }
restore() {
  if [ -n "$kept" ]; then cp "$kept" "$compilers/warp-self.wasm"; else rm -f "$compilers/warp-self.wasm"; fi
  rm -rf "$scratch"
}
trap restore EXIT
# stage <compiler> <output>: compiler.warp compiled by <compiler>
stage() {
  cp "$1" "$compilers/warp-self.wasm" || exit 1
  rm -f "$built"
  local start=$SECONDS
  (cd "$tree" && "$warp" self build --wasm core/compiler.warp) 2>&1 | tail -5
  [ -f "$built" ] || { echo "refresh-seed: $(basename "$2" .wasm) FAILED: compiled by $(basename "$1") wrote no module" >&2; exit 1; }
  mv "$built" "$2"
  echo "refresh-seed: $(basename "$2" .wasm) $(wc -c <"$2" | tr -d ' ') B in $((SECONDS - start)) s, sha256 $(shasum -a 256 "$2" | cut -c1-16)"
}
previous=$seed
chosen=
for name in $STAGES; do
  stage "$previous" "$scratch/$name.wasm"
  [ "$previous" != "$seed" ] && chosen=$previous
  if [ "$previous" != "$seed" ] && cmp -s "$previous" "$scratch/$name.wasm"; then
    echo "refresh-seed: $name is the same bytes as the stage before: a fixpoint"
    break
  fi
  previous=$scratch/$name.wasm
done
[ -n "$chosen" ] || { echo "refresh-seed: no stage compiled itself once more (WARP_SEED_STAGES: $STAGES); the seed stays" >&2; exit 1; }
# and the chosen stage runs a program (decision no-fixpoint: it compiles compiler.warp again and that build runs hello)
cp "$chosen" "$compilers/warp-self.wasm"
hello=$(cd "$tree" && "$warp" self samples/hello/hello.warp 2>&1)
echo "$hello" | grep -q Hello || { echo "refresh-seed: $(basename "$chosen" .wasm) runs no hello ($(echo "$hello" | tail -1 | cut -c1-200)); the seed stays" >&2; exit 1; }
echo "refresh-seed: NEW SEED $(basename "$chosen" .wasm) $(wc -c <"$chosen" | tr -d ' ') B sha256 $(shasum -a 256 "$chosen" | cut -d' ' -f1)"
# a seed is what every `warp rebuild` runs: one that builds warp.wasm slower than the seed it replaces is no refresh
# (card rebuild-pure-went: two seeds made rebuilds 2.7x slower unnoticed). Each seed's machine code is made by a first
# build, the second is timed; WARP_SEED_SLOWER_OK=1 takes a slower one anyway
rebuild_seconds() {
  (cd "$tree" && WARP_SEED=$1 scripts/rebuild.sh --force pure "$tree" >/dev/null 2>&1)
  local start=$SECONDS
  (cd "$tree" && WARP_SEED=$1 scripts/rebuild.sh --force pure "$tree" >/dev/null 2>&1)
  echo $((SECONDS - start))
}
cp "$chosen" "$scratch/candidate.wasm"
old_seconds=$(rebuild_seconds "$seed")
new_seconds=$(rebuild_seconds "$scratch/candidate.wasm")
echo "refresh-seed: warp rebuild pure takes $new_seconds s with it, $old_seconds s with the seed it replaces"
if [ $((new_seconds * 100)) -gt $((old_seconds * SLOWER_PERCENT)) ] && [ -z "${WARP_SEED_SLOWER_OK-}" ]; then
  echo "refresh-seed: the new seed builds slower than ${SLOWER_PERCENT}% of the old one's time; the seed stays (WARP_SEED_SLOWER_OK=1 takes it)" >&2
  exit 1
fi
if [ -n "$check" ]; then
  cmp -s "$chosen" "$seed" && echo "refresh-seed: the seed is that stage" || echo "refresh-seed: the seed differs from that stage (--check: kept)"
  exit 0
fi
cp "$chosen" "$seed" && echo "refresh-seed: replaced $seed"
release=seed-$(date +%Y-%m-%d)-$(shasum -a 256 "$seed" | cut -c1-8)
gh release create "$release" "$seed" --repo pannous/warp --prerelease --title "$release" \
  --notes "The seed compiler: core/compiler.warp of $(git -C "$tree" rev-parse --short HEAD) compiled by a stage that compiles itself once more." &&
  echo "refresh-seed: published $release"
# its machine code beside it, for the engine runs use (card precompiled-compilers): no build JIT-compiles the seed
(cd "$tree" && "$warp" rebuild seed)
