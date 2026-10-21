#!/bin/zsh
# Card self-warming: a `warp pure` run of a freshly built binary builds compiler.warp's module for every copy of it, so
# the installed copy (another path, a new time) runs `warp pure` without building it again.
# usage: probes/speed/self_warming.sh <built warp binary>  (run from a checkout: its lib/ is the one warmed)
built=${1:?usage: self_warming.sh <built warp binary>}
checkout=${0:A:h:h:h}
scratch=$checkout/scratch/self_warming
max_warm_seconds=2
zmodload zsh/datetime

rm -rf $scratch && mkdir -p $scratch/bin
export WARP_MODULE_CACHE=$scratch/cache WARP_NO_WINDOW=1
cd $checkout || exit 1

[[ $($built pure samples/hello/hello.warp 2>/dev/null) == *Hello* ]] || { echo "FAIL: the built binary's warp pure failed"; exit 1 }
cp $built $scratch/bin/warp  # the install: another path and a new time
start=$EPOCHREALTIME
said=$($scratch/bin/warp pure samples/hello/hello.warp 2>/dev/null)
seconds=$(( EPOCHREALTIME - start ))
[[ $said == *Hello* ]] || { echo "FAIL: installed copy said: $said"; exit 1 }
(( seconds < max_warm_seconds )) || { echo "FAIL: installed copy took ${seconds}s, building compiler.warp again"; exit 1 }
printf "ok: installed copy ran pure hello in %.2fs\n" $seconds
