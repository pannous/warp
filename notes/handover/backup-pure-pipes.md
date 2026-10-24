# backup/pure-pipes: branch handover

Written 2026-10-24 by the Integrator (branch-card rule, notes/agents/common.md). Update it when you pick the branch up.

- Card: draft card hDLRbM 'pure lane: land tests/operators/test_pipe_functions.warp (backup/pure-pipes)' (Next)
- Branch: `backup/pure-pipes` at `f0af29847`, 1 ahead / 227 behind origin/main `b8d2c4e99`; merges cleanly: yes
- Kind: warp only
- Owner: No owner: the card waits in Next for the planner to assign.
- State: One test file, tests/operators/test_pipe_functions.warp (`3 | inc`, `f = inc | dbl` in the pure lane), saved from the pure-pipes tree; the pipes feature itself is on main (0c9660041).

## Done
The test file (19 lines).

## Missing
Run it on main's pure lane and land it if it passes; nothing else.

## How to test
Warp-only gate: `/workspace/box-slot.sh scripts/gate-probes.sh --base origin/main --folders <tree>` (tier 0 rebuild pure+self + hellos, tier 1 probes/gate, tier 2 the changed test folders); compare tier 2's failing count with main's run of the same folders (it must not go up).

## Known failures
Integrator batch check 2026-10-24 (main d665c3937 + this, sol-warp-9a634432a, pure `warp test tests/operators`): 4 of 6 cases pass, 2 fail: line 7 `inc(x) := x + 1; print(4 | inc)` got ø (want 5); line 16 `" ab " | trim` got Error("outside the core: the unknown name trim") (a pipe into a library word). tests/operators would go 182 -> 184 failing, so it was not merged (fewer-failures rule).

## Next step
Not merged (backup/* branches are never merged): the planner assigns it; the owner copies the file onto a fresh branch from main, fixes the two cases (a pipe inside call arguments; a pipe into a library word, cf. test_pipe_stages' `2|square`), and lands that branch.
