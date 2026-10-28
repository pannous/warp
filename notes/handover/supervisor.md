# Supervisor handover (2026-10-28 13:10)

The only Supervisor is warp-supervisor (transcript 94499e47-894d-4ba0-8da9-afb7c361ea5e); the old "Supervisor"
session (b72bb4ea…) retired by user decision. Respawn: `notes/agents/start.sh supervisor`. The :17 watch round is a
session cron (CronCreate, prompt in notes/decisions.md "night-watch"): recreate it after a respawn.

## Sessions
| session | card / queue |
|---|---|
| warp-fixer | seed-compiler-runtime FIRST (seed hold until it lands), then takes-ms, sugar-rule-passes |
| warp-tables-as-constants | modules-prebuilt: module command (branch 056f91cf09), harness OK given (plain `warp module`, rebuild.sh publishes module .wasm, list from the compiler's bundled list) |
| warp-separate-modules | pure-call-function-values (a lambda/call result as callee is applied) |
| warp-native-harness | native-harness milestone 1 (notes/roadmap_native_harness.md): fib native works on the branch; hooks OK'd, `@primitive` through the attribute layer (decision primitive-hooks); next class/struct layout |
| warp-integrator | tier 3 for b8c5b794ec and 571df8e60a; releases-homebrew (ship runtime.wasm) |
| warp-interviewer, warp-reviewer | services |
| warp-8e | the user's own session, leave it |

## State
- Full suite at b2a7483738: 6823 of 11139 pass (61 %, 44 % on 2026-10-22); 3411 of the failures are "outside the
  core". test/results.txt is the ✓/✗ file, rewritten and committed by each nightly full run (integrator.md).
- Installed ~/dev/bin/warp follows main (after-push.sh, cron every 2 min).
- User decisions today: notes/decisions.md (native-harness, primitive-annotation, capitalized-field-list,
  test-results-file, drop-pure-word …).
