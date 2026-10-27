# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending

## To present
- declared-class (core + harness OK, user away): each declared class is its own wasm GC struct (descriptor + fields in declared order, subclass = struct subtype); `p.code` with a known class is struct.get; methods' self typed, the "class with methods" refusal goes; maps and undeclared classes unchanged; an undeclared field turns an instance into a map with a warning. Stages 1–2 OK; stage 3 (unboxed int fields) after kinds-as-cases.
- class-field-slots (core OK, user away): `p.code` on a declared class reads the field at its declared position (one name check), falling back to the map lookup for maps and copies; same values and errors, about half of a rebuild's parse saved. Later: one wasm GC struct per class.
- kinds-as-cases: compiler kinds are enum cases as values, texts only at edges. Open alternative (warp-core-types-emitter's A): a spelled case IS its spelling at run time, `print Op.Add` → + instead of Add (would change enum-case).
  Cost (warp-core-types-emitter, recommends A): under D the emitter's ~40 kind-keyed tables become match functions, messages need a spelling call, ~400 kind compares become ValueKind·is calls in hot paths; under A a literal swap at today's speed. Counterpoint: D's cost is today's enum implementation; a case as a small int should compare and index faster than a text. Analyzer already on D (d2c61023c0), works either way.
- enum-raw-values: `enum W { A = "a·b", B }` spells cases; unspelled ones spell as their name (no mix error); `Dime = 10` its int.

## Parked
- graphics-plain (warp-markup, parked): display alias table with notes (stays in Rust); bare width/height as canvas sides with did-you-mean (stays).
- maybe-javascript (card in Later): typed `js "…"` / `wgsl "…"` templates like `sql` and `sh`, once glue code is needed that warp/wasm can't produce; until then `shader {…}` and `use js`.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- #10 Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
