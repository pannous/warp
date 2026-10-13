# Docs sync sweep

Keeps the wiki (wiki/, its own repo, pushed to master) and the playground guides (web/playground/guide.md,
guide-expert.md) in line with main. Card docs-sync, session warp-docs-sync.

## Last swept

- main commit **264c844a4** (2026-10-13; sweeps docs-sync 19f021863, docs-sync-2 48e5f3b27, docs-sync-3 3f01a83b5, keeper 264c844a4), checked with
  /Users/me/dev/bin/warp built from 19f021863 (no user-visible commits in between).
- notes/decisions.md read through its end at that commit.

The next sweep starts with `git log --oneline 264c844a4..origin/main | grep -E 'feature|fix:'` and the decisions added
since (`git diff 264c844a4 origin/main -- notes/decisions.md`).

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

## Not swept yet

Everything up to the last swept commit is covered; the open gaps are cards (midi-receive-message, types-first-run, untyped-block-parameter).
