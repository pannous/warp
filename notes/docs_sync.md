# Docs sync sweep

Keeps the wiki (wiki/, its own repo, pushed to master) and the playground guides (web/playground/guide.md,
guide-expert.md) in line with main. Card docs-sync, session warp-docs-sync.

## Last swept

- main commit **d19060ec9** (2026-10-13; sweeps docs-sync 19f021863, docs-sync-2 48e5f3b27, docs-sync-3 3f01a83b5, keeper 264c844a4, 17b6b455b, 0c2b20013, c1f479e10 (the src regrouping) and d19060ec9), checked with
  /Users/me/dev/bin/warp (19f021863) and, for 17b6b455b and 0c2b20013, a debug build copied to scratch/docs_sync/warp.
- notes/decisions.md read through its end at that commit.

The next sweep starts with `git log --oneline d19060ec9..origin/main | grep -E 'feature|fix:'` and the decisions added
since (`git diff d19060ec9 origin/main -- notes/decisions.md`).

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

User, 2026-10-13: everything the docs mention that is not implemented gets a card, named on the page; conflicts and
major corrections go to warp-interviewer, tiny fixes are made directly.
- `probes/docs_sync/wiki_fences.py` runs every warp/wasp/untagged fence (data/docs_sync/fences.tsv); a fence that
  fails alone is retried after the page's earlier fences. Sketches, other languages and error demos are ```` ```text ````
  (or cpp, swift); `--show page.md:line` prints one fence with its run.
- `--values` runs each `code // value` line and lists mismatches (data/docs_sync/values.tsv); most hits are prose
  comments, not values.
- `grep -n "not yet" wiki/*.md | grep -v "card "` lists the unimplemented notes without a card; the audit
  (2026-10-13, wiki 0b4b33f and 4b2a2db) left none, apart from Footguns' detail limitations and hyperreals' prose.
- Questions sent to warp-interviewer: «» quotes or group, bool.md numbers or bool type, constant binding or value,
  `reduce` decrement or fold, truthiness `== false` (data/docs_sync/audit_questions.md).

## Not swept yet

Everything up to the last swept commit is covered. Unimplemented spec features are sub-cards of the topic card spec-gaps (Later, supervisor 2026-10-13: not Next, so workers keep to the plan); documented-but-failing bugs stay in Next.
