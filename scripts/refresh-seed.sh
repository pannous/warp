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
scripts=$(cd "$(dirname "$0")" && pwd)
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
# in_slot <log> <command…>: the command in the machine's fair build line (scripts/build-slot.sh), as every compile of
# the compiler is, then the log's last lines
in_slot() {
  local log=$1 lines=$2
  shift 2
  : >"$log"
  (cd "$tree" && "$scripts/build-slot.sh" "$log" "$@")
  local status=$?
  tail -"$lines" "$log"
  return $status
}
stage() {
  WARP_SEED=$1 in_slot "$scratch/warm.log" 2 "$warp" rebuild seed  # its machine code and runtime, outside the time
  rm -f "$built"
  local start=$SECONDS record=$scratch/$(basename "$2" .wasm).record
  # the speed guard's measure is the instructions the rebuild retired (build-slot.sh's record, /usr/bin/time -l): the
  # same work retires the same count whatever the load and the cores it ran on, where the CPU seconds don't (2026-11-02:
  # one compile took 268 s CPU on a quiet Mac, 869 s at load 30 on efficiency cores, both 4.7e12 instructions); the CPU
  # seconds (user + system) only where no record counts instructions (no lockf, no time -l)
  # a failed rebuild fails the stage even when it wrote warp.wasm (a runtime.wasm or the warming that failed)
  # the line's messages (stderr) go with the output: the cpu file holds only time's line
  { time (WARP_SEED=$1 WARP_BUILD_RECORD=$record in_slot "$scratch/rebuild.log" 5 "$warp" rebuild pure 2>&1); } 2>"$scratch/cpu" &&
    [ -f "$built" ] || { echo "refresh-seed: $(basename "$2" .wasm) FAILED: the rebuild by $(basename "$1") failed" >&2; exit 1; }
  cp "$built" "$2"
  local seconds instructions
  seconds=$(awk '{ printf "%d", $1 + $2 }' "$scratch/cpu")
  instructions=$(awk '/instructions retired/ { print $1 }' "$record/time" 2>/dev/null)
  if [ -n "$instructions" ]; then stage_cost=$instructions stage_unit=instructions; else stage_cost=$seconds stage_unit="s CPU"; fi
  echo "refresh-seed: $(basename "$2" .wasm) $(wc -c <"$2" | tr -d ' ') B in $((SECONDS - start)) s, $seconds s CPU, ${instructions:-?} instructions, sha256 $(shasum -a 256 "$2" | cut -c1-16)"
}
previous=$seed
chosen=
old_cost=
new_cost=
for name in $STAGES; do
  stage "$previous" "$scratch/$name.wasm"
  if [ -z "$old_cost" ]; then old_cost=$stage_cost old_unit=$stage_unit; elif [ -z "$new_cost" ]; then new_cost=$stage_cost new_unit=$stage_unit; fi
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
# stage B the new compiler's of the same source: their retired instructions are the comparison, no extra rebuilds (decision
# seed-speed-from-stages). WARP_SEED_SLOWER_OK=1 takes a slower one anyway
echo "refresh-seed: a rebuild took $new_cost $new_unit with the new compiler, $old_cost $old_unit with the seed it replaces"
[ "$old_unit" = "$new_unit" ] || { echo "refresh-seed: the stages were measured in different units; the seed stays" >&2; exit 1; }
if [ $((new_cost * 100)) -gt $((old_cost * SLOWER_PERCENT)) ] && [ -z "${WARP_SEED_SLOWER_OK-}" ]; then
  echo "refresh-seed: the new compiler takes over ${SLOWER_PERCENT}% of the old seed's time; the seed stays (WARP_SEED_SLOWER_OK=1 takes it)" >&2
  exit 1
fi
if [ -n "$check" ]; then
  cmp -s "$chosen" "$seed" && echo "refresh-seed: the seed is that stage" || echo "refresh-seed: the seed differs from that stage (--check: kept)"
  exit 0
fi
cp "$chosen" "$seed" && echo "refresh-seed: replaced $seed"
# the old seed's runtime.wasm and machine code go with it: the native warp writes warp-seed.runtime.wasm only when it is
# missing, so a stale one kept the old runtime words (the new seed's build failed: no field built_spelled_list, 2026-10-29)
rm -f "${seed%.wasm}.runtime.wasm" "${seed%.wasm}.runtime.wasm.cwasm" "$seed.cwasm" "$seed.driver.cwasm" "$seed.driver.cwasm.key" "$seed.driver.build.cwasm" "$seed.driver.build.cwasm.key"
release=seed-$(date +%Y-%m-%d)-$(shasum -a 256 "$seed" | cut -c1-8)
# the release's tag on the very commit the seed was built from, and seed.lock committed beside it, so every later commit
# names its seed and any commit's compiler can be built again (scripts/seed.sh, user 2026-10-30)
source_commit=$(git -C "$tree" rev-parse HEAD)
gh release create "$release" "$seed" --repo pannous/warp --prerelease --title "$release" --target "$source_commit" \
  --notes "The seed compiler: core/compiler.warp of ${source_commit:0:10} compiled by a stage that compiles itself once more." &&
  echo "refresh-seed: published $release"
echo "$release $(shasum -a 256 "$seed" | cut -d' ' -f1)" >"$tree/seed.lock"
git -C "$tree" add seed.lock && git -C "$tree" commit -q -m "chore: seed.lock names $release, built from ${source_commit:0:10}" -- seed.lock &&
  echo "refresh-seed: seed.lock committed in $tree: push it with the tree"
# its machine code beside it, for the engine runs use (card precompiled-compilers): no build JIT-compiles the seed
(cd "$tree" && "$warp" rebuild seed)
