#!/bin/zsh
# usage: probes/aot/run_speed.sh <warp binary>: median ms over 9 runs of `warp run <file>` and plain `warp <file>` on fresh
# (never seen) programs and on an unchanged one, and of the front end alone (`warp lower`, `warp compile --wasm`).
# Plain runs need a stub: WARP_RUNTIME_STUB or warp-runtime next to the binary (notes/aot.md "Run speed").
W=${1:a}; D=$(mktemp -d "${TMPDIR:-/tmp}/run_speed.XXXX"); zmodload zsh/datetime
ms() { local s=$EPOCHREALTIME; "$@" >/dev/null 2>&1; printf '%.0f\n' $(( (EPOCHREALTIME - s) * 1000 )); }
median() { sort -n | sed -n 5p; }
program() { printf 'square(x) := x * x\nfib(n) := if n < 2 then n else fib(n-1) + fib(n-2)\nprint square(%s)\nfib(20) + %s\n' $1 $1 > $2; }
fresh() { for i in {1..9}; do f=$D/p$RANDOM$i.warp; program $RANDOM$i $f; ms "$@" $f; rm -f $f ${f:r} ${f:r}.wasm; done | median; }
same=$D/same.warp; program 3 $same; $W $same >/dev/null 2>&1
unchanged() { for i in {1..9}; do ms "$@" $same; done | median; }
echo "warp run:       fresh $(fresh $W run) ms, unchanged $(unchanged $W run) ms"
echo "warp <file>:    fresh $(fresh $W) ms, unchanged $(unchanged $W) ms"
echo "compile --wasm: fresh $(fresh $W compile --wasm) ms"
rm -rf $D
