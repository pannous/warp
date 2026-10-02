# Supervising fixer sessions (welcoming-syntax round, 2026-10-02)

Workflow that worked:
- Field-test workers write samples in their natural style, report failures as minimal snippets; fixer sessions get one
  topic each (template probes/algo/fixer_prompt.md); every branch is built on origin/main in a scratch export.
- Workers run targeted tests only (`CARGO_BUILD_JOBS=2 … --test <file> -- --test-threads=2`); the supervisor merges a
  batch in scratch/integrate and runs the ONE full suite before pushing to main. Parallel full suites pushed the load to
  ~160 on 10 cores; ~/dev/bin/claude-remote.sh now refuses to spawn while load >= cores.
- Dependent fixes: publish an integration branch and let the next fixers rebase onto it, merge in dependency order.

Pitfalls seen:
- A worker's test passed only because the shared checkout held other agents' uncommitted edits (sorting sample);
  only a clean export proves anything.
- A worker built on a stale integration tip and its commit silently reverted merged work; check `git diff` of a branch
  against its claimed base before merging.
- `pkill -f <pattern>` matched the session's own prompt and killed it; kill by PID.
- zsh: `$new:refs/...` is a history modifier, write `"${new}:refs/..."`.
- The user's checkout falls behind while agents keep uncommitted WIP in it; save a patch to data/, then reset and pull.
