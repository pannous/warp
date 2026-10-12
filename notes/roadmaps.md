# Roadmaps: one plan note, one topic card, its sub-cards

A big goal (a web framework, a type system, sound) has three parts that point at each other:

1. **The plan note** `notes/<goal>.md`: why, the order of the steps and what each builds on, the decided defaults,
   and per step "what is done and what is left". Its first line under the title names the topic card:
   `Roadmap card: <topic-key> (board, topic n/m done)`.
2. **The topic card** on the board (key = the goal, e.g. `web-framework`): one line of goal, the body links the plan
   note. It stays in the column of the goal's priority (Soon, Later…) until every sub-card is Done.
3. **The sub-cards**: one per step of the note's table and one per open "what is left" item, made with
   `todo add "…" --topic <topic-key>` or attached later with `todo sub <topic-key> <card>…`. The note's step table
   uses the same card keys, so a reader can go note → card → commit and back.

Rules
- Status lives on the board, not in the note: `todo list` shows "(topic n/m done)"; the note keeps the plan and what
  each step left behind, not checkboxes.
- When a step finishes with leftovers, the leftovers become new sub-cards of the same topic (not prose only in the
  note). A topic whose sub-cards are all Done but whose note still lists open items is not done.
- A new plan note gets its topic card in the same commit; a worker that finds a plan note without one creates it.
- Roadmap of the whole project = `todo list` filtered to topic cards; no separate roadmap file to keep in sync.

Applied first to notes/web_framework.md (topic card web-framework, 2026-10-12).
