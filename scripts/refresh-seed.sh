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
# WARP_BINARY: the warp host to run with (default `warp` on PATH). A stage takes ~1.5-2.5 min on the Mac: run it in
# the background. The bytes depend on where lib/ lies (compiler.warp keeps its
# folder), so a seed refreshed in another tree differs in that path alone.
set -u -o pipefail
STAGES=${WARP_SEED_STAGES:-A B C}
# how much longer than the old seed the new compiler may take to compile compiler.warp, in percent: the stages' own
# times, A (the old seed) against B (the new compiler), the same source; only a clearly slower one is refused
SLOWER_PERCENT=${WARP_SEED_SLOWER_PERCENT:-150}
TIMEFORMAT="%U %S"
check=
[ "${1-}" = --check ] && { check=1; shift; }
tree=$(cd "${1:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
warp=${WARP_BINARY:-warp}
seed=${WARP_SEED:-$(dirname "$(command -v "$warp")")/warp-seed.wasm}
compilers=$tree/lib
built=$compilers/warp.wasm  # where `warp rebuild pure` writes, with WARP_LIB set
[ -f "$seed" ] || { echo "refresh-seed: no seed at $seed" >&2; exit 2; }
# a rebuild runs for minutes: an exported short limit (600 s) must not cut it
export WARP_LIB=$tree/lib WARP_REBUILD_TIME_LIMIT=${WARP_SEED_TIME_LIMIT:-7200} WARP_NO_WINDOW=1 WARP_HINTS=0
unset WARP_SELF_HOSTED WARP_EMITTER
scratch=$(mktemp -d "${TMPDIR:-/tmp}/refresh-seed.XXXXXX") || exit 2
trap 'rm -rf "$scratch"' EXIT
# stage <compiler> <output>: `warp rebuild pure` with <compiler> as the seed, the one thing a seed is asked to do (a seed
# older than the harness may lack its newer commands: the seed of 14:52 traps in `warp self build` since the build
# moved into compiler.warp, 2026-10-28). It leaves the tree's lib/ compilers those of the last stage's rebuild.
stage() {
  (cd "$tree" && WARP_SEED=$1 "$warp" rebuild seed 2>&1 | tail -2)  # its machine code and runtime, outside the time
  rm -f "$built"
  local start=$SECONDS
  # the CPU seconds (user + system) are the speed guard's measure: wall time swings with the box's load (2026-10-28:
  # the same compile took 95 s at load 10, 233 s at load 89)
  # a failed rebuild fails the stage even when it wrote warp.wasm (a runtime.wasm or the warming that failed)
  { time (cd "$tree" && WARP_SEED=$1 "$warp" rebuild pure 2>&1 | tail -5); } 2>"$scratch/cpu" &&
    [ -f "$built" ] || { echo "refresh-seed: $(basename "$2" .wasm) FAILED: the rebuild by $(basename "$1") failed" >&2; exit 1; }
  cp "$built" "$2"
  stage_seconds=$(awk '{ printf "%d", $1 + $2 }' "$scratch/cpu")
  echo "refresh-seed: $(basename "$2" .wasm) $(wc -c <"$2" | tr -d ' ') B in $((SECONDS - start)) s, $stage_seconds s CPU, sha256 $(shasum -a 256 "$2" | cut -c1-16)"
}
previous=$seed
chosen=
old_seconds=
new_seconds=
for name in $STAGES; do
  stage "$previous" "$scratch/$name.wasm"
  if [ -z "$old_seconds" ]; then old_seconds=$stage_seconds; elif [ -z "$new_seconds" ]; then new_seconds=$stage_seconds; fi
  [ "$previous" != "$seed" ] && chosen=$previous
  if [ "$previous" != "$seed" ] && cmp -s "$previous" "$scratch/$name.wasm"; then
    echo "refresh-seed: $name is the same bytes as the stage before: a fixpoint"
    break
  fi
  previous=$scratch/$name.wasm
done
[ -n "$chosen" ] || { echo "refresh-seed: no stage compiled itself once more (WARP_SEED_STAGES: $STAGES); the seed stays" >&2; exit 1; }
# and the chosen stage runs a program (decision no-fixpoint): the last rebuild, by the chosen stage, runs hello
hello=$(cd "$tree" && "$warp" samples/hello/hello.warp 2>&1)
echo "$hello" | grep -q Hello || { echo "refresh-seed: $(basename "$chosen" .wasm) runs no hello ($(echo "$hello" | tail -1 | cut -c1-200)); the seed stays" >&2; exit 1; }
echo "refresh-seed: NEW SEED $(basename "$chosen" .wasm) $(wc -c <"$chosen" | tr -d ' ') B sha256 $(shasum -a 256 "$chosen" | cut -d' ' -f1)"
# a seed is what every `warp rebuild` runs: one that builds warp.wasm clearly slower than the seed it replaces is no
# refresh (card rebuild-pure-went: two seeds made rebuilds 2.7x slower unnoticed). Stage A is the old seed's rebuild,
# stage B the new compiler's of the same source: their times are the comparison, no extra rebuilds (decision
# seed-speed-from-stages). WARP_SEED_SLOWER_OK=1 takes a slower one anyway
echo "refresh-seed: a rebuild took $new_seconds s CPU with the new compiler, $old_seconds s CPU with the seed it replaces"
if [ $((new_seconds * 100)) -gt $((old_seconds * SLOWER_PERCENT)) ] && [ -z "${WARP_SEED_SLOWER_OK-}" ]; then
  echo "refresh-seed: the new compiler takes over ${SLOWER_PERCENT}% of the old seed's time; the seed stays (WARP_SEED_SLOWER_OK=1 takes it)" >&2
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
