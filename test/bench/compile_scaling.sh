#!/usr/bin/env bash
# How a compile's cost grows (card library-scaling-passes), in instructions retired (steadier than seconds on a shared
# Mac), each run warm (a first unmeasured run compiles the modules' machine code):
# - flatness: hello beside a library index grown by EXTRA_WORDS words its modules define costs at most
#   FLAT_PERCENT more than beside the real one (a pass going over every word of the index per program word grows it);
# - linearity: a generated program's cost from SHORT to LONG lines grows at most LINEAR_SLACK times as fast per line
#   as from TINY to SHORT (a pass going over the whole program per statement grows it with the square).
# Not part of smoke or `warp test`: run it after a change to the passes. macOS only (/usr/bin/time -l).
# Usage: test/bench/compile_scaling.sh        WARP_LIB names the lib/ to measure (default: the one beside warp)
set -eu
EXTRA_WORDS=400
FLAT_PERCENT=5
TINY=100 SHORT=1000 LONG=2000
LINEAR_SLACK=1.3
checkout=$(cd "$(dirname "$0")/../.." && pwd)
hello=$checkout/samples/hello/hello.warp
lib=${WARP_LIB:-$(dirname "$(realpath "$(command -v warp)")")}
work=$(mktemp -d "$checkout/data/compile-scaling.XXXXXX")
trap 'rm -rf "$work"' EXIT
export WARP_NO_WINDOW=1

# instructions <lib> <program>: the instructions a warm run of the program retires; a failing run ends the check
instructions() {
	WARP_LIB=$1 warp "$2" >/dev/null 2>"$work/run.log" || { echo "FAIL  $2 doesn't run with $1:" >&2; cat "$work/run.log" >&2; exit 1; }
	WARP_LIB=$1 /usr/bin/time -l warp "$2" 2>&1 >/dev/null | awk '/instructions retired/ { print $1 }'
}

# generated_program <lines>: statements of the four shapes a program is mostly made of, each using the ones before
generated_program() {
	awk -v lines="$1" 'BEGIN { for (i = 0; i < lines; i += 4) {
		printf "a%d = %d + 1\nf%d(x) := x * %d + a%d\nl%d = [f%d(1), %d, a%d]\nif a%d > 5 { print l%d#1 }\n", i, i, i+1, i+1, i, i+2, i+1, i+2, i, i, i+2
	} }'
}

failed=0
verdict() { # verdict <ok?> <line>
	if [ "$1" = 1 ]; then echo "ok    $2"; else echo "FAIL  $2"; failed=1; fi
}

grown=$work/grown-lib
mkdir "$grown" && cp -cR "$lib"/ "$grown"/
awk -v extra=$EXTRA_WORDS 'BEGIN { for (i = 1; i <= extra; i++) printf "word\tscaling_word_%d\tmath\n", i }' >> "$grown/modules/index.tsv"
real=$(instructions "$lib" "$hello")
wide=$(instructions "$grown" "$hello")
verdict $(awk -v r=$real -v w=$wide -v p=$FLAT_PERCENT 'BEGIN { print (w * 100 <= r * (100 + p)) ? 1 : 0 }') \
	"hello: $real instructions, $wide beside $EXTRA_WORDS more indexed words (at most +$FLAT_PERCENT %)"

for lines in $TINY $SHORT $LONG; do generated_program $lines > "$work/lines$lines.warp"; done
tiny=$(instructions "$lib" "$work/lines$TINY.warp")
short=$(instructions "$lib" "$work/lines$SHORT.warp")
long=$(instructions "$lib" "$work/lines$LONG.warp")
growth=$(awk -v t=$tiny -v s=$short -v l=$long -v T=$TINY -v S=$SHORT -v L=$LONG \
	'BEGIN { printf "%.2f", ((l - s) / (L - S)) / ((s - t) / (S - T)) }')
verdict $(awk -v g=$growth -v k=$LINEAR_SLACK 'BEGIN { print (g <= k) ? 1 : 0 }') \
	"$TINY/$SHORT/$LONG lines: $tiny/$short/$long instructions, a line from $SHORT on costs $growth times one before (at most $LINEAR_SLACK)"
exit $failed
