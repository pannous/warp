#!/bin/bash
# N (default 10^8) calls of each function form against a plain call: what defaults, named arguments, overloads and closures
# cost; then map, broadcasting and `all` over a list of M (default 10^6) items. Usage: call_benchmark.sh [N] [M]
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

# over a list of M items (P142: broadcasting and `all` against map)
items="${2:-1000000}"
build="square(x) := x*x; xs = []; i = 0; while i < $items { xs.add(i); i = i + 1 }"
list_forms=(
	"build|0"
	"map|map(xs, square)"
	"lambda|xs.map(x => x * x)"
	"broadcast|square xs"
	"all|square all xs"
)
for form in "${list_forms[@]}"; do
	IFS='|' read -r name applied <<< "$form"
	code="$build; ys = $applied; count(ys)"
	[ "$name" == "build" ] && code="$build; count(xs)"
	start=$(perl -MTime::HiRes=time -e 'print time')
	got="$(timeout 120 "$warp" --no-ask --fuel 100000000000 eval "$code" 2>&1 | tail -1)"
	end=$(perl -MTime::HiRes=time -e 'print time')
	printf "%-9s %6.0f ms  %s\n" "$name" "$(echo "($end - $start) * 1000" | bc)" "$got"
done
