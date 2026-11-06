# Card sprint: the lib cards (2026-11-06)

The triage (triage.md, triage.tsv) found 338 cards doable without core/. They are split into areas,
one local worker session each (tmux warp-sprint-<area>), card list in areas/<area>.tsv (key, topic, triage evidence).
misc.tsv (cards without a topic) is taken by whichever worker empties its own list first (tell the Supervisor).

## Rules (decisions.md, 2026-11-06 card sprint)
- Work on branch sprint-<area> in the worktree /Volumes/Worktrees/sprint-<area> (`cowtree add`).
- Cards in a quick row, one quick commit each, the card key in the message (`fix(<key>): …`).
- No general tests when certain a change needs none (sample, tour example, note, wording). Anything that can change
  compiled behavior gets its smoke test before it reaches main.
- A card whose change affects core/ is NOT approached: report `core: <key> <reason>` and take the next card.
  An already done card: `done: <key> <commit or evidence>`; an obsolete one: `obsolete: <key> <reason>`.
- Workers never write the board. Report to the Supervisor in batches of a few lines `done|core|obsolete: <key> …`;
  it applies them with `todo batch`.
- lib/prelude.warp belongs to warp-sprint-spec alone; another area needing a prelude change sends it the hunk.
- Land often: merge origin/main into the branch, smoke test on that merge (scripts/rebuild.sh pure <worktree>, then
  WARP_LIB=<worktree>/lib warp samples/hello/hello.warp prints Hello), push to main. Feedback tests: `nice -n 19`,
  at most two folders a run, in the background, never a merge condition.
- Build slots are 4 for the whole Mac (rebuild.sh waits for one): skip the rebuild for a change that needs none.
- Questions for the user go to warp-interviewer, never to the user; never block on one: take the next card.
