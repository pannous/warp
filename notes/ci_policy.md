# CI policy across pannous repos

Decision 2026-10-03: test workflows do not run on every push to the default branch (failure mails were noise).
They run nightly when the branch changed in the last ~25 h (schedule + `decide` job doing `git log --since`), by hand
(`gh workflow run "<name>" -R pannous/<repo>`), on `v*` tags, on a push whose head commit message contains `[ci]`,
on pull requests, and always on `claude/*` branches where a repo uses them (only warp does).
The `decide` job always succeeds, so a skipped push sends no mail. Reference: `.github/workflows/rust.yml` here.

Gotcha: a commit message that *describes* the policy ("... on [ci] commits ...") contains `[ci]` and triggers the full
run. Happened for the policy commits in warp, russh and warpgate. Avoid the literal `[ci]` in commit messages unless meant.

## Applied (2026-10-03)

| repo | workflow | change | verified |
|---|---|---|---|
| warp | rust.yml (Rust CI) | reference implementation | later pushes: decide=success, test skipped, run success |
| russh | rust.yml (Rust) | decide gate on all 7 jobs, nightly cron, dispatch, v* tags | decide=success; this push ran fully because the message had `[ci]` |
| russh | semver.yml (Semver check) | same | same |
| russh | codeql.yml (CodeQL Advanced) | same, kept the weekly cron (Sat 03:30) with an 8-day change window | same |
| warpgate | test.yml (Test) | push restricted to main + v* tags (was every branch), nightly cron, dispatch | same |
| warpgate | cargo-deny.yml (Cargo Deny) | same | same |

The rewrite is scripted: `probes/ci-policy/apply_policy.py <workflow.yml> <owner/repo> <branch> [cron] [window]`.

## Left alone

- warpgate server-build.yml: deployment build for pannous.com, needs push.
- warpgate biome/build/clippy/codeql/fmt-toml/lockfile/reprotest/check-schema-compatibility: disabled manually in GitHub.
- hieros jekyll-gh-pages.yml: Pages deploy (fails on every push, 13/13 red: deploy problem, not test noise),
  plus dependabot-automerge, npm-audit-fix, wiki-sync (maintenance; wiki-sync also failing).
- warp/hieros/Listen npm-audit-fix.yml, warp offline-build-refresh.yml: scheduled maintenance/vendoring.
- wasp test.yml: triggered by workflow_run of the disabled build.yml, never runs; claude*.yml are agent bots.
- Upstream forks with workflows but no runs on the fork (qemu, mozjs, MarkdownEditing, conan-center-index, LyricsKit,
  goo, rust-script, apple-mail-mcp, TRX64, BoostNote-App): not the user's CI, no mails.
- ~/jobs: client work, skipped.
- No other recently pushed pannous repo has workflows.

## Open
- russh (fixed 2026-10-04, cecaa23): clippy on Linux (`speed_t` is u32 there: allow useless_conversion), the Windows job
  builds the workspace without the unix-only rssh crate, russh 0.64.0 for the fork's breaking changes (semver check
  against crates.io 0.63.3). Rust and Semver dispatched green. CodeQL Advanced is disabled on the fork: code scanning
  needs GitHub Advanced Security on a private repo ("Code scanning is not enabled for this repository").
- warpgate Test (fix c0a084ac, 2026-10-04): the autouse session fixture `report_generation` (tests/conftest.py) runs
  `cargo llvm-cov run --all-features`, a full rebuild that exceeded pytest's 300 s timeout; the workflow now prebuilds
  it before `run.sh`. Verify the dispatched run (pannous/warpgate actions); dependabot PRs still run.
- warpgate Test, later 2026-10-04: two runs lost the runner with coverage-instrumented binaries; integration tests now
  run on a plain `cargo build --all-features` (7fb2e47f). Run 37176521946 then completed the whole suite with ~15 real
  failures (mysql CLI missing on the runner: mysql/target_credential_encryption/db_migrations/admin_approval_protocols;
  vnc ×4; web_ssh hung 14 min, then exit 143 before pytest's summary). 3758040a/6551bc0a: mysql-client, a resource line
  every 2 min, `timeout --signal=INT 80m` around pytest (an early end still prints failures), -rfE, per-test 180 s.
  Run 37181090813 lost the runner again during pytest ("The hosted runner lost communication", no log of the Run
  step survives that). 5f5bb7f9: the Tests job is a matrix of three shards (every third test file; shard 1 also runs
  the Rust unit tests, the API SDK tests and SonarCloud), each on its own runner. Run 37186712652.
