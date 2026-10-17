# Docs sync sweep

Keeps the wiki (wiki/, its own repo, pushed to master) and the playground guides (web/playground/guide.md,
guide-expert.md) in line with main. Card docs-sync, session warp-docs-sync.

## Last swept

main commit **782cc3878**, checked with /Users/me/dev/bin/warp; notes/decisions.md read through its end at that commit.
The next sweep starts with `git log --oneline 782cc3878..origin/main | grep -E 'feature|fix:'` and the decisions added
since (`git diff 782cc3878 origin/main -- notes/decisions.md`). Update the commit here after each sweep.

## How

1. List what changed for users: decisions.md entries and feature/fix commits since the last sweep.
2. Find the wiki page for each (`grep -ril <word> wiki/`); a topic without a page gets one plus a _Sidebar.md entry
   (links are `[[Text|page]]`).
3. Write the examples down as `code ⇒ value` or `code ⇒ !error part`, one per line, in probes/docs_sync/<topic>.txt,
   and run `probes/docs_sync/run_examples.sh probes/docs_sync/<topic>.txt` (WARP=<binary> for another build).
4. A failing example is a card (`todo add`), never dropped: either the page shows a working form and names the card,
   or the example goes into a ```` ```warp planned ```` fence with a *Planned:* sentence (notes/docs_planned.md).
5. Guides: their fences run in tests/web/test_guide.rs and test_expert_guide.rs (`tests/queue.sh -- web::test_guide`,
   `web::test_expert_guide`); an "Examples:" line names only existing tour examples or samples.
6. Style: an intro phrase, a minimal snippet, the explanation; one concept at a time.

## Standing keeper

warp-docs-sync stays on: when idle and main has user-visible commits past the last swept one, sweep them as above.
Merge requests' "docs:" lines come from warp-integrator; a "docs: none" on a user-visible feature goes back to its
worker. Samples should run without hints: `for f in samples/*.warp; …grep -c '^hint'` (list in
data/docs_sync/sample_hints.txt, card sample-hints).

## Full wiki audit

Everything the docs mention that is not implemented gets a card, named on the page; conflicts and major corrections
go to warp-interviewer, tiny fixes are made directly.
- `probes/docs_sync/wiki_fences.py` runs every warp/wasp/untagged fence (data/docs_sync/fences.tsv); a fence that
  fails alone is retried after the page's earlier fences. Sketches, other languages and error demos are ```` ```text ````
  (or cpp, swift); `--show page.md:line` prints one fence with its run.
- A run with page arguments overwrites data/docs_sync/fences.tsv with only those pages: copy the full run first
  (`cp data/docs_sync/fences.tsv data/docs_sync/fences.full.tsv`) to diff a later full run against it.
- `--values` runs each `code // value` line and lists mismatches (data/docs_sync/values.tsv); most hits are prose
  comments, not values.
- `grep -n "not yet" wiki/*.md | grep -v "card "` lists the unimplemented notes without a card.

## Pending doc changes after landed cuts

Remove each dropped form or rewrite it to its replacement, with no card (a decision, not a gap):
- spaced type heads `fib int i`, `foo of int`, `combine number with number` → `fib(i:int) := …`: wiki/=.md:11,
  Bad.md:51, type.md:59 still show them. `square of a number = …` stays. Recheck Features.md:60.
- OCaml let-in and `fun`, Elixir `def …, do:`, C++ `[](…){…}`, R/PHP forms (to `use R` / `use PHP`): grep the wiki
  for examples.
- ORM shows only `stored people: [Person]`.

Unimplemented spec features are sub-cards of the topic card spec-gaps (Later); documented-but-failing bugs stay in Next.
