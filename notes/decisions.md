# Language decisions: an inbox

A language decision lands here as one line (≤160 chars): date, key, the rule, and the card that absorbs it. When that
card closes, the line is deleted. Its home is a wiki passage plus an outcome test; the wiki wins where they differ.
Only language decisions: core OKs, night decisions and one-moment rulings are notes on their card, process rules live
in AGENTS.md or a role file. Under 40 lines, nothing older than 7 days (card decisions-prune, warp-interviewer).
Grep a key or topic, never read it whole. Open questions: notes/open_decisions.md.

## Inbox
- 2026-11-12 int-is-i64: a plain int is wasm i64, overflow wraps, no checks; big integers are number/exact/nat; a warning may suggest number (card int-is-i64).
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
- 2026-11-11 night, user-approved 11-12: method-let-local: a let/var/const name in a method body stays local, never bound to a field (`fresh() := { let body = 99; body }` leaves f.body 42).
- 2026-11-11 night, user-approved 11-12: member-before-library: a receiver whose keys are known at compile time (map literal, class) reads its own member before a library word (`p = {first: 1}; p.first()` → 1); untyped receivers keep the library word, no run-time member check.
- 2026-11-11 night, user-approved 11-12: constructor-spaces-refused: `person("Ann" 40)` is refused with the fix-it `person("Ann", 40)`, as `f(1 2)` is; constructors read spaces as functions do; old spreading tests go to the triage list.
- 2026-11-11 primitive-global: `global x = 0` in a @primitive file is a mutable wasm global, type from declaration or literal.
- 2026-11-11 underscore-is-it: `_` is an alias of `it`, the last result at the top level (`3+4` then `_ * 2` → 14, got-it note naming `it`); seeded random sequences are not spec, only "the same seed repeats the numbers".
- 2026-11-11 no-str: `str` is gone as function and type word (lib/python keeps it); warp writes text.
- 2026-11-11 hex-subtype: hex(n) / `n as hex` / `hex n` give a hex subtype of int shown as 0x7b (lowercase, no quotes); subtypes of number types equal their plain number by value (not Quantity, Angle); colours: argb_hex / rgba_hex.
- 2026-11-11 canvas-value: canvas is a Canvas value (width, height, clear(), resize(w, h)); clear(canvas).
- 2026-11-11 quiet-implicit-uses: runs don't hint implicit standard uses; warp lint/beautify write the use.
- 2026-11-11 foreign-use: `use python json` canonical (source first, like use js / use cargo); `use json from python` an alias; one table of foreign runtimes.
- 2026-11-11 console-shell: the console runs a line through sh only when warp's refusal is an unknown name, marked dim (sh).
- 2026-11-11 complex-shows-imaginary: complex numbers show as 3+4ⅈ.
- 2026-11-11 unit-beside-variable: after a number a unit expression stays units even when a variable has the unit's name (hint); `440Hz` shows as 440Hz.
- 2026-11-11 markup-needs-html kept: a bare element (`div{…}`) outside html{} is data and gets a loud warning naming the fix (`x: html = …`).
- 2026-11-11 self-contained-executables: `warp build` makes an executable that carries the runtime and prelude it needs and reads no lib/.
- 2026-11-11 whole-arrow: `↑` takes whole exponents only (refusal names `^`); the compiler itself carries no big numbers (extension-row-optional). fast-mode: a `--fast` compile argument drops overflow checks (ints wrap at 64 bits) for programs that want speed over checks; the compiler is built with it. Scope: the program's code and every runtime word of the run (prelude and module runtime words share the switch; modules' own inline typed `a + b` stays checked, being compiled without --fast); a value already big keeps its exact arithmetic, only the promotion on overflow is dropped.
- 2026-11-11 page-runs-live: an animating page run lives until the next Run or Stop, as natively.
- 2026-11-11 sync-fetch: `fetch` waits everywhere, the page included; `go fetch` is the asynchronous form.
- 2026-11-10 test-slots-shared: `warp test` slots stay one machine-wide pool (cores − 2 × builds + 1); no per-run floor.
- 2026-11-10 spaced-unit-precedence: `2 km == 2000 m` follows from juxtaposition multiplying, which binds tighter than comparison; rejoin rules only if that reading fails.
- 2026-11-10 lambda-scope-cells: a lambda's block body is a scope; its blocks share its names (handler-blocks-share holds there too).
- 2026-11-10 is-type-variable: `x is v` tests the type a variable holds; a name assigned again is refused (`type Paint = Color`); no wiki text.
- 2026-11-10 time-of-day-order: a local datetime or a time of day orders against a duration by its time of day (`ref.time >= 6h`).
- 2026-11-10 block-call-parked: a glued `{…}(…)` is not a call yet (parked).
- 2026-11-10 lambda-broadcast: a lambda applied to a list broadcasts like a named function (`g = x => x*2; g([1 2 3])` is [2, 4, 6]).
- 2026-11-10 warp-sound-env: `WARP_SOUND=0 warp app.warp` mutes a run (`warp --mute` sets it), the one environment variable kept as an interface (exception to no-bash-exports).
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

  - julia-pair-range: `1:4` stays a pair; iterated (for, sum, comprehensions) a pair of two numbers is the range.
  - julia-text-times-hint: `"a" * "b"` concatenates with a got-it warning naming `+`; an error only when a text holds a number (`"3" * "4"`).
- foreign-spellings: foreign names are cheap aliases of existing words, not lowering code. Kept: Rust `<T>`/`let mut`, Enum.map/List.map/Seq.sum, Python lambda, variadic lambdas, JS destructured parameters, Ruby lambda/proc/blocks/to_s, C# LINQ, C++ `auto x`. Dropped: OCaml let-in and `fun`, Elixir `def …, do:`, C++ `[](…){…}`. R/PHP forms live in `use R` / `use PHP`. Kotlin `when x { 1 -> … }` = `match` with a warning; `match` takes `->` arms and `else`; `case` is optional before any arm.
- P157 no generic syntax: ported `fn id<T>(x: T)` compiles untyped with a note. P161 no Swift argument labels; ported labels compile with a note.
- P167 Go's `Point{1, 2}` builds `Point(1, 2)` for a known class with a note; unknown names stay tagged data.
- phrase-functions: a definition head may be a phrase with holes: `send v to c := c.send(v)`, `on every period (body: block) := timer_every(period, body)`, `await x within t or d := …`, `set field of object to value := …`; one phrase-row mechanism like the operator rows (~60–120 core lines), replacing ~35 lib rewrite rules (tasks, timers, signals, channels, english, which stay as such declarations).

## Comments, separators and lexing
- floor-division: `//` is always a comment; floor division is `/!` (`7 /! 2` = 3, `x /!= 2`); where `//` was clearly meant as division (`n = 7//2`), a strong warning names `/!` and `warp fix` rewrites it; `div` warns with the fix `/!` (`div` stays the html tag). A comment hiding a closing bracket is a parse error naming the `//`.
- P227 a comma next to `==` without parentheses is an error asking for them.
- P68 a braceless call whose argument holds an operator (`square 3 + square(4)`) warns naming both groupings. The prelude signature decides it: a word giving nothing (`: nothing`) has no second grouping and never warns. output-words-nothing: the output words (print, puts, puti, putl, putf) all give nothing.
- P143 `f x == y` is an error naming both groupings only when both type-check; P149 an untyped parameter's type is inferred from the body first. Statement words (print, return) take the whole expression.
- call-as-grouping: `f x as T` is `f(x as T)`, with a warning naming `f(x as T)` and `(f x) as T`.
- P189 extra control-flow words stay display-only keywords; ambiguous names need quoting.
- implicit-words-reserved: `it`, `self`, `this` are reserved: binding one warns (error under use strict); as the explicit parameter (`f(it) := …`) it is allowed.
- data-reserved: `data` is a keyword (the `data …` quoting prefix); a variable named data is an error.

## Numbers
- D16 `byte`, `int8..int32` are declared FFI types that trap on overflow; `u8` = `uint8` = `byte` (`int` and `nat`: nat-and-int). i64-wraps: `i64`/`int64` wrap silently at 2^63 like `fast int`; `use strict` catches it.

- hex-printing: `255 as hex` is an int shown as 0xFF; arithmetic gives a plain int. binary-int: `5 as binary` shows 0b101.
- gpu-f32 (also: P240): heavy float maps (from 10× the measured break-even) run on the GPU in f32 with a one-time notice; `@cpu` and `WARP_GPU=off` keep f64; `@gpu @cpu` together is an error; `@gpu` warns when the GPU doesn't apply; int lists fitting i32 may use the GPU with an overflow flag. go-heavy-map: a `go` map of a heavy kernel runs as the plain map.
- #11 exact reals may go into a declared float with precision loss. rational-float: `x: rational = sqrt(2)` is a compile error, `x: real = sqrt(2)` works.
- lossy-return (also: float-result): a fraction reaching a declared int result is an error ("declare number, or truncate with as int"). P49/P49b a float, even a written `2.0`, to an int parameter is a compile error. P131 an int variable or field refuses a fraction loudly. P127 int class fields keep exact ints.
- round-halves: `round` rounds halves away from zero; banker's rounding is `round_even`; an exact half seen at compile time gets a hint about Python. `⌊x⌋` floor, `⌈x⌉` ceil, `⌊x⌉` round. `x as int` truncates toward zero.
- P226/P226b log glyphs: `b⌞x` and `x⌟b` are log base b; postfix `x⌟` is ln. easy-add: `x↑n` power and `x↓b` log are typed infix operators for numbers, other types define their own.
- P57 hyperreals: `1/(1+ε)` stays an error; dual numbers later. P56(2-4) `∞` is f64 infinity, ω the hyperreal infinite.

## Units and quantities
- dimensions-units: quantity names are types (`3 cm is length`; `h: length = 3 s` errs); the time quantity is `duration`; units live in lib/units.warp; a unit after a list applies to each element (`[3, 4] cm`). Predefined derived names: speed, area, volume, frequency.
- unit-definitions: `quantity length meter` declares a base and its SI anchor; `unit cm := meter / 100`, `unit N := kg*m/s²` (derived quantities follow from the expression); offsets `unit °F := (it + 459.67) * 5/9 K`; SI prefixes only for SI-anchored/derived units. unit-target: `unit °C as K := it + 273.15`. unit-prefix-lines: `prefix k := 10^3`, `SI dimension length meter`.
- Unit symbols bind only after a number, never as bare names; a variable gets a unit by `x * 1 m`.
- unit-after-number (also: variable-named-like): `m = 5; 3m` is 3 meters with a hint "write 3*m to multiply".
- display-names: display.density (aliases dpi, ppi) in dpi; display.size width × height in px (alias dimensions, with a hint); display.diagonal in inch; display.resolution is an ambiguous alias reading display.size, with a hint.
- unit-types: a unit can type a field (`display { density: dpi }`): it fixes the kind of quantity, converts other units, refuses other quantities. unit-field-plain-number: a plain number there warns with a fix adding the unit.

## Text
- digit-as-char: `2 as char` is the digit '2', like P25 `int('5')` is 5; a code point becomes a character by `char(0x41)`.
- #35 lists join to "[1 2]"; `"x" as float` is a loud error. P74 `codepoint(c)` (aliases ord, ordinal, P27) gives an Int; P25 `int('5')` is 5; `int(c)` of a non-digit is invalid_number; P65 arithmetic on a character is not_a_number.
- An untyped parameter given a one-character text takes it as Text. P31 the Printable operation is `text(p:person)`, the one allowed type-word function name. P32 `print xs` prints its str(). P41 `print "x changed to " value` joins without a separator after a text literal; commas join with a space.
- chars-codepoints: `"abc".chars` is a list of characters printing `['a' 'b' 'c']`; a one-character text is a character; `'a' == "a"`. A character is a grapheme; code points are `.codepoints` ("שָלוֹם": 4 characters, 6 code points). text-unit-plurals: every plural unit word is the list (also as a cast: `t as words` = `t.words`) (`.chars`, `.codepoints`, `.graphemes`, `.bytes`); counting is `count t.codepoints`; `.bytes` is the byte list of any value, size `x.bytes.count` / `byte_size(x)`.


## Types and annotations

- type-text (also: type-print, time-word's type clause, type-result-shown; revised by real-type-display): a type value shows by its name in every text form (`type(π)` → real, `type(now)` → Instant, a class as declared); warp test prints it as its quoted name.
- unknown-type: an unknown declared type is an error with the near type as fix.
- return-as (also: union-return): return types are `def f(x) as int or text`; `->` works with a hint; no `returns`.
- empty-field: ø needs `?` everywhere (`class Tree{left: Tree}` errs, fix `left: Tree?`).
- P185 `none(xs, f)`, the null spelling with arguments, is an error naming the null.
- P125 optional parameters: `x=ø`, `maybe x`, `x?` alike; typed `x:int?`; `a ?? b` only for ø.
- P179 sum types auto-unwrap like optionals; payload fields by position `rgb(1,2,3)#1`; a payload-free variant is its symbol and `red is Color`. P178 enum cases with values are sealed classes.
- P28 `real x;` read before assignment is its type's zero value.
- type-words-reserved (also: P23, P20): every type word is reserved as a function name (`double(x) := …` errs, "name it to_double"), except P31's `text(p)`. type-word-local-warns: binding a variable to a type word (`symbol = ":"`) warns, since it shadows the type in `is`/`as`.
- list-cast-each: `as` a list type converts each item like `as` its item type (`["1", "2"] as ints` → [1 2], `[1, 2] as texts` → ["1" "2"]; an item that can't convert is that item's error); a declared result type `: ints` checks, never converts.
- #9 `list of int`; plural type words (`numbers`) are lists; `list<int>` reads too; `x : 100 int` and `pixel:int[100]` declare typed arrays.
- index-position: `index` (from 0, `xs[i]`) and `position` (from 1, `xs#p`) are int types; index_of, argmax, argmin, insert use them by declared type; position ± int stays a position, position − position is a count, position + position an error; a missing element is an error, never -1 (notes/index_position.md). index-units: base `index` from 0 (`offset` its synonym), `position` from 1; unit: characters (graphemes) by default, `byte` as the adjective (`byte index`, `byte position`). One word `find`, its result chosen by the receiver's declared type: `position p = "a👍🏽b".find "b"` 3, `index i = …` 2, `byte index b = …` 9; untyped a position. Byte and character indexes do not mix without a conversion. `byte_index_of` stays as the explicit word. A bare `index_of` on a text works as find with a warning; on lists `index_of` stays. core writes the explicit forms; lib/ parameters named offset are renamed so the type word does not shadow them. Card index-of-bytes.
- schema-as: `x as Config` converts field by field or errs naming the field; `x is Config` stays strict.
- D10/P17 dispatch on return type (`render "hello" as pdf`); ambiguous → got-it warning (notes/dispatch.md).
- function-type: `type(gcd)` prints `function: (a, b) -> int`. parse-type / body-code: quoted program parts (`parse("1+1")`, `f.body`) have type `code`, print their source, keep `.op`.

## Variables, binding and charged definitions
- op-assign-target-once: `x[f()] op= y` evaluates the target's parts once, as C, Python, JS.
- P159 `const x=7; x=7` warns "remove the redundant assignment". P130 pi and named constants are constants; a class field named pi is the class's own.
- P69b a block arriving at run time is pure by default. P69c `warp compile` with run-time `!` warns and loads a shared compiler lazily.
- time-word (also: print-time, time-now, anchor-time): bare `time` is the type; instant, date, local time, zoned time, duration are its subtypes; `now.time` is the time of day, `now.date` the date; `now > 17h` / `now >= October` compare the matching part, `==` stays exact; `time > 18h` is an error with the fix `now > 18h`.

- #23 a loop's `it` shadows a function's `it`, with a warning. P174 `mk(k) := { it * k }`: `it` stays the parameter, with a warning. P239 in a lambda naming its parameter, `it` is the outer `it`.
- spaced-call-parentheses: `f (3)` is the call f(3) as `f 3` is; `f ()` calls f with no arguments as `f()` does (a one-parameter f gets ø), not a list `[f ø]`.
- Ranges: `r: 1..n` is the range itself; `upto` excludes the end with a hint. range-in-brackets: a range inside [ ] is one item, never spread.

## Functions and calls
- postfix-words: two rules. (1) backward application: a phrase of exactly two items, a value then a known one-parameter function, is the call, `4 square` → 16, `xs sorted` → [1, 2, 3]; one typed sugar rule (typed-rule-holes, `($x: any) ($f: any -> any) => $f($x)`), never inside `[…]` (`[1 f]` stays a list), a function first keeps prefix meaning (`square 4`). (2) an unbound word ending in -ed/-d looks up its stem as a one-parameter function with a got-it note (`xs reversed` → reverse(xs), `x doubled` → double(x)); two matching stems refuse naming both; at most ~20 core lines in name lookup, else parked.
- map-all-test: the test reads as English, `all xs are even` and `are all xs even`, `any xs is even` and `is any xs even` (extensions/sugar/broadcasting.warp); the call all(xs, f) / xs.all(f) stays yes/no.
- words-as-values: a function word used as a value is the lambda of its call (`f = sqrt; f(16)` 4, `apply(floor, 2.5)` 2), bringing its module; with several arities the one its use needs, never silently the first.
- reflection: function objects have .name, .signature, .return_type, .parameters, .doc; printing one shows its doc above its typed .dump; `methods(x)` returns the function objects; `use reflection` adds compilation unit, compiler, origin, wasm index, effects. A bare parameterless function inside print/help/dump/type is called, with a warning and hint `function now`.
- receiver-field-assign: `fresh(f: F) := { body = 99 }` writes f.body, same as in a class-body method (one-method-form, P115/P116); `let body = …` makes a local; any warning applies equally in both places.
- random-forms: bare `random` is a float in [0,1) where a value is used; `random T` is a random value of type T (`random bool`, `random int`); `random(n)` 0…n-1; `random(a, b)` whole numbers inclusive, floats [a, b); `random 1..10` picks from the range. Any parameter of type Type accepts bare type words.
- function-references: a bare function name compared with a type or in `type(f)` is the function itself; `.floor` in argument position is a lambda (`xs.map(.floor)`, `people.sort(.age)`); `&floor` and `function floor` are the explicit references.
- declared-belonging: calling stays uniform (`f(x)` ≡ `x.f()` for every function, silently), but belonging is declared, not inferred from the first parameter: a function is a method of a type (dir, completion, docs, the playground's method list) only when declared in the class, the type's own module, or an extension; any other function is a free function of its module, dot-callable where that module is used (P171). Reflection keeps both: `Text.methods` the declared ones, `Text.functions` every function taking a Text. A type alias carries its own methods (`type Name = text; argument_result(n: Name)` is a method of Name, not of text). class-extensions: outside code adds a method by its dotted name `text.shout() := upper(it) + "!"` (canonical) or groups several in `extend text { shout() := …; whisper() := … }`.
- module-alias: an `alias pw = power` line in a program or a directly run file adds a name and never shadows one (`alias exp = exponential` beside a builtin exp leaves exp the builtin).
- (override-keyword governs redefining a library word; a foreign export is called qualified.) P39 no `id`/`square` builtins; P191 `square` lives in math.
- reflection-plain: one builtin `meta(x): map` gives what the compiler knows about a class, module or function; lib/reflection.warp builds typed views (`.name .fields .methods .parent`, `.words .doc`) and dir/fields/methods/help on it. `help(x)` gives the text; the REPL/program value prints it, mid-program `print help(x)`.

- same-as-result: a result typed like an argument is spelled `same as <param>` (`shuffled(xs): same as xs`); an element of an argument is `element of xs`, `value of m`, `key of m` in type position (`first(xs): element of xs`); no type variables per P157.
- P24 suffix forms (`4 doubled`) call user functions. P164 a type word after a parameter name is its type (`phone number`); `phone-number:text` is one hyphenated name in declarations.
- P48 custom operators: `prefix operator ⁻ := …`, `suffix`, `infix operator ⊕ := left+right`, word operators `infix operator divides(d, n) := …`. P54 only declared operators are ranked (`has precedence above +`). P166 element-wise operators on plain numbers act plainly.
- values-number: a system value is `$mouse.x`; `$mouse_x` hints `mouse.x`.
- D6 `|` is one operator dispatched on types: logical or, or pipe (`2|square|root`). operator-shall: `&`/`|` of two whole numbers combine bits; `and`/`or`/`&&`/`||` stay logical. amp-compose: `&` of two functions composes in reading order, `∘` alias. P158 `then` pipes only into a stage missing its argument with no `else`.
- `root` is an alias of sqrt. P2 tuple returns `return a, b` / `x, y = f()` as wasm multi-value. P3 closures keep boxing.
- D4 constructors: `T{…}` of a known type constructs and validates, `k:{…}` is plain data; missing fields are errors unless `?` or defaulted. #41 spaced `V { x: 1 }` constructs a declared V.

## Lists, maps and indexing
- html-value (Interviewer, by markup-needs-html, card html-anywhere): markup is a value of `class Html { element: pair }` (a cheap class, not a subtype of pair: a pair would fill a subtype and data would count as markup), printed by its own `format` overload, rendered by dispatch on `is Html`. An element written inside html{…}, in an html-typed function, or as the argument of an html-typed parameter (to_html, render, page_render take `value: html`) becomes an Html; a data entry like `"a": "Apple"` stays a pair.
- format-types-inline-html (refines jsx under html-value, card inline-div-format): `warp format` turns inline HTML into warp markup and adds the html type where the element is a whole value: `p = <p>Hi</p>` → `p: html = p{ "Hi" }`, `f() := <div/>` → `f(): html := div{}`; elsewhere (an argument, inside an expression) the inline HTML stays as written.
- P29 `pair.0` counts from 0. P30 `xs.pop()` gives the last item, `m.remove(k)` the value or ø.
- Word slices: `xs from #2 to #4`, `xs starting from #2`, `xs up to #2` or ordinals, both ends inclusive; a bare number is an error. P58 `xs#a..b` keeps the range with a strong warning naming `xs#(a..b)` and `(xs#a)..b`.
- P176 repeated `key: value` in a named tag block are children; a plain `{…}` keeps the duplicate-key error.

## Control flow, matching and errors

- match-exhaustive: a match on a known enum or sealed class missing a case is a compile error.
- `when` by shape: `when click {…}` = `on click`, `when x > 3 {…}` is refused naming `if`, `once` and `whenever` (11-11, replaces = whenever), `when x {… -> …}` = `match`, `when {cond -> …}` = an if chain; each warns naming the word meant.
- P46 type-name and condition patterns in `for` filter with a got-it warning (`for friend in xs`, `for (it>2) in xs`).
- P61 `is` is always a comparison; with an undefined x it teaches `x be <value>`. #27 `be` is `:=`; `≤ ≥ ≠ × ÷ ¬ √` are accepted and normalized (÷ is floor division: julia-floor-divide).
- fail-effect: the `Fail` effect tracks explicit raises, division by a value that may be 0, and library calls answering with an Error; index out of range, missing fields and failed casts are untracked bugs.
- error-classes: errors are classes: the prelude declares `class Error` and the run-time errors as subclasses with Python names (IndexError, ZeroDivisionError, …); users declare their own (`class ParseError: Error`), an undeclared error name is a compile error with that hint. `except T as e` catches T and its subclasses; `except Error`, bare `except:` and `catch e` catch everything.

## Classes, objects and enums
- capitalized-field-list: a capitalized plural field named after a declared class declares a list of it under the lowercase name: `class Signature { Parameters = [] }` is `parameters: list[Parameter] = []`, read as `sig.parameters`; otherwise an ordinary field.
- property-accessors: accessors are overloads by arity: getter `canvas.width := canvas_width`, setter `canvas.width(new_width) := …` or with `it`; a getter without setter is read-only (assigning is a loud error).
- setter-calls: a setter is called by `canvas.width = 12`, `canvas.width(12)`, `width(canvas, 12)`; `width of canvas = 12` is `canvas.width = 12`; `set x to y` is `x = y` with a style hint.
- P177 `implements` / `struct Square: Shape` is a checked claim.
- P200c every object has `copy()` and `clone()`. P205 `copy()` is deep; `copy(shallow: true)` shallow. P206 resources inside are shared with a note (error under --strict). P207 `clone` calls `copy`; a copy keeps its class. `===` (taste call, the code): same value and same kind of thing (`1 === 1.0` yes, both numbers; `0 === false` no); `≡` spells it; identity, if ever needed, gets its own word.
- enum-identity: each enum case is one unique value; `==` with a case is identity, plus its spelling as text (`Op.Add == "+"`, `Coin.Dime == "Dime"` yes); cases of different enums never equal (`Level.Error == ValueKind.Error` no); text becomes a case only by `"Int" as ValueKind` (`symbol("Int") == ValueKind.Int` no).
- field-or-minus: `p.phone-number` without that field is "p has no field phone-number" with the fix `p.phone - number`.

## Signals, events and tasks

- P129 `broadcast v on "chat"` / `send v to "chat"` reach `on message from "chat"` machine-wide; P129b named events cross programs. P155 channels: `channel()` local, `channel "chat"` machine-wide.
- P156 `whenever cond {…}` is edge-triggered. P38 listeners see later writes in their statements.
- P120 the CLI keeps running while handlers listen. P121 `exit` ends the run.
- P33 real threads for `go` (WASM threads natively, Web Workers in the browser). P47 a started job runs on its own; only a use of its value waits; `await all jobs`. P153 `await any [...]` races. P154 `await job within 100 ms or 0`.
- go-voice: a `go {}` task is a voice sounding along from where its starter stands. deleted-loop: a live_loop deleted from the source stops on re-evaluation (≤ ~4 s fade).

## Modules, library and foreign code
  - or-fallback: `a or b` is b when a raises, else `or` as before, falsy values included: `0 or 10` is 10.

## Web, markup and server
  - data-tag-words (user + interviewer): samples write markup elements glued, `head{ title: "My Page" }`; inside data, a prelude function before a block (head, first, last, sort) stays a tag; only words the program defines or `use`s make `word{…}` a call (`json{…}` with use json); head of a block in data is written head(…). One check in extensions/quotes.warp is_data_tag, no core.
- P188 stay close to HTML and CSS; only the basic machinery. web-keyframes: CSS's own `@keyframes`; warp adds only `leave:`.
- unknown-tag: an unknown element warns with the near name; custom elements and components are not flagged.
- Served routes: a raising form route answers 400 and re-renders; missing field 400, unknown id 404; /api and /rpc give instances as plain objects. route-access: a route gives out only what it displays.

## Sound
- sound-at: `at 2 beats play C4` places sounds from the voice's first sound.
- shared-buses: a bus is a named effect chain `room = bus() |> reverb(0.4)`; `osc(C4, 1s) |> room`, `play(C4, 1s, bus: room)`; `room.reverb = 0.6` applies from that moment.



## Process and platform decisions, continued
- pair-map-equality: `{a:1} == (a:1)` but `{a:1} !== (a:1)`: a one-entry map and a pair are equal loosely (==), not strictly (=== compares the type too, P196); `!==` and `=!=` both spell strict not-equal.
- exact-decimal-big: a decimal literal beyond int64's 10^18 denominator stays exact with an unbounded-int denominator, not a float; a warning names the cost where such a value is used heavily, e.g. inside a loop.


## Later rounds

- color-show-variants (user, refines color-prints): Color's own text form is `format Color c as text` in lib (format-as-text; nothing color-specific elsewhere). Besides the default (a name, else #RRGGBB), a color can be shown as hexadecimal (`red.hex` → 0xFFFF0000) or as the "#FF0000" form, `hex_text(c)` (shader-hash), plain lib words on Color.
- juxtaposition-one-rule (Interviewer, from function-words-consume and call-takes-right): how far a word's argument reaches never depends on where the phrase stands (statement start, after `=`, inside an expression); it is the parameter-type fit alone: `print x > 0` = print(x > 0), `count xs > 3` = count(xs) > 3, `max xs > 0` = (max xs) > 0, and `all xs > 0` = (all xs) > 0, a quantified view (each-all-broadcast); a difference by position is a bug.
- of-in-selectors: `count of words in t`, `number of bytes in t`, `first of words in "ab cd"` are kept if they fall out of the right-associative `of` chain (X of Y of t = X(Y(t))) with `in` closing the chain as the source, without special code beside it; otherwise held and brought back to the user. `x in xs` alone stays membership.
- untyped-fit-ambiguous: when a word with an untyped parameter is followed by a phrase that could end before or after an operator (`sq(x) := x*x; sq 2 == 4`), it is an ambiguity error naming both readings with their parentheses: `sq(2 == 4)` or `(sq 2) == 4`; a typed parameter decides by fit (juxtaposition-one-rule), `print x > 0` stays print(x > 0).
- list-insert-one-order: `xs.insert(i, v)` position first, `xs.insert(v, at: i)` by name, no order guessed by type.

- cast-checks (user, refines nat-and-int; generalised by checked-forced-conversions): `x as int` (as i64, every fixed-width int) raises a catchable Error when x does not fit ("too big for int: …"); `x as! int` converts no matter what (wraps); the compiler warns when it knows a value will not fit. Int arithmetic keeps wrapping silently. No `wrapped` word. Warp's `as!` forces the conversion (Swift's traps).
- checked-forced-conversions: `x as T` is a checked conversion, exact or a catchable refusal; `x as! T` a forced conversion, the target type's one declared lossy rule (library: a `forced` overload beside convert), refused where a type has none (enums, parsing text). Ints and bytes wrap; colour channels and audio samples saturate (300 → 255); bytes → text replaces invalid UTF-8 with U+FFFD; map → class drops unknown fields. Floats approximate in both variants (`1/3 as float` rounds). The compiler warns where it knows a checked conversion will refuse.
- degree-constant: `°` in lib/units.warp is the plain number π/180 (radians are dimensionless): `60°` is 1.0472, `sine(30°)` works everywhere, shaders included; it shows as the number. No Angle quantity class.
- units-ignored-by-shaders (user, reinterprets no-units-in-shaders): values a painted (shader) function reads are written with their units (`wind_speed = 7 m/s`); the shader (and the CPU fallback) ignores the unit, receiving the quantity's base SI amount (20 km → 20000); the WGSL pass drops the unit, the CPU fallback pulls units through (math words read a Quantity's SI number through a float() hook).
- root-dispatch-float: with f(int) and f(float), an irrational exact root (sqrt(2)) picks the float variant; a whole root (√9) counts as int.
- rational-not-dispatched: `f(x: int)` and `f(x: rational)` stay the loud refusal "cannot be told apart"; branch inside one f with `if x is rational` (card fails-unknown).
- call-negative-plus-test: `f -2 + 1` is f(-1) (call-takes-right's reading).
- unit-runs-spaced-only (user, core one-liner): phrase rules try runs only in a space-separated group, never across comma items (`[1, s]` stays two items).
- unit-binds-below-power (user, core one-liner): a spaced unit binds looser than ^ and tighter than + and comparisons: `4 m²` is 4*(m²).
- word-operator-overloads (user, ~5 core lines): `infix operator <built-in word>(…)` (as, each, in, to …) overloads that built-in word instead of declaring a new operator (card rewrite-rules).
- loop-scoped-it: a for loop's implicit `it` is its own, gone after the loop.
- typed-parse-message: parse's mismatch says the warp type, "expected object, received list of int".

- word-arithmetic-infix: `plus`, `divided by` are aliases only, not infix words; `one plus two` is not arithmetic. word-arithmetic-parked: test lines expecting `3 plus 4` → 7, `5 subtract 2`, `2 add 3` (test_add_subtract_words, test_operator_spellings) stay parked until those words are declared as infix operators (`infix operator plus := +` in a library or the program), never as core language.
- arm-symbols: an undeclared capitalised word in a match arm is an unknown name; matching.warp declares enum Day.
- regex-error-from-os (user, "leave as is"): a malformed regex passes the OS's regcomp text through; tests expecting older wording stay red; no warp-owned message table.
- core-dir-order: dir(module) lists names sorted alphabetically (`dir(time)#1` → "day_number"), as Python.
- use-require-import: `use <word>` refuses with a fix-it naming the module ("square is a word of math: use math").
- backtick-strings: inside backticks `$x` interpolates the expression `$x`, as outside (a page element's value on a page); a plain variable is `${who}`; `\$` escapes. extensions/, no core.
- fine-updates: a style block prints with braces as written, `style{.done{color:"green"}}`.
- markup-attr-call: in markup a call to a known warp word is evaluated (`div(class: join(xs, " "))` → class="a b"); CSS functions (min, max, calc, var, rgb) inside style{} stay text; refused when both fit.
- core-code-point-list-chars: `xs = "ab" as codepoints` gives codepoints where `xs#1 == 97` and `xs#1 == 'a'` both hold (a codepoint equals its number and its char).
- core-if-branch-float-widening: mixed int/float branches widen to float.
- verify-function: a program may not name a function after a type word: `double(x) := …` refuses ("double is a type").

- stem-lookup-ed: general rule: any unknown `<f>ed`/`<f>d` (doubled consonant: stopped → stop) applies f to the value before it, for every function taking an argument (`xs sorted`, `twice` → "twiced"); core name resolution (~20 lines), Core column.
- clock-plain: clock() returns a Time quantity, so `clock() - started < 2s` works.
- it-last-result: inside a lambda `it` is the implicit parameter; outside one, `it` is the last statement's value (`3 + 4; it * 2` → 14).
- render-pdf-string: `x = 3 as pdf` with pdf undeclared refuses, "unknown type pdf: use a library that defines it"; an unknown type name is never invented.
- dollar-interpolation (user, card julia-dollar-interpolation): `"x is $x"` interpolates globally in double quotes, as `"${x}"` does, no `use julia` toggle; single quotes never interpolate (`'$HOME'` stays "$HOME"). In double quotes `$` before a digit is never interpolated (money "$1,234.56"); `$0` warns with did-you-mean `${$0}`; a `$name` with no variable or element of that name is a compile error with a fix-it (write '…' or \$). Data files never interpolate. The context decides: `$x` in a text interpolates, `$name` in a rewrite rule is a hole, `$dir` is a word (taste call). "$ref" (lib/json_tools.warp:171) and shell texts move to single quotes.

- rough-fallback (user, card instances-compared): a class that defines only one of approximately(o) / similar(o) uses it for both ≈ and ~; with both defined each serves its own operator.
- path-after-operand (user core OK, ~2 lines starts_path_at): spacing decides, as list-sign: a blank before `/` and a name glued after it starts a path literal even after an operand, `show /tmp` is show(/tmp); `a / b`, `a/b` and `x /2` stay division; `x /y` becomes a path and, where y is a variable, is refused with the fix-it "write x / y for division"; warp format writes the reading out.
- welcoming-lines: english-comparisons: `a is greater than b`, `is less than`, `is at least`, `is at most` are not built, lines deleted. function-equality: `===` is identity (same definition or an alias); `==` on functions is equal when parameters and body are the same (built); equal up to renamed parameters and self-calls (`fib == fibo`) parked. nfc-equality: text `==` compares after NFC normalization (`"é" == "é"` → yes); bytes stay as written. `t.tzdata` dropped.
- glued-tag-mark (user, refines fine-updates; core parser, after the sprint): the parser marks a tag glued to its braces (`point{…}`, `style{…}`) and printing keeps that form; data written `a: {…}` keeps the colon.
- modules-n-z: `use log` is no module: log and log10 live in math, `use log` warns with the fix-it `log 10` or `ln`; version literals (`v1.2.3`) are built in core (card version-literals); a program's own class named like a library class (HashSet) shadows it: local names beat used ones.
- signature-defaults (user, card reflection-dot-forms): help(f) and f.signature show default values (`nearest(x, kind, limit = 1000000, within = 0) -> node`); Rust-era expectations are upgraded to the typed lib signatures (`(y:float, x:float) -> float`); an alias answers with its target's signature.
- spaced-tag (user core OK, card samples-web): spaced `word {…}` is the same tag as glued `word{…}` when the word is no binding of the program and no rule word, so `do/forever/go/once/defer/async {…}` keep their meaning and `head {…}` in markup is the head element; warp format still prints glued (glued-tag-mark).
- style-ul (Interviewer): a CSS descendant selector is written as a text key, `"ul .done": {…}`, as `"nav a": {…}`; no parser change; card should-dot decides whether a dot spaced before and glued after starts a new item everywhere (the glued-negative whitespace rule).

- block-definitions (user, parked): a definition inside a block (if, while, for, bare {…}) is refused with the fix "define f outside the block, or write f = y => …"; scoped functions are out of scope (card inside, Later).
- format-as-text-class (user, cards format-as-text-definition, hGgX-w): a class's own text form is `format as text := quantity_text(this)` in the class body; the top-level `format Point p as text := …` works for classes too; callers write str(x) or "${x}", never x.show(); show() goes (~5 core lines, classes.warp form_method).
- bar-never-or: `|` is never the logical `or` (no short-circuit, no or-node in the parser): bit-or between numbers, a pipe into a function, a union between types; logic is written `or`.
- rgb-color-literal (user; refined by color-text-css): an RGB color is `color(0xFF8000)` (`rgb(0xFF8000)` the same word with one argument): 6 hex digits are opaque, 8 digits carry alpha last in CSS order (0xFF800080); `0xFF8000 as Color` follows the same rule; no CSS text form, no bare `#FF8000`; lib/color only. color-text-css: a color prints in CSS order, alpha last: `with_alpha(red, 0.5)` → #FF000080; `color(0xFF000080)` is red at half alpha; the stored int layout is internal.
- d-and-clock-left: `d` stays as today (glued `2d` the double suffix, spaced `2 d` unknown); `15:00` stays the pair (only `at 9:30 {…}` reads a time); tests asking otherwise are ignored; now.time / now.date work.
- chord-word-hint: `chord A B C` (and chord([A B C]), chord(A, B, C)) gives a playable Chord value with the got-it note "chord isn't needed: write A+B+C"; chord(C4, major) stays the named-chord builder.
- int-type-overloads (user, core OK, card module-word): named int types (`type Color = int`) are types an overload tells apart: `hx(n: int)` beside `hx(c: Color)` is valid anywhere; the narrower type is tested first: `use color; hex_text(255)` → "0xFF", hex_text(red) → "#FF0000" (a mark test in the dispatcher).
- operator-words-top-level (Interviewer, by operator-english-names; card greater-list-all): a top-level definition named by an operator's English word with typed parameters, at least one of them not a number type, also defines the glyph: `greater(a: list, b: int) := all a > b; [3 4] > 2` → yes. Untyped ones (lib's `add(xs, x)`, `power(base, exponent)`) stay plain functions.
- operator-unicode-aliases (user; details: Interviewer): an operator's Unicode character names are fallback aliases (`≤` also less_than_or_equal_to, `↑` upwards_arrow, `·` middle_dot) as long as the tables stay small: only the names of symbols the alias table already spells (~20 names), generated once by a script from Unicode's name list into lib/aliases.warp, a separate generated `names` field read as aliases, resolved when the alias table is built into the module image; never a Unicode database in the compiler, no names on keyword rows (and, or, not, xor, to), no typographic look-alikes, never infix words; a program's own binding of such a name wins.
- block-values-forced (Interviewer): data stays data, not a block; a quoted value forced through a parameter is a block value: `f(b: block) := b!; xs=[block a+1, block 0]; a=1; f(xs#1)` → 2.

- ctrl-c-runs-exit: ctrl-c, kill or SIGTERM runs `on exit {…}` (when there is no `on interrupt`), then ends with status 130 (143 for kill); a second ctrl-c within a second ends at once.
- event-delivery-points: one rule native and page: a handler runs on the program's thread at its waits (sleep, await, receive, a painted frame) and at loop starts, by one compiled flag check in programs that listen; the waiting (clock, keys, signals) runs on a host thread that only queues; never inside a running handler (`n = 0; on every 50 ms { n += 1 }; sleep 175 ms; n` is 3). Wasmtime epochs may replace the native check later.
- threaded-handlers: real parallelism is the `go` task (go-unified) plus `shared` atomic values (P106, lib-oddities): `shared n = 0; on every 50 ms go { n += 1 }` runs the handler as a task on its own thread; it sees only shared values, the rest are copies; simple scenarios natively first, the page cooperative.
- rounding-text-overload: dropped for good, never asked again: `round("2.7")` stays refused; write `round("2.7" as float)`. (Over the 3-line limit: text overloads beside the runtime and libm variants made calls ambiguous.)
  - named types are cheap classes too (Interviewer, follows cheap-classes-everywhere): `type pdf = text`, `type Pt = int` are their own types, so result overloads tell them apart (`render(t) as pdf` beside `render(t) as docx`) and values keep their type across a module link; mix-ff0000's lost marks and render-pdf-string part 2 wait for it, no mark or alias fixes meanwhile.
- use-path-literal (FYI, 4 core lines, card shader-quote-path-uses): `use ./x.warp` / `include ./x.warp` take the path literal; lib/quote_helper.warp moves to test/web/quote_helper.warp.
- enum-constants-typed (Interviewer, ~5–10 parser lines, card one-on): a library constant holding an enum case, `ESC = Key.escape`, is typed for rules like `km = Unit("km")`; `on X {…}` is a named event only when X is unbound and no library constant, else listened(X, handler).
- traits-are-protocols: no separate trait concept: a protocol (a lowercase type word) may give its methods default bodies, and a class that fits it gets them (`shape { area(); describe() := "area " + text(area()) }`); two protocols giving the same default are refused unless the class defines the method itself. No new keyword.
  - dot-call answers (user, core OK for notes/dot-call-plan.md steps 1–5, then 8): `o.f(3)` with a function-valued field f calls that field (fields first; the global is `f(o, 3)`); on untyped data the field wins (`{"count": 3}.count` is 3, the word is `count(m)`; keeps_word goes); any function in scope works by dot, local closures included (`g = x => x*2; 3.g` is 6); `x.w = v` without a field w calls a setter overload `w(x, v)` if one takes x, else a map gets the entry and an instance a warned extra field. Dot on a whole number (`16.sqrt`, `3.type`) falls out of the rule and stays (Interviewer, the user unsure: refusing would be a special case); `1.e5`-style exponents stay numbers.
  - map-methods-first: own methods first, then fields, then functions: a map's protocol methods (keys, values, entries, get, has, set, remove) beat an entry of that name (`m.keys` is the key list even when m holds "keys"; that entry is `m["keys"]`); other entries stay field-first (`{"count": 3}.count` is 3).
  - setters-declared (Interviewer, narrows the dot-call Q4 setter answer): `x.w = v` calls a setter only where x is statically an instance of a class with its own one-parameter method `w(v)` or a function whose first parameter is declared that class (`double(p: P, v)`); maps and untyped values keep the entry write (`m.merge = 3` adds an entry, never merge(m, 3)). An extension rule, no core. Step 8 (canonical w(x, …) for dot calls) parked on card dot-call-canonical until the rule engine matches a call and its dot form in one pattern; step 5 (node parts as global words) dropped.
- name-shadows-overload (FYI, card color-power-variant): any name a definition or lambda binds hides an overloaded library function of that name, as parameters already did.
