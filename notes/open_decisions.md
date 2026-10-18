# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending

## Parked
- element-loops (warp-markup, parked by self-hosting-focus): a loop among a plain tag's children (`friends{ for x in xs { x } }`, fails today) gives a nested list, elements splice it? Default: HTML-only as today.
- graphics-plain (warp-markup, parked): shader `$holes` captured at the shader vs read at each paint (pass stays); display alias table with notes (stays in Rust); bare width/height as canvas sides with did-you-mean (stays).
- rule-implicit (gh #2069, warp-parsing, parked): a name applies to the next operand after `*` `/` `^` and inside `?:` too (`2 * f y` = 2 * f(y))? Default: leave as is, power windows documented.
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
