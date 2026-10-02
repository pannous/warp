#!/bin/bash
# Fails loudly when a tracked file under probes/ is not a small hand-written source:
# repo copies (src/ tests/ target/ vendor/ .git paths), blobs over 100 KB, or extensions outside the .gitignore allowlist.
# Usage: probes/check_layout.sh [tree-ish]   (default: the index; pass a commit to check what was committed)
MAX_BYTES=102400
FORBIDDEN_PATH='(^|/)(src|tests|target|vendor|\.git)/'
cd "$(dirname "$0")/.."

allowed_extensions=$(sed -nE 's#^!probes/\*\*/\*\.([A-Za-z0-9]+)$#\1#p' .gitignore | paste -sd'|' -)
allowed_name="\.(${allowed_extensions})$"

if [[ -n $1 ]]; then
	entries=$(git ls-tree -r --format='%(objectname) %(path)' "$1" -- probes)
else
	entries=$(git ls-files -s -- probes | awk '{print $2, $4}')
fi

offenders=()
while read -r blob path; do
	[[ -z $path ]] && continue
	relative=${path#probes/}
	if [[ $relative =~ $FORBIDDEN_PATH ]]; then offenders+=("$path (repo copy path)")
	elif ! [[ $path =~ $allowed_name ]]; then offenders+=("$path (extension not allowlisted)")
	elif (( $(git cat-file -s "$blob") > MAX_BYTES )); then offenders+=("$path (over $((MAX_BYTES / 1024)) KB)")
	fi
done <<< "$entries"

if (( ${#offenders[@]} )); then
	echo "❌ probes/ layout violated: probes/ holds hand-written sources only (scratch/ for copies, data/ for logs)" >&2
	printf '  • %s\n' "${offenders[@]}" >&2
	exit 1
fi
