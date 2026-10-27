# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending

## To present
- kinds-as-cases: compiler kinds are enum cases as values, texts only at edges. Open alternative (warp-core-types-emitter's A): a spelled case IS its spelling at run time, `print Op.Add` → + instead of Add (would change enum-case).
- enum-raw-values: `enum W { A = "a·b", B }` spells cases; unspelled ones spell as their name (no mix error); `Dime = 10` its int.

## Parked
- graphics-plain (warp-markup, parked): display alias table with notes (stays in Rust); bare width/height as canvas sides with did-you-mean (stays).
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
