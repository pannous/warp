#!/bin/bash
# Gives Codex one or two cards from the board and hands its finished branches to the Integrator:
#   notes/agents/codex_worker.sh            (cron, every 5 hours; user, 2026-10-11)
# A fresh headless `codex exec` takes free cards from the Next column, works each on its own branch and worktree,
# and ends with one MERGE REQUEST block per branch. Codex has no SendMessage, so a short headless claude run (Haiku)
# relays those blocks to warp-integrator. A run still going when the next one is due makes the next one skip.
REPO=/Users/me/dev/angles/warp
INTEGRATOR=warp-integrator
LOG_DIR=$REPO/data/agents/codex
LOCK=$LOG_DIR/running.pid
RELAY_MODEL=claude-haiku-5-5
# ~/.local/bin holds claude (the relay), ~/.cargo/bin rustup's toolchain ahead of Homebrew's rust (card
# homebrew-rustc); cron's PATH has none of these
export PATH=/Users/me/.local/bin:/Users/me/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/Users/me/dev/bin:/usr/bin:/bin:$PATH

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

echo "$(date +%FT%T) Codex run starting, log $RUN.jsonl"
codex exec --dangerously-bypass-approvals-and-sandbox -C "$REPO" --json -o "$RUN.last.md" "$PROMPT" > "$RUN.jsonl" 2>&1
echo "$(date +%FT%T) Codex run ended with exit $?"
grep -o '"message":"You[^"]*usage limit[^"]*"' "$RUN.jsonl" | head -1 | sed 's/^/CODEX REFUSED: /'

REQUESTS=$(awk '/^MERGE REQUEST/{keep=1} keep{print} /^END/{keep=0}' "$RUN.last.md" 2>/dev/null)
if [ -z "$REQUESTS" ]; then
	echo "no merge request in $RUN.last.md"
	exit 0
fi
# the prompt goes on stdin: --allowedTools takes several values and would swallow a prompt argument
unset CLAUDECODE
printf '%s\n\n%s\n' "Use the SendMessage tool (load it with ToolSearch \"select:SendMessage\" first) to send the text below, unchanged, to \"$INTEGRATOR\". Start the message with the line \"Merge request from the timed Codex worker (log $RUN.jsonl):\". Then stop." "$REQUESTS" |
	claude -p --model "$RELAY_MODEL" --allowedTools "SendMessage,ToolSearch" >> "$RUN.relay.log" 2>&1
if [ "${PIPESTATUS[1]}" -ne 0 ]; then
	echo "$(date +%FT%T) RELAY FAILED (see $RUN.relay.log): merge requests in $RUN.last.md never reached $INTEGRATOR"
	exit 1
fi
echo "$(date +%FT%T) relayed $(grep -c '^MERGE REQUEST' <<< "$REQUESTS") merge request(s) to $INTEGRATOR"
