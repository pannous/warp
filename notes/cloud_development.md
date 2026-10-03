# Developing warp in the cloud (evaluation 2026-10-03)

Question (user): "we have some spare cloud tokens: can we develop features in the cloud with them?"
Short answer: yes, cloud routines work end to end for warp today (pilot below: 23 min, branch pushed, suite identical
to baseline). They run on Anthropic's machines, so the Mac's load drops. A separate token pool for them is NOT
confirmed (see Billing).

## 1. Cloud execution options for this account

| Option | Works here? | Notes |
|---|---|---|
| **Routines** via `RemoteTrigger` (`create` + `run_once_at`, or `/schedule`) | yes, used since 2026-09-28 | Best for hands-off work: one prompt, the run log is readable via `RemoteTrigger list_runs` / `get_run_log`, no local process. Caps: 100 scheduled runs/h, 30 "run now"/h. |
| **Cloud sessions** (claude.ai/code, mobile app, `claude --cloud "<task>"`) | available (CLI 2.1.288 has `--cloud`, `--teleport`) | Interactive: the user can steer it from the browser or phone. `--teleport <session>` pulls one back into a local terminal. |
| Agent tool `isolation: "remote"` | unreliable | On 2026-09-29 it silently fell back to local worktrees (plan usage, Mac load). Don't use it for cloud work. |
| `claude ultrareview` | available | Cloud-hosted multi-agent review of a branch: a cheap second opinion before merging cloud branches. |
| GitHub Actions `claude-code-action` | not set up | pannous/warp has no Anthropic secrets. It could run on the subscription (`claude setup-token` → `CLAUDE_CODE_OAUTH_TOKEN` secret) or on API credits (`ANTHROPIC_API_KEY`). It gives nothing a routine doesn't, and GitHub runners have 2 to 4 cores. |
| Managed Agents / Messages API | needs a Console API key + API credits | Separate pay-as-you-go billing. Not worth it for this repo. |
| `--remote-control` (claude-remote.sh) | local | Runs on the Mac; only the UI is remote. It does not reduce load. |

Sandbox (routine run 2026-10-03): Ubuntu x86_64, 4 vCPU / 16 GB / 30 GB, rustc 1.94.1, lean NOT installed.
Network: crates.io and static.rust-lang.org return 403 "Host not in allowlist" (probe routine 2026-09-30, environment
env_011CUKr7fLEneY74cFok55eT). The docs say the default "Trusted" level allows crates.io, so the environment's network
setting is probably Custom or None. The user can change it under claude.ai/code → environment settings. Until then the
`vendor` branch (notes/cloud_offline_build.md) is the build path. It works; pilot below.

## 2. Billing: what draws from where

Measured with `~/dev/bin/claude-usage.sh`. It reads the OAuth usage endpoint with the keychain token and never prints it.

Account: Max 20x (keychain `Claude Code-credentials-0bd83d07`). At 2026-10-03 10:42Z:
- `five_hour` 9 %, `seven_day` 59 %: the plan limits shared by ALL Claude Code use (local and cloud).
  `seven_day_breakdown`: Claude Code 98 %, Cowork 2 %.
- **A separate $250 bucket**, internal name `iguana_necktie`: $0.47 used, $249.53 left, resets 2026-11-05.
  This is the most likely candidate for the user's "spare cloud tokens". BUT it did NOT move during the 23-minute cloud
  pilot (still $0.467345 at 11:13Z, after the run ended at 11:10Z), and the earlier routines (2026-09-28..30) also left it
  near zero. So routines bill against the plan limits, not this bucket. The name is a server-side codename (not in
  the CLI binary). What spends it is unknown. Plausibly a promotional credit for another surface (Claude Code on the web
  promo, Cowork, design or ultrareview). → **The user should look at claude.ai/settings/usage**, where it shows with
  its real label. (The headless browser is stopped by Cloudflare's bot check there, so this could not be automated.)
- `extra_usage` (pay-as-you-go usage credits): `user_disabled: true`, `credits_ever_enabled: true`. The user turned
  overage off. Turning it on lets work continue past the 5-hour limit, at a cost.
- A second keychain login exists: a **Pro** account (`Claude Code-credentials-fe6e0099`, token expired 2026-09-28).
  It has its own, separate limits. Logging into it on claude.ai/code would give a second, independent cloud budget
  (Pro limits are small, and it needs GitHub access to pannous/warp).

The docs (code.claude.com/docs/en/routines.md#usage-and-limits, …/claude-code-on-the-web.md) agree: cloud sessions and
routines share the subscription's limits. There is no separate compute charge for the VM, and API keys don't work for
cloud sessions. Check usage with `/usage` in the CLI, claude.ai/settings/usage, or `~/dev/bin/claude-usage.sh`.

So: cloud runs do NOT save tokens, but they DO save the Mac. Each cloud run compiles and tests on its own 4-core VM.
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
2. **Launch** one routine per task with `RemoteTrigger create` (`run_once_at` = now + 1 min; job_config.ccr as in
   the pilot: environment env_011CUKr7fLEneY74cFok55eT, source github.com/pannous/warp, model `claude-sonnet-5-5`, tools
   Bash/Read/Write/Edit/Glob/Grep). Or from the terminal: `claude --cloud "<prompt>"`.
3. **Prompt template** (the rules part is in notes/cloud_tasks.md; refer to it instead of repeating it):
   ```
   Repository pannous/warp. Task <ID>: <what, with file pointers and decisions>.
   Branch claude/<task-id> from origin/main. Follow notes/cloud_tasks.md (rules) and notes/cloud_offline_build.md
   (offline build from the vendor branch; run the suite with run_in_background, it takes ~9 min).
   Record baseline totals first; red test first in tests/test_<topic>.rs; never edit existing tests.
   Commit per step, conventional commits, no Claude co-author/session lines. Push after every step. Do NOT merge.
   Report: commit notes/cloud_reports/<task-id>.md (start/end UTC, baseline vs final totals, changes, open items).
   ```
4. **Parallelism**: at most 3 Sonnet runs at a time (2 with Opus). Stagger the rest by ~25 min, and check
   `~/dev/bin/claude-usage.sh` before each wave (stop above ~70 % of five_hour). Every run pushes after every step,
   so a limit cut loses at most one step: re-arm with a RESUME prompt that names the branch.
5. **Collect**: `RemoteTrigger list_runs` / `get_run_log` for status. The result is the branch and its report file.
   The Integrator (warp-6c) merges the claude/* branch after a local full suite (`tests/queue.sh`). It drops the
   report file, or moves it into notes/, and then deletes the remote branch.
6. **Cheap wins to do first**: (a) user: set the cloud environment's network to Trusted (crates.io), so runs can build
   without the vendor branch and the package tests pass; (b) user: check claude.ai/settings/usage to find out what the
   $250 bucket is; (c) add the packages' lockfiles to the vendor refresh workflow.
