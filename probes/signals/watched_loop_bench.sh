#!/bin/bash
# What a watched variable costs in a tight loop (card cleanup-signal-family step 3): the plain variable, the static
# check variable_signals inserts after each write (`on change x`), and the cell signal_values makes of a variable a
# function listens to (`watch(x)`), which step 3 would make of every watched variable. Usage: watched_loop_bench.sh [warp] [N]
warp="${1:-$(dirname "$0")/../../scratch/warp}"
writes="${2:-10000000}"
loop="i = 0; while i < $writes { x = x + 1; i = i + 1 }; print seen; x"
forms=(
	"plain|x = 0; seen = 0"
	"static|x = 0; seen = 0; on change x { if x % $writes == 0 then seen = seen + 1 }"
	"cell|watch(s) := on change s { if value % $writes == 0 then seen = seen + 1 }; x = 0; seen = 0; watch(x)"
)
for form in "${forms[@]}"; do
	IFS='|' read -r name setup <<< "$form"
	start=$(perl -MTime::HiRes=time -e 'print time')
	got="$(WARP_NO_WINDOW=1 WARP_HINTS=0 timeout 300 "$warp" --no-ask --fuel 100000000000 eval "$setup; $loop" 2>&1 | tail -2 | tr '\n' ' ')"
	end=$(perl -MTime::HiRes=time -e 'print time')
	printf "%-7s %7.0f ms  %s\n" "$name" "$(echo "($end - $start) * 1000" | bc)" "$got"
done
