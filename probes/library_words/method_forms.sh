#!/bin/sh
# x.f(a) is f(x, a) for builtins: extrema, text builtins, conversions; a program's own or a class's word wins
warp=${1:-scratch/warp}
for program in 'xs=[1,5,2]; xs.max()' '[3,1].min()' 'xs=[1,5,2]; xs.max() + max(xs)' '" a ".trim()' '"abc".byte_at(2)' \
	'"a b".split(" ").count()' 'class P{x:int; def max():=x}; P(4).max() + [1,2].max()' 'max(a, b) := a; [1,2].max(9)' \
	'"42".int() + 1' 't="ab"; t.upper().lower()' 'x=2.5; x.round()' 'x=3; x.float()'; do
	printf '%s  =>  ' "$program"; echo "$program" | WARP_NO_WINDOW=1 $warp 2>&1 | tail -1
done
