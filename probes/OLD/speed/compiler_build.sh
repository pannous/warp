#!/bin/zsh
# Decision compiler-build: compiler.wasm is a build output next to the binary. A fresh binary's first `warp pure` builds
# it, an install copies it beside the installed binary, which then runs at once; a changed lib/ only gets a one-line
# notice naming `warp rebuild`, and `warp rebuild pure` builds it anew. A lib/ elsewhere keeps its own in <lib>/../data/.
# usage: probes/speed/compiler_build.sh <built warp binary>  (run from a checkout: its lib/ is the one built for)
built=${1:?usage: compiler_build.sh <built warp binary>}
checkout=${0:A:h:h:h}
scratch=$checkout/scratch/compiler_build
max_seconds=2
zmodload zsh/datetime

rm -rf $scratch && mkdir -p $scratch/bin $scratch/other
export WARP_NO_WINDOW=1 WARP_HINTS=0
cd $checkout || exit 1
fail() { echo "FAIL: $*"; exit 1 }

cp $built $scratch/bin/warp
[[ $($scratch/bin/warp pure samples/hello/hello.warp 2>/dev/null) == *Hello* ]] || fail "the first warp pure failed"
[[ -f $scratch/bin/compiler.wasm ]] || fail "no compiler.wasm next to the binary"

mkdir $scratch/installed && cp $scratch/bin/warp $scratch/bin/compiler.wasm $scratch/installed/
start=$EPOCHREALTIME
said=$($scratch/installed/warp pure samples/hello/hello.warp 2>$scratch/stderr.txt)
seconds=$(( EPOCHREALTIME - start ))
[[ $said == *Hello* ]] || fail "installed copy said: $said"
(( seconds < max_seconds )) || fail "installed copy took ${seconds}s"
[[ -s $scratch/stderr.txt ]] && fail "installed copy said on stderr: $(<$scratch/stderr.txt)"
printf "ok: installed copy ran pure hello in %.2fs\n" $seconds

cp -R lib $scratch/other/lib && echo "// changed" >> $scratch/other/lib/compiler/compiler.warp
other_said=$(WARP_LIB=$scratch/other/lib $scratch/installed/warp pure samples/hello/hello.warp 2>&1)
[[ -f $scratch/other/data/compiler.wasm ]] || fail "another lib/ got no compiler.wasm of its own: $other_said"
echo "ok: another lib/ builds $scratch/other/data/compiler.wasm"

echo "// changed again" >> $scratch/other/lib/compiler/compiler.warp
stale=$(WARP_LIB=$scratch/other/lib $scratch/installed/warp pure samples/hello/hello.warp 2>&1)
[[ $stale == *"warp rebuild"* && $stale == *Hello* ]] || fail "a changed lib/ ran without the notice: $stale"
echo "ok: a changed lib/ says: $(echo $stale | grep 'warp rebuild')"
WARP_LIB=$scratch/other/lib $scratch/installed/warp rebuild pure >/dev/null || fail "warp rebuild pure failed"
rebuilt=$(WARP_LIB=$scratch/other/lib $scratch/installed/warp pure samples/hello/hello.warp 2>&1)
[[ $rebuilt == *"warp rebuild"* ]] && fail "the notice stays after warp rebuild: $rebuilt"
echo "ok: warp rebuild pure ends the notice"
