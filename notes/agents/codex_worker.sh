#!/bin/bash
# Gives Codex one or two library cards the Supervisor picked (notes/agents/codex_cards.txt); Codex lands them itself.
# Scheduled by the user's crontab every five hours. CODEX_PROBE_PROMPT runs a small isolated probe.
set -uo pipefail

DEFAULT_REPO=/Users/me/dev/angles/warp
DEFAULT_MODEL=gpt-6.1-terra
DEFAULT_EFFORT=high # ~/.codex/config.toml says low, which ends a run after one look at the card
SUPERVISOR=warp-supervisor
CODEX_BOSS="Warp Handler"
CODEX_APP_SOCKET=/Users/me/.codex/app-server-control/app-server-control.sock
DEFAULT_TURN_TIMEOUT_SECONDS=16200
EXEC_EXIT_GRACE_SECONDS=30
RELAY_TIMEOUT_SECONDS=60
MAX_ROUNDS=4 # runs of Codex per timer firing while a card stays parked
CAPACITY_PAUSE_SECONDS=600
POLL_SECONDS=1
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=${CODEX_REPO:-$DEFAULT_REPO}
export CODEX_MODEL=${CODEX_MODEL:-$DEFAULT_MODEL}
CODEX_EFFORT=${CODEX_EFFORT:-$DEFAULT_EFFORT}
CARD_LIST=$REPO/notes/agents/codex_cards.txt
LOG_DIR=$REPO/data/agents/codex${CODEX_PROBE_PROMPT:+/probe}
LOCK=$LOG_DIR/running.pid
TURN_TIMEOUT_SECONDS=${CODEX_TURN_TIMEOUT_SECONDS:-$DEFAULT_TURN_TIMEOUT_SECONDS}
export PATH=/Users/me/.local/bin:/Users/me/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/Users/me/dev/bin:/usr/sbin:/usr/bin:/bin:$PATH

mkdir -p "$LOG_DIR"
if [ -f "$LOCK" ] && [ "$(cat "$LOCK")" != $$ ] && kill -0 "$(cat "$LOCK")" 2>/dev/null; then
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
	timeout "$RELAY_TIMEOUT_SECONDS" "$1" "$2" "$3" >> "$RUN.relay.log" 2>&1 && return
	echo "$(date +%FT%T) RELAY FAILED (see $RUN.relay.log): $2 never got: ${3%%$'\n'*}"
	return 1
}

# The Codex app is a second audience only when it runs; the Supervisor always gets the outcome (claude-relay.sh)
tell_boss() {
	if [ ! -S "$CODEX_APP_SOCKET" ]; then
		echo "$(date +%FT%T) Codex app not running, $CODEX_BOSS not told: ${1%%$'\n'*}"
		return
	fi
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

blocks() {
	awk -v start="^$1" '$0 ~ start{keep=1} keep{print} /^END/{keep=0}' "$2" 2>/dev/null
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

PREVIOUS_RESULT=$(ls -t "$LOG_DIR"/*.last.md 2>/dev/null | head -1)
# cards the Supervisor took back from Codex (`retired: <card>` lines of the list): never continued, even when parked
RETIRED=$(sed -n 's/^retired: *\([A-Za-z0-9_-]*\).*/\1/p' "$CARD_LIST" 2>/dev/null | paste -sd' ' -)
not_retired() { awk -v retired=" $RETIRED " '/^PARKED/{block=""; skip=0} {block=block $0 "\n"} /^card:/{if (index(retired, " " $2 " ")) skip=1} /^END/{if (!skip) printf "%s", block; block=""}'; }
PARKED_BEFORE=$([ -n "$PREVIOUS_RESULT" ] && blocks PARKED "$PREVIOUS_RESULT" | not_retired)

PROMPT="You are a warp worker session named codex, started by a timer; nobody watches this run. Read AGENTS.md and
notes/agents/common.md and follow them, with these differences: you cannot use SendMessage, ListAgents or ask anyone,
so never wait for input, and make undoable default choices yourself (note them in your final message).

FIRST, finish what earlier runs left (user 2026-10-30: a run continues the previous one when it was not finished and
cleaned up): every card whose Agent is codex (\`todo list Now\`), every PARKED card below, and every codex branch on
origin not merged into main, each with its notes/handover/<card>.md. Reuse the worktree if it still exists, otherwise
\`cowtree add\` it from the pushed branch; merge origin/main and carry on from the handover's next step. A leftover
worktree with nothing new against main is removed, its branch deleted (local and remote).
${RETIRED:+The Supervisor took these cards back: never continue them, leave their cards, branches and worktrees alone: $RETIRED
}${PARKED_BEFORE:+Parked by the previous run:
$PARKED_BEFORE
}

THEN, with time left, take one or two cards from the Supervisor's list $CARD_LIST, in its order (user, 2026-10-26: library work
that does not disturb the sessions working on the compiler). Skip a card that already names an agent, whose branch
exists on origin, that is no longer on the board (\`todo show <card>\`) or whose notes already say \"needs core\". None
free on the list: take the next lib-only card from \`todo pick\` instead; stop empty-handed only when there is none. Work
in lib/ (and samples/, test/, wiki/); never change core/ or the Rust harness. Claim each with \`todo take <card> codex\`.

Reproduce before you judge: run the card's example with WARP_NO_WINDOW=1 /Users/me/dev/bin/warp '<code>'; an old
investigation on the card is a hint, not a verdict. A card that truly needs core is not a dead end: look for the
library way first (common.md Principles: a function, enum, overload or lib/sugar rule); only when there is none, write
the smallest core shape into \`todo note <card> \"needs core: <files, functions, why in 3 lines>\"\`, release it and
take the next card.

You have hours: use them on real work. Investigating, reproducing and writing a handover are the start of a card,
never its result: do not end your turn while a card is open and untried. Progress is a pushed commit that adds a test
or changes lib/, samples/ or wiki/; a run whose only commit is a handover has failed. When a choice is open (coercion,
optional fields, a name), take the recommended default, write it into the handover and keep going. End your turn only
when every card you took is landed, parked with such a commit, or released as needing core.

Per card: its own branch and worktree (\`cowtree add\`, as in common.md), a test first in test/<topic>/, then common.md
\"Done\": format, merge origin/main, \`scripts/smoke.sh <worktree>\`, follow the current testing rules, push to
main yourself, \`todo done <card> <commit>\`. Never change existing tests unless common.md allows it. A working part of a
card may land on its own (smoke green), the rest stays on the card.

Leave something usable at every point, since a run can end any minute (timeout, usage limit):
- commit after each step that works (a test, a rule, a fix), small commits with clear messages, and push the branch
  (\`git push origin <card>\`) right after each commit;
- after each push, \`todo note <card> \"progress: <what works, what is next> (tip <sha>)\"\`;
- keep notes/handover/<card>.md on the branch up to date (state, what works, what fails with the exact command and
  output, next step, defaults you chose) and commit it with the work.

Your FINAL message must contain, for every card landed on main, one block exactly like this:
LANDED
card: <card key>
commit: <sha on main>
changes: <what it does, defaults chosen, existing tests touched and why>
END
and for every card you worked on but did not finish (branch pushed, handover written; the card stays taken by codex
so the next run continues it) one block exactly like this:
PARKED
card: <card key>
branch: <branch> <tip sha>
handover: notes/handover/<card>.md
left: <what is missing, in one line>
END"

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
	codex exec --dangerously-bypass-approvals-and-sandbox -m "$CODEX_MODEL" -c model_reasoning_effort="$CODEX_EFFORT" -C "$REPO" --json -o "$RUN.last.md" "$PROMPT" >> "$RUN.jsonl" 2>&1 &
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
CARDS=$(grep -oE 'todo take [A-Za-z0-9][A-Za-z0-9_-]*' "$RUN.jsonl" 2>/dev/null | cut -d' ' -f3 | sort -u | paste -sd, - || true)
LANDINGS=$(blocks LANDED "$RUN.last.md")
PARKED=$(blocks PARKED "$RUN.last.md")
card_keys() {
	grep '^card:' <<< "$1" | cut -d' ' -f2 | paste -sd, -
}
OUTCOME="landed: $(card_keys "$LANDINGS"); parked: $(card_keys "$PARKED") (run status $RUN_STATUS)"
if [ -z "$LANDINGS$PARKED" ]; then
	OUTCOME="nothing landed or parked (run status $RUN_STATUS, cards taken: ${CARDS:-none}; see $RUN.last.md)"
fi
if ! relay claude-relay.sh "$SUPERVISOR" "The timed Codex worker ended: $OUTCOME. Log $RUN.jsonl
$LANDINGS
$PARKED"; then
	OUTCOME="RELAY FAILED (see $RUN.relay.log): $OUTCOME never reached $SUPERVISOR"
	[ "$FINAL_EXIT" -eq 0 ] && FINAL_EXIT=1
fi

echo "$(date +%FT%T) Codex run ended: $RUN_STATUS (exit $FINAL_EXIT)"
echo "$(date +%FT%T) $OUTCOME"

# Codex ends its turn after a few minutes even with hours left: a parked card that needs no core gets another round,
# which continues from the handover (PARKED_BEFORE); exec keeps the pid, so the lock stays ours
# A round the model's capacity cut short (its pushed commits survive) continues after a pause, as a parked one does
ROUND=${CODEX_ROUND:-1}
continuable=""
if [ -n "$PARKED" ] && ! grep -qi 'needs core' <<< "$PARKED"; then
	continuable="parked $(card_keys "$PARKED")"
elif tail -3 "$RUN.jsonl" 2>/dev/null | grep -q 'at capacity'; then
	continuable="cut short by the model's capacity"
	sleep "$CAPACITY_PAUSE_SECONDS"
fi
if [ -z "${CODEX_PROBE_PROMPT:-}" ] && [ -n "$continuable" ] && [ "$ROUND" -lt "$MAX_ROUNDS" ]; then
	echo "$(date +%FT%T) round $ROUND $continuable; round $((ROUND + 1)) of $MAX_ROUNDS continues it"
	trap - EXIT INT TERM HUP
	CODEX_ROUND=$((ROUND + 1)) exec "$0"
fi
exit "$FINAL_EXIT"
