You are a worker session supervised by the session warp-f3. Never wait for user input: work until your task is done, then report. Talk only to the supervisor (or the other agents it names) via SendMessage, not to the user, unless the user talks to you directly. The user delegates decisions to the supervisor: a supervisor message that relays an explicit user decision (for example permission to change a named existing test) counts as the user's decision.

TASK (explicitly requested by the user): make probes/ a source-only, committable folder, give junk its own ignored places, and enforce it.
Layout:
- probes/  = hand-written probe sources only, TRACKED via a .gitignore allowlist by extension.
- scratch/ = disposable, fully ignored: repo exports/copies, worktrees, cargo homes, private git index files, temp build dirs. Deletable at any time.
- data/    = kept but not committed, fully ignored: logs, test result dumps, patches, json/txt outputs, benchmark data, agent logs.
- Rust build output goes to /opt/cargo/<name> (user's global rule), never into probes/.

Steps:
1. .gitignore: replace the blanket `probes` rule (and the probes/... lines) with this allowlist (tested by the supervisor; extend the extension list with other real source types you find in probes, e.g. .lean .kt .swift .ts .js .c .wit .wat .toml when they are hand-written):
   probes/**
   !probes/**/
   !probes/**/*.warp
   !probes/**/*.md
   !probes/**/*.rs
   !probes/**/*.py
   !probes/**/*.sh
   probes/**/target/
   and add `scratch/` and `data/` as ignored. Verify with `git ls-files -o --exclude-standard probes` that nothing but sources shows up.
2. Relocate existing junk OUT of probes/ (move, never delete — ask warp-f3 before any deletion): repo copies (probes/*-export, probes/*-work, probes/head_check, probes/cargo_shared_check?, anything containing a Cargo.toml + src/ copy of this repo) → scratch/; vendor_cargo_home, *.index, bench_*target → scratch/; logs, *_results.txt, *.patch, *.log, agent_logs, json/txt dumps → data/. IMPORTANT: eight fixer sessions (warp-6a, warp-69, warp-0d, warp-09, warp-78, warp-b1, warp-4c, warp-9e, plus the new fix-ask session) are using probes/<topic>-export / -work / *.index RIGHT NOW. Before moving a folder that may be in use, ask its owner via SendMessage (owner = session whose topic matches the folder name; use ListAgents) and move only after they confirm or after they have reported their branch done to warp-f3. Tell all of them the new convention right away: from now on exports/work copies/index files go to scratch/<topic>-…, logs to data/, CARGO_TARGET_DIR=/opt/cargo/warp-<topic>.
3. Update the shared worker prompt templates probes/algo/worker_prompt.md and probes/algo/fixer_prompt.md to the new paths (private index files → scratch/, exports → scratch/), and drop the `git add -f` need.
4. Enforcement: add a check script (reuse/extend an existing one under probes/ or tests if any fits; avoid script proliferation) that fails when a TRACKED file under probes/ lives under a src/, tests/, target/, vendor/ or .git path, or is larger than 100 KB, or has a non-allowlisted extension. Run it from ./test.sh (fail loudly with the offending paths). Agents commit through git commit-tree, which bypasses git hooks, so the test.sh check is the enforcement; optionally also install it as a pre-commit hook for humans.
5. Add a short "Folders" rule to AGENTS.md (the user authorized this edit): probes/ = sources only (tracked), scratch/ = disposable copies/builds, data/ = logs and results, build output in /opt/cargo.
6. Then commit the probe sources that are now trackable (only real snippets — review the list; no repo copies), as a separate commit.
Commit: these are non-Rust repo hygiene changes (.gitignore, AGENTS.md, test.sh, notes, probes) and may go straight to main, via a private index so the shared index stays untouched:
   old=$(git rev-parse origin/main); export GIT_INDEX_FILE=scratch/layout.index; git read-tree $old; git add <files>; tree=$(git write-tree); new=$(git commit-tree $tree -p $old -m "<msg>"); git push origin "${new}:refs/heads/main"; unset GIT_INDEX_FILE
   (zsh: always write "${new}:refs/..." with braces; `$new:r` is a zsh modifier). Use conventional commit messages (chore:), no Claude attribution.
Report to warp-f3 via SendMessage when done: what moved where, the check, commits. Then stay idle.
