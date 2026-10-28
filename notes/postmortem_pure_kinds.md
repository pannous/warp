# Post-mortem: `warp pure hello` red on main for ~2 h (2026-10-21, card pure-kinds)

## What broke
`warp pure samples/hello/hello.warp` failed on main with
`Error("kinds needs 1 argument at core/emitter/emitter.warp:1883:59; fix: function kinds")`: the field in
`module.kinds["value"]` was read as a call of analyzer.warp's global function `kinds`.
Culprit: local-named-like-5 (61ec6eb386, Rust: library_words' per-function VariableScope), landed in batch-32
(948575673c, 09:28). Fixed by reverting it (bisect-scopes, user: "revert now", ~12:00) plus a guard test
(079b080870: a main-level object's field named like a program function reads the field); the feature is re-landed
corrected as local-named-like-6.

## Timeline (CEST)
- 09:28 batch-32 lands with local-named-like-5; the gate's pure-chain check passes. 09:50 batch-33 lands, passes too.
- 09:57 main's binary rebuilt (c8bb9c815): from here a cold `warp pure hello` fails.
- ~10:05 warp-interviewer reports it; Supervisor alerts the Integrator, warp-selector-casts (suspected bare-name) bisects.
- ~10:15 warp-speed starts bisecting too (duplicate, stopped by the Supervisor).
- ~10:45 old lib + new binary fails as well: the cause is Rust. Cloud bisect rounds of ~8 min each, the pool full.
- ~11:35 the Supervisor brings in warp-standalone (owner of the local-named-like family the error pattern points at);
  ~11:45 it names its own local-named-like-5. A revert branch is green, but held: the user's rule "never git revert
  unprompted"; the question goes to the user via the Interviewer.
- ~12:00 user: "revert now"; the Integrator lands it.

## Why it got through
1. The gate's pure-chain check ran main's previous binary with the batch's lib: a batch's Rust changes never met
   the pure chain. (Fixed by the Integrator: a batch whose Rust differs from main's binary is judged with its own
   release build, +10 min.)
2. The cloud suite has no running test of the whole `warp pure` path: tests/parser/test_warp_pure.rs
   `warp_pure_runs_hello_world` is `#[ignore]`d as slow (540 s), and test_self_pure exercises the stage lanes, not
   compiler.warp compiled as one program. So batch-32/33's cloud runs were green.

## Why it took so long
3. No single owner for the first 1.5 h: two sessions bisected in parallel, neither the owner of the bug's family.
   The message itself ("a field read as a function") pointed at the local-named-like family at once.
4. Bisecting a Rust regression needs a build per point; workers can't build locally, so every point was an ~8 min
   cloud run, queued behind a full pool.
5. Revert-first was not the default, and a revert needs the user's OK (standing rule), which took one more round.

## Fixes (cards / rules)
- Gate with the batch's own build: done (Integrator, integrator_tools/pure_chain_check.sh).
- Card pure-hello-in-cloud: a cloud job that runs `warp pure hello` cold (the ignored test with --ignored at
  opt-level 2, or a lighter twin), so a pipeline break is red in the worker's own targeted run.
- Card keep-main-binaries: keep the last ~10 main release binaries (warp-<commit>) so a Rust regression is bisected
  locally in minutes by running old binaries, no build.
- Rule (supervisor.md "Pipeline breaks"): within 15 min of a red pure chain on main, the Supervisor names ONE owner
  — the session owning the area the error message points at, else the author of the newest suspect merge — and
  tells everyone else not to bisect.
- Rule revert-first (user, standing, 2026-10-21; notes/decisions.md, notes/agents/integrator.md): when main's
  `warp pure hello` breaks and the suspect merge is known, the Integrator reverts it at once without asking, names
  revert-first in the commit, tells the owner and the Supervisor; the owner re-lands it fixed.
