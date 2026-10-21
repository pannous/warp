#!/bin/zsh
# The warp-only acceptance check (decision warp-only-acceptance): the Rust-compiled compiler.warp compiles itself and
# that self build runs hello world. One check at a time on the Mac: each self build runs wasm-opt -O3 on ~2.5 cores,
# and eight at once pushed the load past 60 (supervisor 2026-10-21), so a check waits for the running one.
# usage: scripts/self-check.sh [<worktree>, default the current checkout]   (exit 0 = Hello printed)
worktree=${1:-$(git rev-parse --show-toplevel)}
warp=/Users/me/dev/bin/warp
lock=/Users/me/dev/angles/warp/scratch/self-check.lock
expected="Hello"

mkdir -p ${lock:h}
until mkdir $lock 2>/dev/null; do
  holder=$(cat $lock/pid 2>/dev/null)
  [[ -n $holder ]] && ! kill -0 $holder 2>/dev/null && rm -rf $lock && continue  # its holder died
  sleep 5
done
echo $$ > $lock/pid
trap "rm -rf $lock" EXIT

cd $worktree || exit 1
# WARP_WASM_OPT=0: the stages of a one-off build skip wasm-opt -O3, ~40% of its CPU (card self-check)
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$worktree/lib WARP_WASM_OPT=0
$warp rebuild self >/dev/null 2>&1 || { echo "SELF CHECK RED: warp rebuild self failed in $worktree"; exit 1; }
said=$($warp self samples/hello/hello.warp 2>&1 | tail -1)
[[ $said == *$expected* ]] || { echo "SELF CHECK RED: warp self hello said: $said"; exit 1; }
echo "SELF CHECK ok: $said"
