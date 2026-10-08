#!/bin/zsh
# PostToolUse(Bash) hook with asyncRewake: after a push to main (a `git push` naming main, or the Integrator's
# push_tested.sh) it waits in the background for the playground deploy of the pushed commit (.github/workflows/pages.yml)
# and, when that deploy fails, wakes the session that pushed (exit 2) with the failing lines on stderr. A deploy that
# succeeds, or a command that pushed nothing to main, ends silently (exit 0).
# By hand: watch-playground-deploy.sh [commit]   (default origin/main; prints the outcome either way)
REPO=pannous/warp
WORKFLOW=pages.yml
PUSH_TO_MAIN='push_tested\.sh|git[^|;&]*[[:space:]]push[[:space:]][^|;&]*(main|HEAD:main)'
APPEAR_SECONDS=180    # GitHub lists a run a few seconds to a minute after the push
POLL_SECONDS=10
FAILED_LINES=25
WOKEN=2               # asyncRewake: this exit code wakes the session and shows it stderr

checkout=${CLAUDE_PROJECT_DIR:-${0:A:h:h:h}}
if [[ -n $1 ]]; then
  by_hand=1
  commit=$(git -C $checkout rev-parse $1) || exit 1
else
  command=$(python3 -c 'import json, sys; print(json.load(sys.stdin).get("tool_input", {}).get("command", ""))')
  [[ $command =~ $PUSH_TO_MAIN ]] || exit 0
  git -C $checkout fetch -q origin main && commit=$(git -C $checkout rev-parse origin/main) || exit 0
fi

run_id=""
for (( waited = 0; waited < APPEAR_SECONDS; waited += POLL_SECONDS )); do
  run_id=$(gh run list -R $REPO --workflow $WORKFLOW --commit $commit --json databaseId -q '.[0].databaseId' 2>/dev/null)
  [[ -n $run_id ]] && break
  sleep $POLL_SECONDS
done
if [[ -z $run_id ]]; then
  [[ -n $by_hand ]] && echo "no playground deploy for ${commit:0:9} (the push touched no path pages.yml watches?)"
  exit 0
fi
if gh run watch $run_id -R $REPO --exit-status >/dev/null 2>&1; then
  [[ -n $by_hand ]] && echo "playground deployed: ${commit:0:9} (run $run_id)"
  exit 0
fi
{
  echo "PLAYGROUND DEPLOY FAILED for ${commit:0:9}: https://github.com/$REPO/actions/runs/$run_id"
  echo "The live playground keeps the last good deploy until this is fixed. Failing lines:"
  gh run view $run_id -R $REPO --log-failed 2>/dev/null | /usr/bin/grep -E 'FAIL|value:|failed:|##\[error\]|panicked' | cut -f3- | tail -$FAILED_LINES
} >&2
exit $WOKEN
