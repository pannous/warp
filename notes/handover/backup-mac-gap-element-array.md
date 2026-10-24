# backup/mac-gap-element-array: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md) after the Mac /Volumes/Worktrees cleanup. Update it when you pick the branch up.

- Card: #3200 x is a 100 element array declares x as 100 items ([gap-element-array](https://github.com/pannous/warp/issues/3200))
- Branch: `backup/mac-gap-element-array` at `887d31b214`, 1 ahead / 227 behind origin/main `c196fc916`; merges cleanly: yes
- Kind: warp only (lib/compiler/parser.warp, lib/sugar/lists.warp, probes, tests)
- Owner: none (Mac backup of a retired agent's worktree; the planner assigns it).
- State: Uncommitted Mac worktree work: one parser.warp line in applies() letting a rule's `$hole` stand where a name would (`a $n element array`), plus the lists.warp rule, probes/sugar/element_array and tests/lists/test_element_arrays.warp. An earlier version of the parser attempt that origin/gap-element-array dropped because it made pure builds loop.

## Next step
Archive unless the card's owner wants this variant of the parser line: origin/gap-element-array (handover notes/handover/gap-element-array.md) carries the rule and tests; if the parser approach is retried, start from this line and check that `warp rebuild self`/pure builds do not loop.
backup/* branches are never merged directly.
