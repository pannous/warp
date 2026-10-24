# backup/mac-rewrite-zip-runs: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md) after the Mac /Volumes/Worktrees cleanup. Update it when you pick the branch up.

- Card: #3105 rewrite engine F11 zip over rest holes, F12… (Done; [rewrite-zip-runs](https://github.com/pannous/warp/issues/3105))
- Branch: `backup/mac-rewrite-zip-runs` at `bc7db483ba`, 3 ahead / 463 behind origin/main `c196fc916`; merges cleanly: no (conflicts: lib/compiler/rewrites.warp, notes/rewrites.md)
- Kind: warp only (lib/compiler/rewrites.warp, notes, probes, tests)
- Owner: none (Mac backup of a retired agent's worktree; the planner assigns it).
- State: The parked original rewrite-zip-runs (F11 zip over rest holes named without dots, fresh rest holes; F12 a shrunk run retried, statement-run matching in the sugar step). Main landed the feature in another form via rewrite-zip-runs-2 (bd2a0c09f, merge f3b46668b); this tip's probes/rewrites/zip/*.warp and tests/modules/test_rewrite_zip.warp are not on main.

## Next step
Archive (superseded by f3b46668b) after checking whether tests/modules/test_rewrite_zip.warp and probes/rewrites/zip pass on main; if they do and main lacks equivalents, land those three files from a fresh branch first.
backup/* branches are never merged directly.
