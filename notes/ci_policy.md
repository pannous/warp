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
- russh CodeQL fails also on its schedule (likely code scanning not enabled for a private repo) and russh Rust/Semver
  were red on every push: the nightly runs will keep mailing until those are fixed.
- warpgate Test was red on main and on dependabot PRs; PRs still run (policy keeps pull_request).
