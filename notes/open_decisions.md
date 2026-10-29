# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- resident-compiler (warp-fixer, card program-fixed): a resident compiler instance `warp` reuses over a socket, all state live, so a program costs only its own parse + emit (~20 ms instead of 0.10 s for `"hi"`). Concerns: process lifetime, concurrency, staleness after a rebuild. Options: build it (auto-started, restarts when warp.wasm changes) / not now (keep cutting fixed cost) / opt-in flag.
- class-index-test (warp-global-named-capital, test/types/test_class_index_operator.pairs.warp:4, from the Rust suite): `class P{n:int; index(o) := n * 100 + o}; F = P(5); F#4` expects 504 (index read as `#`); under position-index-words it is position(4) = index(3) = 503. Options: upgrade the test to 503 (Recommended, follows the naming) / keep 504 (`#` calls index unshifted; then position-index-words changes). Failed on main before too; ask at 10:00.

## To confirm (decided on the user's behalf, confirm or undo at 10:00)
- sound-system-player (supervisor): sound plays through the system player, afplay/paplay, configurable by WARP_SOUND_PLAYER; under WARP_NO_WINDOW or CI only an explicit WARP_SOUND_PLAYER plays (warp-native-harness)
- fixed-time-units (supervisor's call, separate-modules): `1 minute`, `2 hours`, `90 seconds` are fixed quantities of lib/units.warp (a minute is 60 s, `sleep 1 minute` sleeps); day, week, month and year stay calendar durations (a day can be 23 or 25 h across DST)
- is-pattern-binds (interviewer): `if p is Point{x, ..}: print x` binds x where the test is known true (then-branch, rest of an `and` chain, match arm); one rule for `is` and `match`.
- implicit-standard-words, hint part (interviewer): a word used without `use` gets a hint naming its module, so `warp beautify` writes the `use`.
- implicit-standard-words, index scope (worker samples-implicit-use): the word index (lib/modules/words.tsv) covers every standard module's definitions but the prelude, the dialects R, PHP and english too, read from their sources at rebuild, not only from modules that prebuild; a program's own class names never pull a module in (`class C` is not sound's note C).
- program-match lowering (supervisor): "merge all existing branches" taken as the OK for its run-time matcher in lib/prelude.warp + core/node_matches.warp (landed 9f9ef1ce43).
- test path edit (supervisor): test/node/test_parsed_node_walks.pairs.warp:34 reads core/parsing/parser.warp instead of core/parser.warp after the core/ grouping; the only test edit of the reshuffle.
- warp test handoff (supervisor): until the native test runner lands, `warp test` on the native warp hands off to warp-rust with one stderr line.
- build stamp without inputs hash (supervisor): warp.build records lib/, builder, commit and time; the inputs-hash slot stays empty for now (the native warp has no sha256/directory host words yet); nothing compares it.
- round(2.5) = 3 (supervisor): the decision round-halves (away from zero) beats the Footguns note; the note gets corrected, not the test.
- triage holds (reviewer): the lines marked `# triage-held 2026-10-28` (list in notes/test_triage.md "Held (esoteric)") are commented out as forms warp doesn't carry over; restore any you want back.
- unit tests without `use units` (supervisor): the 83 lines the triage listed as wrong are gaps under implicit-standard-words, not wrong expectations.
- release workflow quiet (supervisor): Release re-enabled with one summary at the end, no per-job notifications; the other workflows stay off.
- no extra workers tonight (supervisor): the ten triage gap groups wait on the board (Next) for the four feature workers instead of new sessions (rule: at most two feature workers, already exceeded by the runtime-linking team).
- split-whitespace (interviewer, card sample-word-functions): `t.split` with no argument splits on runs of whitespace (Python's split()).
- named-arguments-general (interviewer): any parameter can be named at the call, so `t.replace "tea" with: "coffee"` works because replace's second parameter is named `with`; no special case for replace.
- length-all-held (interviewer): `length all xs` is triage-held; the forms are `xs.map length` and, per julia-compat, the broadcast `length.(xs)`. A general `all`/`each` broadcast word is for you to revisit.

## To present
- of-binds-tighter (parser OK, user away, card foo-of-any): `a of b = e` is `(a of b) = e`, so `square of int = it * it` defines square(it: int) and `name of p = "Al"` stays an assignment to p.name.
- ffi-link-any-import (harness OK, user away, card pure-compiled): pure runs link C libraries for any import module other than wasi/host (was: only libm), so `import strlen from "c"`, sqlite3, raylib run (test/ffi 110/133 failed). Grown: one C linker shared by `warp pure` and warp-host (the module's warp.c_functions, moved into warp-runtime), header parsing only for executables until carded away; `--sandbox` refuses C imports.
- error-expectation (harness OK, user away, card pairs-test): a test expectation `error` matches any Error result (refusal or raise); no message substrings (no wording pins); by kind (`error(charged_assignment)`) later.
- global-word-marker: a module marks a word usable without its `use` as `global play(file: text) := …` (no new keyword; the sense of `global tuning = 440`); core reads the markers, its global_words table goes. Alternatives: a module line `global play, beep`, or `@global`.
- parameter-typed (core OK, user away): `a or b` / `a | b` as parameter, field or local type holds the members' shared kind, else a node; a known kind fitting no member refused at compile time, a node checked at run time (union-or).
- host-list-one-shot (harness OK, user away): a host word's list result is built in one shot (shared vector_list), not cell by cell; 44100 sound samples trapped at the 2 GB cap (n² copies).
- shown-non (core OK, user away): show/print of a statically known Text/Int/Float/Bool/Char uses its own word; node_text's rare-kind branches (Time, Ratio, big int, closure, TypeDef) only when the program makes them, via one kind_case; `"hi"` compiled 119 functions, ~40 unreachable.
- charged-values-no-pass: 9cc25cc1e0 landed a new pass charged_values (a `h := e` reading a variable becomes `h() := e`, P71) without the user's yes; being folded into a sugar rewrite rule (or an existing pass). Keep the fold, or allow the pass?
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
