#!/bin/zsh
# The warp-only acceptance check (decision warp-only-acceptance): the Rust-compiled compiler.warp compiles itself and
# that self build runs hello world. Few checks at a time on the Mac: each self build runs wasm-opt -O3 on ~2.5 cores,
# and eight at once pushed the load past 60 (supervisor 2026-10-21); one at a time queued checks for 45 min at load 9,
# so check_slots run at once and a further check waits for a free slot.
# usage: scripts/self-check.sh [<worktree>, default the current checkout]   (exit 0 = Hello printed)
worktree=${${1:-$(git rev-parse --show-toplevel)}:a}  # absolute: the check cds into it, then reads $worktree/lib
main_warp=/Users/me/dev/bin/warp
locks=/Users/me/dev/angles/warp/scratch/self-check
check_slots=2
expected="Hello"

# a slot is a directory holding its check's pid; a slot whose check died is freed
take_slot() {
  local slot
  mkdir -p ${locks:h}
  while true; do
    for slot in {1..$check_slots}; do
      if mkdir $locks.$slot.lock 2>/dev/null; then echo $$ > $locks.$slot.lock/pid; lock=$locks.$slot.lock; return; fi
      holder=$(cat $locks.$slot.lock/pid 2>/dev/null)
      [[ -n $holder ]] && ! kill -0 $holder 2>/dev/null && rm -rf $locks.$slot.lock
    done
    sleep 5
  done
}
take_slot
trap "rm -rf $lock" EXIT

cd $worktree || exit 1
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$worktree/lib
# both compilers of the branch's lib/: compiler.wasm (`warp pure`, built by Rust) and compiler-self.wasm (`warp self`,
# built by itself); a lib/ that passed self alone broke main's `warp rebuild pure` twice on 2026-10-22
check_lane() {
  local built
  built=$($warp rebuild $1 2>&1) || { verdict="SELF CHECK RED: warp rebuild $1 failed in $worktree: $(print -r -- $built | grep -v '^warning\|^ *[0-9]* |\|^ *|' | tail -3)"; return 1; }
  said=$($warp $1 samples/hello/hello.warp 2>&1 | tail -1)
  [[ $said == *$expected* ]] || { verdict="SELF CHECK RED: warp $1 hello said: $said"; return 1; }
  verdict="SELF CHECK ok: $said"
}
# main's binary is refreshed after each pushed batch: a compiler.wasm built by the old binary and a self build by the
# new one make a self compiler that traps on every program (card self-trap-function-param). The check runs its own copy
# of the binary, taken once, so a refresh never overlaps it (retrying instead kept checks queued for hours, 2026-10-22)
warp=$lock/warp
cp $main_warp $warp || exit 1
check_lane pure && check_lane self
passed=$?
echo $verdict
exit $passed
