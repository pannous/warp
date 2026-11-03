# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending

## To confirm (decided on the user's behalf; built as described, keep or undo)
- extension-passes: Extension rows may insert a pass after a named core pass, so multi-clause functions leave the core (core 29249 → 29121 lines); same pass order. Landed 29ecbbb3c6 + 89f9a64b9f; undo: revert both.

## Parked
- class-text (sound notes), unit plurals, def-first (b), note tests: discuss later (user).
- event-handlers (prose discussion with the user, no popups; block-parameters-captures landed): proposed `on(event: Event, body: block)` in the prelude, `emit(event, data)` runs the listeners, no handler core. Open: (1) event data as `it`, a chosen name (`on key k {…}`) or both; (2) user events declared (`enum Event += ask`) or any symbol; (3) decided: on-frame-headless. Branch event-handlers-unknown (a4d0dde771) only a reference.
- graphics-plain (warp-markup, parked): display alias table with notes; bare width/height as canvas sides with did-you-mean.
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
