#!/bin/bash
# The cloud test lane's job hand-over (scripts/cloud-test.sh push_job / run_job) against GitHub, no cloud session:
# the Mac pushes the job's arguments to cloud-jobs/<job>, the cloud side reads them back unchanged, the branch is deleted.
set -u
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
eval "$(grep -v '^main "\$@"; exit$' "$REPO_ROOT/scripts/cloud-test.sh")"   # macOS bash 3.2 can't source <(…)
HERE_ROOT=$REPO_ROOT/scratch/cloud-job-probe
THIS_SCRIPT=$REPO_ROOT/scripts/cloud-test.sh
job=probe-job-$(date +%s)
sent=(--on main some-branch 0123456789abcdef0123456789abcdef01234567 test_alias_definition:: "test_a b" --full)

mapfile() { local line; eval "$2=()"; while IFS= read -r line; do eval "$2+=(\"\$line\")"; done; }   # bash 3.2 lacks it
run_here() { shift; printf '%s\n' "$@"; }
push_job "$job" "${sent[@]}"
received=$(cd "$REPO_ROOT" && run_job "$job")
again=$(cd "$REPO_ROOT" && run_job "$job")   # a repeated call reads the kept arguments
script_travels=$(git -C "$REPO_ROOT" show "refs/$JOBS_PREFIX/$job:cloud-test.sh" | cmp -s - "$THIS_SCRIPT" && echo yes)
git -C "$REPO_ROOT" push -q origin --delete "$JOBS_PREFIX/$job"
git -C "$REPO_ROOT" update-ref -d "refs/$JOBS_PREFIX/$job"
rm -rf "$HERE_ROOT"

expected=$(printf '%s\n' "${sent[@]}")
[ "$script_travels" = yes ] || { echo "job probe: FAIL, the job's cloud-test.sh differs from the launching script"; exit 1; }
[ "$received" = "$expected" ] && [ "$again" = "$expected" ] && echo "job probe: ok" ||
	{ echo "job probe: FAIL"; echo "sent:"; echo "$expected"; echo "received:"; echo "$received"; echo "again:"; echo "$again"; exit 1; }
