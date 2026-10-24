#!/bin/bash
# card write-hook: runs .claude/hooks/claudeignore-hook.sh on Write payloads, in bash's normal and POSIX mode (where
# macOS bash's echo expands backslashes and broke the JSON), and checks that an allowed path passes and an ignored one
# is blocked with its reason on stderr (a hook's exit 2 shows only stderr; stdout gave "No stderr output")
cd "$(dirname "$0")/../.." || exit 1
HOOK=.claude/hooks/claudeignore-hook.sh
CONTENT_FILE=probes/write_hook/fstrings.py
failures=0

payload() {
	jq -n --arg path "$1" --rawfile content "$CONTENT_FILE" '{tool_name: "Write", tool_input: {file_path: $path, content: $content}}'
}

check() { # mode, path, expected exit, expected stderr pattern
	local stderr
	stderr=$(payload "$2" | env $1 CLAUDE_PROJECT_DIR="$PWD" "$HOOK" 2>&1 >/dev/null)
	local code=$?
	if [[ $code -ne $3 || ! "$stderr" =~ $4 ]]; then
		echo "FAIL [$1] $2: exit $code, stderr '$stderr'"
		failures=$((failures + 1))
	fi
}

for mode in "" "POSIXLY_CORRECT=1"; do
	check "$mode" "$PWD/probes/write_hook/out.py" 0 "^$"
	check "$mode" "$PWD/vendor/out.py" 2 "Blocked: path matches .claudeignore pattern 'vendor'"
done
[[ $failures -eq 0 ]] && echo "write hook: ok"
exit $failures
