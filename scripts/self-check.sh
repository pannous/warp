#!/bin/zsh
# The warp-only acceptance check (decision warp-only-acceptance): the Rust-compiled compiler.warp compiles itself and
# that self build runs hello world. Few checks at a time on the Mac: each self build runs wasm-opt -O3 on ~2.5 cores,
# and eight at once pushed the load past 60 (supervisor 2026-10-21); one at a time queued checks for 45 min at load 9,
# so the check runs in one of scripts/slot.sh's fleet-wide slots.
# usage: scripts/self-check.sh [<worktree>, default the current checkout]   (exit 0 = Hello printed)
worktree=${${1:-$(git rev-parse --show-toplevel)}:a}  # absolute: the check cds into it, then reads $worktree/lib
# main's Rust binary, warp-rust beside the launcher warp (notes/pure_wrapper.md); plain warp until warp-rust is installed
main_warp=/Users/me/dev/bin/warp-rust
[[ -x $main_warp ]] || main_warp=/Users/me/dev/bin/warp
main_checkout=/Users/me/dev/angles/warp
expected="Hello"

# trees to check live on /Volumes/Worktrees (cowtree) or, for throwaway lib/ copies, /Volumes/Junk: a copy under the
# main checkout loads fseventsd for every session watching it
if [[ $worktree == $main_checkout/* ]]; then
  print -u2 "self-check: $worktree is inside the main checkout; check a worktree under /Volumes/Worktrees or a copy under /Volumes/Junk"
  exit 2
fi
[[ -n $WARP_SLOT ]] || exec $main_checkout/scripts/slot.sh ${0:a} $worktree

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
warp=$WARP_SLOT/warp
cp $main_warp $warp || exit 1
check_lane pure && check_lane self
passed=$?
echo $verdict
exit $passed
