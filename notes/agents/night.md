# Night rules (user 2026-11-02)

How the fleet works while the user sleeps, until the user is back (usually 10:00). Read with common.md and your role file.

## Decisions
- The Supervisor and the Interviewer decide themselves: meaning, spellings, core OKs, process. Courageous, not
  cautious; the user reviews in the morning and keeps or undoes.
- No questions to the user and no waiting on answers: the Interviewer asks nobody, workers never block on a decision.
- Every decision goes into the night log notes/night_<date>.md, section "Decisions (keep or undo at 10:00)": what, why,
  who builds it, how to undo. Append only; never rewrite another session's lines.
- What truly needs the user (a test change, a destructive or outward step, money) goes into the night log's "Open
  questions for the morning", each with a recommendation, and is asked once at 10:00.

## Build slots
- As many local build slots as the machine carries: the Supervisor sets <store>/build-slots/count
  (/Volumes/Build/compilers/build-slots/count; scripts/build-slot.sh re-reads it on every try) and removes it in the morning,
  back to the day's 3.
- Watch the load each round (`uptime`, `sysctl -n hw.ncpu`): a build uses one core, so raise while the 1-minute load stays
  below the core count, lower at once when it passes 1.5× the cores or a session stalls. Never above what the load allows
  (5 slots once drove the load to ~460 under a game).

## Cloud
- Use the cloud more at night than by day: cloud workers build and test in their own boxes, beside the Mac's slots, so
  they add throughput where the Mac's build line is the bottleneck. Fast mode where the agent's own work is the limit.
- Cloud workers take lib/-only cards that need no page or Mac host words; core and page work stay on the Mac.
- The full suite runs in the cloud routine (test/results-cloud.txt).
- No Rust in the cloud (user 2026-11-02): a cloud box needs no cargo or rustc. It takes the latest release's
  warp-v*-<arch>-unknown-linux-gnu.tar.gz (warp with its wasmtime library) and the seed seed.lock names, then
  `WARP_LIB=<checkout>/lib warp rebuild`. gh in a cloud session: REST only (`gh api repos/pannous/warp/...`, assets via
  `Accept: application/octet-stream`); GraphQL, and with it `gh release download` and the `todo` board CLI, is refused.

## Morning
- At 10:00 the Supervisor gives the user one summary: the numbers then and now, what landed, the decisions to review,
  the open questions. Then the slot count goes back to the day's.
