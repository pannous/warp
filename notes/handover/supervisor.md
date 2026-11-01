# Supervisor handover (2026-10-30 23:50)

The only Supervisor is warp-supervisor (tmux warp-supervisor; this transcript: 19cc9aeb-0f11-4d78-a3a9-a71850ba82fa).
It took over from the old warp-supervisor session (99331, gone) at 22:23 on 2026-10-30. Respawn:
`notes/agents/start.sh supervisor`, then rename to warp-supervisor (crontab jobs and claude-relay.sh send there).
The :17 watch round is a session cron (CronCreate, prompt in notes/decisions.md "night-watch"): recreate it after a
respawn.

## Night rule (user 2026-10-30, the Interviewer is archived)
During the night the Supervisor makes smart decisions itself, and since everything is undoable it decides eagerly.
Only when really unsure: write the question into notes/night_2026-10-30.md "Open questions" and keep going. Every
decision made tonight goes into notes/night_2026-10-30.md "Decisions", so the user can check them in the morning.
Language-meaning questions that would go to the Interviewer go there too.

## Sessions
| session | card / task |
|---|---|
| warp-speed | speed work stopped by the user at 166 s (820 at the start of 10-31); full-suite regressions, groups down to "3 × ø" (notes/full_suite_2026-10-31.md) |
| warp-quality | full-suite regressions, groups from "2 × T{n: 'x'}" on (emit-wasmtime landed 5044e2a7af) |
| warp-shaders | use-sound (128 red suite lines; field vs outer word of the same name, Codex repro on branch datetime-arithmetic-fail) |
| warp-units | units failing tests (lib/), scripts-self; units-rewrite-assign (core one-liner) waits for the user |
| warp-operators (spawned 15:5x, tmux warp-operators) | rewrite-rules + union-later-member: rules whose meaning depends on operand kinds become typed operators, carefully, one at a time against a test baseline |
| warp-integrator | seed e02b1cff (rebuild 446 → 225 s), full suite 302b5481cd: 3488/11643 failing |
| Codex (cron 02/07/12/17/22:35) | notes/agents/codex_cards.txt; `retired:` lines are never continued (781e839056) |

Retired tonight: warp-fixer, warp-wgsl, warp-exact-decimal-big, warp-cards, warp-diff-core, warp-build-failed-store
(landed 06d61d3be1), warp-interviewer (archived by the user).

## Waiting
- codex-cron: 4 rounds per firing now; check data/agents/codex/*.last.md of the 23:5x run and the 02:35 run.
- Promise: tell the user what warp-speed's profile finds and when lib-hash-rebake lands.
