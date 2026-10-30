# Board triage 2026-10-30

Session warp-card-triage (user's task, 2026-10-30): every open card of the board (Now/Next/Soon/Later, 1123 cards,
Done and Roadmaps untouched) read once and checked for being still open and still relevant. The per-card log
(card, from, verdict, commit, reason) is data/cards/triage.md in the main checkout (ignored), with the board
snapshot it worked from (board.json, issues.json).

## How
- One snapshot: `todo list --json` plus `gh issue list --json …,body` (two calls), joined by issue id.
- Skipped: Now cards with an Agent, cards whose key names a live branch or worktree (16), topic cards with sub-cards (37).
- Still open was checked by running the card's code on the installed native warp (main 07f8c18ee and f971d85e33,
  `WARP_NO_WINDOW=1 ~/dev/bin/warp '<code>'`): every backticked snippet of every card (281 snippets, 177 cards), plus
  ~300 one-liners written from the card text.
- Board changes went through `todo note` + `todo done/move` in paced batches (20 cards, then 45 s; the GraphQL
  budget is barely touched: a key lookup costs ~1 point).

## Result
| verdict | cards | commit |
|---|---|---|
| closed: the one-liner now gives the expected value | 171 | f971d85e33 (main) |
| closed: Rust compiler port/twin/lane/suite card | 81 | 475d65f8c7 (Rust compiler deleted) |
| closed: Rust harness card (warp-host, crates, Rust files) | 24 | d115331edf (harness deleted) |
| closed: otherwise obsolete (main builds again, self-hosting, core/ move, probes/ gone, no Ready column, test/launcher) | 11 | named per card |
| moved to Later: esoteric foreign spellings, parked ideas | 15 | |
| upgraded to Next: wrong values / refusals of everyday forms | 10 | |

Upgraded to Next: bool-and-field-int (a bool field written from a comparison stores 0), pure-lane-position
(`xs[xs.index_of(20)]` gives the next item: index_of counts from 1, `[]` from 0), emitter-refuses-reduce
(`xs.reduce(…)` refused), emitter-refuses-block (`xs.map { it * 2 }` refused), interpolated-number-literal
(`"${12345}"` → not a list), interpolated-count-fail (`"${#xs}"` refused), format-missing (`warp format` cannot read
hello.warp, so the Done step `warp format` is broken), hour-minute-equality (`x == 120 minutes` gives `no 1min`),
enum-case (`O.W(3)` gives `W{n: 0}`), pure-type-values (`x is (int or text)` gives no for 3).

## What stays open (seen while testing)
- Many "pure lane" cards were plain warp bugs that the switch carried over; most of them are fixed, the rest stay
  with their cards (e.g. `for byte in "äb"` walks characters, `5.byte_size`, enum parameter accepts 5, setters ignored).
- Common refusals still on main: `if(c, a, b)`, `super.m()`, `static` members, `record`, brace definitions
  `g(n) { … }`, implicit `it` outside a block, `xs[end]`, `x is a number`, `±` without `use uncertain`, units only
  after `use units` (`2 s` works, `200ms` in a literal does not).
- Wiki-promised Later cards (operator word aliases, ordinal words, mutation phrases, purity predicates …) were
  re-tested and stay in Later: none of them works yet.
- Cards closed as Rust ports may hide a feature gap (shared arrays, parallel_map, live_loop, assert as lib words):
  they were judged port cards; a real gap gets a fresh warp card when a program needs it.
- `use cargo <crate>` is silently accepted and does nothing since the harness went (carded).
