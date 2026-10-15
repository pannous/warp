#!/bin/bash
# Gives Codex one or two cards from the board and hands completed branches to the Integrator.
# Scheduled by the user's crontab every five hours. CODEX_PROBE_PROMPT runs a small isolated probe.
set -uo pipefail

DEFAULT_REPO=/Users/me/dev/angles/warp
INTEGRATOR=warp-integrator
CODEX_BOSS="Warp Boss"
DEFAULT_TURN_TIMEOUT_SECONDS=16200
EXEC_EXIT_GRACE_SECONDS=30
POLL_SECONDS=1
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=${CODEX_REPO:-$DEFAULT_REPO}
LOG_DIR=$REPO/data/agents/codex${CODEX_PROBE_PROMPT:+/probe}
LOCK=$LOG_DIR/running.pid
TURN_TIMEOUT_SECONDS=${CODEX_TURN_TIMEOUT_SECONDS:-$DEFAULT_TURN_TIMEOUT_SECONDS}
export PATH=/Users/me/.local/bin:/Users/me/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/Users/me/dev/bin:/usr/sbin:/usr/bin:/bin:$PATH

mkdir -p "$LOG_DIR"
if [ -f "$LOCK" ] && kill -0 "$(cat "$LOCK")" 2>/dev/null; then
	echo "$(date +%FT%T) Codex run $(cat "$LOCK") still going; skipping this one"
	exit 0
fi

RUN=$LOG_DIR/$(date +%Y-%m-%d-%H%M%S)
THREAD_NAME="codex cron $(date '+%F %H:%M')"
RESULT=$RUN.result.json
APP_OUTPUT=$RUN.app.log
RUN_STATUS=preflight
RUN_DETAIL=""
THREAD_ID=""
OUTCOME="no merge request"
CARDS=""
APP_PID=""
END_SENT=0
START_NOTIFIED=0
FINAL_EXIT=0

echo $$ > "$LOCK"

relay() {
	"$1" "$2" "$3" >> "$RUN.relay.log" 2>&1 && return
	echo "$(date +%FT%T) RELAY FAILED (see $RUN.relay.log): $2 never got: ${3%%$'\n'*}"
	return 1
}

tell_boss() {
	relay codex-notify.sh "$CODEX_BOSS" "$1"
}

ensure_artifact() {
	[ -s "$RUN.last.md" ] && return
	printf '# Codex run %s\n\nCodex lifecycle: **%s**. %s\n' "$RUN_STATUS" "$RUN_STATUS" "${RUN_DETAIL:-No final agent message was produced.}" > "$RUN.last.md"
}

json_value() {
	/usr/bin/python3 - "$RESULT" "$1" <<'PY'
import json
import sys

value = json.load(open(sys.argv[1])).get(sys.argv[2])
print("" if value is None else value)
PY
}

process_is_running() {
	local state
	state=$(ps -o stat= -p "$1" 2>/dev/null | tr -d ' ')
	[ -n "$state" ] && [[ "$state" != Z* ]]
}

notify_started_thread() {
	[ "$START_NOTIFIED" -eq 0 ] || return
	local started
	started=$(grep '^started ' "$APP_OUTPUT" 2>/dev/null | tail -1)
	[ -n "$started" ] || return
	THREAD_ID=$(cut -d' ' -f2 <<< "$started")
	START_NOTIFIED=1
	echo "$(date +%FT%T) $started"
	tell_boss "Codex run active: thread \"$THREAD_NAME\" ($THREAD_ID) in the Codex app, log $RUN.jsonl" ||
		RUN_DETAIL="start notice failed; see $RUN.relay.log"
}

stop_child() {
	[ -n "$APP_PID" ] || return
	process_is_running "$APP_PID" || return
	kill -TERM "$APP_PID" 2>/dev/null || true
	local deadline=$((SECONDS + EXEC_EXIT_GRACE_SECONDS))
	while process_is_running "$APP_PID" && [ "$SECONDS" -lt "$deadline" ]; do
		sleep "$POLL_SECONDS"
	done
	process_is_running "$APP_PID" && kill -KILL "$APP_PID" 2>/dev/null || true
	wait "$APP_PID" 2>/dev/null || true
	APP_PID=""
}

finish() {
	local incoming_exit=$1
	trap - EXIT INT TERM HUP
	stop_child
	ensure_artifact
	rm -f "$LOCK"
	[ "$END_SENT" -eq 0 ] || exit "$incoming_exit"
	END_SENT=1
	local refusal=""
	if [[ "$RUN_DETAIL" == *"usage limit"* ]]; then
		refusal=" Refused: $RUN_DETAIL"
	fi
	local notice="Codex run \"$THREAD_NAME\" ended: $RUN_STATUS (exit $incoming_exit). Thread: ${THREAD_ID:-none}. Cards taken: ${CARDS:-none}. $OUTCOME.$refusal Final artifact: $RUN.last.md"
	if ! tell_boss "$notice"; then
		echo "$(date +%FT%T) END NOTICE FAILED: $notice"
		[ "$incoming_exit" -eq 0 ] && incoming_exit=1
	fi
	exit "$incoming_exit"
}

handle_signal() {
	local signal_name=$1
	RUN_STATUS=interrupted
	RUN_DETAIL="wrapper received $signal_name"
	FINAL_EXIT=3
	stop_child
	exit "$FINAL_EXIT"
}

trap 'finish $?' EXIT
trap 'handle_signal SIGINT' INT
trap 'handle_signal SIGTERM' TERM
trap 'handle_signal SIGHUP' HUP

if ! GH_TOKEN=$(cat /Users/me/.config/gh/cron-token); then
	RUN_STATUS=failed
	RUN_DETAIL="missing /Users/me/.config/gh/cron-token"
	echo "$(date '+%FT%T') $RUN_DETAIL" >&2
	exit 1
fi
export GH_TOKEN

PROMPT="You are a warp worker session named codex, started by a timer; nobody watches this run. Read AGENTS.md and
notes/agents/common.md and follow them, with these differences: you cannot use SendMessage, ListAgents or ask anyone,
so never wait for input, and make undoable default choices yourself (note them in your final message).

YOUR TASK: take one or two cards that look quickly done (a small, clear bug or gap with a probe or example in the card),
from any topic: first the Next column (\`todo list Next\`), then Soon (\`todo list Soon\`) (user, 2026-10-15: random
topics that seem quickly done; replaces the cleanup-only rule of 2026-10-13). Skip big design cards, roadmaps and
cards waiting on a user decision. None free: stop with nothing taken. State the net src/ line delta in your final
message (simpler, more general code is the aim). Skip any card that
already names an agent or whose branch exists on origin (another session works on it). Skip every card whose key
starts with \`user-\`: those are the user's own to-dos (clicks in web UIs, cloud settings, accounts), not repository
work, and you must not take, branch or push for them. Take only cards solved by changing this repository. Claim each with
\`todo take <card> codex\`. Per card: its own branch and worktree (\`cowtree add\`, as in common.md), test first,
targeted tests only through \`tests/queue.sh -- <filter>\`, CARGO_BUILD_JOBS=2, clippy and the wasm32 check clean,
push the branch, then \`gh workflow run Playground -R pannous/warp --ref <branch>\` and wait for it with
\`gh run watch --interval 60\`. Never push main, never run ./test.sh, never change existing tests unless common.md allows it.
Do not close the cards: the Integrator closes them after the merge.

Your FINAL message must contain, for every branch that is pushed and green, one block exactly like this (the tip
copied from \`git rev-parse\` output):
MERGE REQUEST
branch: <branch>
tip: <full sha>
card: <card key>
playground: <run id> <green|red>
filters: <test filters>
changes: <what it does, defaults chosen, existing tests touched and why>
END
A card you could not finish: push what you have, then \`todo release <card> \"done: …; left: …; branch: …\"\` so it goes back to Next with no agent, say why in the final message, and put no block for it."

[ -n "${CODEX_PROBE_PROMPT:-}" ] && PROMPT=$CODEX_PROBE_PROMPT

echo "$(date +%FT%T) Codex run starting, log $RUN.jsonl"
CODEX_TURN_TIMEOUT_SECONDS=$TURN_TIMEOUT_SECONDS /usr/bin/python3 "$SCRIPT_DIR/codex_app_run.py" "$REPO" "$THREAD_NAME" "$PROMPT" "$RUN.jsonl" "$RUN.last.md" "$RESULT" > "$APP_OUTPUT" 2>&1 &
APP_PID=$!
while process_is_running "$APP_PID"; do
	notify_started_thread
	sleep "$POLL_SECONDS"
done
wait "$APP_PID"
FINAL_EXIT=$?
APP_PID=""
notify_started_thread
cat "$APP_OUTPUT"

if [ -s "$RESULT" ]; then
	RUN_STATUS=$(json_value status)
	THREAD_ID=$(json_value threadId)
	RUN_DETAIL=$(json_value error)
fi

if [ "$FINAL_EXIT" -eq 2 ]; then
	RUN_STATUS=execFallback
	RUN_DETAIL="no app-server daemon answered; codex exec is not visible in the Codex app"
	echo "$(date +%FT%T) $RUN_DETAIL"
	tell_boss "Codex run active through codex exec, NOT visible in the Codex app: \"$THREAD_NAME\", log $RUN.jsonl" ||
		RUN_DETAIL="$RUN_DETAIL; fallback start notice failed"
	codex exec --dangerously-bypass-approvals-and-sandbox -C "$REPO" --json -o "$RUN.last.md" "$PROMPT" >> "$RUN.jsonl" 2>&1 &
	APP_PID=$!
	local_deadline=$((SECONDS + TURN_TIMEOUT_SECONDS))
	while process_is_running "$APP_PID" && [ "$SECONDS" -lt "$local_deadline" ]; do
		sleep "$POLL_SECONDS"
	done
	if process_is_running "$APP_PID"; then
		RUN_STATUS=timedOut
		RUN_DETAIL="codex exec exceeded $TURN_TIMEOUT_SECONDS seconds"
		stop_child
		FINAL_EXIT=4
	else
		wait "$APP_PID"
		FINAL_EXIT=$?
		APP_PID=""
		if [ "$FINAL_EXIT" -eq 0 ]; then
			RUN_STATUS=completed
			RUN_DETAIL=""
		else
			RUN_STATUS=failed
			RUN_DETAIL=$(grep -o '"message":"[^"]*"' "$RUN.jsonl" | tail -1 | sed 's/^"message":"//; s/"$//' || true)
		fi
	fi
fi

ensure_artifact
CARDS=$(grep -oE 'todo take [A-Za-z0-9_-]+' "$RUN.jsonl" 2>/dev/null | cut -d' ' -f3 | sort -u | paste -sd, - || true)
REQUESTS=""
if [ "$RUN_STATUS" = completed ]; then
	REQUESTS=$(awk '/^MERGE REQUEST/{keep=1} keep{print} /^END/{keep=0}' "$RUN.last.md")
fi
if [ -z "$REQUESTS" ]; then
	OUTCOME="no merge request (run status $RUN_STATUS)"
elif relay claude-relay.sh "$INTEGRATOR" "Merge request from the timed Codex worker (log $RUN.jsonl):
$REQUESTS"; then
	OUTCOME="$(grep -c '^MERGE REQUEST' <<< "$REQUESTS") merge request(s) sent to $INTEGRATOR (cards $(grep '^card:' <<< "$REQUESTS" | cut -d' ' -f2 | paste -sd, -))"
else
	OUTCOME="RELAY FAILED (see $RUN.relay.log): the merge requests in $RUN.last.md never reached $INTEGRATOR"
	[ "$FINAL_EXIT" -eq 0 ] && FINAL_EXIT=1
fi

echo "$(date +%FT%T) Codex run ended: $RUN_STATUS (exit $FINAL_EXIT)"
echo "$(date +%FT%T) $OUTCOME"
exit "$FINAL_EXIT"
