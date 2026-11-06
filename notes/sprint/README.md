# Card sprint: the lib cards (2026-11-06)

The triage (triage.md, triage.tsv) found 338 cards doable without core/. They are split into areas,
one local worker session each (tmux warp-sprint-<area>), card list in areas/<area>.tsv (key, topic, triage evidence).
misc.tsv (cards without a topic) is taken by whichever worker empties its own list first (tell the Supervisor).

## Rules (decisions.md, 2026-11-06 card sprint)
- Judge each card first (user): an esoteric feature (foreign syntax, desk-invented phrasing, bloat) or important to
  the language? Esoteric: don't build it, report `obsolete: <key> esoteric: <why>`. Unsure: ask warp-interviewer
  (cloud: mark the result line `question`) and take the next card meanwhile.
- Simple to complex (user): first sort your list by effort and clear every easy card before touching a bigger one;
  a card that turns out bigger than ~30 minutes goes to the end of your list.
- Work on branch sprint-<area> in the worktree /Volumes/Worktrees/sprint-<area> (`cowtree add`).
- Cards in a quick row, one quick commit each, the card key in the message (`fix(<key>): …`).
- No general tests when certain a change needs none (sample, tour example, note, wording). Anything that can change
  compiled behavior gets its smoke test before it reaches main.
- A card whose change affects core/ is NOT approached: report `core: <key> <reason>` and take the next card.
  An already done card: `done: <key> <commit or evidence>`; an obsolete one: `obsolete: <key> <reason>`.
- Workers never write the board. Report to the Supervisor in batches of a few lines `done|core|obsolete: <key> …`;
  it applies them with `todo batch`.
- lib/prelude.warp belongs to warp-sprint-spec alone; another area needing a prelude change sends it the hunk.
- Land often: merge origin/main into the branch, `scripts/smoke.sh <worktree>` on that merge, push to main.
  smoke.sh builds NOTHING for a lib-only change (it prebuilds just the changed modules with the installed compilers);
  it rebuilds the compiler only when the branch touches core/, extensions/ or a baked lib file, and says so. Never
  call `scripts/rebuild.sh pure` yourself in the sprint (user: the sprint's cards need no build, or only their library's). Feedback tests: `nice -n 19`,
  at most two folders a run, in the background, never a merge condition.
- Every push that changes lib/, extensions/, core/ or a sample is smoked on the very merge being pushed, even when
  the merged-in origin commits touch nothing: no landing script skips the smoke (2026-11-06: da1e8f51c2 went in
  unsmoked that way).
- Landing livelock (Supervisor, bent rule): when main moved during your smoke, you may push the smoked merge after a
  clean re-merge if main's new commits touch none of your branch's files; after-push builds and checks every main tip.
- Build slots are 4 for the whole Mac (rebuild.sh waits for one): skip the rebuild for a change that needs none.
- Questions for the user go to warp-interviewer, never to the user; never block on one: take the next card.
