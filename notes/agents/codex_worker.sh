#!/bin/bash
# Gives Codex one or two cards from the board and hands its finished branches to the Integrator:
#   notes/agents/codex_worker.sh            (cron, every 5 hours; user, 2026-10-11)
# A fresh Codex thread on the local app-server daemon (codex_app_run.py, so the user sees it in the Codex app) takes free cards from the Next column, works each on its own branch and worktree,
# and ends with one MERGE REQUEST block per branch. Codex has no SendMessage, so ~/dev/bin/claude-relay.sh (a short
# headless Haiku run) hands those blocks to warp-integrator. ~/dev/bin/codex-notify.sh tells the user's Codex thread
# "Warp Boss" when the run starts and how it ended (user, 2026-10-12: "inform the Codex Boss every time we spawn").
# A run still going when the next one is due makes the next one skip.
#   CODEX_PROBE_PROMPT="Reply with just: ok" notes/agents/codex_worker.sh   a probe: that prompt, logs in codex/probe
REPO=/Users/me/dev/angles/warp
INTEGRATOR=warp-integrator
CODEX_BOSS="Warp Boss"
LOG_DIR=$REPO/data/agents/codex${CODEX_PROBE_PROMPT:+/probe}
LOCK=$LOG_DIR/running.pid
# ~/.local/bin holds claude (the relay), ~/.cargo/bin rustup's toolchain ahead of Homebrew's rust (card
# homebrew-rustc); cron's PATH has none of these
export PATH=/Users/me/.local/bin:/Users/me/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/Users/me/dev/bin:/usr/sbin:/usr/bin:/bin:$PATH
# cron can't read gh's keyring login: without the token file every todo and gh call of the run fails with 401, and
# the overnight runs of 2026-10-11/12 stopped without claiming a card (`gh auth token > ~/.config/gh/cron-token`)
GH_TOKEN=$(cat /Users/me/.config/gh/cron-token) || { echo "$(date '+%FT%T') no ~/.config/gh/cron-token" >&2; exit 1; }
export GH_TOKEN

mkdir -p "$LOG_DIR"
if [ -f "$LOCK" ] && kill -0 "$(cat "$LOCK")" 2>/dev/null; then
	echo "$(date +%FT%T) Codex run $(cat "$LOCK") still going; skipping this one"
	exit 0
fi
echo $$ > "$LOCK"
trap 'rm -f "$LOCK"' EXIT

RUN=$LOG_DIR/$(date +%Y-%m-%d-%H%M)
PROMPT="You are a warp worker session named codex, started by a timer; nobody watches this run. Read AGENTS.md and
notes/agents/common.md and follow them, with these differences: you cannot use SendMessage, ListAgents or ask anyone,
so never wait for input, and make undoable default choices yourself (note them in your final message).

YOUR TASK: take one or two cards from the Next column of the board (\`todo list\`), easy ones first. Skip any card that
already names an agent or whose branch exists on origin (another session works on it). Skip every card whose key
starts with \`user-\`: those are the user's own to-dos (clicks in web UIs, cloud settings, accounts), not repository
work, and you must not take, branch or push for them. Take only cards solved by changing this repository. Claim each with
\`todo take <card> codex\`. Per card: its own branch and worktree (\`cowtree add\`, as in common.md), test first,
targeted tests only through \`tests/queue.sh -- <filter>\`, CARGO_BUILD_JOBS=2, clippy and the wasm32 check clean,
push the branch, then \`gh workflow run Playground -R pannous/warp --ref <branch>\` and wait for it with
\`gh run watch\`. Never push main, never run ./test.sh, never change existing tests unless common.md allows it.
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
A card you could not finish: push what you have, say why in the final message, and put no block for it."

[ -n "$CODEX_PROBE_PROMPT" ] && PROMPT=$CODEX_PROBE_PROMPT
THREAD_NAME="codex cron $(date '+%F %H:%M')"
# relay <sender script> <recipient> <message>: loud in the log when the message doesn't get through
relay() {
	"$1" "$2" "$3" >> "$RUN.relay.log" 2>&1 && return
	echo "$(date +%FT%T) RELAY FAILED (see $RUN.relay.log): $2 never got: ${3%%$'\n'*}"
	return 1
}
tell_boss() { relay codex-notify.sh "$CODEX_BOSS" "$1"; }

echo "$(date +%FT%T) Codex run starting, log $RUN.jsonl"
# on the app-server daemon the thread shows up in the user's Codex app (user, 2026-10-12); `codex exec` threads don't
/usr/bin/python3 "$REPO/notes/agents/codex_app_run.py" "$REPO" "$THREAD_NAME" "$PROMPT" "$RUN.jsonl" "$RUN.last.md" |
	while read -r event thread_id; do
		echo "$(date +%FT%T) $event $thread_id"
		[ "$event" = started ] &&
			tell_boss "Codex run started: thread \"$THREAD_NAME\" ($thread_id) in the Codex app, log $RUN.jsonl" &
	done
STATUS=${PIPESTATUS[0]}
if [ "$STATUS" -eq 2 ]; then
	echo "$(date +%FT%T) NO CODEX APP-SERVER DAEMON (codex app-server daemon start): running codex exec instead, invisible in the Codex app"
	tell_boss "Codex run started with codex exec, NOT visible in the Codex app (no app-server daemon answered), log $RUN.jsonl" &
	codex exec --dangerously-bypass-approvals-and-sandbox -C "$REPO" --json -o "$RUN.last.md" "$PROMPT" >> "$RUN.jsonl" 2>&1
	STATUS=$?
fi
echo "$(date +%FT%T) Codex run ended with exit $STATUS"
REFUSAL=$(grep -o '"message":"You[^"]*usage limit[^"]*"' "$RUN.jsonl" | head -1 | sed 's/^/CODEX REFUSED: /')
[ -n "$REFUSAL" ] && echo "$REFUSAL"

CARDS=$(grep -oE 'todo take [A-Za-z0-9_-]+' "$RUN.jsonl" | cut -d' ' -f3 | sort -u | paste -sd, -)
REQUESTS=$(awk '/^MERGE REQUEST/{keep=1} keep{print} /^END/{keep=0}' "$RUN.last.md" 2>/dev/null)
if [ -z "$REQUESTS" ]; then
	OUTCOME="no merge request"
elif relay claude-relay.sh "$INTEGRATOR" "Merge request from the timed Codex worker (log $RUN.jsonl):
$REQUESTS"; then
	OUTCOME="$(grep -c '^MERGE REQUEST' <<< "$REQUESTS") merge request(s) sent to $INTEGRATOR (cards $(grep '^card:' <<< "$REQUESTS" | cut -d' ' -f2 | paste -sd, -))"
else
	OUTCOME="RELAY FAILED (see $RUN.relay.log): the merge requests in $RUN.last.md never reached $INTEGRATOR"
fi
echo "$(date +%FT%T) $OUTCOME"
tell_boss "Codex run \"$THREAD_NAME\" ended (exit $STATUS). Cards taken: ${CARDS:-none}. $OUTCOME. ${REFUSAL:+$REFUSAL. }Final message: $RUN.last.md"
wait
[[ $OUTCOME != RELAY\ FAILED* ]]
