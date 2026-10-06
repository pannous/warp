#!/bin/bash
# Probe for ~/dev/bin/context-swap: real transcript, real tmux session, a spawner that only records its arguments.
set -e
HOME_SWAP=$HOME/.claude/swap
TRANSCRIPT=$(ls -t ~/.claude/projects/-Users-me-dev-angles-warp/*.jsonl | head -1)
SESSION_ID=probe-$$
export SWAP_LIMIT_FILE=/nonexistent SWAP_TOKENS=1000 SWAP_SPAWN_SCRIPT=$PWD/probes/context_swap_fake_spawn.sh
export SWAP_RETIRE_GRACE=1 SWAP_RETIRE_POLL=1
# a background job of the session, as the Bash tool runs it: a shell sourcing a snapshot under the pane, here 6 s long
BACKGROUND_JOB='zsh -c "true ~/.claude/shell-snapshots/probe; sleep 6; true"'
tmux new-session -d -s warp-probeworker "$BACKGROUND_JOB & sleep 120"
PANE=$(tmux list-panes -t =warp-probeworker -F '#{pane_id}')
wait_retired() {
	for _ in $(seq 15); do tmux has-session -t =warp-probeworker-retiring 2>/dev/null || return 0; sleep 1; done
	echo "the retiring session is still there"; exit 1
}
run_hook() { echo "{\"session_id\":\"$SESSION_ID\",\"cwd\":\"/Users/me/dev/angles/warp\",\"transcript_path\":\"$TRANSCRIPT\"}" | TMUX_PANE=$PANE ~/dev/bin/context-swap; }

echo "1. over the limit, no handover yet -> block asking for it"; run_hook | tee $HOME_SWAP/probe_block_reason.txt | grep -q '"decision": "block"'
HANDOVER=$(python3 -c "import json;print(json.load(open('$HOME_SWAP/$SESSION_ID.json'))['handover_path'])")
echo "2. handover still missing -> asked again (up to 3 times)"; run_hook | grep -q block
echo "3. handover written -> replacement spawned, old session renamed"; echo handover > "$HANDOVER"; run_hook
tail -1 $HOME_SWAP/swaps.log; grep -q "SWAPPED probeworker" $HOME_SWAP/swaps.log
tmux has-session -t =warp-probeworker-retiring
grep -q "BACKGROUND JOBS" $HOME_SWAP/probe_block_reason.txt
echo "4. a second Stop does nothing"; run_hook; [ "$(grep -c "SWAPPED probeworker $SESSION_ID" $HOME_SWAP/swaps.log)" = 1 ]
echo "5. the retiring session lives while its background job runs, then goes"
sleep 3; tmux has-session -t =warp-probeworker-retiring
wait_retired; tail -1 $HOME_SWAP/swaps.log | grep -q "RETIRED warp-probeworker-retiring"
rm -f "$HANDOVER" $HOME_SWAP/$SESSION_ID.json $HOME_SWAP/probe_block_reason.txt
echo "6. a job outliving the cap is killed with the session, logged"
SESSION_ID=probe-cap-$$
tmux new-session -d -s warp-probeworker "${BACKGROUND_JOB/sleep 6/sleep 60} & sleep 120"
PANE=$(tmux list-panes -t =warp-probeworker -F '#{pane_id}')
run_hook > /dev/null; HANDOVER=$(python3 -c "import json;print(json.load(open('$HOME_SWAP/$SESSION_ID.json'))['handover_path'])")
echo handover > "$HANDOVER"; SWAP_RETIRE_MAX_WAIT=2 run_hook
wait_retired; grep -q "RETIRE TIMEOUT warp-probeworker-retiring" $HOME_SWAP/swaps.log
rm -f "$HANDOVER" $HOME_SWAP/$SESSION_ID.json
echo "7. under the limit -> silent"; SWAP_TOKENS=99999999 run_hook | grep -q . && exit 1
echo PASS
