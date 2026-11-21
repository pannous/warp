# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- untyped-nested-maps (warp-wap-binding, 2026-11-21, for the user's morning, ask with variable-retyping): `out = {}; section = out; section = {}; out["a"] = section` (lib/ini.warp) makes out's value type a map of itself. Built now: refused, "this gives map<text, α1>, the place holds α1" (+2 core lines, stops a stack overflow; ini.warp writes `out: map<text, any>`). B) the self-reference settles as any, out is map<text, map<text, any>>, untyped nested maps work as under warp (about +4 core lines).
- variable-retyping (warp-wap-operators, 2026-11-21, for the user's morning): `x = 1` then `x = "a"`: wap refuses it, while wiki/type.md:316 says only a `let`/`const` name has a fixed type and other names give a warning. A) the wiki wins: x becomes `any` with a warning, a few wap core lines in variable typing (recommended if type.md is current). B) the compiler wins: type.md:316 is rewritten to "a name keeps its first type". Decides how dynamic wap feels, so held for the user.
- enum-case-as-type (warp-lead, 2026-11-21, for the user's morning): may a parameter be typed as one case of an enum and read its fields by name? `enum Shape { Dot(size: int), Line(parts: list<Shape>) }; first_size(d: Dot) := d.size` is refused today. A) a case is a type: `d: Dot` narrows, `d.size` reads the field, est. 15–30 wap core lines + a variant.md passage; it lets review 2's Matcher/Lexeme cleanup shrink core (recommended). B) no, cases only matched, 0 lines. C) park. A new language feature, so held for the user, not decided at night.

## To confirm (decided on the user's behalf; built as described, keep or undo)

## Parked
- instance-truthy-wiki (user: park, 2026-11-19): falsey.md:17 lists `cat{}` (an object without fields) as falsy, truthiness.md too; the user's 11-18 answer "truthy unless its class defines empty" was given on a misread of falsey.md. wap does truthy-unless-empty; the wiki pages stay unchanged until decided.
- float-ranges (user: park, 2026-11-16, from card plot-siren-range): `0..π` as a range of real numbers (plot(siren, 0..π)); today a real end fails ("int of node"); core: range literal with real ends, likely 20+ lines. Meanwhile plot(f, from, to).
- data-tag-words (user: park, 2026-11-16; built as is): in data, glued `head{…}` / `first{…}` (prelude words) stay tags; only program-defined or used words call. Spaced `first {…}` stays the prelude call (spaced-tag keeps prelude words; only head is a tag via the library index). Keep, or make spaced prelude words tags too? (decisions.md)
- phrase-params (user: park for now; card definition-heads-phrases): in `send v to c := …` which words are parameters: (a) the body decides (used or typed words), () optional, unifying the older `to add a to b:` reader (an unused plain word there turns fixed); (b) always parentheses. Also: a phrase head starting with a keyword word (`send` is an emit word) refused aloud vs outranking the keyword. Provisional (a) on the branch only, not landed. Meanwhile phrases are written with named arguments (`give 2 to: 40`, named-argument-phrases).
- note tests (decisions.md note-tests): discuss later.
- event-handlers (prose discussion with the user, no popups; branch function-events): `on(event, body: block)` and `emit(event, data)` run the listeners, no handler core. Open: (1) event data as `it`, a chosen name (`on key k {…}`) or both; (2) user events declared (`enum Event += ask`) or any symbol.
- graphics-plain (warp-markup, parked): display alias table with notes; bare width/height as canvas sides with did-you-mean.
- body-statements: a function body looped as a list of statements, each with its kind (operation, call, assignment…); no `.kind` until then.
- P69a: may a run-time block assign the `!` site's locals? Default (wiki/charged.md): no, it reads them as at `!` and assigns only declared globals. Until run-time `!` is built.
- polish-notation (card polish-notation): Polish notation for .wat/.wast (`(module a b)` as nodes, a ParserOptions mode); test_wast stays ignored.
