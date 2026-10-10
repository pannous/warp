#!/bin/bash
# Runs a warp worker headless on the Anthropic API key (ANTHROPIC_KEY_WARP in ~/.keys) instead of the subscription:
#   notes/agents/api_worker.sh <name> "<task>"
# Headless `claude -p` in detached tmux (warp-<name>); an interactive session on the key fails with "credit balance
# too low" (notes/llm_tools.md). The worker runs its one task and exits; its log and cost go to data/llm/api_workers/.
# SendMessage reaches it while it runs; it has no claude.ai URL (Remote Control is off for the key's organization).
# Optional: CLAUDE_MODEL (default claude-sonnet-5-5), WORKER_BUDGET_USD per run (default 15).
# Refuses once all API spending (data/llm/cost.json5 + earlier workers) reaches TOTAL_BUDGET_USD (user: stop at $150).
REPO=/Users/me/dev/angles/warp
LOG_DIR=$REPO/data/llm/api_workers
LEDGER=$LOG_DIR/ledger.tsv
SCRIPT_COSTS=$REPO/data/llm/cost.json5
TOTAL_BUDGET_USD=150
MODEL=${CLAUDE_MODEL:-claude-sonnet-5-5}
WORKER_BUDGET_USD=${WORKER_BUDGET_USD:-15}

[ -n "$1" ] && [ -n "$2" ] || { echo "usage: $0 <name> \"<task>\"" >&2; exit 1; }
NAME="warp-${1#warp-}"
TASK=$2
mkdir -p "$LOG_DIR"

spent() {
	{ grep -o '"dollars": [0-9.]*' "$SCRIPT_COSTS" 2>/dev/null | awk '{print $2}'
	  cut -f4 "$LEDGER" 2>/dev/null; } | awk '{s+=$1} END {printf "%.2f", s}'
}
SPENT=$(spent)
if awk -v s="$SPENT" -v w="$WORKER_BUDGET_USD" -v t="$TOTAL_BUDGET_USD" 'BEGIN { exit !(s + w > t) }'; then
	echo "API budget: \$$SPENT spent, another \$$WORKER_BUDGET_USD run would pass \$$TOTAL_BUDGET_USD; not spawning $NAME" >&2
	exit 1
fi
tmux has-session -t "$NAME:" 2>/dev/null && { echo "$NAME already runs (tmux attach -t $NAME)" >&2; exit 1; }

PROMPT="You are the warp worker $NAME. Read notes/agents/common.md and notes/agents/worker.md (use \`git -C $REPO show origin/main:notes/agents/worker.md\` if the checkout is stale) and act as described there. Your Supervisor is warp-supervisor. You run headless on a pay-per-token API key: work efficiently, do your one task, report to warp-supervisor with SendMessage, then end. YOUR TASK: $TASK"
LOG=$LOG_DIR/$(date +%Y-%m-%d-%H%M)-$NAME.jsonl
printf '%s\n' "$PROMPT" > "$LOG.prompt"

# The key stays out of argv and the tmux command line: the inner shell reads it from ~/.keys
RUNNER="source ~/.keys; export ANTHROPIC_API_KEY=\$ANTHROPIC_KEY_WARP; unset CLAUDECODE
claude -p --name $NAME --model $MODEL --permission-mode bypassPermissions --max-budget-usd $WORKER_BUDGET_USD \
  --output-format stream-json --verbose < '$LOG.prompt' > '$LOG' 2>&1
cost=\$(grep '\"type\":\"result\"' '$LOG' | tail -1 | grep -o '\"total_cost_usd\":[0-9.]*' | cut -d: -f2)
printf '%s\t%s\t%s\t%s\n' \"\$(date +%FT%T)\" $NAME $MODEL \"\${cost:-0}\" >> '$LEDGER'"
tmux new-session -d -s "$NAME" -c "$REPO" "bash -c $(printf '%q' "$RUNNER")"
echo "Spawned $NAME on the API key ($MODEL, ≤ \$$WORKER_BUDGET_USD; \$$SPENT of \$$TOTAL_BUDGET_USD spent so far). Log: $LOG"
