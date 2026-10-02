# probes / scratch / data layout (2026-10-02)

Rule (AGENTS.md "Folders"): `probes/` tracked, hand-written sources only; `scratch/` disposable copies, worktrees,
cargo homes, private index files; `data/` logs, results, patches, dumps; build output in `/opt/cargo/warp-<topic>`.

- Allowlist lives in `.gitignore` (`!probes/**/*.<ext>`); `probes/check_layout.sh [tree-ish]` reads the same list and
  fails on tracked probes under src/ tests/ target/ vendor/ .git, blobs over 100 KB, or other extensions.
- `./test.sh` runs the check first: agents commit through `git commit-tree`, which skips hooks. The local
  `.git/hooks/pre-commit` runs it too, for humans.
- Templates `probes/algo/fixer_prompt.md` / `worker_prompt.md` use `scratch/<tag>.index`, `scratch/<tag>-export`, `data/<tag>/`.

Migration: everything in probes/ that was not a source moved to scratch/ (repo copies, index files, binaries, copies of
src/tests files) or data/ (logs, results, patches, renders); nothing deleted. The fixer exports still in probes
(`*-export`, `*-work`, in use on 2026-10-02) stay ignored via `probes/*-export/`, `probes/*-work/` until their owners
move them. Their topic snippet dirs (break elements listexpr listparams maps parse rangeblock sugar ask) were left for
the owners' branches to commit, avoiding add/add conflicts.
