#!/bin/bash
# Refresh the seed compiler (seed/warp-self.wasm, card rust-bootstrap): the seed compiles core/compiler.warp
# (stage A), each stage compiles it again with the one before (B, C; WARP_SEED_STAGES names them), and the last stage
# that compiled itself once more (the next stage built) replaces the seed; no fixpoint is needed (user 2026-10-26), one
# ends the stages early. A feature the compiler uses on itself (a runtime word, a bundled lib/) arrives a stage after
# the stage that brings it, so such a change asks for more stages. The seed is what `warp rebuild` builds warp.wasm
# with; no Rust-compiled stage takes part. (scripts/promote-self.sh is the light check of warp.wasm alone.)
# usage: scripts/refresh-seed.sh [--check] [<tree>, default this checkout]   (--check: run the stages, replace nothing)
# WARP_BINARY: the warp host to run with (default `warp` on PATH). A stage takes ~17-20 min on the Mac: run it
# in the background. The bytes depend on where lib/ lies (compiler.warp keeps its folder), so
# a seed refreshed in another tree differs in that path alone.
set -u
STAGES=${WARP_SEED_STAGES:-A B C}
check=
[ "${1-}" = --check ] && { check=1; shift; }
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
warp=${WARP_BINARY:-warp}
seed=$tree/seed/warp-self.wasm
data=$tree/data
built=$tree/core/compiler.wasm  # where `warp self build --wasm core/compiler.warp` writes
[ -f "$seed" ] || [ ! -f "$tree/seed/compiler-self.wasm" ] || seed=$tree/seed/compiler-self.wasm  # the old name (transition)
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
cp "$chosen" "$data/warp-self.wasm"
hello=$(cd "$tree" && "$warp" self samples/hello/hello.warp 2>&1)
echo "$hello" | grep -q Hello || { echo "refresh-seed: $(basename "$chosen" .wasm) runs no hello ($(echo "$hello" | tail -1 | cut -c1-200)); the seed stays" >&2; exit 1; }
echo "refresh-seed: NEW SEED $(basename "$chosen" .wasm) $(wc -c <"$chosen" | tr -d ' ') B sha256 $(shasum -a 256 "$chosen" | cut -d' ' -f1)"
if [ -n "$check" ]; then
  cmp -s "$chosen" "$seed" && echo "refresh-seed: the seed is that stage" || echo "refresh-seed: the seed differs from that stage (--check: kept)"
  exit 0
fi
cp "$chosen" "$seed" && echo "refresh-seed: replaced $seed (commit it: git add seed/warp-self.wasm)"
# its machine code beside it, for the engine runs use (card precompiled-compilers): no build JIT-compiles the seed
(cd "$tree" && "$warp" rebuild seed)
