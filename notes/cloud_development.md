# Developing warp in the cloud (evaluation 2026-10-03)

Question (user): "we have some spare cloud tokens: can we develop features in the cloud with them?"
Short answer: YES. The "spare cloud tokens" are the **Cloud session credits**: $250 included with Max, expiring
2026-11-05 08:59 GMT+1. On claude.ai/settings/usage the card reads: "Applies automatically to cloud sessions. After it's
used or expires, your plan's regular usage applies." It pays for **cloud sessions** (`claude --cloud`, claude.ai/code,
the app) but NOT for routines (RemoteTrigger / `/schedule`). Verified: a 23-min routine left it untouched, while one
`claude --cloud` probe moved it $0.47 → $0.83. Cloud work also unloads the Mac.

## 1. Cloud execution options for this account

| Option | Works here? | Notes |
|---|---|---|
| **Cloud sessions** (claude.ai/code, mobile app, `claude --cloud "<task>"`) | yes, **paid by the $250 credit** | `claude --cloud` needs a TTY ("--cloud requires an interactive terminal"), so from an agent run it in detached tmux and read the `View: https://claude.ai/code/session_…` line. It clones the GitHub remote at the current branch/commit (push first). Follow-ups: `claude -p "msg" --cloud <session-id>` (posts and exits). The user can steer from the browser or phone. `claude --teleport <id>` pulls one back into a local terminal. |
| **Routines** via `RemoteTrigger` (`create` + `run_once_at`, or `/schedule`) | yes, but **plan limits**, not the credit | Readable run log (`list_runs` / `get_run_log`), no TTY needed. Caps: 100 scheduled runs/h, 30 "run now"/h. Use them after the credit is gone or expired. |
| Agent tool `isolation: "remote"` | unreliable | On 2026-09-29 it silently fell back to local worktrees (plan usage, Mac load). Don't use it for cloud work. |
| `claude ultrareview` | available | Cloud-hosted multi-agent review of a branch: a cheap second opinion before merging cloud branches. |
| GitHub Actions `claude-code-action` | not set up | pannous/warp has no Anthropic secrets. It could run on the subscription (`claude setup-token` → `CLAUDE_CODE_OAUTH_TOKEN` secret) or on API credits (`ANTHROPIC_API_KEY`). It gives nothing a routine doesn't, and GitHub runners have 2 to 4 cores. |
| Managed Agents / Messages API | needs a Console API key + API credits | Separate pay-as-you-go billing. Not worth it for this repo. |
| `--remote-control` (claude-remote.sh) | local | Runs on the Mac; only the UI is remote. It does not reduce load. |

Sandbox (routine run 2026-10-03): Ubuntu x86_64, 4 vCPU / 16 GB / 30 GB, rustc 1.94.1, lean NOT installed.
Network: crates.io and static.rust-lang.org return 403 "Host not in allowlist" (probe routine 2026-09-30, environment
env_011CUKr7fLEneY74cFok55eT). The docs say the default "Trusted" level allows crates.io, so the environment's network
setting is probably Custom or None. **Where to edit it** (the user found it 2026-10-03): open a cloud session at
claude.ai/code → the chevron next to the session title → **Edit cloud environment**. The docs' path is the cloud icon
showing the environment name in the row above the message box → Cloud → hover the environment → gear icon. There is no
settings page or URL for it. The dialog holds the name, Network access (None / Trusted / Custom / Full), env vars
(e.g. `BASH_MAX_TIMEOUT_MS=1800000`, since the suite takes ~9 min) and a setup script. A setup script that runs
`git fetch origin vendor && git archive origin/vendor vendor | tar -x` and pre-builds would be cached if it finishes in
~5 min. `/remote-env` in the CLI picks the default environment for `claude --cloud`. Until the network is Trusted, the
`vendor` branch (notes/cloud_offline_build.md) is the build path. It works; pilot below.

## 2. Billing: what draws from where

Measured with `~/dev/bin/claude-usage.sh`. It reads the OAuth usage endpoint with the keychain token and never prints it.

Account: Max 20x (keychain `Claude Code-credentials-0bd83d07`). At 2026-10-03 10:42Z:
- `five_hour` 9 %, `seven_day` 59 %: the plan limits shared by ALL Claude Code use (local and cloud).
  `seven_day_breakdown`: Claude Code 98 %, Cowork 2 %.
- **Cloud session credits** (usage API key `iguana_necktie`): $250, resets/expires 2026-11-05, a promo for Pro ($100) and
  Max ($250) subscribers that was claimable until 2026-10-07 (`/claim-credit`); this account has already claimed it. The CLI's
  `/usage` does not show it. claude.ai/settings/usage and `~/dev/bin/claude-usage.sh` do. Measured: the routine pilot
  (10:46–11:10Z) left it at $0.467345, and a `claude --cloud` probe session at 11:32Z moved it to $0.833833. So it
  pays for cloud sessions only, not routines.
- `extra_usage` (pay-as-you-go usage credits): `user_disabled: true`, `credits_ever_enabled: true`. The user turned
  overage off. Turning it on lets work continue past the 5-hour limit, at a cost.
- A second keychain login exists: a **Pro** account (`Claude Code-credentials-fe6e0099`, token expired 2026-09-28).
  It has its own, separate limits. Logging into it on claude.ai/code would give a second, independent cloud budget
  (Pro limits are small, and it needs GitHub access to pannous/warp).

Without the credit, cloud sessions and routines share the subscription's limits (code.claude.com/docs/en/claude-code-on-the-web#limitations). There is no separate compute charge for the VM, and API keys don't work for
cloud sessions. Check usage with `/usage` in the CLI, claude.ai/settings/usage, or `~/dev/bin/claude-usage.sh`.

So: until 2026-11-05, cloud SESSIONS are effectively free (up to $250), and every cloud run saves the Mac. Each cloud run compiles and tests on its own 4-core VM.
The 2026-09-28 limit crash came from SIX parallel **Opus** runs. Sonnet 5.5 costs much less of the limit per run.

## 3. Pilot (2026-10-03): routine `trig_01HMT7SPg8Ggzm4XmGm4a5uU`

Task: shared `KIND_MASK` / `BYTE MemArg` constants (refactor, no behaviour change). Model: Sonnet 5.5.
Session: https://claude.ai/code/session_01QMSxkvhC3Pxcs5gFWEv1wr. Result: branch **`claude/shared-kind-constants`**
(987dd412 refactor, bb972476 report). 13 files, +43/−31: one `pub const KIND_MASK` in src/type_kinds.rs and one
`pub(crate) const BYTE` in src/wasm_emitter/mod.rs. The `kind & 0xFF` tag masks in node.rs, wasm_reader.rs,
wasmtime_runner.rs and mod.rs now use KIND_MASK. `(kind >> 8) & 0xFF` is untouched.

| Step | Time |
|---|---|
| sandbox allocated → Claude running | 5 s |
| clone + fetch vendor + checkout | 15 s |
| cold build + full suite (offline, vendor) | 729 s (build ≈ 185 s, tests ≈ 544 s) |
| refactor | ~1 min |
| incremental rebuild + full suite | 544 s (rebuild 29 s) |
| total wall time, fire → pushed | 23 min |

Suite in the cloud: 1442 passed, 7 failed, 81 ignored, the same before and after. The 7 failures are environmental:
- lean missing: test_law ×2, probe_footguns::test_proof_model_matches_unbounded_int
- package sub-builds need `miniz_oxide`, missing from the vendor branch: test_package_tools ×2,
  test_uniscript::the_index_matches_the_readable_entities, test_package_pin::header_signatures_are_parsed_once.
  Fix: vendor the packages' own lockfiles into the `vendor` branch as well.

Plan usage: `five_hour` went 9 → 21 % during the run, but ~6 local sessions were working at the same time, so the
cloud share can't be isolated. The $250 bucket did not move.
Reporting: the run cannot message back. The channel is the pushed branch (with a report file) plus the run log
(`RemoteTrigger get_run_log`). GitHub "Rust CI" also starts on every claude/* push. It ran twice here (one push per
commit), so a cloud task should push all its commits once at the end, or once per step.

## 4. Recommended workflow (supervisor → cloud)

1. **Pick cloud-suitable tasks**: pure Rust in src/ and tests/ that needs no Mac and no network (parser, analyzer,
   emitter, refactors, new tests, ignored-test sweeps). **Keep local**: uniscript, packages and headers (until vendor
   carries the package lockfiles), Lean proofs (no lean in the sandbox), macOS/iOS apps, anything that reads ~/ or
   needs secrets, and tasks that need fast back-and-forth.
2. **Launch** one cloud SESSION per task (credit-funded) from a pushed commit:
   `tmux new-session -d -s cloud-<task> -c <clean checkout> 'claude --cloud "<prompt>"; sleep 300'`, then
   `tmux capture-pane -p -t cloud-<task>` to get the `View:` URL and session id. Give that URL to the user. Killing the
   tmux pane does not stop the cloud session. Steer with `claude -p "<msg>" --cloud <session-id>`. After the credit is
   used up or expires: routines via `RemoteTrigger create` (`run_once_at` = now + 1 min; job_config.ccr as in the pilot:
   environment env_011CUKr7fLEneY74cFok55eT, source github.com/pannous/warp, model `claude-sonnet-5-5`, tools
   Bash/Read/Write/Edit/Glob/Grep).
3. **Prompt template** (the rules part is in notes/cloud_tasks.md; refer to it instead of repeating it):
   ```
   Repository pannous/warp. Task <ID>: <what, with file pointers and decisions>.
   Branch claude/<task-id> from origin/main. Follow notes/cloud_tasks.md (rules) and notes/cloud_offline_build.md
   (offline build from the vendor branch; run the suite with run_in_background, it takes ~9 min).
   Record baseline totals first; red test first in tests/test_<topic>.rs; never edit existing tests.
   Commit per step, conventional commits, no Claude co-author/session lines. Push after every step. Do NOT merge.
   Report: commit notes/cloud_reports/<task-id>.md (start/end UTC, baseline vs final totals, changes, open items).
   ```
4. **Parallelism**: while the credit lasts, the budget is dollars. The tiny probe cost ~$0.37, so a 20–30 min
   Sonnet task is probably a few dollars. Estimate after the first real batch with `~/dev/bin/claude-usage.sh`.
   Start with 3 parallel sessions, and watch both the credit and five_hour: it is unverified whether credit-funded
   sessions also count toward the 5-hour limit. Every run pushes after every step, so a limit cut loses at most one
   step: resume with a follow-up message (`claude -p … --cloud <id>`) or a RESUME prompt that names the branch.
5. **Collect**: session status at claude.ai/code (routines: `RemoteTrigger list_runs` / `get_run_log`). The result is the branch and its report file.
   The Integrator (warp-6c) merges the claude/* branch after a local full suite (`tests/queue.sh`). It drops the
   report file, or moves it into notes/, and then deletes the remote branch.
6. **Cheap wins to do first**: (a) user: Edit cloud environment → Network access Trusted (crates.io), so runs build
   without the vendor branch and the package tests pass, and add `BASH_MAX_TIMEOUT_MS=1800000`; (b) add the packages'
   lockfiles to the vendor refresh workflow; (c) spend the credit before 2026-11-05: it is lost afterwards.
