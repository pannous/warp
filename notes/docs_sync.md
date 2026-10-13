# Docs sync sweep

Keeps the wiki (wiki/, its own repo, pushed to master) and the playground guides (web/playground/guide.md,
guide-expert.md) in line with main. Card docs-sync, session warp-docs-sync.

## Last swept

- main commit **19f021863** (2026-10-13), checked with /Users/me/dev/bin/warp built from it.
- notes/decisions.md read through its end at that commit.

The next sweep starts with `git log --oneline 19f021863..origin/main | grep -E 'feature|fix:'` and the decisions added
since (`git diff 19f021863 origin/main -- notes/decisions.md`).

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

## Not swept yet (at 19f021863)

sound (voice pacing, fade_sound, play_signal/play_blocks, stereo, `use sound` for rare words, patterns and live
loops, MIDI), generic functions and multiple dispatch, reflection (`.signature`, `.params`, `dir`, `help`), dates and
time zones at run time, units (°, temperatures), python aliases (pop, clear, fromkeys, hex(…) casts), forms
(`on submit`, event.fields), stored values (`stored x default v`, store words as maps), signals in object fields,
table slices, `go xs.map`, runtime error positions, the bit_and and `type(x) == "int"` hints, let-mut, float exit
status, W0 (prover pages), crontab/exec, `warp app`.

Clash to settle: the planned `pattern Adult = {…}` keyword (card patterns-values) meets the sound module's `pattern`
word ("pattern is in the standard module sound").
