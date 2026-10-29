# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending

## To confirm (decided on the user's behalf, confirm or undo at 10:00)
- float-int-power (interviewer, core runtime pow, samples-gpu): float ^ int uses square-and-multiply instead of the generic pow, as Julia does, keeping the error within about 1 ulp (Julia's compensated power_by_squaring); if that stays inaccurate, only for |exponent| ≤ 16. Exact values untouched. A last-digit difference from libm pow is accepted.
- hello-size-exports (interviewer, core ~20 lines, card hello-wasm-size): core/emitter/host.warp exports shown_at and trap_message only from a program that reaches raise/throw, a raising runtime word or a host word with arguments (the reachability rule of runtime-shaking); hello 25.8 KB → ~1.5 KB. Cost: in other programs an engine trap (cast failure, stack overflow) prints as wasmtime names it, without `at 1:15`. Alternative: a light trap_message always (~6–7 KB, bigger change).
- signals-test-15 (interviewer): test/control/test_variable_signals.pairs.warp:15 `x=3; n=0; on set x : n+=1; while x-->0 : x; n` upgraded 3 → 4: the failing loop test's x-- (0 → -1) is a write too, no special case.
- after-participle-held (interviewer): :18 `after tested: n+=10` (a hook named by a function's past participle) triage-held as esoteric; a lib wrapper/decorator can do it later.
- one-duration (interviewer): `2 hours` and time literals are one duration type; day/week/month/year calendar, compared on bounds, undecidable comparisons an error.
- sample-spellings (supervisor, warp-native-harness; user 2026-10-29: ask later): samples rewritten to the spec instead of new syntax — `height * width int` → `int[height * width]` (circle, filled_circle, mandelbrot_canvas), `(y-center)^2` → `(y - center)^2` (circle; hyphenated names stay names), `moved all particles` → `particles.map(moved)` (particles), and a stray tab before viewer's `def triangles` removed (an indented line continues the one above); keep the originals as language wishes?
- implicit-standard-words, index scope (worker samples-implicit-use): the word index (lib/modules/words.tsv) covers every standard module's definitions but the prelude, the dialects R, PHP and english too, read from their sources at rebuild, not only from modules that prebuild; a program's own class names never pull a module in (`class C` is not sound's note C).
- program-match lowering (supervisor): "merge all existing branches" taken as the OK for its run-time matcher in lib/prelude.warp + core/node_matches.warp (landed 9f9ef1ce43).
- test path edit (supervisor): test/node/test_parsed_node_walks.pairs.warp:34 reads core/parsing/parser.warp instead of core/parser.warp after the core/ grouping; the only test edit of the reshuffle.
- warp test handoff (supervisor): until the native test runner lands, `warp test` on the native warp hands off to warp-rust with one stderr line.
- build stamp without inputs hash (supervisor): warp.build records lib/, builder, commit and time; the inputs-hash slot stays empty for now (the native warp has no sha256/directory host words yet); nothing compares it.
- round(2.5) = 3 (supervisor; user 2026-10-29: ask later): the decision round-halves (away from zero) beats the Footguns note; the note gets corrected, not the test.
- unit tests without `use units` (supervisor): the 83 lines the triage listed as wrong are gaps under implicit-standard-words, not wrong expectations.
- release workflow quiet (supervisor): Release re-enabled with one summary at the end, no per-job notifications; the other workflows stay off.
- no extra workers tonight (supervisor): the ten triage gap groups wait on the board (Next) for the four feature workers instead of new sessions (rule: at most two feature workers, already exceeded by the runtime-linking team).
- computed-hole-shape (fixer): the words a rule's `where` calls at compile time live in lib/rule_words.warp, a std
  module the compiler itself uses (`use rule_words` in core/passes/rewrites.warp), named in its `rule_words` table: a new
  word needs `warp rebuild`, as a sugar rule does. `where $h = word(…)` of such a word binds $h (a `[…]` result is a
  rest hole's items), `where word(…)` tests it, `$program` is the program; the same syntax with any other call stays
  the origin's shape. A rule `where program` rewrites the whole program once, after the rules inside it; a module's
  program rules come with its others. Instead of the store `x ← v`, a rule word gives ø where it already did its work
  (routes' served). The routes rules sit in lib/web.warp, not lib/sugar/routes.warp, so a firing brings `use web`;
  core/passes/routes.warp is gone. A plain `warp app.warp` serves the routes over HTTP through lib/web.warp.

## To present
- ffi-link-any-import (harness OK, user away, card pure-compiled): pure runs link C libraries for any import module other than wasi/host (was: only libm), so `import strlen from "c"`, sqlite3, raylib run (test/ffi 110/133 failed). Grown: one C linker shared by `warp pure` and warp-host (the module's warp.c_functions, moved into warp-runtime), header parsing only for executables until carded away; `--sandbox` refuses C imports.
- error-expectation (harness OK, user away, card pairs-test): a test expectation `error` matches any Error result (refusal or raise); no message substrings (no wording pins); by kind (`error(charged_assignment)`) later.
- parameter-typed (core OK, user away): `a or b` / `a | b` as parameter, field or local type holds the members' shared kind, else a node; a known kind fitting no member refused at compile time, a node checked at run time (union-or).
- host-list-one-shot (harness OK, user away): a host word's list result is built in one shot (shared vector_list), not cell by cell; 44100 sound samples trapped at the 2 GB cap (n² copies).
- shown-non (core OK, user away): show/print of a statically known Text/Int/Float/Bool/Char uses its own word; node_text's rare-kind branches (Time, Ratio, big int, closure, TypeDef) only when the program makes them, via one kind_case; `"hi"` compiled 119 functions, ~40 unreachable.
- charged-values-no-pass (user 2026-10-29: ask later): 9cc25cc1e0 landed a new pass charged_values (a `h := e` reading a variable becomes `h() := e`, P71) without the user's yes; being folded into a sugar rewrite rule (or an existing pass). Keep the fold, or allow the pass?
- test-oversubscribes-mac (harness OK, user away): `warp test` case processes compile single-threaded when cases already run in parallel (load 170 on 10 cores).
- position-index-words: class `#` = position(p), `[]` = index(i), `#b` = count(); writes position(p, value) / index(i, value), defining one derives the other.

## Parked
- event-handlers (prose discussion with the user, no popups; block-parameters-captures landed): proposed `on(event: Event, body: block)` in the prelude, `emit(event, data)` runs the listeners, no handler core. Open: (1) event data as `it`, a chosen name (`on key k {…}`) or both; (2) user events declared (`enum Event += ask`) or any symbol; (3) decided: on-frame-headless. Branch event-handlers-unknown (a4d0dde771) only a reference.
- graphics-plain (warp-markup, parked): display alias table with notes (stays in Rust); bare width/height as canvas sides with did-you-mean (stays).
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
