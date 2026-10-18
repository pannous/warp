# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- rewrite-syntax (card compiler-modules, warp-self-passes): how a module extends the compiler. Default built: a line
  `rewrite PATTERN => TEMPLATE`, optionally `where $c = SHAPE`; holes `$x`, `$xs...` as in code patterns; any program
  or module may declare them, a standard module's rules apply wherever their words appear. Example (lib/channels.warp):
  `rewrite $c.send($v) => channel_put($c, $v) where $c = channel()`. Alternatives: the keyword `macro`/`syntax`;
  `data(…)` around both sides; `rules { … }` blocks (matching roadmap step 4).

## Parked
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- unit-edges (cards cleanup-units-answer, cleanup-units-interpreter): eight ignored cleanup-edge tests, kept or retired? Ranges with a unit (`1900 - 2000 AD`, descending, "different units", `== 1950 AD ± 50`), the mixed `5 m ± 1 s` error. Revisit with card discuss-moving.
- P237: what `numbers.sort by size` means (today a silent no-op); leaning to an alias chain size → abs → norm. Open: does `size [3, 4]` stay the count 2 (recommended) or become the norm 5?
- P226c: subscript log base `₁₀⌟100` → 2 (recommended yes); skip! in test_logarithm2.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- P76: grant syntax for run-time blocks: `def f(b:block) ! IO` (recommended) / `interpret(x, grant: [io])` / `use eval io`. No grants exist yet.
- P45 script mode: may a variable change kind (`x=[1]; x=5`) under `use script`? Today a compile error everywhere.
- Extended rationals: ±1/0 as exact infinities instead of the divide-by-zero error (footgun-divzero).
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.

## domains-plain-libraries (supervisor, 2026-10-18)
The user decided music is a plain library with no compiler magic (decisions.md music-plain-library). Does the same hold
for the other domains whose Rust passes are being moved: web routes (serve.rs), markup/styles, graphics/window
properties, shader placeholders, reflection/help? Default until answered: yes as the goal; rule-shaped sugar moves to
rewrite rules meanwhile, the rest waits for module-passes or a general core feature.
