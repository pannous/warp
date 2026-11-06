#!/usr/bin/env bash
# scripts/repeated-code.sh <tree> [--report]: repeated code within one warp file (card repeated-code-check; user
# 2026-11-06: lib/units.warp wrote one 35-unit list 12 times). A code line of LONG_LINE+ characters or a literal list
# of LONG_LIST+ written again in the same file is a repeat: name it once and use the name.
# Without --report: exit 1 naming both lines when the tree's change (against origin/main) adds a repeat (smoke.sh);
# repeats already on main never block. --report: every repeat of core/, extensions/ and lib/ into
# data/repeated-code.txt, so each gets a card (after-push.sh)
set -uo pipefail
LONG_LINE=60
LONG_LIST=40
REPORT=data/repeated-code.txt
tree=$(cd "${1:?usage: repeated-code.sh <tree> [--report]}" && pwd) || exit 2
cd "$tree" || exit 2
# repeats <file>: "<line>\t<its first line>\t<text>" for each long line, else long list, met before in the file
repeats() {
	awk -v long_line=$LONG_LINE -v long_list=$LONG_LIST '
	function seen(text) { if (text in first) { printf "%d\t%d\t%s\n", FNR, first[text], text; return 1 } first[text] = FNR; return 0 }
	{
		line = $0; gsub(/^[ \t]+|[ \t]+$/, "", line)
		if (line ~ /^(#|\/\/)/) next
		if (length(line) >= long_line && seen(line)) next
		rest = line
		while (match(rest, /\[[^][]*\]/)) { list = substr(rest, RSTART, RLENGTH); if (length(list) >= long_list && list != line) seen(list); rest = substr(rest, RSTART + RLENGTH) }
	}' "$1"
}
shown() { awk -F'\t' -v file="$1" '{ printf "%s:%d repeats line %d: %s\n", file, $1, $2, substr($3, 1, 100) }'; }
if [ "${2-}" = --report ]; then
	mkdir -p "$(dirname "$REPORT")"
	for file in $(git ls-files 'core/*.warp' 'extensions/*.warp' 'lib/*.warp'); do repeats "$file" | shown "$file"; done >"$REPORT"
	echo "repeated-code: $(wc -l <"$REPORT" | tr -d ' ') repeats in $tree/$REPORT"
	exit 0
fi
base=$(git merge-base origin/main HEAD) || exit 2
added=$(for file in $(git diff --name-only "$base" -- '*.warp'); do
	[ -f "$file" ] || continue
	lines=" $(git diff -U0 "$base" -- "$file" | sed -n 's/^@@ .* +\([0-9]*\)\(,\([0-9]*\)\)\{0,1\} @@.*/\1 \3/p' | while read -r start count; do seq "$start" $((start + ${count:-1} - 1)); done | tr '\n' ' ')"
	repeats "$file" | awk -F'\t' -v lines="$lines" 'index(lines, " " $1 " ") || index(lines, " " $2 " ")' | shown "$file"
done)
[ -z "$added" ] && exit 0
echo "repeated-code: this change writes code a second time in one file; name it once and use the name:" >&2
echo "$added" >&2
exit 1
