# Fixes ("I meant: …")

User, 2026-10-05: "suggest fixes and a mechanism to apply the fixes in the web demo for all the warnings that we have,
in case the user did indeed intend something different". Builds the "Later: change the code" of notes/welcoming.md.

## Mechanism
- `fixits::Fix { meaning, written, replacement, at }` (src/fixits.rs): one reading the user might have meant. `written`
  is the text as the compiler shows it (serialized: spacing and list commas may differ from the source), `at` an
  optional position when the text stands elsewhere than the diagnostic (main's `n=0` for `global n=0`), `also` the
  further edits one reading needs elsewhere (`.and(written, replacement, at)`: a data key and its reads).
- `Diagnostic.fixes: Vec<Fix>` and `Diagnostic.topic` (src/diagnostic.rs); `.offer(meaning, written, replacement)`
  adds one, dropping a fix that changes nothing (the reading already written). Every `Ask` turns each reading into a
  fix by itself (`written` → `explicit_form`); a reading whose edit differs says so with `reading(..).replacing(written,
  replacement)` or `.fixed_by(fix)`. The prose `fix` stays as it was (messages unchanged).
- `fixits::locate(source, line, column, written)`: every place the text stands, comparing without whitespace and
  commas, on word boundaries (`n =` never matches inside `fn =`); the one nearest the position wins.
  `fixits::edit` → `Edit { range (bytes), replacement }`; `fixits::edits` → all of a fix's edits, the last in the
  source first (None when one is missing or two overlap); `fixits::fixed(source, line, column, fix)` → the new source.
  A later `warp fix` or an LSP code action starts there (`utf16_offset` for LSP/JS positions).
- Errors lose their Diagnostic when they become `Node::Error`, so `Diagnostic::into_error` keeps the ones with fixes
  (`diagnostic::take_error_diagnostics`).
- Hints (`normalize::hint`) are rewrites: `CapturedHint::fix()`; `normalize::advise` is a hint whose preferred form
  is no plain rewrite of the text: said elsewhere (`global x` for a block's assignment) or meaning something else
  (`(a<b) |> f` for `a<b then f`). It and `diagnostic::advise_once` take the "I meant" fix as an argument: the edit the
  advice stands for (`global x=1` at main's assignment, `normalize::rewrite` for a replacement of the text itself),
  `None` when the advice names no text to write (`on … before the loop`).
- Web report (src/web.rs `evaluate`): warnings, `errors` (only when the program failed) and hints carry `fixes`:
  `{label: "I meant: …", meaning, written, replacement, start, end, edits: [{start, end, replacement}]}` with UTF-16
  offsets of the editor text; `start: null` (and no edits) when a text was not found (the page shows a disabled
  button: loud, not silent). Warnings also carry
  `topic` for "got it".
- Playground (web/playground/playground.js): each fix is a button; clicking applies its edits, the last in the text
  first (CodeMirror `posFromIndex`), and runs again. Next to the fixes, "got it" in the three scopes of got-it-scope (user, 2026-10-05):
  "got it" (this expression: the warning's `expression_key` `topic@expression` joins the page's acknowledged list),
  "got it: all <topic>" (the topic), "// got it" (appends the comment to the warning's line). Notes without a
  warning use the report's `got_it` [{topic, expression}].

## Hints, notes and advice: which call (card let-reassign, 2026-10-10)
All print `hint … prefer <preferred> over <written>` (or `note …` when both are equal) with the reason below, at the
position `normalize::set_position_of(node)` set last (call it first, else the position is stale or empty). Whether
the playground gets an "I meant: <preferred>" button is the `rewrites` flag of `normalize::show_hint`:

| call | fix button | shown | use for |
|---|---|---|---|
| `normalize::hint(written, preferred, reason)` | yes | every time (HintMode::Once: once per text) | a style rewrite (`str(x)` → `x as text`) |
| `normalize::advise(written, preferred, reason)` | **no** | as hint | advice said elsewhere, not a replacement of `written` (`global n` for a block's `n = …`) |
| `diagnostic::educate_once(topic, written, preferred, reason)` | yes | until "got it" (remembered as `ack:<topic>`) | teaching a warp form (`let mut x` → `var x`) |
| `diagnostic::advise_once(topic, …)` | no | until "got it" | teaching advice that replaces nothing |
| `diagnostic::note_alias(written, warp_word)` | yes | until "got it" (topic `alias-<word>`) | another language's word for a warp word (`__add__` → `plus`); educate_once with the reason "warp says …" |

`hint` and `advise` also take a "got it" (topic `hint:<reason>`). All are silent with hints off (WARP_HINTS=0); the
`diagnostic::*` ones also when quiet or silenced by a comment on the line. A hint never stops the program; for
something wrong use a `Diagnostic` (error) or a warning (`diagnostic::ask`). Tests capture hints with
`normalize::capture_hints(|| …)` (tests/node/test_normalization.rs `expect_hint`).

## Inventory: every warning, its default reading and its fixes
| warning / error (topic) | where | default (taken) | fixes offered |
|---|---|---|---|
| `a upto b` (upto) | warp_parser `range_reading` | exclusive | `..<` exclusive, `...` inclusive |
| `for i in 0..n-1` (kotlin-range) | same | exclusive (warp) | `..<`, `...` (Kotlin's inclusive) |
| `1 -1` (signed-operand) | warp_parser `signed_number_starts_a_list` | the list | `1 (-1)` list (also inside `[…]`), `1 - 1` |
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
| `1==1==1` (error) | warp_parser `chained_equality` | none | `1==1 and 1 == 1`, `(1==1) == 1` |
| `3 & 4 == 4` (error) | warp_parser `logic_mixed_with_comparison` | none | `3 & (4 == 4)`, `(3 & 4) == 4` |
| `"a" as int` (error) | warp_parser string literal | none | `codepoint('a') as int` (P74) |
| `'a' as float` (hint; the value is invalid_number) | wasm_emitter `emit_character_cast` | none | `codepoint('a') as float` |
| `c and t or o` | analyzer lint | as written | `if c then t else o` |
| `2 * 1.5 as int` | analyzer lint | the whole | `(2 * 1.5) as int`, `2 * 1.5:int` |
| `-7 % 3` | analyzer lint | Euclidean | `-7 rem 3` (C/Java/JS) |
| loop `it` hides the function's `it` (`for 1..3 {…}` and `for i in xs {…}`) | analyzer `hidden_function_it` | the loop's | `outer_it=it; for … {… outer_it …}` |
| `(1, 2) == 1, 2` | tuples.rs | `((1,2) == 1), 2` | `(1, 2) == (1, 2)` |
| `xs#1..3` | warp_parser `hash_range_warning` | range from xs#1 | `xs#(1..3)` slice, `(xs#1)..3` |
| kebab data key `a-b:2`, a and b variables (P81) | analyzer `kebab_fixes` | the data key | `"a-b":2` (only when nothing reads `a-b` bare); the subtraction `a - b` at every bare read (the key quoted); the key renamed `a_b` at the key and every bare read |
| `for int in xs`, `for Friend in xs`, `for (it>2) in xs` (for-filter) | warp_parser `filtered_body` | the filter | the header `for x in xs.filter(x => x is int)` / `for it in xs.filter(it => it>2)`, the body unchanged (none when the body names the item by the type word) |
| unterminated string, `(`/`[`/`{` open to the end, `name { …` never closed, `‖x` (errors, card more-fixes) | warp_parser `missing_closer` | none | add the closer: a quote at the end of the opening line, a bracket at the end of the input, `‖` at the end of the line |
| `(7 // 2)`: the comment hides the closer (error) | warp_parser `glued_floor_division` | none | `7//2` |
| `\alpha` (error) | atoms.rs | none | `\:alpha` |
| undefined variable / function / word near a name (`coutn`, `sqaure(3)`) (errors) | wasm_emitter `offer_near_name` | none | the near name; an undefined word also `data …` |
| `x is 5` of an undefined x (error) | wasm_emitter `emit_undefined_comparison` | none | `x be 5` |
| `f(nmae: …)` (error) | named_arguments | none | the near parameter |
| `f(x: flaot)` (error) | analyzer `unknown type` | none | the near type (built-in type words, the program's classes) |
| `x:int = 2.5`, `x:int = ø` (errors) | analyzer `assignment_mismatch` | none | `int(2.5)`, `x:float`; `x:int?` |
| `sort x` as a statement (unused copy) | mutation.rs | as written | `x.sort!` |
| style hints, educate_once notes (`let`, `**`, quotes, `&&`, `len(x)`, `x == int`, `a.copy()`, `o.field`) | normalize, lowering | as written | the preferred form (when the text is literally in the source) |

## No fix yet (todo.md)
- type-filter loop whose body names the item by the type word (`for int in xs: print int`): the explicit header
  renames the item, so the body's reads would need edits too.
- `like`-checked data lacking or adding fields (traits.rs `field_warnings`): no value to invent for a missing field.
- task control that never happens (declarations.rs `never_happens`), folder-scope module warnings (modules.rs):
  nothing in the source to rewrite.
- hints whose original is a pattern (`x ? y : z`, `trait name{…}`): `start: null`, a disabled button.

## Errors without an evident fix (card more-fixes, 2026-10-09)
The ~330 error sites (data/more_fixes/sites.txt) were read; these kinds have no single edit to offer:
- several equally plausible edits, none evident: arity errors (`f takes 1 argument, got 2`), `x is a block, no value`
  (run it or assign it), `fits two definitions of f`, `try` needs an `else`, `times` needs a body, a lambda without
  its arrow or body, `{+}` without operands, duplicate keys and twice-defined names (which one to keep), an unmatched
  closing tag, a hard keyword as a name (which other name), class/mixin/trait declarations missing, a `nonlocal`/
  `global` the program needs elsewhere, closures changing a captured variable, const reassignment.
- values the compiler cannot invent: a missing named argument's value, missing data fields (`like`), a constructor
  field to declare.
- run-time and environment failures: modules/packages/includes not found or unreadable, foreign runtimes, FFI headers,
  wasm tools (wasm-opt, metadce, split), channels, tasks (waits forever, closed channels, panics), web/host/database/
  time/calendar/units dimension errors, numeric literal overflow (`Invalid int`, `Exponent too large`).
- prose already naming the form but tied to text the compiler does not have: an unterminated interpolation hole (the
  text's end is unknown), the `where` filter without `it` (the condition is rewritten, not located).
Candidates for later: blocks.rs `name!` (needs the use's position), nonlocal outside a function (remove the line).

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
