# Supervisor handover (2026-10-27, 18:00, before the user's reboot)

## Restart after the reboot
1. Supervisor first, in tmux warp-supervisor: `notes/agents/start.sh supervisor`, or resume this conversation:
   `FORCE_SPAWN=1 CLAUDE_RESUME=f2cd2fb2-3718-4833-b9b0-2b85ead3c858 ~/dev/bin/claude-remote.sh supervisor /Users/me/dev/angles/warp "<RESUME prompt>"`.
2. Services: `notes/agents/start.sh` (integrator, interviewer, fixer, reviewer), each reads its notes/handover/<role>.md.
3. Workers, each resumed or respawned from its handover (resume keeps the whole context):

| session | handover | resume id (from the transcript titles; check `claude --resume` if one fails) |
|---|---|---|
| warp-fixer | notes/handover/instance-words-build.md | ca3ff03f-a60e-48ac-8778-a23ecf360f7a |
| warp-text-field | notes/handover/core-types-emitter.md (branch emitter-kinds WIP) | 356f4529-9849-463e-b812-38049e490485 |
| warp-samples | notes/handover/sample-gaps.md (branch pure-paint WIP) | 3f9a2566-f302-4179-9600-522967854bab |
| warp-integrator | notes/handover/integrator.md | 042c9a17-9831-4855-8bc0-52a02d84416a |
| warp-interviewer | notes/handover/interviewer.md (9eccb1459a) | 43f62aa9-b30f-4ef2-a667-684c0816c38f |
| warp-reviewer | notes/handover/reviewer.md | d39cb067-4ff7-499c-b8e0-e2698d740b6c |

   Respawn instead of resume: `notes/agents/start.sh worker <name> "Resume from notes/handover/<file>.md"`.
4. The hourly watch round was a session cron (dies with the session): recreate it (CronCreate, minute 17, prompt in
   notes/decisions.md "night-watch"). The Codex timer is in the user's crontab and survives (02/07/12/17/22:35).

## Direction (user)
Performance, then compiler quality (types, classes, enums in core/), then features (sample gaps). Standing rules added
today, all in notes/agents/common.md: elegance first; plain warp first (a rewrite rule only for what a function cannot
say); vocabulary is library; samples are not the spec; tests are spec only when backed (notes/taste.md); new syntax of
any size needs an OK; the Supervisor records user decisions itself.

## State at handover
- Rebuild ~145 s → compile stage ~77 s CPU (TypedCode structs); `warp 3+3` 0.1 s (startup-time, kept-value-key).
- Classes are wasm GC structs with methods and operators (declared-class stages 1–2, class-operator-overloads);
  emitter kinds are ValueKind cases, TypedCode and lambdas tuples are classes; core `: any` params 933 → ~350.
- Units as values in lib/units.warp; `sleep(3ms)` without `use units` (expected-type-names part 1); π exact in
  lib/reals.warp; data blocks and `data` parameters; block parameters with shared captures. Samples 80/224.
- SEED HOLD: the instance-runtime fast paths landed (instance-words-build, 1e8dcd512c: stage B 86–111 s CPU, was
  2.6× slower), but the hold stays until warp-fixer's 131-file class test comparison on main is clean
  (notes/handover/instance-words-build.md); then warp-fixer lifts it.

## Waiting for the user
- Test audit page (60 constructs, keep/rewrite/drop): https://claude.ai/code/artifact/74a487d1-9874-4267-bfa1-ef8fd60f352e,
  choices readable with the Artifact tool's read_db (collection `choices`); notes/test_audit.md.
- Event-handler shape (parked; proposal: `on(key: Key, handler: block)` in lib, `on escape {…}` via expected type).
- Undeclared field write keeps the instance (extra fields) instead of converting to a map: confirm.
- data_blocks.warp as its own file vs folded into definitions.warp.
- Stray uncommitted src/extensions.rs (old Rust assert_throws) in the main checkout: the user reverts or keeps it.

## Queues
- warp-fixer: instance-runtime fast paths (lifts the seed hold) → method-name-clash → runtime-global-shadowing →
  run-time speed of compute samples.
- warp-text-field: class-text-coercion (results print via text(): 3.5km) → expected-type-names part 2 (fix-it).
- warp-samples: units-shadowing → sweep → biggest sample group except `on` and foreign spellings.
- Codex list: notes/agents/codex_cards.txt (library cards only).
