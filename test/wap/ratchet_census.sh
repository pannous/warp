#!/usr/bin/env bash
# The road to self-build as a ratchet (notes/wap.md "Road to self-build"): scripts/wap-census.warp over all of wap's
# source counts the files wap types whole and the functions it types alone; neither may drop below the floors here.
# Raise a floor by hand when the census's total line shows more. The compiler's own source stays fully typed (dynamic
# typing, any, is for programs; user decision via warp-lead, 2026-11-18): no place of it is typed any, outside the
# preload, the language programs get.
# Run it with bash test/wap/ratchet_census.sh (or test/wap/run.sh --wait test/wap/ratchet_census.sh, never in run.sh's
# default set: about 4 min, the rest about 1); the warp under test is WARP (default the warp on PATH)
# files outside the preload: a preload file is no program but every program's library, typed function by function
# (warp-lead, 2026-11-18)
FILES_TYPED_WHOLE=4
FUNCTIONS_TYPED=511
# the census runs in this many batches at once, each loading the preload once (one batch of all files: 10 min)
BATCHES=6

source "$(dirname "$0")/compile.sh"
root="$(cd "$(dirname "$0")/../.." && pwd)"

# each batch's report in a file of its own ($stage, compile.sh's), then the total lines summed into one: files typed
# whole, files, functions typed, functions, places typed any
files=$(cd "$root" && git ls-files 'wap/*.wap')
(cd "$root" && echo "$files" | split -l $(( ($(echo "$files" | wc -l) + BATCHES - 1) / BATCHES )) - "$stage/batch.")
ls "$stage"/batch.* | (cd "$root" && xargs -P "$BATCHES" -I{} sh -c 'scripts/job-slot.sh "$1" --no-window scripts/wap-census.warp $(cat "$2") >"$2.out" 2>&1' _ "$warp" {})
total=$(cat "$stage"/batch.*.out | grep '^total' | sed 's/[^0-9]\{1,\}/ /g' |
	awk '{ for (i = 1; i <= NF; i++) sum[i] += $i } END { if (NR) printf "%d of %d files typed whole, %d of %d functions typed, %d places typed any", sum[1], sum[2], sum[3], sum[4], sum[5] }')
whole=$(echo "$total" | sed -n 's/^\([0-9]*\) of .* files typed whole.*/\1/p')
typed=$(echo "$total" | sed -n 's/.*files typed whole, \([0-9]*\) of .* functions typed.*/\1/p')
# the places typed any in the compiler's own files: the preload's are the language's dynamic words for programs,
# `show(x: any)`, a Node's `Data(value: any)`
compiler_anys=$(cat "$stage"/batch.*.out | awk '/^== /{ file = $2 } /^any\t/ && file !~ /^wap\/preload\// { print file ": " substr($0, 5) }')
at_least() { [ -n "$1" ] && [ "$1" -ge "$2" ] && echo yes || echo "${1:-no total line} < $2"; }

check "the files wap types whole stay at least $FILES_TYPED_WHOLE ($total)" "$(at_least "$whole" $FILES_TYPED_WHOLE)" "yes"
check "the functions wap types alone stay at least $FUNCTIONS_TYPED" "$(at_least "$typed" $FUNCTIONS_TYPED)" "yes"
check "no place of the compiler's own source is typed any" "$( [ -n "$total" ] && echo "$compiler_anys" || echo "no total line")" ""
exit $((failures > 0))
