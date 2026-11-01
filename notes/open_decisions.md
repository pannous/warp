# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- gitignore-narrow (supervisor; user chose "narrow it"): ~/.gitignore `*_REMOTE*` `*_LOCAL*` → `*_REMOTE_[0-9]*` `*_LOCAL_[0-9]*` (merge-tool leftovers like file_LOCAL_12345.c); yes → the Supervisor writes it.

## To confirm (decided on the user's behalf; built as described, keep or undo)
- f32-growing (core/runtime + warp_node.h): a GPU [f32] list is mutable and grows (push/insert/remove/+=), refused only where its declared type is violated.
- marked-floats-shape (core int_types + runtime marks_kept): `type X = float` like `= int`; marks survive arithmetic (left mark, else right, same Kind only, so Color * 0.5 is plain); `°` an Angle shown in degrees.
- type-shows-nat: type(3) and an undeclared whole value show `nat`, a declared int shows `int`.
- fast-int-readings: a literal beside a fast int stays int (`xs[i + 1]` in a range loop); `i++` stays int, `s += i` grows like + (a nat).
- definition-name-canonical: a definition's own name is canonical for dir and output (count stays count, not size); internal names get their canonical name (map_keys → keys).
- words-as-values (core lambdas.warp): a function word used as a value is the lambda of its call (`f = sqrt; f(16)` 4); several arities: the fitting one, never silently the first.
- op-assign-target-once: `x[f()] op= y` evaluates f once, as C, Python, JS.
- lambda-assign-writes-through (core ~20–40 lines): `=` in a lambda/block to an outer name writes it (`seen=0; f=() => {seen=3}; f(); seen` → 3); `let x` makes a new local.
- int-boxed-overflow-big (core ~25 lines): boxed int + - * overflow gives the big-int result instead of trapping.
- for-over-iterator (core +2 lines): `for x in Countdown(3)` iterates by next() until ø.
- hello-size-exports (core ~20 lines): shown_at/trap_message exported only from programs that can raise; hello 25.8 KB → ~1.5 KB, but other programs' engine traps lose `at 1:15`.
- one-duration: `2 hours` and time literals are one duration type; day/week/month/year calendar, compared on bounds, undecidable comparisons an error.
- sample-spellings: samples rewritten to the spec (`int[height * width]`, `(y - center)^2`, `particles.map(moved)`); keep the originals as language wishes?
- parameter-typed (core): `a or b` as a parameter/field/local type holds the members' shared kind, else a node; mismatch refused at compile time, nodes checked at run time.
- charged-values-no-pass: a new pass charged_values landed without the user's yes; being folded into a sugar rule. Keep the fold, or allow the pass?
- position-index-words: class `#` = position(p) from 1, `[]` = index(i) from 0, `#b` = count(); defining one derives the other.

## Parked
- class-text (sound notes), unit plurals, def-first (b), note tests: discuss later (user).
- event-handlers (prose discussion with the user, no popups; block-parameters-captures landed): proposed `on(event: Event, body: block)` in the prelude, `emit(event, data)` runs the listeners, no handler core. Open: (1) event data as `it`, a chosen name (`on key k {…}`) or both; (2) user events declared (`enum Event += ask`) or any symbol; (3) decided: on-frame-headless. Branch event-handlers-unknown (a4d0dde771) only a reference.
- graphics-plain (warp-markup, parked): display alias table with notes; bare width/height as canvas sides with did-you-mean.
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
