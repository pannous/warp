#!/bin/zsh
# usage: r3-meta_fuel.sh <binary> <program>: the fuel (wasm steps) the program needs, by binary search
lo=1; hi=2000000000
while (( hi - lo > hi / 1000 )); do
	mid=$(( (lo + hi) / 2 ))
	if WARP_FUEL=$mid CI=1 $1 "$2" 2>&1 | grep -q "out of fuel"; then lo=$mid; else hi=$mid; fi
done
echo "$1 $hi"
