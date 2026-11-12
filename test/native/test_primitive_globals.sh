#!/usr/bin/env bash
# a primitive file's `global count = 0` is one mutable value every function reads and writes, as `global` names one
# in a GC program; a top-level name without `global` stays a constant (card primitive-class-gc, user OK 11-11). A fixed
# array `i64[8]` (wiki/array.md) holds zeros, read and written by index, a global one shared by every function, an
# index outside it a trap (OK 11-12). Run it
# with bash test/native/test_primitive_globals.sh; the warp under test is WARP (default the warp on PATH), its lib LIB
# (default its own; a tree's: LIB=<tree>/lib)
warp=${WARP:-warp}
lib=${LIB:-$(dirname "$(readlink -f "$(command -v "$warp")")")}
export WARP_WINDOW=0
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0
check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
ran() { # ran <name> <program>: the primitive file's last printed line
	printf '@primitive\n%s\n' "$2" >"$stage/$1.warp"
	"$warp" --lib "$lib" "$stage/$1.warp" 2>&1 | tail -1
}

check "a counter bumped twice" "$(ran counter 'global painted = 0
next_painting(): i64 := {
	painted += 1
	painted
}
next_painting()
next_painting()')" 2
check "a float global" "$(ran float 'global total = 0.5
add(x: f64): f64 := {
	total = total + x
	total
}
add(1.0)
add(2.0)')" 3.5
check "a parameter hides the global" "$(ran hidden 'global n = 7
twice(n: i64): i64 := n * 2
twice(3) + n')" 13
check "a global given no number refused" "$(ran refused 'global name = "a"
1' | grep -c 'a global not given a number')" 1
check "a global array kept by slot" "$(ran kept 'global kept = i64[8]
keep(slot: i64, value: i64) := {
	kept[slot] = value
}
kept_value(slot: i64): i64 := kept[slot]
keep(7, 40)
keep(0, 2)
kept_value(7) + kept_value(0) + kept_value(3)')" 42
check "a local array counted from 1" "$(ran local 'f(): i32 := {
	bytes = u8[4]
	bytes#2 = 300 - 45
	bytes[1] + bytes#4
}
f()')" 255
check "an index outside the array traps" "$(ran outside 'global kept = i64[8]
kept[8] = 1
1' | grep -c -i 'unreachable\|trap')" 1
exit $((failures > 0))
