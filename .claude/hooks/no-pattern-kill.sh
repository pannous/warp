#!/bin/sh
# Refuses pkill and killall in Bash: kills go by PID only (user 2026-11-07, notes/postmortem_0413_crash.md:
# `pkill -f "…" -n` read the trailing -n as a second pattern and killed every claude session at 04:13).
command=$(jq -r '.tool_input.command // empty')
if printf '%s\n' "$command" | grep -Eq '(^|[;&|(`]|\$\(|then|do|else)[[:space:]]*(sudo[[:space:]]+)?(pkill|killall)([[:space:]]|$)'; then
	echo "refused: pkill/killall match by pattern and once killed every warp session (notes/postmortem_0413_crash.md). Find the PID (pgrep -fl, ps) and kill <pid>." >&2
	exit 2
fi
exit 0
