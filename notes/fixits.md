# Fixes ("I meant: …")

User, 2026-10-05: "suggest fixes and a mechanism to apply the fixes in the web demo for all the warnings that we have,
in case the user did indeed intend something different". Builds the "Later: change the code" of notes/welcoming.md.

## Mechanism
- `fixits::Fix { meaning, written, replacement, at }` (src/fixits.rs): one reading the user might have meant. `written`
  is the text as the compiler shows it (serialized: spacing and list commas may differ from the source), `at` an
  optional position when the text stands elsewhere than the diagnostic (main's `n=0` for `global n=0`).
- `Diagnostic.fixes: Vec<Fix>` and `Diagnostic.topic` (src/diagnostic.rs); `.offer(meaning, written, replacement)`
  adds one, dropping a fix that changes nothing (the reading already written). Every `Ask` turns each reading into a
  fix by itself (`written` → `explicit_form`); a reading whose edit differs says so with `reading(..).replacing(written,
  replacement)` or `.fixed_by(fix)`. The prose `fix` stays as it was (messages unchanged).
- `fixits::locate(source, line, column, written)`: every place the text stands, comparing without whitespace and
  commas, on word boundaries (`n =` never matches inside `fn =`); the one nearest the position wins.
  `fixits::edit` → `Edit { range (bytes), replacement }`, `fixits::fixed(source, line, column, fix)` → the new source.
  A later `warp fix` or an LSP code action starts there (`utf16_offset` for LSP/JS positions).
- Errors lose their Diagnostic when they become `Node::Error`, so `Diagnostic::into_error` keeps the ones with fixes
  (`diagnostic::take_error_diagnostics`).
- Hints (`normalize::hint`) are rewrites: `CapturedHint::fix()`; `normalize::advise` is a hint whose preferred form
  is said elsewhere (`global x` for a block's assignment) and offers no fix.
- Web report (src/web.rs `evaluate`): warnings, `errors` (only when the program failed) and hints carry `fixes`:
  `{label: "I meant: …", meaning, written, replacement, start, end}` with UTF-16 offsets of the editor text;
  `start: null` when the text was not found (the page shows a disabled button: loud, not silent). Warnings also carry
  `topic` for "got it".
- Playground (web/playground/playground.js): each fix is a button; clicking replaces the range (CodeMirror
  `posFromIndex`) and runs again. Next to the fixes, "got it" in the three scopes of got-it-scope (user, 2026-10-05):
  "got it" (this expression: the warning's `expression_key` `topic@expression` joins the page's acknowledged list),
  "got it: all <topic>" (the topic), "// got it" (appends the comment to the warning's line). Notes without a
  warning use the report's `got_it` [{topic, expression}].

## Inventory: every warning, its default reading and its fixes
| warning / error (topic) | where | default (taken) | fixes offered |
|---|---|---|---|
| `a upto b` (upto) | wasp_parser `range_reading` | exclusive | `..<` exclusive, `...` inclusive |
| `for i in 0..n-1` (kotlin-range) | same | exclusive (wasp) | `..<`, `...` (Kotlin's inclusive) |
| `1 -1` (signed-operand) | wasp_parser `signed_number_starts_a_list` | the list | `1 (-1)` list (also inside `[…]`), `1 - 1` |
| `n = …` in a function, main has n (local-or-global) | analyzer `ask_local_or_global` | new local | `let n =` there; `global n=` at main's assignment |
| `[x]*n` (list-times, error) | analyzer `list_times` | none | `n times [x]`, `[x].map(x => x*n)` |
| `[1 2 3]+4` (list-plus, error) | analyzer `list_plus` | none | `[1 2 3] + [4]`, `[1 2 3] .+ 4` |
| `xs.insert(0, 4)` (insert-order, error) | list_emitter | none | `insert(4, at: 0)`, `insert(0, at: 4)` |
| `a=1 2 3` (bare-list, error) | ambiguous_forms | none | `a=[1 2 3]`, `a=1; 2; 3` |
| `1+2 squared` (suffix-precedence, error) | ambiguous_forms | none | `1+(2 squared)`, `(1+2) squared` |
| `x : a+b` (uncharged-block) | blocks.rs | a block | `x = a+b` (its value) |
| `render "hi"` with variants (overloads) | overloads.rs | first declared | `render "hi" as pdf`, `… as docx` |
| `pirnt` names nothing (near-miss) | list_emitter `warn_near_miss` | the symbol | `data pirnt`, `print` |
| `square 3 + square 4` (error) | analyzer `check_ambiguous_calls` | none | `square(3) + square(4)`, `square(3 + square(4))` |
| `true + true` (error) | analyzer `check_boolean_arithmetic` | none | `int(true) + int(true)` |
| `1==1==1` (error) | wasp_parser `chained_equality` | none | `1==1 and 1 == 1`, `(1==1) == 1` |
| `3 & 4 == 4` (error) | wasp_parser `logic_mixed_with_comparison` | none | `3 & (4 == 4)`, `(3 & 4) == 4` |
| `"a" as int` (error) | wasp_parser string literal | none | `codepoint('a') as int` (P74) |
| `'a' as float` (hint; the value is invalid_number) | wasm_emitter `emit_character_cast` | none | `codepoint('a') as float` |
| `c and t or o` | analyzer lint | as written | `if c then t else o` |
| `2 * 1.5 as int` | analyzer lint | the whole | `(2 * 1.5) as int`, `2 * 1.5:int` |
| `-7 % 3` | analyzer lint | Euclidean | `-7 rem 3` (C/Java/JS) |
| loop `it` hides the function's `it` (`for 1..3 {…}` and `for i in xs {…}`) | analyzer `hidden_function_it` | the loop's | `outer_it=it; for … {… outer_it …}` |
| `(1, 2) == 1, 2` | tuples.rs | `((1,2) == 1), 2` | `(1, 2) == (1, 2)` |
| `xs#1..3` | wasp_parser `hash_range_warning` | range from xs#1 | `xs#(1..3)` slice, `(xs#1)..3` |
| style hints, educate_once notes (`let`, `**`, quotes, `&&`, `len(x)`, `x == int`, `a.copy()`, `o.field`) | normalize, lowering | as written | the preferred form (when the text is literally in the source) |

## No fix yet (todo.md)
- kebab data key `a-b:2` with variables a, b: both readings need edits at the key and at its reads (several edits per
  fix; Fix has one).
- type-filter loop `for int in xs` (filter-loop): the explicit form `for x in xs { if x is int {…} }` needs the body.
- `like`-checked data lacking or adding fields (traits.rs `field_warnings`): no value to invent for a missing field.
- task control that never happens (declarations.rs `never_happens`), folder-scope module warnings (modules.rs):
  nothing in the source to rewrite.
- hints whose original is a pattern (`x ? y : z`, `trait name{…}`): `start: null`, a disabled button.

## Tests
tests/welcoming/test_fixits.rs: one test per category applies the fix and checks the result's value with warnings as
errors (the explicit form must not warn again). Browser: probes/web_playground.py clicks "I meant: ..." on the
ambiguity example (6 → 10) and the braceless-call error fix (→ 25), "got it" for one expression and the "// got it"
comment button; tests/welcoming/test_fixits.rs `the_page_says_got_it_for_one_expression` checks the report's keys.

## Findings on the way
- `[1 -1]` still warns (signed-operand inside brackets), so the list's explicit form is `1 (-1)`.
- `for i in 1..3 {…it…}` in a function hid the function's `it` without a warning (the loop binds `it` to the item
  too); warned now (#23).
- P74 (user): `codepoint(c)` is the preferred name (`ord`, `ordinal` stay synonyms; inside the compiler the word is
  `ord`, since `codepoint` is also a type word and `codepoint(x)` would read as its constructor). `'x' as float` was
  120 at compile time but `'x' as int` an error: both are invalid_number now, hinted toward `codepoint('x') as …`.
  `codepoint(c) as float` (and a library word's number in float arithmetic) failed with "cannot extract a numeric
  value": fixed.
