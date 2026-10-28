# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- class-index-test (warp-global-named-capital, test/types/test_class_index_operator.pairs.warp:4, from the Rust suite): `class P{n:int; index(o) := n * 100 + o}; F = P(5); F#4` expects 504 (index read as `#`); under position-index-words it is position(4) = index(3) = 503. Options: upgrade the test to 503 (Recommended, follows the naming) / keep 504 (`#` calls index unshifted; then position-index-words changes). Failed on main before too; ask at 10:00.

## To present
- shown-text-quoted (warp-text-field's assumption under shown-by-warp, ac50b6e2d6): a text result stays quoted (`"Hello 🌍2026"`, test/gate/hello.expected); other values go through show (3km + 500m → 3.5km, π/2, P(3) → P3). Question to raise: should a result show as warp notation throughout (text quoted, an instance as `P{n: 3}` or `P(3)` rather than P3), so a shown result can be pasted back as code?
- test-oversubscribes-mac (harness OK, user away): `warp test` case processes compile single-threaded when cases already run in parallel (load 170 on 10 cores).
- position-index-words: class `#` = position(p), `[]` = index(i), `#b` = count(); writes position(p, value) / index(i, value), defining one derives the other.

## Parked
- event-handlers (prose discussion with the user, no popups; block-parameters-captures landed): proposed `on(event: Event, body: block)` in the prelude, `emit(event, data)` runs the listeners, no handler core. Open: (1) event data as `it`, a chosen name (`on key k {…}`) or both; (2) user events declared (`enum Event += ask`) or any symbol; (3) `on frame` once headless or never. Branch event-handlers-unknown (a4d0dde771) only a reference.
- graphics-plain (warp-markup, parked): display alias table with notes (stays in Rust); bare width/height as canvas sides with did-you-mean (stays).
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
