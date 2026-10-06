#!/bin/bash
# N (default 10^8) calls of each function form against a plain call: what defaults, named arguments, overloads and closures cost
warp="$(dirname "$0")/../scratch/warp"
calls="${1:-100000000}"
loop='total=0; i=0; while i < N { total = total + CALL; i = i + 1 }; total'
forms=(
	"plain|def f(a, b){ a + b }|f(i, 1)"
	"default|def f(a, b=1){ a + b }|f(i)"
	"named|def f(a, b){ a + b }|f(a: i, b: 1)"
	"overload|def f(a){ a + 1 }; def f(a, b){ a + b }|f(i, 1)"
	"typed|def f(a:int, b:int) -> int { a + b }|f(i, 1)"
	"lambda|f = (a, b) => a + b|f(i, 1)"
	"closure|k = 1; g = x => x + k|g(i)"
	"returned|def mk(k){ x => x + k }; h = mk(1)|h(i)"
	"inline|def f(a, b){ a + b }|i + 1"
)
for form in "${forms[@]}"; do
	IFS='|' read -r name definition call <<< "$form"
	code="$definition; ${loop/CALL/$call}"; code="${code/N/$calls}"
	start=$(perl -MTime::HiRes=time -e 'print time')
	got="$(timeout 120 "$warp" --no-ask --fuel 100000000000 eval "$code" 2>&1 | tail -1)"
	end=$(perl -MTime::HiRes=time -e 'print time')
	printf "%-9s %6.0f ms  %s\n" "$name" "$(echo "($end - $start) * 1000" | bc)" "$got"
done
