#!/bin/bash
# Probe for ~/dev/bin/context-swap: real transcript, real tmux session, a spawner that only records its arguments.
set -e
HOME_SWAP=$HOME/.claude/swap
TRANSCRIPT=$(ls -t ~/.claude/projects/-Users-me-dev-angles-warp/*.jsonl | head -1)
SESSION_ID=probe-$$
export SWAP_TOKENS=1000 SWAP_SPAWN_SCRIPT=$PWD/probes/context_swap_fake_spawn.sh
tmux new-session -d -s warp-probeworker "sleep 120"
PANE=$(tmux list-panes -t =warp-probeworker -F '#{pane_id}')
run_hook() { echo "{\"session_id\":\"$SESSION_ID\",\"cwd\":\"/Users/me/dev/angles/warp\",\"transcript_path\":\"$TRANSCRIPT\"}" | TMUX_PANE=$PANE ~/dev/bin/context-swap; }

echo "1. over the limit, no handover yet -> block asking for it"; run_hook | tee /dev/stderr | grep -q '"decision": "block"'
HANDOVER=$(python3 -c "import json;print(json.load(open('$HOME_SWAP/$SESSION_ID.json'))['handover_path'])")
echo "2. handover still missing -> asked again (up to 3 times)"; run_hook | grep -q block
echo "3. handover written -> replacement spawned, old session renamed"; echo handover > "$HANDOVER"; run_hook
tail -1 $HOME_SWAP/swaps.log; grep -q "SWAPPED probeworker" $HOME_SWAP/swaps.log
tmux has-session -t =warp-probeworker-retiring
echo "4. a second Stop does nothing"; run_hook; [ "$(grep -c "SWAPPED probeworker $SESSION_ID" $HOME_SWAP/swaps.log)" = 1 ]
tmux kill-session -t =warp-probeworker-retiring; rm -f "$HANDOVER" $HOME_SWAP/$SESSION_ID.json
echo "5. under the limit -> silent"; SWAP_TOKENS=99999999 run_hook | grep -q . && exit 1
echo PASS
