# warp-integrator handover (parked 2026-11-07, user PAUSE)

Role: notes/agents/integrator.md + common.md. Supervisor: warp-supervisor (the session on this Mac).

## State at park
- No job of mine running. The 15:00 cron full suite (`scripts/full-suite.sh --commit`, pids 22370/22837/23787,
  ~2 h in) was stopped by PID for the pause; its partial output is unused. Next full suite: the cron's next slot.
- Last committed full suite: 7fb83277a2 (1559 of 14839 failed, 89.5%, at a331690053). Its 146 newly red lines were
  carded: test-text, lines-red (landed 918881accf), field-side, unknown-name, named-patterns-test (unowned).
- Known reds on main with owners, not to bisect: gate tier 1 brackets (warp-sprint-web, try-linked),
  shown_result (warp-playground), refused_call (warp-sprint-web).
- Vitals v103 published (829f47b3f8: hello track in data/vitals/hello.jsonl + Open cards curve). Republish after
  each install; a refusal by a newer version from the same vitals.py is resolved by diffing the template and
  republishing (only data differs).
- after-push page smoke (9a216fd91c) was RED on the canonical_text host import; check after-push.log that it went
  green after text-collections' fix (card after-push-page-smoke-1107).

## Pending
- The user's branch cleanup: `bash ~/dev/angles/warp/data/delete-merged-branches.sh` (theirs to run).
- Seed refreshes only on request, default stages A B C.
