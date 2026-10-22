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
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$worktree/lib
# both compilers of the branch's lib/: compiler.wasm (`warp pure`, built by Rust) and compiler-self.wasm (`warp self`,
# built by itself); a lib/ that passed self alone broke main's `warp rebuild pure` twice on 2026-10-22
check_lane() {
  $warp rebuild $1 >/dev/null 2>&1 || { echo "SELF CHECK RED: warp rebuild $1 failed in $worktree"; exit 1; }
  said=$($warp $1 samples/hello/hello.warp 2>&1 | tail -1)
  [[ $said == *$expected* ]] || { echo "SELF CHECK RED: warp $1 hello said: $said"; exit 1; }
}
check_lane pure
check_lane self
echo "SELF CHECK ok: $said"
