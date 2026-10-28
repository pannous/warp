#!/bin/zsh
# The warp-only acceptance check (decision warp-only-acceptance): the Rust-compiled compiler.warp compiles itself and
# that self build runs hello world.
# The pair checked is the one for the branch's compiler inputs (scripts/rebuild.sh, notes/compiler_rebuilds.md): lib/'s
# or the shared store's. A miss is said ("REBUILD NEEDED") and the existing pair is checked: the check builds only when
# asked to, with --rebuild. Nothing rebuilds the compilers because lib/ changed .
# usage: scripts/self-check.sh [--rebuild] [<worktree>, default the current checkout]   (exit 0 = Hello printed)
rebuild=
[[ ${1-} == --rebuild ]] && { rebuild=1; shift; }
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
# the seed lies beside the launcher; the pair in the build folder the launcher links to (main-builds/<tip>)
cp ${main_warp:h}/warp-seed.wasm $scratch/ 2>/dev/null || cp ${main_warp:A:h}/warp-seed.wasm $scratch/ || exit 1
# main's warp.wasm beside the copy too: the binary takes an empty folder as holding the worktree's pair, and would look
# for it there instead of in the worktree's lib/ (standalone.rs build_path)
cp ${main_warp:A:h}/warp.wasm $scratch/ || exit 1
# the pair for these inputs: lib/'s, else the store's; built by the copy only with --rebuild. The Rust fingerprint is
# that of main's binary, read from the pair beside it (the copy has none beside it)
beside=${main_warp:A:h}/warp.wasm
[[ -f $beside ]] || beside=${main_warp:h}/compiler.wasm  # the old name (transition)
export WARP_RUST_FINGERPRINT=$(LC_ALL=C tr '\0' '\n' <$beside 2>/dev/null | LC_ALL=C sed -n '/warp\.build/{n;n;p;q;}' | LC_ALL=C grep -ao '^[0-9a-f]\{16\}' | head -1)
if [[ -n $rebuild ]]; then
  pair=$(WARP_BINARY=$warp $worktree/scripts/rebuild.sh both $worktree 2>&1)
else
  pair=$(WARP_BINARY=$warp $worktree/scripts/rebuild.sh --check both $worktree 2>&1)
fi
case $? in
  0) ;;
  3) print -r -- "self-check: REBUILD NEEDED: no pair for these compiler inputs in lib/ nor the store; checking the existing pair (scripts/self-check.sh --rebuild builds it)" ;;
  *) print -r -- "SELF CHECK RED: scripts/rebuild.sh failed in $worktree: $(print -r -- $pair | tail -3)"; exit 1 ;;
esac
print -r -- $pair
# warp-host's lanes too, when there is one: WARP_HOST_BINARY, else warp-host beside main's binary (no per-tree cargo
# build, card scripts-off-cargo)
host=${WARP_HOST_BINARY:-${main_warp:h}/warp-host}
check_host() {
  said=$($host $@ samples/hello/hello.warp 2>&1 | tail -1)
  [[ $said == *$expected* ]] || { verdict="SELF CHECK RED: warp-host $* hello said: $said"; return 1; }
  verdict="SELF CHECK ok: $said"
}
# the self test itself (AGENTS.md "Testing"): warp.wasm compiles compiler.warp and that build, promoted to warp-self.wasm,
# runs hello; a warp-self.wasm of these inputs in lib/ or the store is taken as it is (scripts/rebuild.sh self). Red
# when the compile fails: the self lane was red unnoticed for hours while it was skipped here (card self-check-compile)
self_lane() {
  said=$(WARP_BINARY=$warp WARP_TIME_LIMIT=${WARP_PROMOTE_TIME_LIMIT:-7200} $worktree/scripts/rebuild.sh self $worktree 2>&1) || {
    verdict="SELF CHECK RED: warp.wasm does not compile compiler.warp: $(print -r -- $said | tail -2 | cut -c1-400)"; return 1; }
  check_lane self
}
check_lane pure && self_lane && { [[ ! -x $host ]] || { check_host --compiler full run && check_host run; }; }
passed=$?
echo $verdict
exit $passed
