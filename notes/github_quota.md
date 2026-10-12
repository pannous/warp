# GitHub API quota (card github-quota, 2026-10-12)

The user: "5000 requests should be plenty, but somehow we're burning through them all the time" (and considered
GitLab). Measured, then fixed at the source. Tools live in ~/dev/bin (its own repo); logs in data/github-quota/.

## Measuring

- `~/bin/gh` → `~/dev/bin/gh-metered`: wraps every gh call (ahead of /opt/homebrew/bin in interactive shells; `todo`
  calls it directly, also from cron). It runs gh with `GH_DEBUG=api` and logs to data/github-quota/calls.tsv: time,
  `program@session` (GH_METER_CALLER, tmux session), command, exit, seconds, HTTP requests, per quota bucket the
  requests and first/last `X-Ratelimit-Used`, non-2xx statuses, and 5xx / secondary / primary limit hits. No extra
  request. `GH_METER=off` turns logging off.
- `~/dev/bin/gh-quota-sampler [minutes]`: total use once a minute (data/github-quota/rate_limit.tsv).
- `~/dev/bin/gh-quota-report [since] [until] [--exclude caller]`: top consumers.
- `~/dev/bin/probes/board_query_cost.py [filter|-] [page_size] [body]`: points and seconds per board page.

Pitfalls found while measuring:
- REST `/rate_limit` reports GraphQL use hundreds of points behind; GraphQL's own `rateLimit { used }` (free) and the
  `X-Ratelimit-*` headers are right.
- GitHub keeps several counters per resource (same token, different `X-Ratelimit-Reset` and `Used`; e.g.
  `/actions/runs` and `/actions/workflows` in one `gh run list`). A reading is only comparable with one of the same
  reset time.

## Before (14:05-14:40, normal fleet activity, ~10 sessions)

~2200 GraphQL points in 30 min: ~4400/h, so the 5000/h ran out regularly. REST (core) was only ~300/30 min.

| GraphQL points | caller |
|---|---|
| 531 | warp-markup: one `gh project item-list --limit 400` |
| 207 | warp-dispatch: one `gh project item-list --limit 1000` (died with 502) |
| ~170 | `todo list` / `todo key` (whole board: 13 pages of 100 cards with descriptions) |
| 30 | cron urgent-card-watch (`todo keys` + `todo urgent`, filtered: 1 point each) |
| ~600 | this card's own probes and the archive run |

- `gh project item-list` asks for every field of every card: ~130 points per 100 cards. Two calls = a third of all.
- The 502/504s: the board had 1319 cards (1180 Done; `todo archive` had never worked, its cron ran unauthenticated).
  A page of 100 took 1-11 s, and GitHub cuts at 10 s (504). A few slow pages in a row trip the **secondary** limit
  on GraphQL server time ("You have exceeded a secondary rate limit"), which locks every session out for minutes
  while the hourly points are fine. That was the 504s at REST 5000/5000, GraphQL 4953/5000.
- `gh run watch` polls every 3 s by default (~2 REST requests each).

## Fixes (~/dev/bin)

- gh-metered refuses `gh project item-list` (`GH_ALLOW_ITEM_LIST=1` overrides) and points to `todo list [--json]
  [column]`; it adds `--interval 60` to a `gh run watch` that has none.
- todo: whole-board listings leave out descriptions (fetched per card when needed) and share one snapshot for 60 s
  machine-wide (~/.cache/todo-board/warp.snapshot.json, dropped by any mutation from here). `todo list <column>`,
  `todo try`, `todo recur` and crash folding ask the server for just their cards. `-status:Done` timed out; positive
  `status:Now,Next,Soon,Later` is quick. Queries (and idempotent mutations) retry a 5xx after 5 and 20 s.
- Board shrinks (user decision): Done cards older than 3 days (by the time their Status changed) leave the board,
  repeating cards excepted. `todo archive` (cron 04:30) records each card (fields, description, try line) in the
  wiki repo, cards-archive/<month>.json5 (pushed from a clone in ~/.cache/todo-board/warp.wiki), closes its issue
  and archives the item. `todo restore <key>` unarchives it, or re-adds it from the record. `todo list Archived`
  lists the records. New cards (`todo add`) are issues of the repo, not drafts; existing drafts stay drafts.
  First run: ~770 cards archived, the board went from 1319 to 553 cards (the other ~425 Done cards are under 3 days
  old). A whole-board listing with descriptions now takes ~7 s for 6 pages (was 31 s and a 504 at page 9).

## After (14:56-15:26, same fleet)

~250 GraphQL points in 30 min (~500/h, was ~4400/h), REST ~90, 3 retried 502s, no secondary-limit hit. Top: the
Integrator's `gh run view` polling (74 REST), the Supervisor's `todo key` (31), cron (30). New cards are issues
(#380, #381 were the first).

Still open: the server's `status:Done` filter times out even with 430 Done cards (archived items may still be
scanned; deleting them instead of archiving is the next step if it matters); `key:<key>` lookups (`todo key`,
`todo add`) sometimes get a 502 and are retried. ETags were not needed: REST use is ~200/h of 5000.

## Would GitLab help?

Hardly. Its API allows far more (about 2000 requests a minute per user), and issue boards there are views over
issues, with no separate project items to page through. But our problem wasn't GitHub's quota: two habits made it
(`gh project item-list` for whole-board dumps, and a 1319-card board where 90 % of the cards were finished), and
both are fixed. After the fixes we use ~10 % of the hourly quota. A move would cost Actions CI and the Pages deploy,
the wiki, the gh-based tooling (todo, codex_worker, watchers), the Grok Bot app and the Claude/GitHub integration of
the cloud sessions. GitLab's GraphQL has complexity limits and timeouts of its own, so the same dump habits would
hurt there too.
