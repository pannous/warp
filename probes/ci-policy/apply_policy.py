#!/usr/bin/env python3
"""Rewrite a push-triggered test workflow to the CI policy of pannous/warp (.github/workflows/rust.yml):
tests run nightly when the default branch changed, by hand, on v* tags and on [ci] commits, not on every push.
usage: apply_policy.py <workflow.yml> <owner/repo> <default-branch> [cron] [window]"""
import re
import sys
import yaml

DEFAULT_CRON = "23 2 * * *"
DEFAULT_WINDOW = "25 hours ago"

DECIDE_JOB = """  decide:
    runs-on: ubuntu-latest
    outputs:
      run: ${{ steps.decide.outputs.run }}
    steps:
    - uses: actions/checkout@v4
      with:
        fetch-depth: 50
    - id: decide
      env:
        EVENT: ${{ github.event_name }}
        REF: ${{ github.ref }}
        MESSAGE: ${{ github.event.head_commit.message }}
        WINDOW: %(window)s
      run: |
        run=false
        case "$EVENT" in
          workflow_dispatch|pull_request) run=true ;;
          schedule) [ -n "$(git log --since="$WINDOW" --format=%%h)" ] && run=true ;;
          push)
            case "$REF" in refs/tags/*) run=true ;; esac
            case "$MESSAGE" in *"[ci]"*) run=true ;; esac ;;
        esac
        echo "run=$run" >> "$GITHUB_OUTPUT"
        echo "tests run: $run ($EVENT $REF)" >> "$GITHUB_STEP_SUMMARY"

"""

GATE = "    needs: decide\n    if: needs.decide.outputs.run == 'true'\n"


def header(name, repo, cron_note):
    return (f"# The tests do not run on every push to the default branch (user decision 2026-10-03: the failure mails\n"
            f"# were noise; same policy as pannous/warp). They run {cron_note} when the branch changed, by hand\n"
            f"# (`gh workflow run \"{name}\" -R {repo} [--ref <branch>]`), on a version tag (v*), on a push whose head\n"
            f"# commit message contains [ci], and on pull requests. The decide job always succeeds, so a skipped push\n"
            f"# sends no mail.\n")


def triggers(branch, cron):
    return (f"on:\n  push:\n    branches: [ {branch} ]\n    tags: [ \"v*\" ]\n  pull_request:\n"
            f"  schedule:\n    - cron: \"{cron}\"\n  workflow_dispatch:\n")


def rewrite(text, repo, branch, cron, window):
    name = yaml.safe_load(text)["name"]
    on_block = re.compile(r"^on:.*?(?=^\S)", re.S | re.M)
    assert len(on_block.findall(text)) == 1, "expected exactly one top-level on: block"
    pull_request_branches = re.search(r"^  pull_request:\n    branches: (.*)\n", text, re.M)
    new_on = triggers(branch, cron)
    if pull_request_branches:
        new_on = new_on.replace("  pull_request:\n", f"  pull_request:\n    branches: {pull_request_branches.group(1)}\n")
    text = on_block.sub(lambda _: new_on + "\n", text)
    text = re.sub(r"^name: .*\n", lambda m: m.group(0) + "\n" + header(name, repo, "nightly" if cron == DEFAULT_CRON else f"on schedule ({cron})"), text, count=1, flags=re.M)
    head, jobs = text.split("\njobs:\n", 1)
    jobs = re.sub(r"^(  [A-Za-z_][\w-]*:\n)", lambda m: m.group(1) + GATE, jobs, flags=re.M)
    return head + "\njobs:\n" + DECIDE_JOB % {"window": window} + jobs


def check(text):
    workflow = yaml.safe_load(text)
    jobs = workflow["jobs"]
    for job_name, job in jobs.items():
        if job_name != "decide":
            assert job["needs"] == "decide" and "decide" in job["if"], job_name
    triggers_ = workflow[True]  # PyYAML reads the bare key `on` as boolean True
    assert set(triggers_) >= {"push", "schedule", "workflow_dispatch"}


if __name__ == "__main__":
    path, repo, branch = sys.argv[1:4]
    cron = sys.argv[4] if len(sys.argv) > 4 else DEFAULT_CRON
    window = sys.argv[5] if len(sys.argv) > 5 else DEFAULT_WINDOW
    result = rewrite(open(path).read(), repo, branch, cron, window)
    check(result)
    open(path, "w").write(result)
    print("ok", path)
