#!/usr/bin/env bash
# The road to self-build as a ratchet (notes/wap.md "Road to self-build"): scripts/wap-census.warp over all of wap's
# source counts the files wap types whole, which may not drop below the floor here, and the functions it does not type
# alone, which may not rise above the ceiling: deleting a function never trips it, a function losing its typing does
# (warp-simplifier, 2026-11-19). Move both by hand when the census's total line shows better. The compiler's own source stays fully typed (dynamic
# typing, any, is for programs; user decision via warp-lead, 2026-11-18): no place of it is typed any, outside the
# preload, the language programs get.
# Run it with bash test/wap/ratchet_census.sh [--compiler <module>] (or test/wap/run.sh --wait test/wap/ratchet_census.sh, never in run.sh's
# default set: about 1 min more); the warp under test is WARP (default the warp on PATH)
# files outside the preload: a preload file is no program but every program's library, typed function by function
# (warp-lead, 2026-11-18)
FILES_TYPED_WHOLE=53
FUNCTIONS_UNTYPED=0

source "$(dirname "$0")/compile.sh"
root="$(cd "$(dirname "$0")/../.." && pwd)"

# one census of every file, in one process: the files share their modules and their typing (scripts/wap-census.warp),
# compiled by the compiler under test (--compiler <module>, or wap's source run by the old compiler), and run from the
# tree's root, where it reads wap/ and notes/
report="$stage/census.out"
census="$stage/wap-census.wasm"
(cd "$root" && $warp --no-window "$compiler" scripts/wap-census.warp "$census" >"$report" 2>&1 &&
	scripts/job-slot.sh $warp --no-window "$census" $(git ls-files 'wap/*.wap') >"$report" 2>&1)
total=$(grep '^total' "$report" | cut -f2)
whole=$(echo "$total" | sed -n 's/^\([0-9]*\) of .* files typed whole.*/\1/p')
counted=$(echo "$total" | sed -n 's/.*files typed whole, \([0-9]*\) of \([0-9]*\) functions typed.*/\2 - \1/p')
untyped=$([ -n "$counted" ] && echo $(($counted)))
# the places typed any in the compiler's own files: the preload's are the language's dynamic words for programs,
# `show(x: any)`, a Node's `Data(value: any)`
compiler_anys=$(awk '/^== /{ file = $2 } /^any\t/ && file !~ /^wap\/preload\// { print file ": " substr($0, 5) }' "$report")
at_most() { [ -n "$1" ] && [ "$1" -le "$2" ] && echo yes || echo "${1:-no total line} > $2"; }
at_least() { [ -n "$1" ] && [ "$1" -ge "$2" ] && echo yes || echo "${1:-no total line} < $2"; }

check "the files wap types whole stay at least $FILES_TYPED_WHOLE ($total)" "$(at_least "$whole" $FILES_TYPED_WHOLE)" "yes"
check "the functions wap does not type alone stay at most $FUNCTIONS_UNTYPED" "$(at_most "$untyped" $FUNCTIONS_UNTYPED)" "yes"
check "wap reads, lowers and binds its own source" "$(if [ -n "$total" ]; then grep '^front' "$report"; else echo "no total line"; fi)" ""
check "no place of the compiler's own source is typed any" "$( [ -n "$total" ] && echo "$compiler_anys" || echo "no total line")" ""
exit $((failures > 0))
