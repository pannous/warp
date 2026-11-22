#!/usr/bin/env bash
# scripts/integrate/nightly.sh: the nightly Mac run (cron at 03:00, card nightly-mac-samples). All three sweeps come from
# one commit, origin/main's tip, in a snapshot tree (/Volumes/Worktrees/nightly), at nice 19 with one build slot:
# - scripts/samples.sh writes test/samples.txt
# - scripts/samples-web.sh writes test/samples-web.txt
# - scripts/tests-web.sh writes test/tests-web.txt
# Each file starts with its header line (scripts/lib/results-header.sh).
# test/pass-rates.md gets one row per run with every tier's rate: these three, plus the full suite (test/results.txt,
# which the cloud routine commits); the native pairs stay with the cloud routine.
# Everything is committed and pushed in the main checkout. A tier whose rate fell against the previous row opens a
# card in Now for the Integrator, with the lines that went red; so does a sweep that wrote no results of the tip, its
# rate a dash instead of an old file's.
set -uo pipefail
source "$(dirname "$0")/../lib/platform.sh"
source "$(dirname "$0")/../lib/results-header.sh"
source "$(dirname "$0")/../lib/sync-main.sh"
NICENESS=19
export WARP_BUILD_SLOTS=1
SNAPSHOT=${NIGHTLY_SNAPSHOT:-/Volumes/Worktrees/nightly}
LOCK=$HOME/.cache/warp/nightly.lock
RATES=test/pass-rates.md
SWEEPS=(samples samples-web tests-web)  # scripts/<sweep>.sh writes test/<sweep>.txt
FULL_SUITE=test/results.txt
NEWLY_RED_SHOWN=60
PASSING='^(ok|SRV) '

scripts=$(cd "$(dirname "$0")/.." && pwd)
main=$warp_main_checkout
say() { echo "nightly $(date +%H:%M): $*"; }
renice -n "$NICENESS" $$ >/dev/null

mkdir -p "$(dirname "$LOCK")"
mkdir "$LOCK" 2>/dev/null || { kill -0 "$(cat "$LOCK/pid" 2>/dev/null)" 2>/dev/null && { say "already running"; exit 0; }; }
echo $$ >"$LOCK/pid"
trap 'rm -rf "$LOCK"' EXIT

cd "$main" || exit 1  # cron starts in ~, and cowtree, like git worktree, works on the repo it is run in
git fetch -q origin || exit 1
tip=$(git rev-parse --short=10 origin/main)
git worktree remove --force "$SNAPSHOT" 2>/dev/null
cowtree add "$SNAPSHOT" --detach "$tip" >/dev/null || { say "no snapshot of $tip at $SNAPSHOT (cowtree's error above)"; exit 1; }
trap 'git -C "$main" worktree remove --force "$SNAPSHOT"; rm -rf "$LOCK"' EXIT
say "sweeps of $tip in $SNAPSHOT"

"$scripts/rebuild.sh" pure "$SNAPSHOT" >/dev/null || { say "RED: rebuild pure of $tip failed, no sweeps"; todo add Now "nightly: rebuild pure of $tip failed, no sweeps ran" --no-topic >/dev/null; exit 1; }
"$scripts/samples.sh" "$SNAPSHOT" | head -1
(cd "$SNAPSHOT" && web/playground/build.sh --lib "$SNAPSHOT/lib" >/dev/null) || say "web/playground/build.sh failed: the page sweeps keep their old results"
"$scripts/samples-web.sh" "$SNAPSHOT" | tail -1
"$scripts/tests-web.sh" "$SNAPSHOT" | tail -1

# full_suite_rate <file>: the percent passing its summary, the last "m of n failed" (a test's own text may say one
# too), — when it has none
full_suite_rate() {
	grep -ao '[0-9]* of [0-9]* failed' "$1" | tail -1 | awk '{ printf "%.1f", 100 * ($3 - $1) / $3 }' | grep . || echo —
}
# newly_red <sweep>: the lines passing in the previous nightly run's results and not in the snapshot's (the main
# checkout's copy may already be a later sweep's, other runs commit it between nightlies)
newly_red() {
	local file=test/$1.txt
	local previous_run=$(git -C "$main" log -1 --format=%H --grep='^chore: nightly pass rates' origin/main -- "$file")
	comm -13 <(grep -aE "$PASSING" "$SNAPSHOT/$file" | cut -c6- | sort) <(git -C "$main" show "${previous_run}:$file" | grep -aE "$PASSING" | cut -c6- | sort) |
		head -"$NEWLY_RED_SHOWN" | sed "s|^|$1: |"
}

# table_row <cells…>: one row of the rates table
table_row() { printf '| %s ' "$@"; echo '|'; }
if [ ! -f "$main/$RATES" ]; then
	{ echo "# Pass rates, one row per nightly run (scripts/integrate/nightly.sh)"; echo
		table_row date commit "${SWEEPS[@]}" "full suite"
		table_row --- --- "${SWEEPS[@]/*/---}" ---; } >"$main/$RATES"
fi
previous=$(grep -a '^| 20' "$main/$RATES" | tail -1)
row="| $(date '+%Y-%m-%d %H:%M') | $tip"
fell=() red=() stale=() files=("$RATES")
for index in "${!SWEEPS[@]}"; do
	sweep=${SWEEPS[$index]}
	rate=$(results_rate "$SNAPSHOT/test/$sweep.txt")
	# a file whose header names another commit is an old sweep's, which this run did not write
	[ "$(results_commit "$SNAPSHOT/test/$sweep.txt")" = "$tip" ] || { stale+=("$sweep"); rate=; }
	row+=" | ${rate:-—}"
	before=$(awk -F' \\| ' -v column=$((index + 3)) '{ print $column }' <<<"$previous")
	if [ -n "$rate" ] && [[ $before =~ ^[0-9.]+$ ]] && awk -v now="$rate" -v was="$before" 'BEGIN { exit !(now < was) }'; then
		fell+=("$sweep $before% → $rate%")
		red+=("$(newly_red "$sweep")")
	fi
	cp "$SNAPSHOT/test/$sweep.txt" "$main/test/$sweep.txt" && files+=("test/$sweep.txt")
done
row+=" | $(full_suite_rate "$main/$FULL_SUITE") |"
echo "$row" >>"$main/$RATES"
say "$row"

git -C "$main" add -- "${files[@]}" &&
	git -C "$main" commit -q -m "chore: nightly pass rates at $tip ${row#*$tip | }" -- "${files[@]}" &&
	up_to_date_main "$main" && git -C "$main" push -q || say "results of $tip not pushed: look at the main checkout"

if ((${#fell[@]})); then
	todo add Now "nightly: pass rate fell at $tip: $(IFS=';'; echo "${fell[*]}")" --no-topic \
		-b "$(printf 'Newly red since the previous nightly run (scripts/integrate/nightly.sh, test/pass-rates.md):\n\n```\n%s\n```\n' "$(printf '%s\n' "${red[@]}")")" >/dev/null &&
		say "card opened: ${fell[*]}"
fi
if ((${#stale[@]})); then
	todo add Now "nightly: the sweeps ${stale[*]} wrote no results of $tip" --no-topic \
		-b "Their files in the snapshot still name an older commit in their header (scripts/integrate/nightly.sh); their rate is a dash in test/pass-rates.md." >/dev/null &&
		say "card opened: no results of $tip from ${stale[*]}"
fi
