# Welcoming syntax

Mantra:
- If the intent is clear, compile it.
- If there is an ambiguity, warn with the default reading and the explicit forms (or refuse when a guess is dangerous).
- If the preferred syntax is different, educate the user.

Limit: implement what a newcomer expects only where it does not clash with a known footgun (wiki/Footguns.md). Where it
clashes, the footgun decision wins and the compiler educates instead. The one exception is text + number
concatenation. Examples: `a[-1]` stays an error with a hint to `last(x)` / `#-1` (silent wrap), `[x]*n` is an
ambiguity error (Python repeats, NumPy multiplies; `n times [x]` repeats), mutating a main-level variable from a
function needs `global`.

"Educate" is a hint naming the preferred warp form, never a refusal.

"Ask" (the name stayed, the question went) is an ambiguity record. The same source always compiles the same way:
nothing asks which reading was meant and no answer is remembered (no context-sensitive execution). Each ambiguity
declares its fallback: a warning (take the default reading, shown until the user says "got it") or an error naming the
explicit forms (too dangerous to guess). Changing the code to the intended form is a separate action (fixes, below).

## Duck typing and `like` (central philosophy)

Truly unknown values are duck typed, as in Python; a value of a known different type that happens to have the fields
is an error that teaches the keyword `like`. What a place declared `photo` accepts, a parameter `keep(p:photo)` and a
typed variable `p:photo = …` alike (assignments to typed variables are checked):

| value | result |
|---|---|
| a photo, or a value of a type declared `like photo` | accepted |
| a value of unknown type (an untyped parameter passed on) | duck typed: accepted, judged by the fields its uses read; a missing field fails loudly ("no field height") |
| written data: an ad hoc name `pic{width:3}` (no `class pic`, so the name is only a label, D4) or a map `{width:3}` | accepted as data without its label; its fields are compared with photo's: a required field it lacks or a field it has extra is a warning ("pic{width:3} lacks field height of photo"), an error under `use strict` |
| a value of a known other type (`image{…}`) | compile error that teaches the keyword: "image is not a photo; fix: declare `image like photo` to use it as one" |
| a known non-object (`3`, `"x"`) | compile error: "keep needs a photo for parameter p, got 3 (an Int)" |

`image like photo` declares that an image may stand wherever a photo is expected. It is a promise, not a conversion:
the image is still judged by its uses, so a field it lacks fails loudly where it is read. Unknown values get the benefit
of the doubt (a newcomer's untyped code runs), known types never mix silently (two declared types that happen to share
fields are a decision the writer states once, in one line). wiki/like.md, test/operators/test_like_known_type_mismatch.warp.

## Forms newcomers write
Samples of standard algorithms written in Python/JS style sit next to idiomatic ones (samples/<name>.warp,
samples/<name>_idiomatic.warp).
- text + number concatenates (`"F:" + 13` → "F:13")
- floor division is `/!` (`7 /! 2`, `x /!= 2`); `//` is always a comment, glued (`7//2`) too, which warns naming `/!`
- `#ident` with no space is count (`#s`); `# text` stays a comment; directives (#use, #include, #!) keep working
- `upto` excludes the end (`..<`/`..` exclusive, `to`/`...` inclusive)
- missing map key `m["Z"]` stays an error (as in Python); `m.get(k)`, `m.get(k, default)`, `k in m`, `m.has(k)` are the soft forms
- an expression of `it` where a function is expected is that function: `xs.map(it * 2)`, `apply(it + 3, 1)`; inside a
  function of one parameter its `it` stays the argument function's

## Ask: how it works (core/analysis/diagnostics.warp)
- An ambiguity is a fact a check diagnoses; its row in warnings.data takes the default reading and names the explicit
  form in its fix column (`warning: <message> at l:c; fix: <explicit form>`); a row in errors.data refuses and names
  every explicit form, for a guess too dangerous to take. It never asks and never remembers a reading.
- A row's 4th column `topic@expression` is what "got it" silences: the playground remembers acknowledged topics in the
  browser (web/playground/playground.js). An acknowledgement silences, it never changes the value.
- Users: `signed-operand` `1 -1` (default the list `[1, -1]`, warning spaced_sign); `local-or-global` (errors.data
  local_or_global: `let n = …` or `global n = …`); and-or, negative-modulo, arithmetic-conversion, optional-result
  (warnings.data). Explicit forms (`..<`, `...`, `to`, `global n`, `[4]`, `.+` …) never warn. test/welcoming/.
- Not ported from the Rust compiler yet: the warnings of `a upto b`, the for-header bound `0..n-1` (kotlin-range),
  and the error fallbacks `list-plus` `[1 2 3]+4`, `bare-list` `a=1 2 3` (both compile today),
  `insert-order`, `suffix-precedence`; `list-times` `[x]*n` is refused as "outside the core", without naming its readings.
- New ambiguities: a fact where the ambiguity is still visible (often the parser, which knows the written form), a row
  giving every reading its explicit form, errors.data only when a wrong guess would silently corrupt results.

## Change the code to the intended form (core/analysis/diagnostics.warp)
Instead of a question that changes what the source means, an action rewrites the source to the explicit form the user
picks, so the file itself says it. The fix is a row's fix column; the playground shows it as an "I meant: …" button
(web/playground/driver.js), after which the warning is gone because the explicit form never warns. Open: `warp fix`, IDE quick fixes, fixes that need several edits (multi-line
`written`, `global n` as a declaration elsewhere).

## Assigning a named constant (P130)
The named numbers (`pi`, `tau`, `euler`, their glyphs) are declared constants, so `pi = …` / `pi := …` is the compile
error "pi is a constant; fix: another name" (it never silently stays π). A field `class circle{pi = 3}` or
`class C{pi:int}` is the type's own field, no message: the type's methods read the field (`f() := pi * r`), everywhere
else `pi` stays the constant. errors.data constant_assignment, test/welcoming/test_constant_shadowing.pairs.warp.

## Control flow as other languages write it
(samples/conditions/control_flow.warp, test/control/test_match_guards_and_finally.warp)
- match guards (Rust, Scala): `n if n < 0 => "negative"` binds n to the subject and holds when the condition does;
  a list pattern takes a guard too (`[a, b] if a > b => …`).
- an assigned match `d = match v {k => …, _ => …}` parses as `match (v {…})`; welcome_forms turns its arrow cases into
  `k: …` / `default: …` as for the statement form.
- `try … catch/else/except … finally {Z}` (Java, Python): `(finally·N = try…; Z; finally·N)`, Z runs either way and
  the value stays the try's or the catch's. parser.warp with_finally.
- `global a, b` (Python) declares both; several `global` lines in a block are no duplicate key.
- English operator words are aliases (samples/hello/natural.warp, wiki/operator.md): `plus`, `minus`, `divided by`,
  `equals`, `is greater than`, `greater than`, `is less than`, `less than`, `is at least`, `is at most`, `through`
  (= `to`), `is in` (= `in`), `for each x in xs`; a word operator needs a blank after it, so `plus(a, b)` stays a call.
  Number words count too (`one plus two` is 3). `10 down to 1` is `reverse(1 to 10)`; letters make ranges
  (`'a' to 'e'`, `c to 'z'` counts codepoints). A parenthesized filter is closed: `(xs where it > 0).slice(0, 1)`
  slices the filtered list.
- a comparison without its left side is a relational pattern in a match arm (`> 100 => "big"`, C#), elsewhere the
  error "`> 100` compares nothing".

## Warning levels (decisions warning-levels, hints-are-warnings)
Three levels: error, warning, hint (core/analysis/errors.data, warnings.data, hints.data). Hints are shown unless the
run asks for none (`--no-hints`, `use relaxed`, `use script`; wiki/warning.md). Hints never fail.
The Rust compiler's weak/severe levels, `--warnings=<level>` and failing on warnings under `--strict` are not ported
(`use strict` is a numbers and JSON mode, core/passes/modes.data); the playground's "show … and up" select still
names hint/weak/normal/severe.
