#!/bin/bash
# Spawns warp agent roles as Remote Control sessions in detached tmux (tmux name warp-<role>) and prints their URLs.
#   notes/agents/start.sh                         integrator, interviewer and fixer (skips ones already running)
#   notes/agents/start.sh <role>                  one of integrator, interviewer, fixer
#   notes/agents/start.sh worker <branch> "<task>"   a feature worker on that branch
# The prompt of a role is notes/agents/<role>.md, read from origin/main by the session itself.
REPO=/Users/me/dev/angles/warp
SPAWN="$HOME/dev/bin/claude-remote.sh"
SERVICES=(integrator interviewer fixer)

supervisor_name() { echo "${WARP_SUPERVISOR:-the session that started you (see ListAgents)}"; }

prompt_for() {
	local role=$1
	echo "You are the warp $role. Read notes/agents/common.md and notes/agents/$role.md (use \`git -C $REPO show origin/main:notes/agents/$role.md\` if the checkout is stale) and act as described there. Your Supervisor is $(supervisor_name)."
}

running() { tmux has-session -t "=warp-$1" 2>/dev/null; }

spawn() {
	local name=$1 prompt=$2
	if running "$name"; then echo "warp-$name already runs (tmux attach -t warp-$name)"; return; fi
	"$SPAWN" "$name" "$REPO" "$prompt" | grep -E "Open:|WARNING|busy"
}

case "$1" in
	"") for role in "${SERVICES[@]}"; do spawn "$role" "$(prompt_for "$role")"; done ;;
	worker)
		[ -n "$2" ] && [ -n "$3" ] || { echo "usage: $0 worker <branch> \"<task>\"" >&2; exit 1; }
		spawn "$2" "$(prompt_for worker) YOUR BRANCH: $2. YOUR TASK: $3" ;;
	*) spawn "$1" "$(prompt_for "$1")" ;;
esac
