# Open decisions for the user

Only open questions: pending or parked (user 2026-10-08: "It's called open decisions for a reason. Open"). Only the
Interviewer asks the user; nothing here blocks, each question names the default the code follows. Answers move to
notes/decisions.md, work goes on the board (`todo add`), rules go where they are applied (AGENTS.md, notes/agents/,
the wiki).

## Pending
(none)

## To present (Interviewer's decisions, 2026-10-15 evening; user: "you make the decisions tonight and tomorrow
morning you present the decisions and we can still undo everything")
Present each in the morning popup batch (keep / change), then move it to decisions.md as the user's or undo it.
- unit-definitions: see decisions.md dimensions-units.
- letters: see decisions.md (Text).
- json-exactness (big ints exact digits, fractions float + warning): see decisions.md (Text).
- property-accessors (`width: get{…} set{…}`, no set = read-only): see decisions.md (Classes).
- saved-words (bare `canvas` = `canvas()` the current Picture, no save table): see decisions.md (Classes).
- duration-units (durations and unit h unify, `2 hour + 1 h` works): see decisions.md (unit lines).
- fail-effect (raises, maybe-zero division, library Errors; not indexing/casts): see decisions.md (Classes… P230).
- foreign-signature (`sin(x: float): float from 'm'` in lib/math.warp): see decisions.md (Modules).
- beautify-preview (diff + "apply? y/n" in a terminal, `beautify yes` for scripts): see decisions.md warp-format.
- random-forms (`random T` any type, `random(1, 6)` and `random 1...6` die roll inclusive, `random 1..6` excludes 6 like the range, Type parameters take bare type words): see decisions.md (Functions and calls).

## Parked (user: "later")
- body-statements (2026-10-14): a function body looped like a list of statements, each statement with its kind
  (operation, call, assignment…); `.kind` was dropped from body-code until then.
- unit-edges (warp-operator-shall, cards cleanup-units-answer and cleanup-units-interpreter): eight ignored
  cleanup-edge tests, kept or retired? Constant `1950 cm ± 50` folding (now at run time, prints right again), ranges
  with a unit (`1900 - 2000 AD`, descending, the "different units" error, `1900 - 2000 AD == 1950 AD ± 50`: three
  more tests), the mixed-dimension `5 m ± 1 s` error. Parked 2026-10-13; revisit with card discuss-moving.
- P237 (warp-class, card natural-phrases): what `numbers.sort by size` means; today a silent no-op. The user leans to
  `size` as an alias chain size → abs → norm (`[3, -5, 2].sort by size` → [2 3 -5]). Open: does `size [3, 4]` stay
  the count 2 (recommended, norm via `norm`) or become the norm 5? Parked 2026-10-09.
- P226c: subscript log base `₁₀⌟100` → 2 (recommended yes), next to the decided `100⌟10` → 2. Parked 2026-10-09;
  skip! in test_logarithm2.
- P150 license: none yet, so nobody may legally reuse the code and crates.io refuses the upload (P151). MIT
  (recommended) / Apache-2.0 / MIT OR Apache-2.0. Parked 2026-10-06.
- P69a: may a run-time block assign the `!` site's local variables? Default (wiki/charged.md): no, it reads them as
  they are at `!` and assigns only declared globals. Parked 2026-10-05 until run-time `!` is built.
- P76: grant syntax for run-time blocks (pure by default): `def f(b:block) ! IO` (recommended) / an argument on the
  forcing word `interpret(x, grant: [io])` / a pragma `use eval io`. Parked 2026-10-05: no grants exist yet.
- P45 script mode: a mode where a variable may change kind (`x=[1]; x=5`); today that is a compile error "unless we
  are in script mode, which is not defined yet". Not designed. Parked 2026-10-05.
- Extended rationals: ±1/0 as exact infinities instead of the divide-by-zero error (footgun-divzero). Parked
  2026-10-10.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode): "Keep parked"; test_wast stays
  ignored. Parked 2026-10-03.
