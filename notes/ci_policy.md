# CI policy across pannous repos

Test workflows do not run on every push to the default branch (failure mails were noise). They run nightly when the
branch changed in the last ~25 h (schedule + a `decide` job doing `git log --since`), by hand
(`gh workflow run "<name>" -R pannous/<repo>`), on `v*` tags, on a push whose head commit message contains `[ci]`, on
pull requests, and always on `claude/*` branches (only warp uses them). The `decide` job always succeeds, so a skipped
push sends no mail. Reference: `.github/workflows/rust.yml` here; applied to russh (rust, semver, codeql) and warpgate
(test, cargo-deny). The rewrite is scripted: `probes/ci-policy/apply_policy.py <workflow.yml> <owner/repo> <branch>
[cron] [window]`.

Gotcha: a commit message that describes the policy ("... on [ci] commits ...") contains `[ci]` and triggers the full
run. Avoid the literal `[ci]` in commit messages unless meant.

Left alone on purpose: deploy and maintenance workflows (warpgate server-build.yml, hieros Pages and wiki-sync,
npm-audit-fix, warp offline-build-refresh.yml), workflows disabled in GitHub, upstream forks without runs, ~/jobs
client work.

warpgate's Test workflow is a Build job (UI, test SDK and the debug binary as artifact warpgate-build; needs
include-hidden-files for warpgate-web/dist/.vite/manifest.json), a Unit job (coverage unit tests, SonarCloud) and three
pytest shards; one runner for everything lost the runner. russh's CodeQL is disabled on the private fork (needs GitHub
Advanced Security).
