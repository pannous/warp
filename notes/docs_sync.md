# Docs sync sweep

Keeps the wiki (wiki/, its own repo, pushed to master) and the playground guides (web/playground/guide.md,
guide-expert.md) in line with main. Card docs-sync, session warp-docs-sync.

## Last swept

- main commit **1098ee155** (2026-10-14; sweeps docs-sync 19f021863, docs-sync-2 48e5f3b27, docs-sync-3 3f01a83b5, keeper 264c844a4, 17b6b455b, 0c2b20013, c1f479e10 (the src regrouping), d19060ec9, 9d84077f6, 6915e892b, 0bcbc6caf, 3801056ee and batch 2eae7fde9 (wiki eb3c384), batch de70c437b, batches 5899b83d1, c65c6a241, 9e6fbf8db, f689a6665, 63ad6eb73, 66dd216a9 (units refactor: no page shows unit tolerances or ranges, nothing to change) 43384c15e, 3be3d7d25 (internal) and eb64602c1 (cargo debug builds already in wiki/cargo.md; C types and Node accessors internal) bcb3f966b (wiki node.md: built lists == written) 2518ccd57 (fix, internal) 3f60ecd4d (wiki cli.md: warp code argument) de4b71071 (wiki function.md: hole fix error) c9c928e54 (matching.md to-get fence runs, routes already quoted) 967d8ce4a (wiki sound.md: fade duration) 9c68b36af (wiki W0.md: first types run) a4f4a44dc (wiki pattern-matching.md: two-key map) 525f1bd69 (units interpreter cut, no page change) ea1abd55c (generators and traits cuts) and 1098ee155 (scoped handlers cut)), checked with
  /Users/me/dev/bin/warp (19f021863) and, for 17b6b455b and 0c2b20013, a debug build copied to scratch/docs_sync/warp.
- notes/decisions.md read through its end at that commit.

The next sweep starts with `git log --oneline 1098ee155..origin/main | grep -E 'feature|fix:'` and the decisions added
since (`git diff 1098ee155 origin/main -- notes/decisions.md`; decisions.md is grouped by topic since 13d194eec).

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

## Pending cuts (decision cleanup-cuts, foreign-spellings, 2026-10-13)

Decided, not cut yet; the features still work, so the pages stay until the cut lands (supervisor 2026-10-14: the big
cuts land about 10:00, start when warp-integrator reports them), then remove each dropped form or rewrite it to its
replacement, with no card (a user decision, not a gap):
- scoped handlers: cut at 1098ee155; wiki/effects.md's handler section now shows the program-wide `on` (fence
  runs: 120); no other page or guide used `on … in {…}`. Done.
- spaced type heads (lands with the cleanup-park-phrases cut; cleanup-drop-type-name-params was withdrawn) `fib int i`, `foo of int`, `square of a number`,
  `combine number with number` → `fib(i:int) := …`: wiki/=.md:11, Bad.md:51, type.md:59, function.md:5 (list of
  natural forms), inventions.md:42/96/137, Purpose.md:455, Features.md:60 (`++ of a number reference`).
  polymorphism.md was rewritten ahead of the cut (wiki 5ec7480, the written form already works).
- generators/yield: cut at ea1abd55c; wiki 'generators' left trait.md's Iterable rows, keyword.md's yield row shows the
  kept Ruby block form (`def twice; yield 1; yield 2; end; twice { |x| print x }`). Done.
- traits alias: cut at ea1abd55c, no user-visible change (trait.md fences run, `interface` hints toward trait). Done.
- compile-time lambdas, natural phrases (parked), OCaml let-in and `fun`, Elixir `def …, do:`, C++ `[](…){…}`,
  R/PHP forms (to `use R` / `use PHP`): grep the wiki for examples when each lands.
- units interpreter: cut at 525f1bd69 (cleanup-units-interpreter); unit.md, int.md, number.md, overflow.md fences
  run on a fresh build, no page shows a unit range or a tolerance in a unit (notes.md:11 "7000-6000 BC" is a loose
  idea). Done.
- reflection dot words → `a of b` ≡ `b.a`; ORM only `stored people: [Person]`.

## Not swept yet

Everything up to the last swept commit is covered. Unimplemented spec features are sub-cards of the topic card spec-gaps (Later, supervisor 2026-10-13: not Next, so workers keep to the plan); documented-but-failing bugs stay in Next.
