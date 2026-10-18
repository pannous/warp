# Developing warp in the cloud

The Cloud session credit ($250 with Max, expires 2026-11-05) pays for **cloud sessions** (`claude --cloud`,
claude.ai/code, the app), not for routines (RemoteTrigger / `/schedule`), which draw on the plan limits. Check it on
claude.ai/settings/usage or with `~/dev/bin/claude-usage.sh` (the CLI's `/usage` doesn't show it). Without the credit,
cloud sessions and routines share the subscription's limits. Cloud work also unloads the Mac.

## Execution options

| Option | Notes |
|---|---|
| **Cloud sessions** (`claude --cloud "<task>"`) | Needs a TTY: from an agent run it in detached tmux and read the `View: https://claude.ai/code/session_…` line. Clones the GitHub remote at the current branch/commit (push first). Steer with `claude -p "msg" --cloud <session-id>`. `claude --teleport <id>` pulls one into a local terminal. |
| **Routines** (`RemoteTrigger create` + `run_once_at`) | Plan limits, not the credit. Readable run log (`list_runs` / `get_run_log`). Caps: 100 scheduled runs/h, 30 "run now"/h. |
| Agent tool `isolation: "remote"` | Unreliable: has silently fallen back to local worktrees. Don't use it for cloud work. |
| `claude ultrareview` | Cloud multi-agent review of a branch. |
| `--remote-control` (claude-remote.sh) | Runs on the Mac; only the UI is remote. |

## Environments
- Edit one: open a cloud session at claude.ai/code → the chevron next to the session title → **Edit cloud environment**
  (name, Network access None/Trusted/Custom/Full, env vars, setup script). No settings page or URL exists.
  `/remote-env` in the CLI picks the default environment for `claude --cloud`. Changes apply to new sessions only.
- The environment "Microsoft" (Custom network: `*.pannous.com`, `pannous.com`, crates.io + the default package-manager
  list) is the one `claude --cloud` from this Mac lands in. Sandbox: Ubuntu x86_64, 4 vCPU, ~15 GB, no Lean.
- Lean tests skip in the cloud: `requires!(LEAN)` checks `CLAUDE_CODE_REMOTE=true`. The test gate hook and the
  tests/queue.sh lock are Mac-local; cloud sessions are exempt.
- The package/uniscript tests need `rustup target add wasm32-wasip1` (put it in the setup script).
- The `vendor` branch (notes/cloud_offline_build.md) is the fallback for environments without crates.io.

## Gotchas
- **SendMessage to a cloud session does not deliver tasks**: a session whose permission mode differs from the sender's
  holds cross-session messages until its user approves them (`system/peer_message_hold` in its log). Use
  `claude -p "<message>" --cloud <session_id>`: a real user message that starts a turn at once. The id is in the
  `View:` URL. Read transcripts with `RemoteTrigger get_run_log <session_id>` (works for any cloud session).
- Cloud sessions cannot reply; the channel is the pushed branch (with a report file) plus the run log.
- The user may archive sessions any time; archived ones reject messages. Ping before assigning.
- Each cloud worktree's build output is ~5 GB: keep ≤ 2 worktrees per VM.
- An old branch (many merges behind) is redone on current main by intent, not rebased blindly.
- Push all commits once per step: GitHub "Rust CI" runs on every claude/* push.
- After an approved `send_message`, the stop hook complains about an uncommitted `.claude/settings.local.json`. Don't
  commit it.

## Cloud tasks
- Cloud-suitable: pure Rust in src/ and tests/ that needs no Mac and no secrets (parser, analyzer, emitter, refactors,
  tests). Keep local: Lean proofs, macOS/iOS apps, anything reading ~/ or needing secrets, fast back-and-forth.
- Launch: `tmux new-session -d -s cloud-<task> -c <clean checkout> 'claude --cloud "<prompt>"; sleep 300'`, then
  `tmux capture-pane -p -t cloud-<task>` for the URL. Give the URL to the user. Killing the pane does not stop the
  session.
- Prompt: task with file pointers, branch `claude/<task-id>` from origin/main, red test first, commit and push per
  step, no merge, report file `notes/cloud_reports/<task-id>.md`. The Integrator merges claude/* branches after a
  local gate and deletes them.
- At most 2–3 busy cloud sessions when the weekly limit is high.

## Cloud test lane (`scripts/cloud-test.sh`)

Runs a branch's tests on a cloud VM instead of the Mac's queue:

    scripts/cloud-test.sh [--card <card>] [--no-wait] [--on-main|--on <base>] <branch> <tip> [filters…|--full]
    scripts/cloud-test.sh --result [--card <card>] <branch> <tip>      # collect a --no-wait run later

- The tip must be pushed (a full commit id from `git rev-parse`). Filters as for tests/queue.sh; `--full` = test.sh's
  flags. `--on-main` tests the tip merged into origin/main, as the gate will; `--on <base>` into another branch.
- How: `claude --cloud` in detached tmux with a two-line prompt; the job's arguments go to the branch
  cloud-jobs/<job>, the session runs `cloud-test.sh --job <job>` (calls wait ≤ 9 min, under the cloud Bash tool's
  10 min) until it prints CLOUD TEST DONE. The detached run checks out the tip, adds the wasm32 targets, builds from
  crates.io, tests with --no-fail-fast and pushes `result.md` + `log.txt.gz` to `cloud-results/<branch>-<tip8>`.
- The Mac polls that branch once a minute (`git ls-remote`), saves to data/cloud/<run>/ (launch.txt, result.md,
  log.txt.gz), deletes the branch, prints the result and, with `--card`, adds a `todo note`. Exit status 0 = green.
- Environmental failures: scripts/cloud-test-environmental.txt, one regex per line with its reason; matches are listed
  apart and don't make the verdict red. Grown only from real runs. `--summarize <log> [list]` re-summarizes a saved log.
- Pool (data/cloud/pool/, one file per session): a finished session gets the next job as a follow-up message and keeps
  its checkout and cargo output (a repeat build of the same branch: ~0 s vs ~350 s cold). Up to 5 busy
  (MAX_PARALLEL_RUNS), 8 sessions in total (MAX_CLOUD_SESSIONS). After 30 jobs a session gets `/clear`. A session that
  refuses or never answers (90 min) is dropped into data/cloud/dropped-sessions.txt: archive those in the web UI. New
  sessions are named "Cloud Test Lane".
- Model: defaults to Haiku. A session keeps the model it was created with; reused sessions get a `/model` message
  before each job. `CLOUD_TEST_MODEL=<model>`; `CLOUD_TEST_FAST=1` passes fast mode (from 2026-11-02, to use up the
  credit; whether it takes effect is unverified).
- A second launch of a pending run (same branch and tip) only waits for its result. A job without a result frees its
  session after 90 min. A run that can't reach its tests pushes a red result naming why; the test run is killed after
  40 min, or once no test finished for 15 min (STALL_SECONDS). `pkill cargo` leaves the test binary running: the
  watchdog kills timeout's process group.
- The runtime stub is built before the tests with the run's crate source, as test.sh does (WARP_TEST_STUBS_BUILT=1):
  in vendor mode the source replacement comes only from --config flags, so a test's own `cargo build --offline` met
  the VM's empty registry cache and all 22 executable tests failed (card cloud-stub-offline).
- Failures listed in notes/known_failures.md (read in the tested checkout, as the gate reads it at the tested tip) are
  listed apart as "known" and don't make a run red, like the environmental ones.
- Cost: ~$0.30 per targeted run, ~$0.50 per full run (tokens, not VM time). A targeted run takes 5–6 min, mostly the
  cold build. A full run is ~25 min and ignores ~150 more tests than the Mac (tests that skip themselves off macOS).
