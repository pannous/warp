#!/bin/zsh
# card fermyon-hosting: warp build --wagi answers each route as WAGI does, run by wasmtime (no Spin needed)
#   probes/hosting/wagi_routes.sh [warp binary]
WARP=${${1:-scratch/warp}:A}
APP=${0:A:h}/wagi_routes.warp
OUT=${0:A:h:h:h}/scratch/wagi_routes
mkdir -p $OUT && cp $APP $OUT/app.warp
(cd $OUT && WARP_NO_WINDOW=1 $WARP build --wagi app.warp) || exit 1

failures=0
expect() { # route, answer
	local got=$(wasmtime run -W gc,function-references,exceptions --env REQUEST_METHOD=GET --env PATH_INFO=$1 $OUT/app.wagi.wasm 2>&1 | tail -1)
	if [[ $got == "$2" ]]; then echo "ok   $1 → $got"; else echo "FAIL $1 → $got (want $2)"; failures=$((failures + 1)); fi
}
expect / 'hello from the edge'
expect /fib/20 '{"n":20,"fib":6765}'
expect /list '[1,2.5,"a",true,[3]]'
expect /nested '{"name":"x","tags":["a","b"],"inner":{"k":false}}'
exit $failures
