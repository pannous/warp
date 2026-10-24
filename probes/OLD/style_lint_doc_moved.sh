#!/bin/bash
# scripts/style_lint.py doc-moved on the review cases that were fixed by hand: each file as the branch left it, against
# the file before the branch, must come out of --fix as the hand fix made it (card style-linter).
# Usage: probes/style_lint_doc_moved.sh
REPO=$(cd "$(dirname "$0")/.." && pwd)
WORK=$REPO/scratch/style_lint_doc_moved
# hand fix, the commit whose parent is the base, the file
CASES=(
	"47eef1a03 1238da99d src/lowering/collections/broadcasting.rs"
	"eee4ff99d bd8c27090 src/wasm_emitter/module_finish.rs"
	"6b6c1e158 57f16270b src/lowering/words/library_words.rs"
)

failures=0
for case in "${CASES[@]}"; do
	read -r fix introduced path <<< "$case"
	rm -rf "$WORK" && mkdir -p "$WORK/$(dirname "$path")" && cd "$WORK" && git init -q
	git -C "$REPO" show "$introduced^:$path" > "$path" && git add -A && git commit -qm base
	git -C "$REPO" show "$fix^:$path" > "$path"
	python3 "$REPO/scripts/style_lint.py" --base HEAD "$path" > /dev/null 2>&1 && { echo "❌ $fix: no violation found in $path"; failures=$((failures + 1)); continue; }
	python3 "$REPO/scripts/style_lint.py" --fix --base HEAD "$path" > fix.log || { echo "❌ $fix: --fix left violations"; cat fix.log; failures=$((failures + 1)); continue; }
	if git -C "$REPO" show "$fix:$path" | diff -q - "$path" > /dev/null; then echo "✅ $fix: $(grep -c '^fixed' fix.log) doc line(s) back in $path"
	else echo "❌ $fix: --fix differs from the hand fix"; git -C "$REPO" show "$fix:$path" | diff - "$path" | head; failures=$((failures + 1))
	fi
done
rm -rf "$WORK"
exit $((failures > 0))
