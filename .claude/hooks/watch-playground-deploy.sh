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
# gh run watch polls every 3 s by default (two REST requests each): one watch per push to main used up the shared
# 5000 requests/h on 2026-10-10. A deploy takes minutes, so a minute's delay in noticing a failure costs nothing.
WATCH_INTERVAL_SECONDS=60
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
  run_id=$(gh run list -R $REPO --workflow $WORKFLOW --commit $commit --event push --json databaseId -q '.[0].databaseId' 2>/dev/null)
  [[ -n $run_id ]] && break
  sleep $POLL_SECONDS
done
if [[ -z $run_id ]]; then
  [[ -n $by_hand ]] && echo "no playground deploy for ${commit:0:9} (the push touched no path pages.yml watches?)"
  exit 0
fi
if gh run watch $run_id -R $REPO --interval $WATCH_INTERVAL_SECONDS --exit-status >/dev/null 2>&1; then
  [[ -n $by_hand ]] && echo "playground deployed: ${commit:0:9} (run $run_id)"
  exit 0
fi
# a newer push to main cancels this run's remaining jobs (usually verify, after the deploy): no job failed, so nothing
# is broken, and the newer push's own watch reports its deploy
failed_jobs=$(gh run view $run_id -R $REPO --json jobs -q '[.jobs[] | select(.conclusion == "failure")] | length' 2>/dev/null)
if [[ $failed_jobs == 0 ]]; then
  [[ -n $by_hand ]] && echo "playground run $run_id for ${commit:0:9} was cancelled by a newer push, no job failed"
  exit 0
fi
{
  echo "PLAYGROUND DEPLOY FAILED for ${commit:0:9}: https://github.com/$REPO/actions/runs/$run_id"
  echo "The live playground keeps the last good deploy until this is fixed. Failing lines:"
  gh run view $run_id -R $REPO --log-failed 2>/dev/null | /usr/bin/grep -E 'FAIL|value:|failed:|##\[error\]|panicked' | cut -f3- | tail -$FAILED_LINES
} >&2
exit $WOKEN
