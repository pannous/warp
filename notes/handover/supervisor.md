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
| warp-speed | compile slowdown, TOP PRIORITY (decision compile-speed-first): self-compile profile; building lib-hash-rebake (user OK) |
| warp-quality (was warp-rule-words-removal) | use-os → test-modules → hotspots from warp-speed (programs-read landed 70a9bfa89a) |
| warp-shaders | angle-type-alias → units-ignored-by-shaders → use-keys → ocean m/s todos (ocean landed fc2ed2681c; dot-product is Codex's) |
| warp-integrator | full suites held tonight for warp-speed's measurements; seed refresh after lib-hash-rebake lands |
| Codex (cron 02/07/12/17/22:35) | notes/agents/codex_worker.sh, 4 rounds per firing; typed-parse and dot-product parked on their branches |

Retired tonight: warp-fixer, warp-wgsl, warp-exact-decimal-big, warp-cards, warp-diff-core, warp-build-failed-store
(landed 06d61d3be1), warp-interviewer (archived by the user).

## Waiting
- codex-cron: 4 rounds per firing now; check data/agents/codex/*.last.md of the 23:5x run and the 02:35 run.
- Promise: tell the user what warp-speed's profile finds and when lib-hash-rebake lands.
