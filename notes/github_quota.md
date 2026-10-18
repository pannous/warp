# GitHub API quota

The fleet uses about 10 % of the hourly GitHub quota (5000 GraphQL points, 5000 REST requests). Tools live in ~/dev/bin
(its own repo); logs in data/github-quota/.

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

## What burns quota

- `gh project item-list` asks for every field of every card: ~130 points per 100 cards; one whole-board dump costs
  more than an hour of normal fleet work. Hence the refusal below.
- Slow GraphQL pages (a big board, ~1-11 s per page of 100; GitHub cuts at 10 s with a 504) trip the **secondary**
  limit on GraphQL server time ("You have exceeded a secondary rate limit"), which locks every session out for
  minutes while the hourly points are fine.
- `gh run watch` polls every 3 s by default (~2 REST requests each).

## In place (~/dev/bin)

- gh-metered refuses `gh project item-list` (`GH_ALLOW_ITEM_LIST=1` overrides) and points to `todo list [--json]
  [column]`; it adds `--interval 60` to a `gh run watch` that has none.
- todo: whole-board listings leave out descriptions (fetched per card when needed) and share one snapshot for 60 s
  machine-wide (~/.cache/todo-board/warp.snapshot.json, dropped by any mutation from here). `todo list <column>`,
  `todo try`, `todo recur` and crash folding ask the server for just their cards. `-status:Done` timed out; positive
  `status:Now,Next,Soon,Later` is quick. Queries (and idempotent mutations) retry a 5xx after 5 and 20 s.
- Board size: Done cards older than 3 days (by the time their Status changed) leave the board,
  repeating cards excepted. `todo archive` (cron 04:30) records each card (fields, description, try line) in the
  wiki repo, cards-archive/<month>.json5 (pushed from a clone in ~/.cache/todo-board/warp.wiki), closes its issue
  and archives the item. `todo restore <key>` unarchives it, or re-adds it from the record. `todo list Archived`
  lists the records. New cards (`todo add`) are issues of the repo, not drafts; existing drafts stay drafts.

Open: the server's `status:Done` filter times out (archived items may still be scanned; deleting them instead of
archiving is the next step if it matters); `key:<key>` lookups sometimes get a 502 and are retried.

## GitLab?

Would not help: the burn came from two habits (whole-board dumps, a board 90 % finished cards), both fixed; a move
would cost Actions CI, the Pages deploy, the wiki, the gh-based tooling, the Grok Bot app and the cloud sessions'
GitHub integration, and GitLab's GraphQL has complexity limits of its own.
