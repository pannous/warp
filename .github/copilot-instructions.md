# Copilot coding agent in warp

You run on a GitHub Actions runner, not on the maintainer's Mac. AGENTS.md describes the local agent team; these rules
replace its local-only parts for you:

- The local test queue (`tests/queue.sh`), the Integrator, worktrees and `~/dev/bin/todo` do not exist for you.
  Run tests directly: `cargo test --test tests <module>::` for the files you touched (tests/<topic>/*.rs are modules
  of ONE test crate, tests/main.rs; a new file is declared as `mod x;` in its folder's mod.rs).
- Deliver working code, never a PR without changes: write a failing test first (tests/<topic>/test_*.rs), make it
  pass, and paste the test command and its result line into the PR description.
- Never change the expected value of an existing test; if one seems wrong, say so in the PR and leave it.
- Keep the PR to the issue's scope. Conventional commit messages (`fix:`, `feature(minor):`, `test:`).
- Language rules: notes/welcoming.md (clear intent compiles; ambiguity is a warning or a loud error naming the
  explicit forms), decided questions in notes/decisions.md.
- The maintainer's Integrator runs the full native and browser suites on your branch and merges it to main itself;
  do not merge.
