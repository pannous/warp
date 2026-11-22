# Open decisions for the user

Only open questions: pending or parked, one line each, naming the default the code follows. Only the Interviewer asks
the user; nothing here blocks. Answers move to notes/decisions.md, work goes on the board (`todo add`), rules go where
they are applied (AGENTS.md, notes/agents/, the wiki).

## Pending
- warn-rule-impossible (warp-fixer, morning 11-23): the approved `=> warn` rule can't tell `"5" + 3` from `"a" + 3`; no rule hole can see what a literal text says. (a) recommended, lib only: the run-time hint prints once per run (+2 lines in texts.wap); (b) a hole that tests a literal's content at compile time (~10–15 core + warn +4), with no other user today; (c) drop the warning (changes the user's decision). Nothing built.
- untyped-map-any (warp-wap-lang, morning 11-23): an untyped `{}` is map<any, any> (user's m['a']=7; m[3]=4). Core: 2 tokens in analyze/data.wap + 1 changed line in letters.wap bound_as (0 added); the compiler's own source writes 4 map types. Branch untyped-map-any 1ede5b3, held for the discussion.

## Shrinks landed (listed for the user, no discussion needed)
- 11-22 simplifier: core folder_of copy −45; core batch (dead typed_file, unused parameters, min/max/sum, quoted_text → show, with_items) −160; console line editor 1594 → 1300 nodes −294 (Ctrl/Shift+arrow no longer move a character by accident); types/ −45 and −35. Total −579 nodes.

## To confirm (decided on the user's behalf; built as described, keep or undo)
- runtime-wasm-shape (interviewer 11-22, user asked "any objections to leaving it?"): left as is, each module carries its runtime words; hello stays standalone (511 B). Revisit when compiler.wasm size (−29% possible) or multi-module programs matter.
- export-dispatcher-size (night): the plain-name export dispatcher came in at +49 wap core lines (estimate ~10); only the plain-name export landed first.
- wap-wiki-from-notes (night): wiki precedence.md gained "Spacing picks an operator's fixity" and "A program's own precedence"; rule.md is a new page.
- wap-primitive-table-dropped (night): one primitive table dropped for too little general benefit.
- wap-c-foreign (night): lib's `f(s: text): i64 from 'c'` lines are built by a rule in wap/extensions/c.
- wap-enum-case-constants (interviewer, +11 core lines): constant case values are built once as shared globals; node ceiling 108193.
- wap-reader-wiki (night, landed): `|` is the pipe; `//` after `:` is not a comment; kebab-case names only inside html; the `a.b`/`a. b` bullet dropped; `a {x}` still marked "// sure?".
- Remote branches to delete (merged; the hook lets only the user delete them): pannous/warp land/* from 11-21 and wap-rules-*; pannous/wap wap-module-reassign, wap-lambda-any, wap-console-variables, wap-function-value-place, seed-refresh-2026-11-21-*, preload-wasm, wap-boxed-adapter, wap-adapted-wrapper, wap-is-bare-list; warp.wiki wap-notes-wiki, wap-reader-wiki, wiki-public.

## Parked
- wap-units-chain (user 2026-11-22: keep paused until the native wap is complete): porting unit_names, reals, time, bits, os, file, tasks for the 22 sound/units samples. Shapes waiting on the units card: `class B extends A` (~30–50 core, needs the user), type aliases (~5–10), marked float `type Angle < float` (large), a spaced call after an infix (~2).
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
