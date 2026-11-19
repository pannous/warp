#!/bin/sh
# AGENTS.md (and its link CLAUDE.md) change only through warp-interviewer, who shows the user the diff and lands it
# once approved (AGENTS.md "Talking"); the user's own edits outside the fleet's tmux sessions pass.
#   scripts/agents-md-gate.sh --tool     PreToolUse hook (.claude/settings.json): the tool call's JSON on stdin
#   scripts/agents-md-gate.sh --commit   pre-commit hook: the staged files
# Exit 2 refuses with the reason on stderr.
GATED='(^|/)(AGENTS|CLAUDE)\.md$'
INTERVIEWER=warp-interviewer

session=$(tmux display-message -p '#S' 2>/dev/null)
{ [ -z "${TMUX:-}" ] || [ "$session" = "$INTERVIEWER" ]; } && exit 0

case "${1:-}" in
	--tool) touched=$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("tool_input", {}).get("file_path", ""))') ;;
	--commit) touched=$(git diff --cached --name-only)
		# a merge that only brings in the other side's AGENTS.md (staged file == MERGE_HEAD's) is no edit of it
		if git rev-parse -q --verify MERGE_HEAD >/dev/null; then
			touched=$(echo "$touched" | while read -r f; do
				[ "$(git rev-parse -q ":$f" 2>/dev/null)" = "$(git rev-parse -q "MERGE_HEAD:$f" 2>/dev/null)" ] || echo "$f"
			done)
		fi ;;
	*) echo "usage: $0 --tool | --commit" >&2; exit 1 ;;
esac
echo "$touched" | grep -qE "$GATED" || exit 0

echo "AGENTS.md changes only through $INTERVIEWER: send it the exact old and new lines; it asks the user with the diff and lands them (AGENTS.md \"Talking\"). Session $session may not change it." >&2
exit 2
