# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- postfix-words (user wants a discussion): `xs sorted`, `x squared`. Options: a typed sugar rule `$x $f => $f($x) where $f takes …` (typed rule conditions exist: core/extensions/sugar/broadcasting.warp), or the original idea, an -ed/-d verb suffix fallback (`sorted` → sort when no `sorted` exists). Default: not a feature.
- true-threads (user asked): `go` tasks are cooperative today (N29); decision P33 says real threads (wasm threads natively, Web Workers in the page). How to tell threads from cooperative tasks?

## To confirm (decided on the user's behalf; built as described, keep or undo)

## Parked
- class-text (sound notes), unit plurals, def-first (b), note tests: discuss later (user).
- event-handlers (prose discussion with the user, no popups; block-parameters-captures landed): proposed `on(event: Event, body: block)` in the prelude, `emit(event, data)` runs the listeners, no handler core. Open: (1) event data as `it`, a chosen name (`on key k {…}`) or both; (2) user events declared (`enum Event += ask`) or any symbol; (3) decided: on-frame-headless. Branch event-handlers-unknown (a4d0dde771) only a reference.
- graphics-plain (warp-markup, parked): display alias table with notes; bare width/height as canvas sides with did-you-mean.
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
