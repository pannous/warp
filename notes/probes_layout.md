# probes / scratch / data layout (2026-10-02)

Rule (AGENTS.md "Folders"): `probes/` tracked, hand-written sources only; `scratch/` disposable copies, worktrees,
cargo homes, private index files; `data/` logs, results, patches, dumps; build output in the one shared target dir from
`~/.cargo/config.toml` (notes/build_speed.md).

- Allowlist lives in `.gitignore` (`!probes/**/*.<ext>`); `probes/check_layout.sh [tree-ish]` reads the same list and
  fails on tracked probes under src/ tests/ target/ vendor/ .git, blobs over 100 KB, or other extensions.
- `./test.sh` runs the check first: agents commit through `git commit-tree`, which skips hooks. The local
  `.git/hooks/pre-commit` runs it too, for humans.
- Templates `probes/algo/fixer_prompt.md` / `worker_prompt.md` use `scratch/<tag>.index`, `scratch/<tag>-export`, `data/<tag>/`.

Migration (2026-10-02): non-sources in probes/ moved to scratch/ or data/.

## Reusable checks (kept by cleanup-probe-scripts, 2026-10-13)
One-off scripts go once their finding is in a note or a test; these stay because they re-check something:
- docs against main: `probes/keyword_examples.py` (wiki/keyword.md results), `probes/primer_check.py` (primer fences)
- queue guards: `probes/queue_memory_cap.sh`, `probes/test_queue_time_limit.sh`; hooks: `probes/write_hook/check_hook.sh`
- terminal (pty): `probes/console_ctrl_c.py`, `probes/hints_dismissed/answer_got_it.py <answer> warp prog.warp`
- builds and packages: `probes/aot/build_exe.sh`, `probes/binary_packages/run_tool.sh`
- speed: `probes/perf/sample_times.py` (slowest samples), `probes/fuel_steps.sh <warp> <program>` (wasm steps a program
  needs), `probes/fd_limit.sh [filter]` (targeted tests under macOS's 256 open files)
- playground: `probes/todo_app/in_playground.sh` (needs a browser: CI only, notes/ci.md); `probes/tour_check.mjs` (tour
  examples against the CLI), `probes/guide_unsorted_samples.mjs` (samples no guide chapter names)
- console completion (pty): `probes/console_member_completion.py <warp>`
- self-hosting: `probes/self_hosting/parser/compare_forms.sh`; uniscript: `probes/uniscript_fonts_test.py`,
  `probes/entities_index/names.py`
- code cleanup: probes/cleanup/ (notes/cleanup.md): dead_scan.py, similar_names.py, duplicate_constants.py,
  drop_function.py, call_to_method.py, split_module.py
