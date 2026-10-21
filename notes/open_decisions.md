# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- keywordless-def (warp-pure-lambdas, card unused-sibling): at statement start, is `f(x) { x + 1 }` (no def, no :=) a definition when f is defined nowhere and the parameters are bare names (got-it note "I meant: def f(x) { … }"), and a call with a trailing block when f exists? Options: definition-if-undefined (recommended; footgun: a typo of a defined name defines a new function, the note shows it) / always an error naming `def f(x) {…}` and `f(x) := …` / leave as is ("undefined function: f"). Today: undefined function; nothing depends on it.

## Parked
- graphics-plain (warp-markup, parked): display alias table with notes (stays in Rust); bare width/height as canvas sides with did-you-mean (stays).
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
