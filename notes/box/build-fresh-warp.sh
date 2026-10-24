#!/bin/bash
# Build current origin/main into /workspace/target-main, install /workspace/cow/sol-warp-<hash>, point
# sol-warp-latest at it (only after a passing smoke `rebuild pure`).
# usage: /workspace/box-slot.sh /workspace/build-fresh-warp.sh      (-h/--help: this text)
# Output (quiet by default): the steps' full output goes to a log (/tmp/warp-logs/build-fresh-warp-<time>.log, or
# $BFW_LOG); you see one line per step and, on failure, the error lines (error:/panic/failed/…) with context.
# VERBOSE=1 also streams everything. Exit status: 0 = LATEST moved; else the failing step's status (a signal death,
# e.g. 137 SIGKILL/out of memory, is named).
# Env overrides (for tests): BFW_TREE (/workspace/wt-main), BFW_TARGET (/workspace/target-main),
#   BFW_DEST (/workspace/cow), BFW_LOG
set -o pipefail
case "${1:-}" in
  -h|--help) sed -n '2,/^[^#]/{/^#/s/^# \{0,1\}//p}' "$0"; exit 0;;
  "") ;;
  *) echo "build-fresh-warp: unknown argument '$1' (usage: build-fresh-warp.sh [-h|--help])" >&2; exit 2;;
esac
tree=${BFW_TREE:-/workspace/wt-main} target=${BFW_TARGET:-/workspace/target-main} dest=${BFW_DEST:-/workspace/cow}
log=${BFW_LOG:-/tmp/warp-logs/build-fresh-warp-$(date +%Y%m%d-%H%M%S).log}
mkdir -p "$(dirname "$log")" && : >"$log" || { echo "build-fresh-warp: cannot write $log" >&2; exit 1; }
verbose=${VERBOSE:-0}
pat='^error(\[[A-Za-z0-9]+\])?:|^Error|Error\(|panicked|panic:|[Ff]ailed|FAIL|refused|^Killed|SIGKILL|out of memory|No such file|fatal:'
say() { echo "$*"; echo "== $*" >>"$log"; }
step() {   # label cmd…: output to the log (and the screen with VERBOSE=1); a failure shows its error lines
  local label=$1 rc t0=$SECONDS from; shift
  from=$(( $(wc -l <"$log") + 1 ))
  echo "== $label: $*" >>"$log"
  if [ "$verbose" = 1 ]; then "$@" 2>&1 | tee -a "$log"; rc=${PIPESTATUS[0]}
  else { "$@" >>"$log" 2>&1; } 2>/dev/null; rc=$?; fi
  [ $rc -eq 0 ] && return 0
  echo "FAILED: $label rc=$rc after $((SECONDS - t0))s (log: $log)"
  [ $rc -gt 128 ] && echo "killed: signal $(kill -l $((rc - 128)) 2>/dev/null || echo $((rc - 128)))$([ $rc -eq 137 ] && echo ' (likely out of memory)')"
  if tail -n +"$from" "$log" | grep -qE "$pat"; then
    tail -n +"$from" "$log" | grep -nE -B2 -A2 "$pat" | grep -v '^[0-9]*[-:]warning' | head -n 30
  else tail -n +"$from" "$log" | tail -n 10; fi
  return $rc
}
cd "$tree" || { echo "build-fresh-warp: no tree $tree" >&2; exit 1; }
step "fetch main" bash -c "git fetch -q origin && git checkout -q --detach origin/main" || exit
h=$(git -C "$tree" rev-parse --short=9 HEAD) || exit 1
say "main $h $(date +%T) (log: $log)"
t0=$SECONDS
step "cargo build --release" env CARGO_TARGET_DIR="$target" nice -n 5 cargo build --release || exit
say "built in $((SECONDS - t0))s"
dst=$dest/sol-warp-$h
step "install $dst" bash -c "cp '$target/release/warp' '$dst.tmp' && chmod +x '$dst.tmp' && mv '$dst.tmp' '$dst'" || exit
say "INSTALLED $dst $(date +%T)"
export PATH=/workspace/tools/binaryen-version_133/bin:$PATH WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$tree/lib
t0=$SECONDS
step "smoke rebuild pure" env WARP_MODULE_CACHE=off "$dst" rebuild pure; rc=$?
git -C "$tree" checkout -q -- data 2>/dev/null
dirty=$(git -C "$tree" status --short | head -5); [ -n "$dirty" ] && say "note: $tree is dirty: $(echo $dirty)"
[ $rc -eq 0 ] || { echo "SMOKE FAILED, sol-warp-latest unchanged"; exit $rc; }
say "smoke rebuild pure ok $((SECONDS - t0))s"
ln -sfn "$dst" "$dest/sol-warp-latest.tmp" && mv -T "$dest/sol-warp-latest.tmp" "$dest/sol-warp-latest" || exit 1
say "LATEST -> $(readlink "$dest/sol-warp-latest")"
