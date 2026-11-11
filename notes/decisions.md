# Decided questions: an inbox, not a law book

How decisions live. A decision is one of three kinds, and each has its own permanent home:
1. Language rules (`type A < T` is a subtype, `÷` floors, markup only inside `html{…}`): the specification. Home: the
   wiki page of the topic, plus at least one outcome test line in test/. The wiki says what the language is, the test
   proves it and goes red when code drifts. Where the wiki and an entry here disagree, the wiki wins.
2. Process rules (quick merges, outcome tests only, where workers work): AGENTS.md or a role file.
3. Rulings for one moment (stage B waits for a seed, park this edge case, core OK for card X): the card and the
   commit message. They are not kept here.

This file is the inbox: a new decision lands here as one line with its key and date, and within a few days it is
absorbed into its home and deleted here (git keeps the history; the old-id index at the end names where it went). So
the file stays short and nothing in it lives long enough to go stale. Never read it front to back: grep a key or a
topic when the wiki and the tests are silent.
The regular round (card decisions-prune, every 3 days) absorbs entries into their homes and flags every entry the
current wiki, code or tests contradict; the flagged ones go to warp-interviewer as one batch question for the user:
keep, update or drop.
Decisions are taken in their moment and can be myopic or age. An agent who finds one blocking a clearly better design
says so with the reason (to warp-interviewer), instead of quietly obeying or quietly breaking it.
Open and parked questions: notes/open_decisions.md. Keys: P = interview questions, D = wiki-survey questions,
#n = the first question lists, slugs such as gpu-f32 for the rest; merged keys follow the survivor ("also: …").

---

## Inbox
- 2026-11-11 build-ships-only: `warp build app.warp` leaves the executable, runtime.wasm and modules/ only; `--wasm` writes app.wasm, app.full.wasm, app.mod.wasm (replaces build-three-outputs).
- 2026-11-11 no-str: `str` is gone as function and type word (lib/python keeps it); warp writes text.
- 2026-11-11 hex-subtype: hex(n) / `n as hex` / `hex n` give a hex subtype of int shown as 0x7b (lowercase, no quotes); subtypes of number types equal their plain number by value (not Quantity, Angle); colours: argb_hex / rgba_hex.
- 2026-11-11 canvas-value: canvas is a Canvas value (width, height, clear(), resize(w, h)); clear(canvas).
- 2026-11-11 quiet-implicit-uses: runs don't hint implicit standard uses; warp lint/beautify write the use.
- 2026-11-11 foreign-use: `use python json` canonical (source first, like use js / use cargo); `use json from python` an alias; one table of foreign runtimes.
- 2026-11-11 console-shell: the console runs a line through sh only when warp's refusal is an unknown name, marked dim (sh).
- 2026-11-11 complex-shows-imaginary: complex numbers show as 3+4ⅈ.
- 2026-11-11 unit-beside-variable: after a number a unit expression stays units even when a variable has the unit's name (hint); `440Hz` shows as 440Hz.
- 2026-11-11 parked: code spacing in printed code (refusal-spelling), painted typed-function core fix, rgb type shorthand.
- 2026-11-11 executables-beside-modules: `warp build` ships the compiled modules beside the executable; it finds its own folder through the OS (its executable path) and loads them at run time; no merge, no env var.
- 2026-11-11 markup-needs-html kept: a bare element (`div{…}`) outside html{} is data and gets a loud warning naming the fix (`x: html = …`).
- 2026-11-11 public-samples-no-todo: the playground and site hide TODO/DONE comment lines of samples; the sources keep them.
- 2026-11-11 prelude-word-variable: parked (a binding hides a function only from the binding on).
- 2026-11-11 menu-follows-example: the playground menu always shows the loaded example, never falls back to hello.
- 2026-11-11 mutating-needs-bang (parked): `!` on a function that changes what it is given is only a hint for readers where it is not clear; the compiler does not enforce it (almost any function modifies something).
- 2026-11-11 self-contained-executables: `warp build` makes an executable that carries the runtime and prelude it needs and reads no lib/.
- 2026-11-11 whole-arrow: `↑` takes whole exponents only (refusal names `^`); the compiler itself carries no big numbers (extension-row-optional). fast-mode: a `--fast` compile argument drops overflow checks (ints wrap at 64 bits) for programs that want speed over checks; the compiler is built with it. Scope: the program's code and every runtime word of the run (prelude and module runtime words share the switch; modules' own inline typed `a + b` stays checked, being compiled without --fast); a value already big keeps its exact arithmetic, only the promotion on overflow is dropped.
- 2026-11-11 page-runs-live: an animating page run lives until the next Run or Stop, as natively.
- 2026-11-11 sync-fetch: `fetch` waits everywhere, the page included; `go fetch` is the asynchronous form.
- 2026-11-11 text-kind-fast-path: text comparison and indexing pick their path from the text's kind (text-representations), never by rescanning.
- 2026-11-11 headless-page-checks: a single headless page check on the Mac is fine; whole tours stay in CI.
- 2026-11-11 general-cast: `T(x)` as the general cast gets a card; months, type(f) signatures, sized `x : 10 float`, gc struct, plural type words held as esoteric.
- 2026-11-11 replace-with parked: whether `replace … with t` evaluates t.
- 2026-11-10 test-slots-shared: `warp test` slots stay one machine-wide pool (cores − 2 × builds + 1); no per-run floor.
- 2026-11-10 compile-heap-128: a compile starts with a 128 MB GC heap (big compiles ~2× faster); a compiler build keeps 1 GB.
- 2026-11-10 shape-tokens: a sugar rule of holes only needs its group's shape token, so it is picked only where the code has that shape.
- 2026-11-10 spaced-unit-precedence: `2 km == 2000 m` follows from juxtaposition multiplying, which binds tighter than comparison; rejoin rules only if that reading fails.
- 2026-11-10 lambda-scope-cells: a lambda's block body is a scope; its blocks share its names (handler-blocks-share holds there too).
- 2026-11-10 is-type-variable: `x is v` tests the type a variable holds; a name assigned again is refused (`type Paint = Color`); no wiki text.
- 2026-11-10 nullish-default stays: `??` replaces errors as well as ø; no Footguns note.
- 2026-11-10 time-of-day-order: a local datetime or a time of day orders against a duration by its time of day (`ref.time >= 6h`).
- 2026-11-10 compile-only-metric: the Vitals page shows the compile alone (hello minus `warp version`, startup apart); `warp version` loads the whole machinery and is not optimized; fingerprint-by-stat parked, stamp-read-tail dropped.
- 2026-11-10 runtime-words-lazy: per compile, runtime words are parsed on demand (a name → offset index over runtime.wasm's word table), library words are bound from the program's side, runtime imports are read from the import section.
- 2026-11-10 text-representations (revised 11-11): an ASCII text is a wasm subtype of the text type, its kind set once when made (literals, joins of ASCII, numbers, host lines scanned once); a UTF-8 text's code points are cached lazily in the per-text cache; stop gate at >3% build cost; internal, no AsciiText spelling.
- 2026-11-10 block-call-parked: a glued `{…}(…)` is not a call yet (parked).
- 2026-11-10 compiler-size-levers: the three size levers of notes/compiler_size.md are built in this order, each landing with before/after bytes: literal-nodes-inline (each distinct literal one global, ~460 KB), globals-constant-init (constant tables as packed data, ~198 KB), each-wasm (~190 KB); all core/emitter, by warp-sugar-program.
- 2026-11-10 text-equality-ascii: text == skips the NFC check when both texts are ASCII or already NFC; results unchanged, measured on the build profile.
- 2026-11-10 lambda-broadcast: a lambda applied to a list broadcasts like a named function (`g = x => x*2; g([1 2 3])` is [2, 4, 6]).
- 2026-11-10 euler-glyph: ℯ (and ⅈ, π) read as the constants' names, not characters: `ln(ℯ) == 1`.
- 2026-11-10 link-reach: linked words record their reach in warp.signature (step 1) and arrive already lowered (step 2), so the passes walk only the program.
- 2026-11-10 compile-fixed-cost: the prelude signature is read packed, sugar rules are indexed by head word, and the GC heap start grows only if RSS stays acceptable.
- 2026-11-10 warp-window-env: `WARP_WINDOW=0` (set by `warp --no-window`) is the twin of `WARP_SOUND=0` (set by `warp --mute`): the two machine switches kept as environment variables; it replaces WARP_NO_WINDOW=1.
- 2026-11-10 warp-sound-env: `WARP_SOUND=0 warp app.warp` mutes a run (`warp --mute` sets it), the one environment variable kept as an interface (exception to no-bash-exports).
- 2026-11-10 aliases-in-rules: a sugar rule's literal word matches every alias spelling of it (`xs.forEach {…}` through the `each` rule); aliases otherwise stay call-only, so data keys and type names never change; the full unification (aliases-unified) is parked: renaming everywhere would rename data keys.
- 2026-11-10 x-bang-refusal: a block's `x!` is refused with "`x!` runs no code: x() runs a block, interpret x runs data".
- 2026-11-10 bare-typed-overloads: `name: T := e` is `name(): T := e`, so `sample: int := 7; sample: text := "seven"` overload by result type.
- 2026-11-11 default-variant: among variants told apart by result type, the one declaring no result is the default: a call no expected type (declaration, typed parameter, declared result, `as`) tells apart takes it; arithmetic tells nothing; a told type no variant declares takes the default with the usual type check; refused: two defaults, a default whose known result a typed variant declares (user OK 11-11 with warp-reviewer's conditions; card float-literal-decimal).
- 2026-11-10 fast-build: `fast` functions get machine arithmetic (i64 wrapping, fast_math), as extensions/fast.warp.
- 2026-11-10 def-arrow-fixit: `def f(x) -> T: body` is refused with the fix "def f(x): T := body".
- 2026-11-10 undo-planned: undo/redo stay a planned feature.
- 2026-11-10 and-or-values: `and`/`or` carry their operands' values, as a general mechanism: `7 and "hi"` is "hi", `0 or 5` is 5.
- 2026-11-10 bool-int-refused: `n: int = true` is refused with the fix `true as int`.
- 2026-11-10 int-list-alias: `xs: int list` is an alias of `xs: [int]`.
- 2026-11-10 it-expression-function: a parameter typed function takes an `it` expression as a lambda, generally.
- 2026-11-10 aliases-unified: one alias mechanism everywhere: `alias a => b` lines (prelude, modules, program; the program's win) feed one table, applied wherever the program doesn't bind the word, class members included; the arrow's target is canonical; lib/aliases.warp keeps only operator and keyword spellings (card prelude-alias-line).
- 2026-11-10 no-bash-exports: no environment variable is an interface; choices are code or CLI options (card no-bash-exports).
- 2026-11-10 wiki: `test` lines are checked on every run; `width /height` is refused as ambiguous; `law` is marked planned; WARP_DRAW leaves graphics.md.
- synonyms-are-aliases (card test-triage): synonym words of existing ones are aliases in lib/aliases.warp pointing at the canonical word, not deleted: properties/fields/attributes/members of (keys, members), clone (copy), warn (warning), require (use), forEach/for_each/foreach (each), invocable/runnable (callable), previous/was (old), ppi/density (dpi), maybe (optional), unit/nil/empty/string/integer as type names; their held test lines are restored.
- definition-by-space: `square x := x*x` defines a one-parameter function like `square(x) := x*x` (the original wasp form, as call-by-space `square 2`); held lines restored, red ones carded.
- typed-array-declaration: a typed declaration with a size and no value makes a zero-filled list of that type: `pixel: int[100]; pixel#1 = 5`, `x : int[100000]; count(x)` → 100000.
- result-type-as: `as T` after a function head is the default way to write its result type: `def f(x) as text {…}`, `f(x) as int := …`, `area() as number := …`; it is the same meaning as the conversion `as` (the result is taken as T). `: T` stays readable but `warp fix` rewrites it to `as T` everywhere (lib, samples, tests, wiki, the compiler's own code; card result-type-as).

## Process


- #1 avoid panics, errors as values. #14 not-implemented errors unless easy.

## Tooling: format, beautify, CLI

## Welcoming, diagnostics and newcomer forms

  - julia-floor-divide: `÷` is floor division as in Julia (synonym of `/!`, `7 ÷ 2` → 3), with a hint for newcomers expecting `/`.
  - julia-pair-range: `1:4` stays a pair; iterated (for, sum, comprehensions) a pair of two numbers is the range.
  - julia-text-times-hint: `"a" * "b"` concatenates with a got-it warning naming `+`; an error only when a text holds a number (`"3" * "4"`).
  - julia-dollar-interpolation: `"x is $x"` only as a toggled feature after the self-hosting migration (`$million`, `$0` clash).
- foreign-spellings: foreign names are cheap aliases of existing words, not lowering code. Kept: Rust `<T>`/`let mut`, Enum.map/List.map/Seq.sum, Python lambda, variadic lambdas, JS destructured parameters, Ruby lambda/proc/blocks/to_s, C# LINQ, C++ `auto x`. Dropped: OCaml let-in and `fun`, Elixir `def …, do:`, C++ `[](…){…}`. R/PHP forms live in `use R` / `use PHP`. Kotlin `when x { 1 -> … }` = `match` with a warning; `match` takes `->` arms and `else`; `case` is optional before any arm.
- P157 no generic syntax: ported `fn id<T>(x: T)` compiles untyped with a note. P161 no Swift argument labels; ported labels compile with a note.
- P167 Go's `Point{1, 2}` builds `Point(1, 2)` for a known class with a note; unknown names stay tagged data.
- negated-call: minus binds weaker than application: `-f x` = -(f x), so `1 + -floor 2.5` → -1 and `-sin 0` → 0; `-a * b`, `-x^2` unchanged and `x=5; -x 1` stays [-5, 1] (a plain value is not a call).
- phrase-functions: a definition head may be a phrase with holes: `send v to c := c.send(v)`, `on every period (body: block) := timer_every(period, body)`, `await x within t or d := …`, `set field of object to value := …`; one phrase-row mechanism like the operator rows (~60–120 core lines), replacing ~35 lib rewrite rules (tasks, timers, signals, channels, english, which stay as such declarations).
- phrase-rules-kept: `on change x {…}` / `on set x {…}` and `set name of p to 3` stay rewrite rules until name parameters (`f(x: name)`) exist, then become plain functions; `await all xs` / `await any xs` stay as written (notes/phrase_inventory.md).

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
- approx-zero: `≈` is relative 1e-9 with absolute floor 1e-12; `tolerance = …` overrides. P211 `~` is looser: 1% for numbers, texts ignoring whitespace and punctuation; `≈` on texts ignores case and accents. P212 a class may define `approximately` and/or `similar`.
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

## Bool and truthiness

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
- dir-lists: built-ins are function values of type `builtin` (a subtype of function); methods(x) lists them.
- receiver-field-assign: `fresh(f: F) := { body = 99 }` writes f.body, same as in a class-body method (one-method-form, P115/P116); `let body = …` makes a local; any warning applies equally in both places.
- random-forms: bare `random` is a float in [0,1) where a value is used; `random T` is a random value of type T (`random bool`, `random int`); `random(n)` 0…n-1; `random(a, b)` whole numbers inclusive, floats [a, b); `random 1..10` picks from the range. Any parameter of type Type accepts bare type words.
- function-references: a bare function name compared with a type or in `type(f)` is the function itself; `.floor` in argument position is a lambda (`xs.map(.floor)`, `people.sort(.age)`); `&floor` and `function floor` are the explicit references.
- declared-belonging: calling stays uniform (`f(x)` ≡ `x.f()` for every function, silently), but belonging is declared, not inferred from the first parameter: a function is a method of a type (dir, completion, docs, the playground's method list) only when declared in the class, the type's own module, or an extension; any other function is a free function of its module, dot-callable where that module is used (P171). Reflection keeps both: `Text.methods` the declared ones, `Text.functions` every function taking a Text. A type alias carries its own methods (`type Name = text; argument_result(n: Name)` is a method of Name, not of text). class-extensions: outside code adds a method by its dotted name `text.shout() := upper(it) + "!"` (canonical) or groups several in `extend text { shout() := …; whisper() := … }`.
- module-alias: an `alias pw = power` line in a program or a directly run file adds a name and never shadows one (`alias exp = exponential` beside a builtin exp leaves exp the builtin).
- own-name-not-builtin: a program wanting its own version of a builtin math word (a Taylor-series sin) defines it under its own name (`sin_test`), not by redefining `sin`.
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
- markup-needs-html (replaces element-named-pair, card data-key-brings-markup): markup is written inside `html{…}`, or in a function known to return html (its result typed html), where a plain `b{"bold"}` still works; anywhere else a tag-named key or tag form is data: `x = {p: {c: 1}}; x` → {p: {c: 1}}, `{i: 1}` stays {i: 1}. Tests and samples that write bare markup get the `html{…}` wrapper.
- html-value (Interviewer, by markup-needs-html, card html-anywhere): markup is a value of `class Html { element: pair }` (a cheap class, not a subtype of pair: a pair would fill a subtype and data would count as markup), printed by its own `format` overload, rendered by dispatch on `is Html`. An element written inside html{…}, in an html-typed function, or as the argument of an html-typed parameter (to_html, render, page_render take `value: html`) becomes an Html; a data entry like `"a": "Apple"` stays a pair.
- format-types-inline-html (refines jsx under html-value, card inline-div-format): `warp format` turns inline HTML into warp markup and adds the html type where the element is a whole value: `p = <p>Hi</p>` → `p: html = p{ "Hi" }`, `f() := <div/>` → `f(): html := div{}`; elsewhere (an argument, inside an expression) the inline HTML stays as written.
- P29 `pair.0` counts from 0. P30 `xs.pop()` gives the last item, `m.remove(k)` the value or ø.
- Word slices: `xs from #2 to #4`, `xs starting from #2`, `xs up to #2` or ordinals, both ends inclusive; a bare number is an error. P58 `xs#a..b` keeps the range with a strong warning naming `xs#(a..b)` and `(xs#a)..b`.
- P176 repeated `key: value` in a named tag block are children; a plain `{…}` keeps the duplicate-key error.

## Control flow, matching and errors
- prefix-keyword-condition: a prefix keyword (whenever/when/on/at) takes its whole condition up to the trailing block (`alarm = whenever t > 30 {…}` is a listener handle); a glued digits:digits after a word stays one time value (`at 9:30 {…}`).

- match-exhaustive: a match on a known enum or sealed class missing a case is a compile error.
- pattern-keyword: named patterns only with `pattern Adult = {age: >= 18}`; a plain `Name = …` is a value.
- #31 a switch without a match reports the run-time value (`no case for n = 4`).
- `when` by shape: `when click {…}` = `on click`, `when x > 3 {…}` is refused naming `if`, `once` and `whenever` (11-11, replaces = whenever), `when x {… -> …}` = `match`, `when {cond -> …}` = an if chain; each warns naming the word meant.
- P46 type-name and condition patterns in `for` filter with a got-it warning (`for friend in xs`, `for (it>2) in xs`).
- P61 `is` is always a comparison; with an undefined x it teaches `x be <value>`. #27 `be` is `:=`; `≤ ≥ ≠ × ÷ ¬ √` are accepted and normalized (÷ is floor division: julia-floor-divide).
- fail-effect: the `Fail` effect tracks explicit raises, division by a value that may be 0, and library calls answering with an Error; index out of range, missing fields and failed casts are untracked bugs.
- error-classes: errors are classes: the prelude declares `class Error` and the run-time errors as subclasses with Python names (IndexError, ZeroDivisionError, …); users declare their own (`class ParseError: Error`), an undeclared error name is a compile error with that hint. `except T as e` catches T and its subclasses; `except Error`, bare `except:` and `catch e` catch everything.

## Classes, objects and enums
- capitalized-field-list: a capitalized plural field named after a declared class declares a list of it under the lowercase name: `class Signature { Parameters = [] }` is `parameters: list[Parameter] = []`, read as `sig.parameters`; otherwise an ordinary field.
- property-accessors: accessors are overloads by arity: getter `canvas.width := canvas_width`, setter `canvas.width(new_width) := …` or with `it`; a getter without setter is read-only (assigning is a loud error).
- setter-calls: a setter is called by `canvas.width = 12`, `canvas.width(12)`, `width(canvas, 12)`; `width of canvas = 12` is `canvas.width = 12`; `set x to y` is `x = y` with a style hint.
- saved-words: lib/draw.warp declares `canvas()` (no arguments) as the current Picture, so `save canvas as: "painting.png"` works through the bare-call rule; likewise `sound()`.
- P177 `implements` / `struct Square: Shape` is a checked claim.
- P200c every object has `copy()` and `clone()`. P205 `copy()` is deep; `copy(shallow: true)` shallow. P206 resources inside are shared with a note (error under --strict). P207 `clone` calls `copy`; a copy keeps its class. `===` (taste call, the code): same value and same kind of thing (`1 === 1.0` yes, both numbers; `0 === false` no); `≡` spells it; identity, if ever needed, gets its own word.
- enum-identity: each enum case is one unique value; `==` with a case is identity, plus its spelling as text (`Op.Add == "+"`, `Coin.Dime == "Dime"` yes); cases of different enums never equal (`Level.Error == ValueKind.Error` no); text becomes a case only by `"Int" as ValueKind` (`symbol("Int") == ValueKind.Int` no).
- Reflection: `dir(x)` from compile time when known, else from WASM metadata; x.type, x.fields (alias members), x.methods, f.params, f.effects; lookup order field → meta → reflection; one custom section warp.meta. P230 `effects of f` gives symbols.
- P114 `x.meta` / `x.@key` stay; comments fill them only with `use comments`; `@key` in a literal is never a field; iteration skips `@` keys.
- field-or-minus: `p.phone-number` without that field is "p has no field phone-number" with the fix `p.phone - number`.
- page-ids: an id in the program's own markup is a name in scope (`who.focus()`); a clash with a variable or function is a compile error naming both; kebab ids resolve; `$who` is the element's value and `$who = "Ann"` writes it; dynamic ids via `element("user-name")`; `$` means something only in warp code.

## Signals, events and tasks

- P129 `broadcast v on "chat"` / `send v to "chat"` reach `on message from "chat"` machine-wide; P129b named events cross programs. P155 channels: `channel()` local, `channel "chat"` machine-wide.
- P156 `whenever cond {…}` is edge-triggered. P38 listeners see later writes in their statements.
- P128 `listeners of x`, `clear listeners of x`, `remove alarm from listeners of t`. P148 `old` (aliases previous, was, before) in `on change x`.
- P134 `on exit {…}`. P135 `on every day at 9:00 {…}`, `at 9:00 {…}` once. P136 system-value rules, one card per value.
- P120 the CLI keeps running while handlers listen. P121 `exit` ends the run.
- P33 real threads for `go` (WASM threads natively, Web Workers in the browser). P47 a started job runs on its own; only a use of its value waits; `await all jobs`. P153 `await any [...]` races. P154 `await job within 100 ms or 0`.
- go-voice: a `go {}` task is a voice sounding along from where its starter stands. deleted-loop: a live_loop deleted from the source stops on re-evaluation (≤ ~4 s fade).

## Modules, library and foreign code
  - or-fallback: `a or b` is b when a raises, else `or` as before, falsy values included: `0 or 10` is 10.
  - nullish-default: `X ?? Y` and its keyword `X default Y` replace only an error or ø and keep falsy values (`0 default 10` 0, `parse_json("{oops") default []` []), the meaning of `stored visits default 0`; `default` canonical, `except`, `otherwise`, `failing` aliases with a note; the one-line `try X except Y` means exactly `X except Y` (errors and ø); the block `try: … except: …` stays the error handler; `else` only if's and try's; a bare `x else 5` is refused with the fix `x default 5` or `x or 5`. A match arm `default:` (key with a colon) stays the catch-all; `try X else Y` and `??` stay.

## Web, markup and server
  - live-word-items: a bare word item read from a variable written later is live too (`p{ greeting }` like `p{ "${greeting}" }`).
  - data-tag-words (user + interviewer): samples write markup elements glued, `head{ title: "My Page" }`; inside data, a prelude function before a block (head, first, last, sort) stays a tag; only words the program defines or `use`s make `word{…}` a call (`json{…}` with use json); head of a block in data is written head(…). One check in extensions/quotes.warp is_data_tag, no core.
- P188 stay close to HTML and CSS; only the basic machinery. web-keyframes: CSS's own `@keyframes`; warp adds only `leave:`.
- unknown-tag: an unknown element warns with the near name; custom elements and components are not flagged.
- Served routes: a raising form route answers 400 and re-renders; missing field 400, unknown id 404; /api and /rpc give instances as plain objects. route-access: a route gives out only what it displays.

## Sound
- sound-at: `at 2 beats play C4` places sounds from the voice's first sound.
- shared-buses: a bus is a named effect chain `room = bus() |> reverb(0.4)`; `osc(C4, 1s) |> room`, `play(C4, 1s, bus: room)`; `room.reverb = 0.6` applies from that moment.

## Compiler internals (technical, decided by the Supervisor or Interviewer)


## Process and platform decisions, continued
- pair-map-equality: `{a:1} == (a:1)` but `{a:1} !== (a:1)`: a one-entry map and a pair are equal loosely (==), not strictly (=== compares the type too, P196); `!==` and `=!=` both spell strict not-equal.
- exact-decimal-big: a decimal literal beyond int64's 10^18 denominator stays exact with an unbounded-int denominator, not a float; a warning names the cost where such a value is used heavily, e.g. inside a loop.


## Later rounds

- color-show-variants (user, refines color-prints): Color's own text form is `format Color c as text` in lib (format-as-text; nothing color-specific elsewhere). Besides the default (a name, else #RRGGBB), a color can be shown as hexadecimal (`red.hex` → 0xFFFF0000) or as the "#FF0000" form, `hex_text(c)` (shader-hash), plain lib words on Color.
- as-is-convert: `n: number as Real := Real(n, 0)` should be identical to `convert(n: number, to: Real) := …` (`x as T` sugar for convert(x, to: T)); needs multiple dispatch on type values (card as-sugar-convert, Later); as-promotes is built on the `as` form meanwhile.
- juxtaposition-one-rule (Interviewer, from function-words-consume and call-takes-right): how far a word's argument reaches never depends on where the phrase stands (statement start, after `=`, inside an expression); it is the parameter-type fit alone: `print x > 0` = print(x > 0), `count xs > 3` = count(xs) > 3, `max xs > 0` = (max xs) > 0, and `all xs > 0` = (all xs) > 0, a quantified view (each-all-broadcast); a difference by position is a bug.
- of-in-selectors: `count of words in t`, `number of bytes in t`, `first of words in "ab cd"` are kept if they fall out of the right-associative `of` chain (X of Y of t = X(Y(t))) with `in` closing the chain as the source, without special code beside it; otherwise held and brought back to the user. `x in xs` alone stays membership.
- untyped-fit-ambiguous: when a word with an untyped parameter is followed by a phrase that could end before or after an operator (`sq(x) := x*x; sq 2 == 4`), it is an ambiguity error naming both readings with their parentheses: `sq(2 == 4)` or `(sq 2) == 4`; a typed parameter decides by fit (juxtaposition-one-rule), `print x > 0` stays print(x > 0).
- one-duration (user, from fixed-time-units): a time-unit quantity and a time literal are one duration type: `2024-01-15T10:00 + 2 hours`, `1 day > 2 hours`, `2 hours == 120 min` just work. second/minute/hour are fixed; day/week/month/year stay calendar durations (a day 23–25 h, a month 28–31 days): added to a date they move the calendar, compared with a fixed duration they compare on their bounds, and a comparison the bounds cannot decide (`1 month > 30 days`) is an error naming the ambiguity. Shape: every duration is a lib/units Quantity of the time dimension; day/week/month/year are calendar units whose length is a range; time nodes are instants only; instant + Quantity moves exact seconds for fixed units, the calendar for calendar units.
- list-insert-one-order: `xs.insert(i, v)` position first, `xs.insert(v, at: i)` by name, no order guessed by type.
- project-root-paths (user; card project-source-root, Later): `•/x` means the project source root, `@/x` is its alias (canonical •/ until refined). The root is the nearest folder above the source file that holds `.git`; a future warp manifest takes precedence. Resolved at compile time, so no runtime word. No marker found: refuse with a fix-it, never a fallback guess. Plain and `./` names stay relative to the current folder. Server root is a separate notation: `://x` is the root of the server the program talks to (the site it was served from, or the server it is), while `./x` and `•/x` stay local.

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
- module-private-words (user, card linked-cross-module-overloads): a library's helper words that are not exported get a module prefix, so two linked modules no longer collide; the user sees only the class.
- core-dir-order: dir(module) lists names sorted alphabetically (`dir(time)#1` → "day_number"), as Python.
- use-require-import: `use <word>` refuses with a fix-it naming the module ("square is a word of math: use math").
- backtick-strings: inside backticks `$x` interpolates the expression `$x`, as outside (a page element's value on a page); a plain variable is `${who}`; `\$` escapes. extensions/, no core.
- fine-updates: a style block prints with braces as written, `style{.done{color:"green"}}`.
- markup-attr-call: in markup a call to a known warp word is evaluated (`div(class: join(xs, " "))` → class="a b"); CSS functions (min, max, calc, var, rgb) inside style{} stay text; refused when both fit.
- core-code-point-list-chars: `xs = "ab" as codepoints` gives codepoints where `xs#1 == 97` and `xs#1 == 'a'` both hold (a codepoint equals its number and its char).
- core-if-branch-float-widening: mixed int/float branches widen to float.
- verify-function: a program may not name a function after a type word: `double(x) := …` refuses ("double is a type").

- find-fallback: `"abc".find("x") or 0`, `?? 0`, `default 0` and `try … else 0` all give 0; an optional result type (`result?`) means null or error and catches the error as ø.
- stem-lookup-ed: general rule: any unknown `<f>ed`/`<f>d` (doubled consonant: stopped → stop) applies f to the value before it, for every function taking an argument (`xs sorted`, `twice` → "twiced"); core name resolution (~20 lines), Core column.
- clock-plain: clock() returns a Time quantity, so `clock() - started < 2s` works.
- it-last-result: inside a lambda `it` is the implicit parameter; outside one, `it` is the last statement's value (`3 + 4; it * 2` → 14).
- render-pdf-string: `x = 3 as pdf` with pdf undeclared refuses, "unknown type pdf: use a library that defines it"; an unknown type name is never invented.
- dollar-interpolation (user, card julia-dollar-interpolation): `"x is $x"` interpolates globally in double quotes, as `"${x}"` does, no `use julia` toggle; single quotes never interpolate (`'$HOME'` stays "$HOME"). In double quotes `$` before a digit is never interpolated (money "$1,234.56"); `$0` warns with did-you-mean `${$0}`; a `$name` with no variable or element of that name is a compile error with a fix-it (write '…' or \$). Data files never interpolate. The context decides: `$x` in a text interpolates, `$name` in a rewrite rule is a hole, `$dir` is a word (taste call). "$ref" (lib/json_tools.warp:171) and shell texts move to single quotes.

- wasm-load (user core OK; mechanism: interviewer): a host word `wasm.load` in core/native/driver.warp; `use x.wasm` / `import fourty_two` link the module's exports as typed imports at compile time (an extension reads the export section; the driver instantiates the module once and defines its exports in the linker; the playground likewise), the same shape as load-time linking (L5); components stay held; a run-time `load_wasm(path)` only when a program needs it.
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
- definition-head-phrases-dropped (Interviewer): colonless phrase heads (`give v to c := …`) stay parked (phrase-params); card to Later.

## Old id index (superseded or historic; git has the text)
P35 → index-position. P42 → P227. P56(2-4) unknown entity → P56 revised. P63 → wiki/charged.md. P66 → footgun-divzero.
P103, P104 → no-executable-by-default + build-should. P110 → P163. P118, P214 → P240. P168 → field-or-minus.
P170 → type-library-words. P200 → P200b. P52 bare labels → labels-calls. D1 → P6 + brace-hole. D7/#33 → wiki/charged.md.
D8 → approx-zero. D9, D12, D13 → got-it rule + reconsider-spaced + list-sign. D14 → in-bool. exact-default →
exact-decimals. #40 → list-sign. dimension-bridges → pixel-lengths. dollar-element → page-ids. definition-cases → cut.
P85, P87, P93, P95, P96, P98, P99, P101, P137, P144, P181, P53, P59, P79, P80, P9, P10, P16, P19, #2, #14b–d, #15–20,
the original questions 1–41: test edits and housekeeping done.
clean-compiler-first, emergent-forms-welcome, self-source-minimal, extension-oks, extension-wording, no-pass-proliferation, passes-first-principles, no-primitive-rewrites, restructuring-regressions, compiler-high-level, edge-cases-general → AGENTS.md Principles.
core-is-self, core-takes-what-it-uses → AGENTS.md "Three layers". prelude-from-disk → card baked-lib-split.
core-shake-small → card core-shake (notes/core_shake.md).
no-fixpoint, compilers-delivered → AGENTS.md "self hosted".
no-slots, root-files, parser-now, playground-paused, launcher-not-yet, sugar-hybrid-lane, cloud-paused, finish-migration, test-keyword, self-compiled-parity, warp-only-acceptance, warp-first, slow-redundant, quality-before-gap, urgent-direct, fix-forward, whole-pipeline-in-warp, sprint-merge-as-is, forward-crunch, lane-floats, cleanup-may-drop-edge-cases, consistency, gate-cuts, git-hook restore, P180 split-debuginfo, Repo layout, warp-ast.wit lives, Bee/wasp entity, P13 libm, browser-server, Playground CI → historic (git).
runtime-prepended → notes/runtime_linking.md. quick-merges → AGENTS.md "Testing".
quick-answer-first → AGENTS.md Talking. no-aur → notes/packaging.md.
beautiful-types, compiler-typed → notes/agents/handbook.md. node-refinements → notes/node_refinements.md.
runtime-split → core/runtime/bridge.warp, core/runtime/extended.warp. wasm-standalone → card standalone.
pass-triggers → card pass-triggers. revert-first → notes/agents/integrator.md.
reviewer-role → retired reviewer.md (git). crash-fixes-first → retired reviewer.md (git), AGENTS.md.
outcome-tests-only → AGENTS.md "Outcome tests only". weekly-cleanup → notes/agents/README.md.
topic-priority → AGENTS.md Cards.
cleanup-means-fewer-lines, restructuring → retired refactorer.md (git).
question-popups → AGENTS.md (superseded: questions go to warp-interviewer).
test-consolidation → AGENTS.md Testing. P150 license → notes/packaging.md, CONTRIBUTING.md.
laws-file → samples/laws/project_laws.warp. deploy-hook → retired watch-playground-deploy.sh (git).
typed-signatures → notes/agents/handbook.md. night-watch → notes/agents/supervisor.md.
codex-library-cards → retired codex_cron.md (git). core-gate-probes → AGENTS.md Done.
rebuild-next-lever, no-seed-before-instance-runtime, sound-core-order, runtime-linking-team, merge-then-reshuffle-then-celebrate, easy-reshuffle-then-pure-warp, native-replaces-rust-first, rust-gone-in-1-5, night-samples-modules → historic (git).
plain-warp-first, samples-not-spec → AGENTS.md Principles. key-op-spelling → core (key_operator).
native-harness → AGENTS.md "Native warp", notes/roadmap_native_harness.md.
test-results-file → AGENTS.md, test/results.txt. primitive-hooks → extensions/primitive.warp.
seed-speed-from-stages → scripts/refresh-seed.sh. libraries-not-core → notes/roadmap_native_harness.md.
native-warp-in-core → AGENTS.md "Native warp". core-is-compiler-and-harness → AGENTS.md "Three layers".
patterns-compiled-later → card pattern-direct-tests. dynamic-on-demand → card dynamic-language.
night-releases-and-test-triage, tests-serve-the-language → AGENTS.md Principles, notes/test_triage.md.
resident-compiler → card resident-wasm-instance.
extension-rule-forms, hello-lazy-runtime-dropped, small-int-cache → historic (git).
extension-parser-hooks, extension-lints, extension-attributes, extension-text-hooks, extension-passes → core/extension.warp.
one-operator-reader → core/parsing/parser.warp, extensions/declared_operators.warp.
atoms-as-rows → core/parsing/parser.warp. node-wasm-type → core/emitter/emitter.warp.
night-2026-11-04-kept → retired night_2026-11-04.md (git).
load-time-linking, library-zero-bake, library-zero-bake-plan → notes/library_zero_bake.md.
units-as-class → lib/units.warp. extension-row-optional → core/compiler.warp, core/emitter/prebuilt.warp.
core-hygiene → notes/core_hygiene.md. linked-variants-by-arity → core/emitter (linking).
made-by-reach → core/emitter (module.made).
comments-not-candidates, implicit-standard-words-index → extensions/implicit_uses.warp.
native-window-sdl3 → window-in-warp. node-attributes → core/node.warp ($MetaNode).
canonical-runtime-names → core/runtime. rule-whose, prelude-rule-no-use → core/passes/rewrites.warp.
inline-code → core/compiler.warp. gpu-driver-plumbing → retired gpu.warp (git). shaders-in-lib → lib/shaders.warp.
signals-test-15 → test/control/test_variable_signals.pairs.warp.
program-match-lowering → core/passes/node_matches.warp. computed-hole-shape → extensions/rules/rule_words.warp.
ffi-link-any-import → core/native/driver.warp. host-list-one-shot → core/emitter/host.warp.
shown-non → core/emitter/output.warp. test-oversubscribes-mac → core/native/tests.warp.
bool-type, predicate-bool, branch-keeps-bool, text-bool, in-bool → wiki/bool.md, test/types/test_bool_and_emptiness.pairs.warp.
keys-of-empty, empty-is-not-missing → wiki/null.md, test/types/test_bool_and_emptiness.pairs.warp.
Truthiness → wiki/truthiness.md, wiki/bool.md, test/types/test_bool_and_emptiness.pairs.warp.
hash-comments, `print a    print b` (an error) → wiki/comments.md, test/parser/test_comments_and_keywords.pairs.warp.
P165, P172 → wiki/keyword.md, test/parser/test_comments_and_keywords.pairs.warp.
unicode-symbols, operator-alias → wiki/operator.md, test/parser/test_comments_and_keywords.pairs.warp.
Text, P1, brace-hole, P126, #26, result-keyword, letters, guillemets, Emoji → wiki/string.md, test/text/test_text_rules.pairs.warp.
parse-json → wiki/string.md, wiki/json.md, test/text/test_text_rules.pairs.warp.
used-module-bare-words → wiki/use.md, test/text/test_text_rules.pairs.warp.
uniscript → wiki/uniscript.md, test/text/test_text_rules.pairs.warp.
bool-int-fallthrough, element-wise-scalars, type-shows-nat, int-boxed-overflow-big, exact-decimals, exact-power, int-list, number-converts, float-text, repeating-decimals, footgun-divzero, `x /= y`, Trailing, P184, nearest-limit → wiki/number.md, test/numbers/test_number_and_unit_rules.pairs.warp.
core-int-division → AGENTS.md Principles.
fast-int-readings, fast-scope → wiki/number.md, test/numbers/test_fast_int.pairs.warp.
marked-floats-shape → wiki/unit.md, test/numbers/test_dimension_types.pairs.warp.
c-int-names → wiki/int.md, test/numbers/test_fixed_width_ints.pairs.warp.
± → wiki/number.md, test/numbers/test_plus_minus_numbers.pairs.warp. P119, units-library → historic (git).
real-runtime → notes/real_runtime.md.
unit-sum-display, #5, quantity-naming, litre-suffix, units-opt-in, pixel-lengths, general-conversions, units-as-values, overloads-before-reals, expected-type-names, duration-units → wiki/unit.md, test/numbers/test_number_and_unit_rules.pairs.warp.
canvas-size → wiki/unit.md, test/programs/test_canvas_window.pairs.warp.
float-intervals → wiki/number.md, test/numbers/test_plus_minus_numbers.pairs.warp.
real-canonical-core, parameter-typed, Duck, #30, type-value-is, union-or, optional-result, instance-position, P45, `type(ø)` → wiki/type.md, test/types/test_type_rules.pairs.warp.
primitive-annotation, primitive-int-width → wiki/primitive.md, test/native/test_primitive.pairs.warp.
map-protocol, lowercase-protocols → wiki/trait.md, test/types/test_map_protocol.pairs.warp.
matrix-protocol → wiki/trait.md, test/modules/test_matrix_typed.pairs.warp.
effect-annotations → wiki/effects.md, test/functions/test_effect_annotations.pairs.warp.
P203 → wiki/type.md, test/welcoming/test_warning_mode.pairs.warp.
error-type → wiki/optional.md, test/control/test_error_not_in_data.warp.
position-index-words → wiki/class.md, test/types/test_class_positions.pairs.warp. D5 → notes/matching.md.
library-constant-refused, lambda-assign-writes-through, let-reassign, let-parallel, P71, P124, P73, data-stays-data, runtime-dates, module-global-assign, global-plain-write, shared-captures, handler-blocks-share, Mutating, count-words-library, range-end-once, kebab-or-minus, operator-even-spacing, dot-space-property, default-parameter-def, stored-visits → wiki/variable.md, test/scope/test_variable_rules.pairs.warp.
charged-values-rule → extensions/sugar/heads.warp. Data → wiki/charged.md. watched-cells → core (watched cells).
P72, handlers-wait-for-blocks → historic (git). esoteric-phrases-commented → notes/test_triage.md.
#6 → wiki/variable.md (kebab-or-minus).
def-brace-body, map-all-canonical, call-function-values, function-in-list, P82, arctan-allow, `sum := fold +`, P50, P37, labels-calls, P52, P132, check-assert, library-words-typed, runtime-function, night-forms-kept, path-type → wiki/function.md, test/functions/test_function_rules.pairs.warp.
sleep-seconds → test/numbers/test_decimal_units.pairs.warp.
letters-only → test/lists/test_down_and_letter_ranges.pairs.warp.
shown-by-warp → test/numbers/test_units_as_values.pairs.warp.
path-literals-preferred → wiki/significant-whitespace.md, test/text/test_fix_path_literals.warp.
use-script → wiki/script.md, test/modules/test_unix_words.pairs.warp.
type-library-words → wiki/standard-library.md, test/modules/test_std_qualified.pairs.warp.
instant-vs-duration → wiki/time.md, test/types/test_partial_time_comparisons.pairs.warp.
cross-module-overload → wiki/script.md, test/types/test_traits.pairs.warp.
default-methods, class-members-free, field-forms → wiki/function.md (Methods), notes/dispatch.md.
library-own-words → AGENTS.md (Never repeat code).
music-plain-library, domains-plain-libraries → AGENTS.md (Plain warp first).
definition-name-canonical → AGENTS.md (Word choices are aliases). batch → cards (to build).
cleanup-cuts, int-overflow-shape, keywordless-def, builtins-name, self-hosting-focus, parser-first, standalone-compiler → historic (git).
rename-clashes → card file-own-words. diagnostics-files → core/analysis/diagnostics.warp.
each-field-broadcast, lazy-range-value, pair-items, bare-pair, to-inclusive, #4/P40, P43, P34, `[x]*n`, list-append-plus, reconsider-spaced, P175, P200b, swizzles, curly-group, #25/#36, `pixels size` → wiki/list.md, test/lists/test_list_rules.pairs.warp.
ordinal-dot-index → wiki/indexing.md (no ordinal dot form; nothing to pin).
node-part-clash → lib (key_of … readers), wiki/node.md.
stored-table-name → wiki/keyword.md, test/modules/test_stored_relations.pairs.warp.
comparison-after-arms, for-over-iterator, rewrite-rules, match-alternatives, `if c {1} else 3+1`, P21, P55, #39 → wiki/switch.md, test/control/test_control_rules.pairs.warp.
raw-rules → core/passes/rewrites.warp (where raw). match-whole-value → wiki/pattern-matching.md.
pattern-roadmap → wiki/pattern-matching.md (roadmap: cards pattern-clauses, binding-alternatives, whole-binding).
valued-field-default, P115, P122, P162, P123, enum-case, enum-raw-values → wiki/class.md, wiki/enum.md, test/types/test_class_rules.pairs.warp.
kinds-as-cases → core (ValueKind enum). declared-class → core (emitter: class structs). tag-assign → wiki/html.md.
finish-await, stopped-no-result, end-refused, task-to-value-awaits, go-unified, after-condition → wiki/async.md, test/control/test_task_rules.pairs.warp.
eval-explicit-with → wiki/evaluation.md, test/control/test_task_rules.pairs.warp.
P109 → wiki/signal.md, test/control/test_task_rules.pairs.warp.
P163 → wiki/keyword.md, test/control/test_task_rules.pairs.warp. task-tests-answers → wiki/async.md.
block-hole-empty-braces → extensions (rule block holes). quote-typo-fix → historic (git).
implicit-operators → wiki/number.md, test/modules/test_module_rules.pairs.warp.
errors-one-rule, error-held-or-raises → wiki/Error.md, test/modules/test_module_rules.pairs.warp.
foreign-signature, bodiless-imports → wiki/haskell.md, test/modules/test_module_rules.pairs.warp.
P169 → wiki/use.md, test/modules/test_module_rules.pairs.warp.
host-callback-exports, packages-by-name, uniscript-on-literal, file-words-implicit, bare-append-file, P182, D15, P94, #14e, package-forms → wiki/use.md and the tests citing it.
write-makes-folders → lib/file.warp (write makes folders). package-repos → github.com/pannous/warp-packages.
P194, P88, P228 → historic (git). P146 → web/playground/lib.
markup-handler-comma → wiki/html.md, test/web/test_markup_and_note_rules.pairs.warp.
note-value → wiki/sound.md, test/web/test_markup_and_note_rules.pairs.warp.
select-shorthand → wiki/html.md (select).
markup-auto-render, instances-not-plural, click-by-id, markup-keeps, html-meta, key-if, Web, P232, ORM, P235, gpu-multipass, play-dispatch, save-sound, markup-plain-library, rewrite-general, block-lazy-body, serve-plain, transitions-data, data-names, music-plain-details → wiki (html, sound, shader, pattern-matching pages) and the tests citing it.
web-words → AGENTS.md (canonical-output; host words stay internal). P186/P187, signal-samples → historic (git).
P221 → wiki/html.md (routes). site-budget, Hosting → notes/hosting.md.
mp3-true-length → lib/sound.warp (mp3 length).
P78, P145, P241, constant-words, spaced-definition-head, named-argument-phrases → wiki/function.md, wiki/assignment.md, test/welcoming/test_newcomer_rules.pairs.warp.
warp-format, beautify, cli-words, P102, no-executable-by-default, P77, P222, test-soft, fuel-metering, fast-math → wiki/cli.md.
kebab-hint-both-fixes, errors-name-file, Newcomer, Asks, Got-it, warning-levels, canvas-zero, P75 → notes/welcoming.md, notes/fixits.md.
prebuild-fails-loudly → scripts/rebuild.sh (prebuild fails loudly). hello-size-exports → core (emitter exports).
sample-spellings → AGENTS.md (Samples are not the spec). drop-pure-word → AGENTS.md (warp self).
P107 → historic (git). Word → AGENTS.md (Word choices are aliases). Error → web/playground (error highlighting).
julia-compat → notes/taste.md (Familiar conventions first). #7/P92 → wiki/Error.md and the tests citing it.
function-words-consume → wiki/function.md and the tests citing it. P86 → wiki/operator.md.
named-arguments-general,, test-host-deleted, core-audit-30h, use-sound-minimal-fix, pinned-tests-update, error-report-detail, bus-test-outcome, playground-today, markup-keeps, operator-rows-stay, parser-cleanup, plan-only-hold, push-through, l4-prelude, self-compile-speed-next, card-sprint, scope-use-packages, list-map-separator, mutable-array, sprint-cleanup, tests-save-to-temp, ordinal-index-words, time-fields-before-ufcs, wasmtime-ffi-test-deleted, pass-rate-sprint, wasm-fixture-paths, dir-membership-test, samples-focus, tour-wasm-modules, urgent-watch-on, red-samples, night-kept, dir-test-order-free, subscript-slices-dropped, reflection-dump-tests, branch-cleanup → historic (git).
native-audio, sample-count-rounds → lib/sound.warp.
triage, note-tests, tests-held-in-bulk, foreign-forms-held, types-esoteric-lines, property-tests-held, hold-js-setter-line → notes/test_triage.md.
reuse-giants → notes/taste.md (Stand on giants). generated-sample-files → .gitignore.
docs-edit-directly, no-autostash → AGENTS.md (Where to work). dates-later → card dates-times.
module-arity-overloads → core (linked modules). gpu-first-asap → cards (gpu shaders).
let-inside-mark, unused-imports-pass, shared-subtrees, def-first, unit-plurals, emit-final-structs, rewrite-line-defines-nothing, l5-load-time-linking-shape, map-chain-fusion, module-link-hard-error, inline-block-library-words, same-multi-dispatch, nfc-tables, samples-modules-wasm, window-in-warp, linked-class-shapes, eprint-stderr, outside-core-call, window-ffi-fixes, listed-prebuilt-only, varying-own-writes, descriptor-method-slot, sugar-program-place → core.
no-slow-tests → notes/agents/handbook.md (Testing). refuse-never-guess → AGENTS.md (Refuse, never guess).
canonical-output → AGENTS.md (Word choices are aliases). library-direct-refs → lib (native libraries).
heap-per-run, use-js-tests, library-search-paths, host-folder, primitive-c-in-driver, repl-editing-keys → core/native.
rebake-tables-only → scripts/rebuild.sh. compile-speed-first → AGENTS.md (Smaller, faster compiler).
lib-hash-rebake, kill-guard, no-lib-reads, build-slots, smoke-reuse-stored, engine-wording-unpinned → AGENTS.md (Testing).
night-decisions → notes/agents/night.md. rules-to-typed-operators → card rules-to-typed-operators.
core-lines-in-questions → AGENTS.md (Clean compiler first).
egraph-outside-library → github.com/pannous/warp-packages. express-mode → AGENTS.md (Express mode).
milestone-messages → notes/agents/supervisor.md. play-mozart-mp3 → samples. power-function → card power-function.
quasi-text → card quasi-text. plural-singular-check → lib/units.warp.
js-webidl-bindings → extensions/foreign_globals.warp. catch-blocks → card catch-blocks.
test-case-timeout → core/native/tests.warp. parallel-compilation → card parallel-compilation.
easy-cards-first → AGENTS.md (Quick cards first). error-gaps → cards. inline-wasm → card inline-wasm.
webgpu-shader-part, shader-hash → lib/graphics. smoked-merge-push, busy-main-landing → AGENTS.md (Done).
module-rule-words → card module-rule-words. never-repeat-code → AGENTS.md (Never repeat code).
one-owner-per-card → AGENTS.md (Cards). sound-rest → card sound-silence-rename. midi-native → lib (midi).
c-lines-stable → samples/system/c_lines. app-webview → lib/webview.warp. sample-checks → scripts (sample checks).
deploy-not-gated, deploy-on-push → notes/agents/integrator.md (Playground). page-text-frames → web/playground.
local-folder-clash → wiki/use.md. sound-in-warp → lib/sound. linked-module → extensions/class_methods.
shader-import-quote → extensions. function-words-as-values → card composition-value-fails. parked → cards (parked).
tests-roadmap → notes/tests_triage.md. vitals-links-outside-core → scripts/vitals.py.
one-name-list → wiki/variable.md, test/lists/test_list_word_rules.pairs.warp.
keys-enum → wiki/graphics.md, test/control/test_key_enum.pairs.warp. show-hook → superseded by format-as-text-class.
shown-spaced → wiki/comments.md, test/types/test_show_hook.pairs.warp.
implicit-use → test/control/test_blocks_run_by_call.pairs.warp.
each-all-broadcast → wiki/Julia.md, test/lists/test_each_all_apply.pairs.warp.
as-promotes → test/numbers/test_float_contagion.pairs.warp.
exact-roots → wiki/number.md, test/control/test_try_index_and_float.pairs.warp.
captured-same-variable → superseded by inner-assignment-refused (wiki/variable.md).
no-count-membership → wiki/list.md, test/lists/test_count_question.pairs.warp.
block-heads-from-signature → wiki/block.md, test/control/test_empty_block_hole.pairs.warp.
reflection-words → wiki/reflection.md, test/control/test_reflection_words.warp.
compare-equal-hooks → test/control/test_brace_next_line.pairs.warp.
float-contagion → wiki/Extension-hooks.md, test/numbers/test_float_contagion.pairs.warp.
no-dot-call-broadcast → wiki/Julia.md, test/lists/test_broadcast_words.pairs.warp.
dot-operators-now, immutable-tuple → wiki/list.md, test/lists/test_list_word_rules.pairs.warp.
op-assign-general → test/operators/test_operator_forms.pairs.warp.
dir-map-keys-first → test/lists/test_list_word_rules.pairs.warp.
nat-and-int → wiki/library-template.md, test/control/test_plural_field_of_elements.pairs.warp.
type-word-aliases → wiki/library-dogfood-apps.md, test/lists/test_range_count_sum.pairs.warp.
article-optional → wiki/inventions.md, test/types/test_article_type_test.pairs.warp.
mixed-number-glyph → wiki/number.md, test/numbers/test_number_word_rules.pairs.warp.
relative-paths-cwd → test/control/test_data_quoting.warp. source-relative-prefix → wiki/significant-whitespace.md.
source-dir-words → wiki/library-regex.md, test/modules/test_file_lines.pairs.warp.
gpu-bound-floats → wiki/async.md, test/gpu/test_gpu_int_lists.pairs.warp.
dot-glyphs → wiki/standard-library.md, test/modules/test_implicit_word_arity.pairs.warp.
middot-scalars, color-light-adding, bang-overloadable → test/numbers/test_number_word_rules.pairs.warp.
member-after-dot → wiki/function.md, test/functions/test_member_beside_nullary.pairs.warp.
declared-int-result → test/numbers/test_declared_int_result.pairs.warp.
shown-separator → superseded by shown-separator-wins.
overload-set-value → wiki/function.md, test/functions/test_overloaded_function_passed.pairs.warp.
named-args-colon → wiki/function.md, test/functions/test_preposition_calls.warp.
bare-overloaded-name → wiki/function.md, test/functions/test_bare_overloaded_names.pairs.warp.
foreign-type-names → wiki/type.md, test/types/test_primary_constructor_fields.pairs.warp.
generic-letter-alias → test/types/test_declared_letter_alias.pairs.warp.
real-canonical → wiki/Julia.md, test/numbers/test_real_canonical.pairs.warp.
implicit-def-parameters → test/functions/test_def_forms.pairs.warp.
then-not-pipe → wiki/multi-value.md, test/operators/test_then_pipe.pairs.warp.
library-words-lexical → test/functions/test_library_words_lexical.pairs.warp.
type-of-small-local → wiki/equality.md, test/types/test_type_static.pairs.warp.
library-word-visibility → wiki/cargo.md, test/functions/test_foreign_modifiers.pairs.warp.
triage-pipes → wiki/function.md, test/numbers/test_number_word_rules.pairs.warp.
pixels-bytes → wiki/string.md, test/scope/test_variable_named_like_method.warp.
cargo-semver-version → wiki/cargo.md, test/ffi/test_cargo_unbuilt.pairs.warp.
class-text → wiki/sound.md, test/programs/test_note_equals_text.pairs.warp.
param-as-type → test/functions/test_parameter_kind_by_scope.pairs.warp.
xs-ys → wiki/variable.md, test/lists/test_absent_sum.pairs.warp.
typed-null → test/types/test_field_of_absent.pairs.warp. all-words-in → superseded by all-any-in (wiki/list.md).
bare-comparison-refusal → test/operators/test_then_pipe.pairs.warp.
markup-done-selector → wiki/Julia.md, test/control/test_database_filters.warp.
program-test → wiki/keyword.md, test/control/test_pure_nil_in_map.warp.
mutation-phrases → test/modules/test_english_phrases.warp.
loop-variable → wiki/inventions.md, test/control/test_loop_variable_units.pairs.warp.
with-do-block → test/functions/test_with_blocks.pairs.warp.
constant-define-operator → wiki/wasm.md, test/scope/test_constant_alias.pairs.warp.
purity-predicates, cancel-timer → test/functions/test_definition_rules.pairs.warp.
add-xs → wiki/list.md, test/lists/test_add_beside_operator_classes.pairs.warp.
methods-declared → superseded by one-receiver-name.
implicit-copy-call → wiki/wasm.md, test/functions/test_captured_copy_of_global.warp.
program-defining → wiki/function.md, test/functions/test_csharp_functions.pairs.warp.
compile-time → wiki/alias.md, test/text/test_fix_path_literals.warp.
jsx → wiki/html.md, test/text/test_format_keeps_pairs.warp.
contracts-by-assert → test/control/test_check_assert.pairs.warp.
time-command → wiki/function.md, test/time/test_time_command.pairs.warp.
text-not-str → wiki/alias.md, test/numbers/test_big_int_text.pairs.warp.
markup-as-html-value → test/web/test_markup_tag_each.pairs.warp.
unit-after-number → test/numbers/test_unit_after_parentheses.pairs.warp.
instant-bare-number → test/time/test_instant_duration.pairs.warp.
lowercase-key-comparisons → wiki/constructor.md, test/control/test_lowercase_key_names.warp.
matrix-class → wiki/async.md, test/control/test_check_holds.warp.
forced-overloads-refused → wiki/uniscript.md, test/types/test_forced_conversions.pairs.warp.
debug-positions, debug-positions-auto → wiki/data.md, test/errors/test_caught_error_place.pairs.warp.
format-as-text-definition → test/control/test_library_signatures.warp.
decrement-immediate → wiki/Todo.md, test/control/test_loops_in_functions.pairs.warp.
c-style-shadows → test/scope/test_local_shadows_library_word.pairs.warp.
override-keyword → wiki/keyword.md, test/types/test_class_cases_ported.warp.
plural-loop-variable-dropped → test/control/test_for_unit_words.pairs.warp.
test_wasm → wiki/graphics.md, test/wasm/test_wasm.pairs.warp. optional-bang → test/control/test_do_block.pairs.warp.
runtime-cleanup → wiki/keyword.md, test/control/test_reflection_words.warp.
batch-c → wiki/pattern-matching.md, test/control/test_glued_block_words.pairs.warp.
class-operators-generic → wiki/library-complex.md, test/web/test_shader_complex.pairs.warp.
page-globals-implicit → wiki/library-json-tools.md, test/control/test_try_host_failure.warp.
foreign-number-broadcast → test/types/test_extension_methods.pairs.warp.
native-timers → test/control/test_clock_timers.pairs.warp.
enum-first-letter-case → test/types/test_time_zones_at_run_time.warp.
text-words → wiki/plural.md, test/text/test_words_of.pairs.warp.
are-in-prefix → wiki/list.md, test/lists/test_are_in.pairs.warp.
fraction-written-form → wiki/library-fractions.md, test/numbers/test_fraction_written_form.pairs.warp.
key-statement-binds → wiki/variable.md, test/control/test_key_bindings.pairs.warp.
shown-separator-wins → wiki/library-map-helpers.md, test/control/test_braceless_call_in_branch.pairs.warp.
local-now → test/time/test_now_utc.pairs.warp. time-utc-property → wiki/time.md, test/time/test_now_utc.pairs.warp.
literal-targets → wiki/hyperreals.md, test/lists/test_linear_arrays.pairs.warp.
fraction-glyphs → wiki/number.md, test/numbers/test_mixed_numbers.pairs.warp.
gathering → wiki/function.md, test/functions/test_gathering.pairs.warp.
notes-are-notes → test/programs/test_notes_add.pairs.warp.
glued-binds-tighter → wiki/comments.md, test/text/test_format_keeps_pairs.warp.
quotient-period → wiki/number.md, test/numbers/test_type_first_declaration.pairs.warp.
play-one-word → test/programs/test_play_one_word.pairs.warp.
all-any-in → wiki/list.md, test/lists/test_all_any_in.pairs.warp.
exact-sums → wiki/hyperreals.md, test/numbers/test_exact_sums.pairs.warp.
dollar-any-element → test/web/test_page_elements.pairs.warp.
as-text-not-cast → wiki/charged.md, test/types/test_pure_class_methods.warp.
single-verbs → card single-verbs (notes/single_verbs.md).
is-node-case-pattern → wiki/type.md, test/control/test_is_node_case.pairs.warp.
class-equality-glyph → test/types/test_class_equality_glyph.pairs.warp.
operator-def-heads → wiki/keyword.md, test/types/test_class_operator_heads.pairs.warp.
old-reds-class-methods → wiki/node.md, test/types/test_class_methods.warp.
link-modules-lazily → core (module linking).
subtype-less-than → wiki/alias.md, test/types/test_marked_numbers.pairs.warp.
int-fits-named-int → wiki/notes.md, test/ffi/test_ffi_signatures.warp.
complex-in-shaders → test/web/test_shader_complex.pairs.warp.
operators-named → wiki/uniscript.md, test/operators/test_then_pipe.pairs.warp.
def-colon-body → test/errors/test_parameter_value_colon.pairs.warp.
one-receiver-name → wiki/Extension-hooks.md, test/types/test_class_extensions.pairs.warp.
ambiguous-result-error → wiki/function.md, test/types/test_result_overloads.pairs.warp.
declared-block-parameters → wiki/block.md, test/control/test_block_parameters.pairs.warp.
function-type-specific → wiki/function.md, test/functions/test_function_value_overloads.pairs.warp.
overload-signatures → wiki/reflection.md, test/control/test_library_signatures.pairs.warp.
text-to-int-trim → test/welcoming/test_footguns.pairs.warp.
color-hash-index-dropped → test/control/test_condition_block.pairs.warp.
int-types-are-classes → wiki/alias.md, test/types/test_numbered_enum.warp.
cheap-classes-everywhere → wiki/Todo.md, test/control/test_match_character_classes.pairs.warp.
map-function-definition → wiki/code.md, test/functions/test_definition_rules.pairs.warp.
inner-assignment-refused, let-local → wiki/variable.md, test/scope/test_inner_assignment_refused.pairs.warp.
fields-are-functions → wiki/library-matrix.md, test/functions/test_fields_are_functions.pairs.warp.
blocks-run-by-call → wiki/full-evaluation.md, test/control/test_run_time_blocks.pairs.warp.
defaults-bare-call → wiki/task.md, test/functions/test_defaults_bare_call.pairs.warp.
bf16-f8-shaders → wiki/standard-library.md, test/lists/test_node_lists.pairs.warp.
class-types-checked, bare-number-to-unit → test/types/test_dimension_parameters.pairs.warp.
zone-literal-kept → test/types/test_time_programs_run.pairs.warp.
typed-places-check → test/types/test_optional_type_words.pairs.warp.
pipe-after-braceless-call → test/functions/test_function_valued_definition.pairs.warp.
function-valued-definition → wiki/pipe.md, test/text/test_scan_by_index.warp.
build-kinds-detected → wiki/cli.md (build outputs, Later cards).
language-layers → AGENTS.md (Three layers), notes/layers.md.
appends-line, refusal-place-of-lib → notes/welcoming.md. alias-mechanism → lib/aliases.warp.
alias-arrows → wiki/alias.md, test/text/test_fix_alias_arrows.warp.
operator-word-optional → wiki/operator.md, test/operators/test_operator_word_optional.pairs.warp.
after-sum → lib/prelude.warp. name-glue → extensions/sugar (rewrite templates). warp-is-tests → notes/is_tests.md.
wasm-instruction-body → wiki/wasm.md. sound-buffer-lib → lib/sound.warp.
pure-it-block, self-parser-drops, sugar-twins-drops, Language, class-index-test, global-word-marker-not-needed, lib-oddities → historic (git).
unicode-glyphs → core/parsing. call-takes-right → wiki/function.md, test/functions/test_definition_rules.pairs.warp.
leave-as-is → historic (git; leave as is). operators-are-functions → wiki/operator.md.
typed-builtin-variant → wiki/function.md. method-by-type, method-routing-by-type → core (method routing).
compiler-node-enum → core/node.warp. sugar-triage → AGENTS.md (Three layers).
exports-kept, package-reexport, implicit-standard-words → wiki/use.md.
uniscript-binary-import → github.com/pannous/uniscript.
trim-default, build-includes-native, shown-warp-notation → wiki/cli.md.
juxtaposed-product-later → test/functions/test_function_rules.pairs.warp.
is-pattern-binds → wiki/pattern-matching.md. range-dots → wiki/list.md, test/lists/test_list_rules.pairs.warp.
glued-negative, of-binds-tighter → test/functions/test_definition_rules.pairs.warp.
computed-hole, typed-rule-holes, rule-not-in-own-definition → core (rewrite engine).
color-prints → lib/color (superseded by color-show-variants). on-frame-headless → lib/graphics.
modules-triage, after-participle-held → notes/test_triage.md. hole-calls-unreached, float-int-power → core.
error-expectation → core/native/tests.warp.
