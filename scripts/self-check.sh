#!/bin/zsh
# The warp-only acceptance check (decision warp-only-acceptance): warp.wasm, compiler.warp built by the seed, compiles itself and
# that self build runs hello world.
# The pair checked is the one for the branch's compiler inputs (scripts/rebuild.sh, notes/compiler_rebuilds.md): lib/'s
# or the shared store's. A miss is said ("REBUILD NEEDED") and the existing pair is checked: the check builds only when
# asked to, with --rebuild. Nothing rebuilds the compilers because lib/ changed .
# usage: scripts/self-check.sh [--rebuild] [<worktree>, default the current checkout]   (exit 0 = Hello printed)
rebuild=
[[ ${1-} == --rebuild ]] && { rebuild=1; shift; }
worktree=${${1:-$(git rev-parse --show-toplevel)}:a}  # absolute: the check cds into it, then reads $worktree/lib
# the native warp checked with: WARP_BINARY (a branch changing the driver brings its own build), else the shared one
main_warp=${WARP_BINARY:-/Users/me/dev/bin/warp}
main_checkout=/Users/me/dev/angles/warp
store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers}  # scripts/rebuild.sh's, where stage 2's outcome is kept too
shown_lines=60  # of a failed stage 2's output
expected="Hello"

# trees to check live on /Volumes/Worktrees (cowtree) or, for throwaway lib/ copies, /Volumes/Junk: a copy under the
# main checkout loads fseventsd for every session watching it
if [[ $worktree == $main_checkout/* ]]; then
  print -u2 "self-check: $worktree is inside the main checkout; check a worktree under /Volumes/Worktrees or a copy under /Volumes/Junk"
  exit 2
fi
# the core runtime (runtime.warp, core/runtime/*.warp) never uses the extended one (core/runtime/extended/,
# runtime/extended.warp; decision runtime-split) nor names a word it defines: it calls the hooks runtime/bridge.warp
# declares, whose bodies are runtime/extended.warp's. Checked first. Plain files only (.); the extended root and
# runtime/durations.warp, an extended part kept in runtime/ for tests/numbers/test_duration_words.rs, count as extended
core=($worktree/core/runtime.warp $worktree/core/runtime/*.warp(.))
core=(${${core:#*/runtime/extended.warp}:#*/runtime/durations.warp})
extended=($worktree/core/runtime/extended/*.warp $worktree/core/runtime/durations.warp)
extended=(${extended:#*/hooks.warp})
extended_words=$(grep -hv ' from runtime$' $extended | sed -n 's/^\([a-z_][a-z_0-9]*\)(.*:=.*/\1/p' | sort -u | paste -sd'|' -)
core_uses_extended=$(grep -HnE "^use .*extended|(^|[^a-z_0-9.])($extended_words)\(" $core)
if [[ -z $extended_words || -n $core_uses_extended ]]; then
  echo "SELF CHECK RED: the core runtime uses the extended runtime, not through runtime/bridge.warp: $core_uses_extended"
  exit 1
fi

cd $worktree || exit 1
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$worktree/lib
# the compilers of the branch's lib/: warp.wasm (plain `warp`'s, built by the seed) and warp-self.wasm (`warp self`,
# warp.wasm once it compiles itself, self_lane below); a lib/ that passed self alone broke main's `warp rebuild pure`
# twice on 2026-10-22
check_lane() {
  said=$($warp $1 samples/hello/hello.warp 2>&1 | tail -1)
  [[ $said == *$expected* ]] || { verdict="SELF CHECK RED: warp $1 hello said: $said"; return 1; }
  verdict="SELF CHECK ok: $said"
}
# main's binary is refreshed after each pushed batch: a warp.wasm built by the old binary and a self build by the
# new one make a self compiler that traps on every program (card self-trap-function-param). The check runs its own copy
# of the binary, taken once, so a refresh never overlaps it (retrying instead kept checks queued for hours, 2026-10-22)
scratch=$(mktemp -d /tmp/self-check.XXXXXX) && trap "rm -rf $scratch" EXIT
warp=$scratch/warp
cp $main_warp $warp || exit 1
# its wasmtime beside the copy, where its rpath looks
cp ${main_warp:A:h}/libwarp_wasmtime.* $scratch/ || exit 1
# the seed lies beside the launcher; the pair in the build folder the launcher links to (warp-builds/<tip>)
# with its runtime (warp-seed.runtime.wasm), the one it compiles compiler.warp behind
for seed_file in warp-seed.wasm warp-seed.runtime.wasm; do
  cp ${main_warp:h}/$seed_file $scratch/ 2>/dev/null || cp ${main_warp:A:h}/$seed_file $scratch/ || exit 1
done
# the pair for these inputs: lib/'s, else the store's; built by the copy only with --rebuild
if [[ -n $rebuild ]]; then
  pair=$(WARP_BINARY=$warp $worktree/scripts/rebuild.sh both $worktree 2>&1)
else
  pair=$(WARP_BINARY=$warp $worktree/scripts/rebuild.sh --check both $worktree 2>&1)
fi
case $? in
  0) pair_hash=$(cut -d' ' -f1 $worktree/lib/warp.wasm.inputs 2>/dev/null) ;;
  3) print -r -- "self-check: REBUILD NEEDED: no pair for these compiler inputs in lib/ nor the store; checking the existing pair (scripts/self-check.sh --rebuild builds it)" ;;
  *) print -r -- "SELF CHECK RED: scripts/rebuild.sh failed in $worktree (its step and log in the store):"; print -r -- $pair; exit 1 ;;
esac
print -r -- $pair
# the self test itself (AGENTS.md "Testing"): warp.wasm compiles compiler.warp and that build, promoted to warp-self.wasm,
# runs hello; a warp-self.wasm of these inputs in lib/ or the store is taken as it is (scripts/rebuild.sh self). Red
# when the compile fails: the self lane was red unnoticed for hours while it was skipped here (card self-check-compile)
self_lane() {
  said=$(WARP_BINARY=$warp WARP_TIME_LIMIT=${WARP_PROMOTE_TIME_LIMIT:-7200} $worktree/scripts/rebuild.sh self $worktree 2>&1) || {
    verdict="SELF CHECK RED: warp.wasm does not compile compiler.warp:
$said"; return 1; }
  check_lane self && stage_two
}
# stage 2 for real, as the next seed's stage will be: warp-self.wasm is only a copy of warp.wasm, so the compiler
# warp.wasm builds from compiler.warp is built here, builds runtime.wasm, then compiles and runs hello behind it, from
# a lib/ of links whose warp.wasm and runtime.wasm are stage 2's (cards compiler-built, compiler-built-wasm: a build
# by warp.wasm failed `-- runtime` with "list of node" while this check, running the copy, passed). Its compiles take
# a build slot (scripts/build-slot.sh) like every compiler build, and its outcome is kept under warp.wasm's inputs hash
# (<store>/<hash>/stage-two.passed, stage-two.failed/ with build.log and message), so a check of that hash runs it once;
# the compile is measured and profiled (stage-two.record/, failed: stage-two.failed/record/: time, profile.txt, build.log);
# --rebuild runs it again. Only a pair of the current inputs has that hash (pair_hash): an existing pair of other or no
# inputs (REBUILD NEEDED) runs stage 2 every time, its outcome only in the tree's data/self-check/ (a lib/ without
# warp.wasm.inputs was answered "stage 2 of unhashed passed before" for every such tree, card scripts-self)
stage_two() {
  local stage=$scratch/lib native=$warp  # the checked native warp: WARP_LIB first, nothing beside it wins
  local hash=${pair_hash:-unkept}
  local record=${${pair_hash:+$store/$hash}:-$worktree/data/self-check}/stage-two log=$scratch/stage-two.log measured=$scratch/stage-two.record
  if [[ -z $rebuild && -n $pair_hash && -f $record.passed ]]; then verdict="SELF CHECK ok: stage 2 of $hash passed before ($record.passed)"; return 0; fi
  if [[ -z $rebuild && -n $pair_hash && -d $record.failed ]]; then verdict="SELF CHECK RED: $(<$record.failed/message)"; return 1; fi
  mkdir -p $stage && ln -s $worktree/lib/*(N) $stage/ && rm -f $stage/warp.wasm* $stage/warp-self.wasm* $stage/runtime.wasm*
  WARP_BUILD_RECORD=$measured WARP_TIME_LIMIT=${WARP_PROMOTE_TIME_LIMIT:-7200} $worktree/scripts/build-slot.sh $log $native $worktree/lib/warp.wasm -- \
    $worktree/lib/compiler/compiler.warp $stage/warp.wasm $worktree/lib/compiler $worktree/lib $worktree/lib/runtime.wasm
  [[ -s $stage/warp.wasm ]] || { stage_two_failed "warp.wasm does not build stage 2"; return 1; }
  $worktree/scripts/build-slot.sh $log.runtime $native $stage/warp.wasm -- runtime $stage/runtime.wasm
  local built=$?
  cat $log.runtime >>$log
  [[ $built == 0 && -s $stage/runtime.wasm ]] && ! grep -q Error $log.runtime ||
    { stage_two_failed "stage 2 (compiler.warp built by warp.wasm) does not build runtime.wasm"; return 1; }
  said=$(WARP_LIB=$stage $native samples/hello/hello.warp 2>&1 | tail -1)
  [[ $said == *$expected* ]] || { print -r -- "hello said: $said" >>$log; stage_two_failed "stage 2's hello said: $said"; return 1; }
  mkdir -p ${record:h} && mv $log $measured/build.log && { [[ -d $record.record ]] || mv $measured $record.record }
  print -r -- "$hash commit=$(git -C $worktree rev-parse --short HEAD) passed=$(date '+%Y-%m-%dT%H:%M:%S%z')" >$record.passed
  verdict="SELF CHECK ok: $said (stage 2 built runtime.wasm and ran hello)"
}
# stage_two_failed <what>: the failure kept beside its hash's compilers, its output and the log's path in the verdict
stage_two_failed() {
  rm -rf $record.failed.new-$$ && mkdir -p $record.failed.new-$$ && mv $log $record.failed.new-$$/build.log
  [[ -d $measured ]] && mv $measured $record.failed.new-$$/record
  print -r -- "$1 (output: the last $shown_lines lines; log $record.failed/build.log; scripts/self-check.sh --rebuild runs it again)
$(tail -$shown_lines $record.failed.new-$$/build.log)" >$record.failed.new-$$/message
  rm -rf $record.failed && mv $record.failed.new-$$ $record.failed
  verdict="SELF CHECK RED: $(<$record.failed/message)"
}
check_lane pure && self_lane
passed=$?
echo $verdict
exit $passed
