# Supervisor handover (2026-11-09 ~22:00, resumed after the evening reboot)

Session c2654d8c-439c-46af-bd89-082515128840 (tmux warp-supervisor). Watch round: session cron at :17 (vitals
refresh + publish to https://claude.ai/code/artifact/b988ec61-123d-4e97-833b-0dd2342cac4c); recreate it after a respawn.

## Sessions (resume: `FORCE_SPAWN=1 CLAUDE_RESUME=<id> ~/dev/bin/claude-remote.sh <role> /Users/me/dev/angles/warp "<RESUME prompt>"`)
| session | id | state |
|---|---|---|
| warp-integrator | d3d6aa42-0e26-43b8-b1ed-04fee0eb1d67 | bisect C/D in /Volumes/Worktrees/bisect-cd; landings |
| warp-interviewer | f7e2bd29-0818-4d5b-83e6-a4ac98e5b1c2 | open questions (units-arithmetic-test, byte-walk, lib-module, play-wrong-arguments, x.type) |
| warp-dot-call-plan | 91a70b10-0d20-48dd-8d5d-7e3fb61eac36 | module-load-time (`use timers` costs 1.26 s), then 65 slow cases >5 s |
| warp-types-finish | 6cc49cc8-963c-45b1-a511-16dc7f422d0b | playground-only-reds: A (hidden_commands) landing, then F (libc/zlib), H |
| warp-parsing-finish | 64d7a348-0460-4c0b-8aad-7980aed332f5 | zoned-europe, then code-points-once; reflection-words waits on user |
| warp-sugar-program | a8acd579-d99f-49d0-9887-dafd2738262c | list-codepoint |
| warp-functions-finish | 0f68efa9-b05a-4ee6-8386-052e889beba1 | old-reds, time-arithmetic-move |
| warp-quick-fix | dc00e4d7-e4e1-4ded-80eb-4772efe7206d | no-core-build lane (title was warp-gcd-overflow-zero) |
| warp-baked-lib-split-3 | aa0f2d5d-2863-431d-8e52-3399a4e330ca | P3 bd1ff2f779 at integrator; builtins-seed-transition waits for seed 11-10 05:47 |

Not resumed (finished or paused): warp-core-shake, warp-data-tidy, warp-decisions-tidy, warp-notes-tidy,
warp-repo-clutter, warp-card-consolidate (finished); warp-events-plan edfa86ef-4e63-4be3-be6f-0b6658b1bdfe (paused
behind the tests roadmap, branch function-events).

## Waiting
- Cron seed 11-10 05:47 files an urgent release card: cut the next minor release on it (bump VERSION in
  extensions/commands/commands.warp).
- User: apply data/commit-guard.patch to ~/.claude/hooks; remove /Volumes/Worktrees/smoke-lib-shadow if wanted.
- Playground 90.3%: regressions C, D (bisect), E (maybe page-only) found via tests-web.
