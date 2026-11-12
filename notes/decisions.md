# Language decisions: an inbox

A language decision lands here as one line (≤160 chars): date, key, the rule, and the card that absorbs it. When that
card closes, the line is deleted. Its home is a wiki passage plus an outcome test; the wiki wins where they differ.
Only language decisions: core OKs, night decisions and one-moment rulings are notes on their card, process rules live
in AGENTS.md or a role file. Under 40 lines, nothing older than 7 days (card decisions-prune, warp-interviewer).
Grep a key or topic, never read it whole. Open questions: notes/open_decisions.md.

## Inbox
- loop-locals: a loop variable is always a fresh local; a function's plain assignment to another file's global is refused: `global x` or `let x` (card loop-variable-shadowing).
- 2026-11-12 of-form-canonical: a one-argument function is `name of type`, `it` the argument (`double of int := it * 2`), all types (card of-form-canonical).
- 2026-11-12 answer-or-act: a function returns a value or changes its arguments, not both; `sort!(xs: list)` returns nothing, a `!` with a result type warns (card answer-or-act).
- 2026-11-12 mutating-needs-bang: `!` marks a changing function for readers; not enforced, functions may change their arguments (card answer-or-act).
- 2026-11-12 fast-flag: typed int is wasm i64 (wraps), untyped numbers unbounded, as built; new: --fast makes untyped ints plain i64, no checks (card int-is-i64).
- 2026-11-12 spaced-data-call: `style { … }` and `f { a: flex }` with a space give the same data call as the glued `f{…}` (spaced-tag: the two spellings are one); warp-reviewer: sound.
- 2026-11-12 js-builtins-declared: JavaScript's own globals (Math, JSON, Array) get declarations so `Math.nonsense(2)` is refused at compile time like `document.nonsense()` (card js-math-nonsense).
- 2026-11-12 user-approved: remove-alarm-rule: `remove h` drops a listener (`on alarm {1}; h = on alarm {2}; remove h; emit alarm` → 1); a listener handle shows as Listener{event, number}. Undo: revert the sugar rule.
- 2026-11-12 user decision, supersedes 11-10 nullish-default: nullish-split: `??` replaces only ø, as in C#/JS/Swift/Kotlin (`m["a"] ?? 0` raises); the words default, otherwise, failing, except and new alias rescue give the fallback on ø or a raise; `or` stays truthiness (card nullish-split). Was: `a ?? b` / `a default b` give b when a is ø or raises (`m["a"] ?? 0` → 0); without it the error stays loud. Undo: revert 674d0ff07f.
- 2026-11-12 user-approved: primitive-arrays: @primitive files get fixed arrays as wiki/array.md spells them (`i64[8]`); later `byte[40]` for C structs. Undo: revert.
- 2026-11-12 user-approved: typed-globals: a typed global keeps its type everywhere (`global cw: int = 1280; cw = "xy"` refused; `level: int = 0; f() := level` reads it). Undo: revert.
- 2026-11-12 user-approved: comment-in-else: a comment line between `}` and `else` keeps the chain. Undo: revert.
- 2026-11-12 user-approved: class-bodies: class bodies keep source positions (errors inside classes name their line). Undo: revert.
- 2026-11-12 user-approved, renamed: core-round-untyped-half: kept, and round_to becomes `round(x, decimals: 2)` (`round(x, 2)` works too; card round-decimals). a decimal literal is exact, so round_to(1.005, 2) → 1.01 and round_to(-1.005, 2) → -1.01. Undo: revert ec1a1535bd.
- 2026-11-12 user-approved: js-globals-checked-when-called: members of the web's interfaces are checked at compile time, JavaScript's own globals (Math, JSON) only when called (wiki/use.md). Undo: revert the wiki line.
- 2026-11-12 user-approved: function-value-call: a function called through a value converts its arguments like a direct call (`g = f; g(1)`, `xs.map(f)`). Undo: revert.
- 2026-11-12 user-approved: loop-in-html: a loop directly inside html{…} gives every item, as in any tag. Undo: revert.
- 2026-11-11 night, user-approved 11-12: constructor-spaces-refused: `person("Ann" 40)` is refused with the fix-it `person("Ann", 40)`, as `f(1 2)` is; constructors read spaces as functions do; old spreading tests go to the triage list.
- 2026-11-11 primitive-global: `global x = 0` in a @primitive file is a mutable wasm global, type from declaration or literal.
- 2026-11-11 no-str: `str` is gone as function and type word (lib/python keeps it); warp writes text.
- 2026-11-11 hex-subtype: hex(n) / `n as hex` / `hex n` give a hex subtype of int shown as 0x7b (lowercase, no quotes); subtypes of number types equal their plain number by value (not Quantity, Angle); colours: argb_hex / rgba_hex.
- 2026-11-11 canvas-value: canvas is a Canvas value (width, height, clear(), resize(w, h)); clear(canvas).
- 2026-11-11 quiet-implicit-uses: runs don't hint implicit standard uses; warp lint/beautify write the use.
- 2026-11-11 foreign-use: `use python json` canonical (source first, like use js / use cargo); `use json from python` an alias; one table of foreign runtimes.
- 2026-11-11 console-shell: the console runs a line through sh only when warp's refusal is an unknown name, marked dim (sh).
- 2026-11-11 complex-shows-imaginary: complex numbers show as 3+4ⅈ.
- 2026-11-11 unit-beside-variable: after a number a unit expression stays units even when a variable has the unit's name (hint); `440Hz` shows as 440Hz.
- 2026-11-11 markup-needs-html kept: a bare element (`div{…}`) outside html{} is data and gets a loud warning naming the fix (`x: html = …`).
- 2026-11-11 whole-arrow: `↑` takes whole exponents only (refusal names `^`); the compiler itself carries no big numbers (extension-row-optional). fast-mode: a `--fast` compile argument drops overflow checks (ints wrap at 64 bits) for programs that want speed over checks; the compiler is built with it. Scope: the program's code and every runtime word of the run (prelude and module runtime words share the switch; modules' own inline typed `a + b` stays checked, being compiled without --fast); a value already big keeps its exact arithmetic, only the promotion on overflow is dropped.
- 2026-11-11 page-runs-live: an animating page run lives until the next Run or Stop, as natively.
- 2026-11-11 sync-fetch: `fetch` waits everywhere, the page included; `go fetch` is the asynchronous form.
- 2026-11-10 test-slots-shared: `warp test` slots stay one machine-wide pool (cores − 2 × builds + 1); no per-run floor.
- 2026-11-10 spaced-unit-precedence: `2 km == 2000 m` follows from juxtaposition multiplying, which binds tighter than comparison; rejoin rules only if that reading fails.
- 2026-11-10 lambda-scope-cells: a lambda's block body is a scope; its blocks share its names (handler-blocks-share holds there too).
- 2026-11-10 is-type-variable: `x is v` tests the type a variable holds; a name assigned again is refused (`type Paint = Color`); no wiki text.
- 2026-11-10 time-of-day-order: a local datetime or a time of day orders against a duration by its time of day (`ref.time >= 6h`).
- 2026-11-10 lambda-broadcast: a lambda applied to a list broadcasts like a named function (`g = x => x*2; g([1 2 3])` is [2, 4, 6]).
- 2026-11-10 aliases-in-rules: a sugar rule's literal word matches every alias spelling of it (`xs.forEach {…}` through the `each` rule); aliases otherwise stay call-only, so data keys and type names never change; the full unification (aliases-unified) is parked: renaming everywhere would rename data keys.
- 2026-11-10 bare-typed-overloads: `name: T := e` is `name(): T := e`, so `sample: int := 7; sample: text := "seven"` overload by result type.
- 2026-11-10 fast-build: `fast` functions get machine arithmetic (i64 wrapping, fast_math), as extensions/fast.warp.
- 2026-11-10 def-arrow-fixit: `def f(x) -> T: body` is refused with the fix "def f(x): T := body".
- 2026-11-10 undo-planned: undo/redo stay a planned feature.
- 2026-11-10 and-or-values: `and`/`or` carry their operands' values, as a general mechanism: `7 and "hi"` is "hi", `0 or 5` is 5.
- 2026-11-10 bool-int-refused: `n: int = true` is refused with the fix `true as int`.
- 2026-11-10 int-list-alias: `xs: int list` is an alias of `xs: [int]`.
- 2026-11-10 it-expression-function: a parameter typed function takes an `it` expression as a lambda, generally.
- 2026-11-10 aliases-unified: one alias mechanism everywhere: `alias a => b` lines (prelude, modules, program; the program's win) feed one table, applied wherever the program doesn't bind the word, class members included; the arrow's target is canonical; lib/aliases.warp keeps only operator and keyword spellings (card prelude-alias-line).
- 2026-11-10 wiki: `test` lines are checked on every run; `width /height` is refused as ambiguous; `law` is marked planned; WARP_DRAW leaves graphics.md.
- synonyms-are-aliases (card test-triage): synonym words of existing ones are aliases in lib/aliases.warp pointing at the canonical word, not deleted: properties/fields/attributes/members of (keys, members), clone (copy), warn (warning), require (use), forEach/for_each/foreach (each), invocable/runnable (callable), previous/was (old), ppi/density (dpi), maybe (optional), unit/nil/empty/string/integer as type names; their held test lines are restored.
- typed-array-declaration: a typed declaration with a size and no value makes a zero-filled list of that type: `pixel: int[100]; pixel#1 = 5`, `x : int[100000]; count(x)` → 100000.
- result-type-as: `as T` after a function head is the default way to write its result type: `def f(x) as text {…}`, `f(x) as int := …`, `area() as number := …`; it is the same meaning as the conversion `as` (the result is taken as T). `: T` stays readable but `warp fix` rewrites it to `as T` everywhere (lib, samples, tests, wiki, the compiler's own code; card result-type-as).

## Process


- #1 avoid panics, errors as values. #14 not-implemented errors unless easy.


## Welcoming, diagnostics and newcomer forms

  - julia-text-times-hint: `"a" * "b"` concatenates with a got-it warning naming `+`; an error only when a text holds a number (`"3" * "4"`).
- foreign-spellings: foreign names are cheap aliases of existing words, not lowering code. Kept: Rust `<T>`/`let mut`, Enum.map/List.map/Seq.sum, Python lambda, variadic lambdas, JS destructured parameters, Ruby lambda/proc/blocks/to_s, C# LINQ, C++ `auto x`. Dropped: OCaml let-in and `fun`, Elixir `def …, do:`, C++ `[](…){…}`. R/PHP forms live in `use R` / `use PHP`. Kotlin `when x { 1 -> … }` = `match` with a warning; `match` takes `->` arms and `else`; `case` is optional before any arm.
- P157 no generic syntax: ported `fn id<T>(x: T)` compiles untyped with a note. P161 no Swift argument labels; ported labels compile with a note.
- P167 Go's `Point{1, 2}` builds `Point(1, 2)` for a known class with a note; unknown names stay tagged data.

## Comments, separators and lexing
- floor-division: `//` is always a comment; floor division is `/!` (`7 /! 2` = 3, `x /!= 2`); where `//` was clearly meant as division (`n = 7//2`), a strong warning names `/!` and `warp fix` rewrites it; `div` warns with the fix `/!` (`div` stays the html tag). A comment hiding a closing bracket is a parse error naming the `//`.
- P143 `f x == y` is an error naming both groupings only when both type-check; P149 an untyped parameter's type is inferred from the body first. Statement words (print, return) take the whole expression.
- call-as-grouping: `f x as T` is `f(x as T)`, with a warning naming `f(x as T)` and `(f x) as T`.
- P189 extra control-flow words stay display-only keywords; ambiguous names need quoting.
- implicit-words-reserved: `it`, `self`, `this` are reserved: binding one warns (error under use strict); as the explicit parameter (`f(it) := …`) it is allowed.

## Numbers

- gpu-f32 (also: P240): heavy float maps (from 10× the measured break-even) run on the GPU in f32 with a one-time notice; `@cpu` and `WARP_GPU=off` keep f64; `@gpu @cpu` together is an error; `@gpu` warns when the GPU doesn't apply; int lists fitting i32 may use the GPU with an overflow flag. go-heavy-map: a `go` map of a heavy kernel runs as the plain map.
- round-halves: `round` of an exact half may go either way; tests never pin it; `round_even` when it matters. `⌊x⌋` floor, `⌈x⌉` ceil, `⌊x⌉` round. `x as int` truncates toward zero.

## Units and quantities
- unit-definitions: `quantity length meter` declares a base and its SI anchor; `unit cm := meter / 100`, `unit N := kg*m/s²` (derived quantities follow from the expression); offsets `unit °F := (it + 459.67) * 5/9 K`; SI prefixes only for SI-anchored/derived units. unit-target: `unit °C as K := it + 273.15`. unit-prefix-lines: `prefix k := 10^3`, `SI dimension length meter`.
- unit-after-number (also: variable-named-like): `m = 5; 3m` is 3 meters with a hint "write 3*m to multiply".
- display-names: display.density (aliases dpi, ppi) in dpi; display.size width × height in px (alias dimensions, with a hint); display.diagonal in inch; display.resolution is an ambiguous alias reading display.size, with a hint.
- unit-types: a unit can type a field (`display { density: dpi }`): it fixes the kind of quantity, converts other units, refuses other quantities. unit-field-plain-number: a plain number there warns with a fix adding the unit.



## Types and annotations

- type-text (also: type-print, time-word's type clause, type-result-shown; revised by real-type-display): a type value shows by its name in every text form (`type(π)` → real, `type(now)` → Instant, a class as declared); warp test prints it as its quoted name.
- P179 sum types auto-unwrap like optionals; payload fields by position `rgb(1,2,3)#1`; a payload-free variant is its symbol and `red is Color`. P178 enum cases with values are sealed classes.

## Variables, binding and charged definitions
- P69b a block arriving at run time is pure by default. P69c `warp compile` with run-time `!` warns and loads a shared compiler lazily.

- spaced-call-parentheses: `f (3)` is the call f(3) as `f 3` is; `f ()` calls f with no arguments as `f()` does (a one-parameter f gets ø), not a list `[f ø]`.
- Ranges: `r: 1..n` is the range itself; `upto` excludes the end with a hint. range-in-brackets: a range inside [ ] is one item, never spread.

## Functions and calls
- reflection: function objects have .name, .signature, .return_type, .parameters, .doc; printing one shows its doc above its typed .dump; `methods(x)` returns the function objects; `use reflection` adds compilation unit, compiler, origin, wasm index, effects. A bare parameterless function inside print/help/dump/type is called, with a warning and hint `function now`.
- receiver-field-assign: `fresh(f: F) := { body = 99 }` writes f.body, same as in a class-body method (one-method-form, P115/P116); `let body = …` makes a local; any warning applies equally in both places.
- random-forms: bare `random` is a float in [0,1) where a value is used; `random T` is a random value of type T (`random bool`, `random int`); `random(n)` 0…n-1; `random(a, b)` whole numbers inclusive, floats [a, b); `random 1..10` picks from the range. Any parameter of type Type accepts bare type words.
- (override-keyword governs redefining a library word; a foreign export is called qualified.) P39 no `id`/`square` builtins; P191 `square` lives in math.

- same-as-result: a result typed like an argument is spelled `same as <param>` (`shuffled(xs): same as xs`); an element of an argument is `element of xs`, `value of m`, `key of m` in type position (`first(xs): element of xs`); no type variables per P157.
- P24 suffix forms (`4 doubled`) call user functions. P164 a type word after a parameter name is its type (`phone number`); `phone-number:text` is one hyphenated name in declarations.
- P48 custom operators: `prefix operator ⁻ := …`, `suffix`, `infix operator ⊕ := left+right`, word operators `infix operator divides(d, n) := …`. P54 only declared operators are ranked (`has precedence above +`). P166 element-wise operators on plain numbers act plainly.
- D6 `|` is one operator dispatched on types: logical or, or pipe (`2|square|root`). operator-shall: `&`/`|` of two whole numbers combine bits; `and`/`or`/`&&`/`||` stay logical. amp-compose: `&` of two functions composes in reading order, `∘` alias. P158 `then` pipes only into a stage missing its argument with no `else`.
- `root` is an alias of sqrt. P2 tuple returns `return a, b` / `x, y = f()` as wasm multi-value. P3 closures keep boxing.

## Lists, maps and indexing
- html-value (Interviewer, by markup-needs-html, card html-anywhere): markup is a value of `class Html { element: pair }` (a cheap class, not a subtype of pair: a pair would fill a subtype and data would count as markup), printed by its own `format` overload, rendered by dispatch on `is Html`. An element written inside html{…}, in an html-typed function, or as the argument of an html-typed parameter (to_html, render, page_render take `value: html`) becomes an Html; a data entry like `"a": "Apple"` stays a pair.
- format-types-inline-html (refines jsx under html-value, card inline-div-format): `warp format` turns inline HTML into warp markup and adds the html type where the element is a whole value: `p = <p>Hi</p>` → `p: html = p{ "Hi" }`, `f() := <div/>` → `f(): html := div{}`; elsewhere (an argument, inside an expression) the inline HTML stays as written.

## Control flow, matching and errors

- match-exhaustive: a match on a known enum or sealed class missing a case is a compile error.
- `when` by shape: `when click {…}` = `on click`, `when x > 3 {…}` is refused naming `if`, `once` and `whenever` (11-11, replaces = whenever), `when x {… -> …}` = `match`, `when {cond -> …}` = an if chain; each warns naming the word meant.
- P46 type-name and condition patterns in `for` filter with a got-it warning (`for friend in xs`, `for (it>2) in xs`).
- fail-effect: the `Fail` effect tracks explicit raises, division by a value that may be 0, and library calls answering with an Error; index out of range, missing fields and failed casts are untracked bugs.

## Classes, objects and enums
- capitalized-field-list: a capitalized plural field named after a declared class declares a list of it under the lowercase name: `class Signature { Parameters = [] }` is `parameters: list[Parameter] = []`, read as `sig.parameters`; otherwise an ordinary field.
- property-accessors: accessors are overloads by arity: getter `canvas.width := canvas_width`, setter `canvas.width(new_width) := …` or with `it`; a getter without setter is read-only (assigning is a loud error).
- setter-calls: a setter is called by `canvas.width = 12`, `canvas.width(12)`, `width(canvas, 12)`; `width of canvas = 12` is `canvas.width = 12`; `set x to y` is `x = y` with a style hint.
- P177 `implements` / `struct Square: Shape` is a checked claim.
- P200c every object has `copy()` and `clone()`. P205 `copy()` is deep; `copy(shallow: true)` shallow. P206 resources inside are shared with a note (error under --strict). P207 `clone` calls `copy`; a copy keeps its class. `===` (taste call, the code): same value and same kind of thing (`1 === 1.0` yes, both numbers; `0 === false` no); `≡` spells it; identity, if ever needed, gets its own word.

## Signals, events and tasks

- P129 `broadcast v on "chat"` / `send v to "chat"` reach `on message from "chat"` machine-wide; P129b named events cross programs. P155 channels: `channel()` local, `channel "chat"` machine-wide.
- P156 `whenever cond {…}` is edge-triggered. P38 listeners see later writes in their statements.
- P120 the CLI keeps running while handlers listen. P121 `exit` ends the run.
- P33 real threads for `go` (WASM threads natively, Web Workers in the browser). P47 a started job runs on its own; only a use of its value waits; `await all jobs`. P153 `await any [...]` races. P154 `await job within 100 ms or 0`.
- go-voice: a `go {}` task is a voice sounding along from where its starter stands. deleted-loop: a live_loop deleted from the source stops on re-evaluation (≤ ~4 s fade).


## Web, markup and server
  - data-tag-words (user + interviewer): samples write markup elements glued, `head{ title: "My Page" }`; inside data, a prelude function before a block (head, first, last, sort) stays a tag; only words the program defines or `use`s make `word{…}` a call (`json{…}` with use json); head of a block in data is written head(…). One check in extensions/quotes.warp is_data_tag, no core.
- P188 stay close to HTML and CSS; only the basic machinery. web-keyframes: CSS's own `@keyframes`; warp adds only `leave:`.
- unknown-tag: an unknown element warns with the near name; custom elements and components are not flagged.
- Served routes: a raising form route answers 400 and re-renders; missing field 400, unknown id 404; /api and /rpc give instances as plain objects. route-access: a route gives out only what it displays.

## Sound
- sound-at: `at 2 beats play C4` places sounds from the voice's first sound.
- shared-buses: a bus is a named effect chain `room = bus() |> reverb(0.4)`; `osc(C4, 1s) |> room`, `play(C4, 1s, bus: room)`; `room.reverb = 0.6` applies from that moment.



## Process and platform decisions, continued
- exact-decimal-big: a decimal literal beyond int64's 10^18 denominator stays exact with an unbounded-int denominator, not a float; a warning names the cost where such a value is used heavily, e.g. inside a loop.


## Later rounds

- of-in-selectors: `count of words in t`, `number of bytes in t`, `first of words in "ab cd"` are kept if they fall out of the right-associative `of` chain (X of Y of t = X(Y(t))) with `in` closing the chain as the source, without special code beside it; otherwise held and brought back to the user. `x in xs` alone stays membership.
- untyped-fit-ambiguous: when a word with an untyped parameter is followed by a phrase that could end before or after an operator (`sq(x) := x*x; sq 2 == 4`), it is an ambiguity error naming both readings with their parentheses: `sq(2 == 4)` or `(sq 2) == 4`; a typed parameter decides by fit (juxtaposition-one-rule), `print x > 0` stays print(x > 0).
- list-insert-one-order: `xs.insert(i, v)` position first, `xs.insert(v, at: i)` by name, no order guessed by type.

- cast-checks (user, refines nat-and-int; generalised by checked-forced-conversions): `x as int` (as i64, every fixed-width int) raises a catchable Error when x does not fit ("too big for int: …"); `x as! int` converts no matter what (wraps); the compiler warns when it knows a value will not fit. Int arithmetic keeps wrapping silently. No `wrapped` word. Warp's `as!` forces the conversion (Swift's traps).
- checked-forced-conversions: `x as T` is a checked conversion, exact or a catchable refusal; `x as! T` a forced conversion, the target type's one declared lossy rule (library: a `forced` overload beside convert), refused where a type has none (enums, parsing text). Ints and bytes wrap; colour channels and audio samples saturate (300 → 255); bytes → text replaces invalid UTF-8 with U+FFFD; map → class drops unknown fields. Floats approximate in both variants (`1/3 as float` rounds). The compiler warns where it knows a checked conversion will refuse.
- units-ignored-by-shaders (user, reinterprets no-units-in-shaders): values a painted (shader) function reads are written with their units (`wind_speed = 7 m/s`); the shader (and the CPU fallback) ignores the unit, receiving the quantity's base SI amount (20 km → 20000); the WGSL pass drops the unit, the CPU fallback pulls units through (math words read a Quantity's SI number through a float() hook).
- root-dispatch-float: with f(int) and f(float), an irrational exact root (sqrt(2)) picks the float variant; a whole root (√9) counts as int.
- rational-not-dispatched: `f(x: int)` and `f(x: rational)` stay the loud refusal "cannot be told apart"; branch inside one f with `if x is rational` (card fails-unknown).
- call-negative-plus-test: `f -2 + 1` is f(-1) (call-takes-right's reading).
- unit-runs-spaced-only (user, core one-liner): phrase rules try runs only in a space-separated group, never across comma items (`[1, s]` stays two items).
- unit-binds-below-power (user, core one-liner): a spaced unit binds looser than ^ and tighter than + and comparisons: `4 m²` is 4*(m²).
- word-operator-overloads (user, ~5 core lines): `infix operator <built-in word>(…)` (as, each, in, to …) overloads that built-in word instead of declaring a new operator (card rewrite-rules).
- loop-scoped-it: a for loop's implicit `it` is its own, gone after the loop.
- typed-parse-message: parse's mismatch says the warp type, "expected object, received list of int".

- arm-symbols: an undeclared capitalised word in a match arm is an unknown name; matching.warp declares enum Day.
- regex-error-from-os (user, "leave as is"): a malformed regex passes the OS's regcomp text through; tests expecting older wording stay red; no warp-owned message table.
- backtick-strings: inside backticks `$x` interpolates the expression `$x`, as outside (a page element's value on a page); a plain variable is `${who}`; `\$` escapes. extensions/, no core.
- markup-attr-call: in markup a call to a known warp word is evaluated (`div(class: join(xs, " "))` → class="a b"); CSS functions (min, max, calc, var, rgb) inside style{} stay text; refused when both fit.
- core-if-branch-float-widening: mixed int/float branches widen to float.

- it-last-result: inside a lambda `it` is the implicit parameter; outside one, `it` is the last statement's value (`3 + 4; it * 2` → 14).
- render-pdf-string: `x = 3 as pdf` with pdf undeclared refuses, "unknown type pdf: use a library that defines it"; an unknown type name is never invented.

- rough-fallback (user, card instances-compared): a class that defines only one of approximately(o) / similar(o) uses it for both ≈ and ~; with both defined each serves its own operator.
- signature-defaults (user, card reflection-dot-forms): help(f) and f.signature show default values (`nearest(x, kind, limit = 1000000, within = 0) -> node`); Rust-era expectations are upgraded to the typed lib signatures (`(y:float, x:float) -> float`); an alias answers with its target's signature.
- style-ul (Interviewer): a CSS descendant selector is written as a text key, `"ul .done": {…}`, as `"nav a": {…}`; no parser change; card should-dot decides whether a dot spaced before and glued after starts a new item everywhere (the glued-negative whitespace rule).

- block-definitions (user, parked): a definition inside a block (if, while, for, bare {…}) is refused with the fix "define f outside the block, or write f = y => …"; scoped functions are out of scope (card inside, Later).
- bar-never-or: `|` is never the logical `or` (no short-circuit, no or-node in the parser): bit-or between numbers, a pipe into a function, a union between types; logic is written `or`.
- rgb-color-literal (user; refined by color-text-css): an RGB color is `color(0xFF8000)` (`rgb(0xFF8000)` the same word with one argument): 6 hex digits are opaque, 8 digits carry alpha last in CSS order (0xFF800080); `0xFF8000 as Color` follows the same rule; no CSS text form, no bare `#FF8000`; lib/color only. color-text-css: a color prints in CSS order, alpha last: `with_alpha(red, 0.5)` → #FF000080; `color(0xFF000080)` is red at half alpha; the stored int layout is internal.
- d-and-clock-left: `d` stays as today (glued `2d` the double suffix, spaced `2 d` unknown); `15:00` stays the pair (only `at 9:30 {…}` reads a time); tests asking otherwise are ignored; now.time / now.date work.
- operator-unicode-aliases (user; details: Interviewer): an operator's Unicode character names are fallback aliases (`≤` also less_than_or_equal_to, `↑` upwards_arrow, `·` middle_dot) as long as the tables stay small: only the names of symbols the alias table already spells (~20 names), generated once by a script from Unicode's name list into lib/aliases.warp, a separate generated `names` field read as aliases, resolved when the alias table is built into the module image; never a Unicode database in the compiler, no names on keyword rows (and, or, not, xor, to), no typographic look-alikes, never infix words; a program's own binding of such a name wins.

- ctrl-c-runs-exit: ctrl-c, kill or SIGTERM runs `on exit {…}` (when there is no `on interrupt`), then ends with status 130 (143 for kill); a second ctrl-c within a second ends at once.
- threaded-handlers: real parallelism is the `go` task (go-unified) plus `shared` atomic values (P106, lib-oddities): `shared n = 0; on every 50 ms go { n += 1 }` runs the handler as a task on its own thread; it sees only shared values, the rest are copies; simple scenarios natively first, the page cooperative.
- rounding-text-overload: dropped for good, never asked again: `round("2.7")` stays refused; write `round("2.7" as float)`. (Over the 3-line limit: text overloads beside the runtime and libm variants made calls ambiguous.)
- use-path-literal (FYI, 4 core lines, card shader-quote-path-uses): `use ./x.warp` / `include ./x.warp` take the path literal; lib/quote_helper.warp moves to test/web/quote_helper.warp.
- enum-constants-typed (Interviewer, ~5–10 parser lines, card one-on): a library constant holding an enum case, `ESC = Key.escape`, is typed for rules like `km = Unit("km")`; `on X {…}` is a named event only when X is unbound and no library constant, else listened(X, handler).
- traits-are-protocols: no separate trait concept: a protocol (a lowercase type word) may give its methods default bodies, and a class that fits it gets them (`shape { area(); describe() := "area " + text(area()) }`); two protocols giving the same default are refused unless the class defines the method itself. No new keyword.
  - dot-call answers (user, core OK for notes/dot-call-plan.md steps 1–5, then 8): `o.f(3)` with a function-valued field f calls that field (fields first; the global is `f(o, 3)`); on untyped data the field wins (`{"count": 3}.count` is 3, the word is `count(m)`; keeps_word goes); any function in scope works by dot, local closures included (`g = x => x*2; 3.g` is 6); `x.w = v` without a field w calls a setter overload `w(x, v)` if one takes x, else a map gets the entry and an instance a warned extra field. Dot on a whole number (`16.sqrt`, `3.type`) falls out of the rule and stays (Interviewer, the user unsure: refusing would be a special case); `1.e5`-style exponents stay numbers.
  - map-methods-first: own methods first, then fields, then functions: a map's protocol methods (keys, values, entries, get, has, set, remove) beat an entry of that name (`m.keys` is the key list even when m holds "keys"; that entry is `m["keys"]`); other entries stay field-first (`{"count": 3}.count` is 3).
  - setters-declared (Interviewer, narrows the dot-call Q4 setter answer): `x.w = v` calls a setter only where x is statically an instance of a class with its own one-parameter method `w(v)` or a function whose first parameter is declared that class (`double(p: P, v)`); maps and untyped values keep the entry write (`m.merge = 3` adds an entry, never merge(m, 3)). An extension rule, no core. Step 8 (canonical w(x, …) for dot calls) parked on card dot-call-canonical until the rule engine matches a call and its dot form in one pattern; step 5 (node parts as global words) dropped.
- big-sweeps (user 2026-11-12 evening, integrator): each landing sweep takes every land/<card> branch waiting, never one sweep per card; workers get more cards and hand off as soon as they finish. Why: sweeps 18-31 landed ~30 cards with no red batch and no halving; a sweep costs 10-20 min whatever its size. Undo: back to small sweeps if batches start going red often (halving then costs log2 N extra tier 0 runs). Double down: wait for a fuller queue before sweeping, re-enable the cloud queue (.github/workflows/land.yml, disabled: a 2-core runner took 1.5 h a batch) on a faster runner, or parallel tier 0 runs. Where: notes/agents/integrator.md "Big sweeps", scripts/integrate/merge.sh (a737b9f59c also added a warp test probe to tier 0).
