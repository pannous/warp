#!/usr/bin/env bash
# A global or main-level variable named as one of the program's functions is refused naming the word and each place
# it is declared, in one file or across the files it uses (card template-holes-duplicate). A loop variable of the name
# is its own. Run it with bash test/errors/test_variable_named_as_function.sh; the warp under test is WARP (default
# the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_WINDOW=0
status=0
folder=$(mktemp -d) && trap 'rm -rf "$folder"' EXIT
gives() {
	local program=$1 expected=$2 said
	said=$(cd "$folder" && "$warp" "$program" 2>&1 | tail -1)
	case $said in
		*"$expected"*) echo "✓ $program: $said" ;;
		*) echo "✗ $program: expected …$expected…, got: $said"; status=1 ;;
	esac
}
printf 'global marks = {}\nmarked(n: int): int := { marks[n] = yes; n }\n' >"$folder/a.warp"
printf 'marks(n: int): list := [n, n]\n' >"$folder/b.warp"
printf 'use "a.warp"\nuse "b.warp"\nprint marks(2)\n' >"$folder/both.warp"
printf 'global marks = 0\nmarks(n: int): list := [n, n]\nprint marks(2)\n' >"$folder/one.warp"
gives both.warp 'marks is declared 2 times, at a.warp:1 and b.warp:1: a variable and a function share the name'
gives one.warp 'marks is declared 2 times, at one.warp:1 and one.warp:2'
gives 'area(r: int): int := r * r; area = area(2); area' 'area is declared'
gives 'area(r: int): int := r * r; for area in [3] { print area }' '3'
exit $status
