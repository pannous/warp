#!/bin/sh
# Refuses pkill and killall in Bash: kills go by PID only (
# `pkill -f "…" -n` read the trailing -n as a second pattern and killed every claude session).
command=$(jq -r '.tool_input.command // empty')
if printf '%s\n' "$command" | grep -Eq '(^|[;&|(`]|\$\(|then|do|else)[[:space:]]*(sudo[[:space:]]+)?(pkill|killall)([[:space:]]|$)'; then
	echo "refused: pkill/killall match by pattern and once killed every warp session. Find the PID (pgrep -fl, ps) and kill <pid>." >&2
	exit 2
fi
exit 0
