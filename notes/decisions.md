# Decided questions (user answers)

Live decisions only, grouped by topic: the decision, then the reason when one was given. Look up a cited id (P71, D5,
#10, a slug such as gpu-f32) or grep a topic before asking the user; the code, tests and wiki are the truth.
Superseded entries are deleted (git keeps them; the old-id index at the end names the replacement). Implementation
details live in the topic notes, open and parked questions in notes/open_decisions.md, user to-dos on the board.
New answers go into their topic section with the date (pruned every 3 days, card decisions-prune).
Numbering: P = interview questions, D = wiki-survey questions (notes/wiki_features.md), #n = the first question lists
(2026-09-29/30).

## Priorities and process
- matching-first (2026-10-13): the matching roadmap (card matching-create) comes before any further self-hosting.
- self-hosting (2026-10-12): for dogfooding, independence from Rust and as a showcase; parser first, each warp part a
  twin of its Rust part with a differential test over every sample; gaps are fixed in the language (notes/self_hosting.md).
- self-hosting-postponed (2026-10-13): no further steps until the missing language features (topic parser-features)
  are in and the Rust code is cleaned up (topic cleanup); the existing twins stay.
- cleanup-only (2026-10-13): until the big Rust cleanup is done, new tasks come only from the cleanup roadmap; other
  workers finish what they hold; the Codex timer takes only cleanup cards; urgent Now cards still go out.
  Exception: use-cargo (crate components, downloads) keeps going, "important and doesn't touch the cleanup much".- weekly-cleanup (2026-10-14, user): a fresh cleanup roadmap every week (topic `cleanup-w<week>`, e.g. cleanup-w42);
  new cleanup cards go into the current week's topic (`todo add "…" --topic cleanup-w42`). Older roadmaps stay as
  they are, solved cards Done in them. Chores that should run regularly get `todo repeat <card> <days>`, and the
  daily `todo recur` brings them back to Next; they are not copied into each new roadmap.
- long-functions (2026-10-14, user): splitting an overly long function is a cleanup card of its own, also when it
  adds a few lines; the "only if it removes lines" rule is for deduplication, not for splits. Split by concept (one
  named helper per case group or per embedded warp template), not into numbered parts.
- reflection (2026-10-14, user; card module-methods): function objects have .name, .signature, .return_type,
  .parameters and .doc (a Python-like doc string, card doc-strings); printing one shows its doc as a comment line
  above its typed .dump. methods(module|class|value) returns the plain list of function objects, descriptions on
  demand (`methods(math) map .doc`). `use reflection` (reflection mode) adds compilation unit, compiler, origin,
  wasm index and effects. A bare name never becomes the function value implicitly (the `print now` case: a
  parameterless function must stay a call, so in a mixed list some names would print and others would be called): `function f` stays explicit, and a bare parameterless
  function inside print/help/dump/type gets a warning that it was called, hint `function now`.
- cleanup-means-fewer-lines (2026-10-13): the cleanup's metric is lines removed: simpler, more general, more elegant
  code. Splitting files or functions alone is not a goal ("not that impressed by splitting"). Each cleanup merge
  request states its net src/ line delta; baseline 161132 lines of src/*.rs at eaa40f98b.
- cleanup-may-drop-edge-cases (2026-10-13): in this cleanup a simplification may make edge-case tests fail. The
  worker marks each `#[ignore = "cleanup-edge: <card>: <what no longer works>"]` (assertions untouched) and lists them
  in its merge request; the Interviewer asks the user whether each edge case is worth the code it needed: yes → the
  code comes back, no → the test is retired. Core behaviour (samples, the tour, common forms) must keep passing.
- cleanup-bold (2026-10-14): "get rid of everything you can without asking": clearly superfluous code, duplicate
  paths, unused or rarely-run branches go without a question; we have everything in GitHub. The suite may drop to
  80 % passing (failures marked cleanup-edge as above, so they stay countable); samples and the tour should still run.
- error-type (2026-10-14): no ugly Rust types. A function that can fail has the sound type `T or error` (warp's
  union spelling, as in `as int or text`), shown and written like that, never as Result; not hidden as plain `T`
  ("`T or error` is not that ugly, keep it as a sound type"). Only the WIT export bytes carry a `result` for Rust callers.
  error-optional (2026-10-14, "YES!!!"): `T?` means T, nothing (ø) or an error; `T or error` stays the precise
  spelling. Rules: writing an Error into a field or list element raises ("a field holds no error: handle it or declare
  `or error`"), so data stays clean while results, locals and arguments carry it; `x is ø` is missing only, `x failed`
  failed only, `if no x` and `x or 0` cover both; at a WIT export a function that can fail gives `result`, one that
  only gives ø an `option`, one doing both `result<option<T>>`; `effects of f` tells which. Where a caller can take it, the Error value propagates instead of
  being raised (card error-gaps, wiki/Error.md "Not yet").
- use-script (2026-10-14): `use script` (alias `use script mode`) = `use relaxed` (no hints) plus the scripting words without further `use`:
  the modules os, file and net (env, exec, write, exists, list_files, lines, post …), plus new words where missing:
  `command("…")` (run a shell command, its output as text; the explicit opt-in to text-as-shell), `arguments` (the
  program's arguments as a list), `cwd`, `glob("*.toml")`, `stdin`/`input`, `sleep`, `exit n`. `use relaxed` stays
  the hint switch alone. P45's "script mode" is this pragma.
- sleep-seconds (2026-10-14): a bare number in `sleep 0.3` means seconds everywhere (like sh and Python), with the
  "needs a unit" warning suggesting `0.3s`; demo samples written with bare ms move to `300ms`. Run-time plain-number values count seconds too (`f(t) := sleep(t)`); no warning under `use script`.
- help-prints (2026-10-14): a statement-level `help(x)` prints, as in Python; inside an expression it returns the text
  (`t = help(x)` silent, `print help(x)` prints once).
- function-in-list (2026-10-14): inside a list or text a function prints as its name (`[gcd lcm clamp …]`); only a
  function printed on its own shows doc + definition (reflection).
- function-type (2026-10-14): `type(gcd)` prints `function: (a, b) -> int`, the kind followed by the signature;
  `gcd is function` is yes.
- parse-type (2026-10-14, from the user's draft "key !?!?!"; refined by body-code): `parse("1+1").type` is `code`,
  not `key`; `.op` keeps the operator.
- time-word (2026-10-14): a bare `time` is always the type (`print time` prints `class time`, as every type value
  prints `class int`, `class Person`: type-print; not `type time`, which pasted back evaluates to `type`; `class int` without
  a body is the reference to that class, like `function f`, so the printout reads back); the class member
  `time.now` is the local time of day, printing 22:27:46 (= `now.time`). instant, date, local time, zoned time, duration are subtypes of time (anchor-time).
  `now > 17h` / `now >= October` / `now ≈ 2026-10-14` compare the matching part; `==` stays exact (`now == 2026-10-14` no).
  `now.time` is the local time of day, printing 22:27:46 (`now.date` the date); the old `time > 18h` special case
  goes: an error with the fix `now > 18h`.
- canvas-size (2026-10-14): `window.width/height`, `display.width/height`, `canvas.width/height` are writable properties,
  natively and in the playground; a bare top-level `width` works too, with a did-you-mean naming the three.
- dollar-element (2026-10-14): in a page program `$xyz` is the element with id xyz (`$xyz.value`, `$xyz.focus()`);
  `$window`, `$document` are those globals; misspelled members are compile-time errors. A route's `$title` stays the
  submitted field's value.
- chars-codepoints (2026-10-14, the user's draft `['a','b','c']`): `"abc".chars` is a list of characters (codepoints),
  printing `['a' 'b' 'c']`; a literal `['a' 'b']` keeps its characters; `'a' == "a"` stays yes. A one-character text is a character, so
  `["a" "b"]` prints `['a' 'b']` and `["a" "bc"]` prints `['a' "bc"]`.
- body-code (2026-10-14, the user's draft "body should be block/code type?"): `type f.body` is `code` for any quoted
  program part (`f.body is code`); printing shows its source; `{x; x+1}` never types as map. No `.kind` member for now
  (parked: a body iterated as a list of statements, each with its kind).
- nearest-limit (2026-10-14): `nearest(x, rational)` limits the denominator to 1,000,000 like Python's
  limit_denominator (sqrt 2 → 665857/470832); `limit = 0` gives the exact fraction of the float.
- float-as-rational (2026-10-14): `x as rational` / `rational(x)` of a float is the exact fraction of its shortest
  printed digits, reduced (1.4142135623730951 → 14142135623730951/10000000000000000); nearest-limit is unchanged.
- restructuring (2026-10-14): new major roadmap after the diff review found few duplicates. Inventory every function
  by name, move functions to the file their concept belongs to (a general parser holds no keyword-specific syntax:
  `finish_if_prefix` goes to e.g. control/if.rs), and have agents check suspiciously similar names for true
  duplicates, which merge. Moves are pure moves (no logic change in the same commit); duplicates merged count as cuts.
- edge-cases-general (2026-10-14): special-cased border cases (one-off ifs, extra arms, word/shape checks, fallbacks
  for a single test) belong in the general machinery or go; cleanup investigation 2 reviews the week's diffs for them.
- cleanup-cuts (2026-10-13, batch from notes/cleanup.md "Feature overlap"): drop compile-time lambdas (run-time
  closures cover them), generators/yield, type-name parameters `fib int i` (samples move to `fib(i:int)`), scoped
  handlers `on … in {…}`. Keep return-type overloads, tuple returns ("multi return is definitely desired"), list
  element checks, instance destructuring, object-shaped parameters, dispatch by class. Rebuild small: traits as a
  thin alias of classes with default methods ("I don't see how that ever took 1900 lines"); ORM keeps only
  `stored people: [Person]`; shared int arrays stay as built, with a note that `shared` for any atomic variable is
  the general form for later; one rule `a of b` ≡ `b.a` replaces the
  reflection dot words; Ruby blocks via the block type and trailing lambdas. Natural phrases parked (tests ignored,
  maybe re-implemented via preposition parameters). Discussion card: all methods on classes as dispatch.
- foreign-spellings (2026-10-13): keep Rust `<T>`/`let mut`, Enum.map/List.map/Seq.sum, Python lambda, variadic
  lambdas (through the normal parameter path), JS destructured parameters, Ruby lambda/proc/blocks/to_s, C# LINQ,
  C++ `auto x`; foreign names are cheap aliases of existing words, not lowering code. Drop OCaml let-in and `fun`,
  Elixir `def …, do:`, C++ `[](…){…}` lambdas. The R/PHP forms move to warp files loaded with `use R` / `use PHP`.
  Kotlin `when x { 1 -> … }` is an alias of `match x { 1 => … }` (`->` ≈ `=>` in arms) with a warning, since `when`
  means other things too. `case` is an optional word before any match arm (`case 1: …`).
- real-runtime (2026-10-13, 2026-10-14): real.rs (compile-time symbolic reals) stays until symbolic reals are a
  run-time number type next to the exact ratios; then it goes. The generalized run-time exact mechanism is parked
  ("for later"), with its open points (sum in a denominator, undecidable comparisons) in notes/real_runtime.md; ⅈ
  comes after π ℯ √ ∛.
- cleanup-cuts-2 (2026-10-14): tests whose setup uses a cut spelling get only that line rewritten (`stored people:
  [Person]`, `play(440Hz, 10ms)`), assertions untouched. All phrase calls are parked, `play hum for 2s`, `melody …
  each 20ms`, `add 1 to 2` and gap feeding included, until prepositions become call keywords in the general call
  mechanism (`move x from a to b`, card prepositions-call); sound uses calls until then. Version literals stay but
  get rebuilt small ("why do we need 200 lines for this simple pattern?"), without the semver crate. The defined-twice
  diagnostic is cut. `nonlocal` only loses its error and checks: the cells behind P124 sharing stay, a nested def
  also shares `n += 1` without it, a plain `y = 5` there stays local unless `nonlocal y` (P124). rustfmt `use_small_heuristics = "Max"` comes
  in one fmt commit after the open cleanup branches are merged.
  emit-rows (2026-10-14): the emitter's instruction lists are packed one logical step per row, like WAT
  (`LocalGet(0), I32Const(1), I32Add, LocalSet(0),`), kept from rustfmt by #[rustfmt::skip] or an emit macro.
  Kotlin `when x { 1 -> … }` works, and `match` takes `->` arms and `else` too. `case` is an optional word before any match arm (`case 1: …`).
- real-runtime (2026-10-13): real.rs (compile-time symbolic reals) stays until symbolic reals are a run-time number
  type next to the exact ratios; then it goes.
- watched-cells (2026-10-13): watched variables use the run-time cell, except main-level variables no function
  writes, which keep the inserted check (same speed, about −300 lines).
- crash-fixes-first (2026-10-12): crash fixes merge with the highest priority (notes/roles.md).
- small-changes-on-main (2026-10-12): small or non-conflicting changes may go on main without a worktree.
- session-names (2026-10-12): a session is named for its current task, never a guessed area.
- consistency (card #376): an agent regularly checks the app against the docs; clear mismatches fixed, ambiguous ones
  go through the Interviewer.
- Questions to the user are always multiple-choice popups (2026-10-09), "so I get informed either here or via the
  interviewer".
- Every major feature ships with a sample in samples/ (2026-10-08).
- Un-ignoring a test that passes unedited needs no question; editing its assertions does (2026-10-05).
- P151 the crates.io package is `warp-lang` ("lang is perfect"), runtime `warp-runtime`.
- Language name (2026-10-08): warp everywhere; `.wasp` files still read; URLs/domains and wisp unchanged.
- Next big topic after effects (2026-10-08): stdlib-standard, the standard library as importable modules.
- #1 (2026-09-29): avoid panics everywhere, errors as values. #2 leave old commit trailers. #14 not-implemented
  errors unless easy; the rest is cleanup.
- cargo-fmt (2026-10-12): format the repo once, then the gate enforces `cargo fmt --check`.
- git-hook restore (2026-10-10): agents may restore only Cargo.toml/Cargo.lock (the build tweak); the hook backs up
  the diff first.
- P180 split-debuginfo stays "unpacked" plus the hourly sweep. P90 ./test.wasm is written at every compile ("keep
  always on"). P91 one shared analysis per compile, refreshed only when a pass changed the program.
- Repo layout: tests/ in topic folders, only probes condense into tests (P14, "OK as listed"); CLAUDE.md is a symlink
  to AGENTS.md (P15); the wisp format stays with a roundtrip test (P11); GcObject is the GC reading API (P12), the
  gc_struct!/wasm_struct!/wasm_object! macros stay as sugar (P97); test macros live in tests/common (P100).
- Vendoring is deactivated for now, kept for a later offline agent (P8); the cdylib is built only by
  web/playground/build.sh (P7).
- warp-ast.wit lives at samples/wit/warp-ast.wit (2026-10-09). scripts/own-warp.sh stays while callers use it.
- Bee/wasp entity names go into uniscript and are regenerated into src/uniscript_entities.tsv, never by hand.
- libm FFI: header-driven linking first, the LIBM_UNARY/LIBM_BINARY table only as fallback and as deliberate
  examples of hand-linked FFI (P13).

## Welcoming, diagnostics and newcomer forms
- Newcomer syntax is eaten, its intent compiled and the warp form hinted (2026-10-02), only where it does not clash
  with a known footgun.
- Asks become got-it warnings (2026-10-03): "the Ask mechanism is not what I expected … turn all the Ask into a
  warning with the got it feature", plus a later intent-based code change. Error-fallback cases (`[x]*n`, insert
  order, `[1 2 3]+4`, bare `a=1 2 3`, `1+2 squared`) stay loud errors naming the explicit forms (D3, D9/P5, D12).
- Got-it scope: `[y = this one, a = all of this kind, n]`; an expression is remembered by its text; a `// got it`
  comment silences the line.
- warning-levels (2026-10-13): levels hint < weak < normal < severe (no "strong"); errors stay separate.
- hints-are-warnings (2026-10-13, undoable): hints and warnings are one Diagnostic with a level; `--no-hints` and
  `use strict` are thresholds (defaults and level lists: notes/welcoming.md "Warning levels").
- canvas-zero (2026-10-10): a missing setting takes a sensible default ("who demanded … errors? We should have
  defaults"); errors only for ambiguity too dangerous to guess.
- P75 fix buttons read "I meant: <replacement>", the meaning as a tooltip.
- P78 modifiers from other languages (`public`, `static`, `inline`, …) are skipped with a got-it note; words with a
  warp meaning keep it.
- Word choices are not asked: the recommended word is canonical, synonyms are aliases with a note naming it.
- rust-forms-welcome (2026-10-13): warp may read Rust's forms (topic rust-reading), `<T>` included, as long as each
  has a nicer warp form; the warp form stays canonical.
- P157 no generic syntax: untyped functions work for every type; ported `fn id<T>(x: T)` compiles as the untyped form
  with a note "warp infers types". P161 no Swift argument labels ("I don't like that redundancy"); ported labels
  compile with a note.
- P167 Go's `Point{1, 2}` builds `Point(1, 2)` for a known class with a note; unknown names stay tagged data.
- P145 R's `x <- 3` assigns with a note; `x<-3` is an error naming both readings; `<-` is reserved for later, docs
  never use it.
- P241 (2026-10-12): no vector type, warp's list holds anything; `Vec<int>`, `vector<int>`, `Array<int>` … read as
  `list of int` with the note "warp says list".
- Error highlighting: CLI carets under the word; web red/amber wavy underlines with the message on hover.
- A bare word statement that names nothing is `undefined variable: foo`. #7 unresolved calls are errors; P92 `foo()`
  is "undefined function: foo", a bare `foo` stays a symbol.
- P86 adding values of unknown kinds builds `a + b` lazily; evaluated operands need Addable, else a type error, never
  a panic.

## Comments, separators and lexing
- `//` is a comment only before a space or the line end; otherwise floor division (`7//2`, `a //b`, `x//=3`)
  (2026-10-08, emphatic). A comment hiding a closing bracket is a parse error naming the `//`.
- `#` glued to a non-space is the count expression (`#s`, `#f(x)`); `# text`, `#!`, `##` and the directives `#use`,
  `#include`, `#import` stay comments/directives.
- `print a    print b` on one line is an error "two statements on one line? separate them with `;`".
- P227 (2026-10-08) a comma next to `==` without parentheses is an error asking for them ("insist on braces to avoid
  errors"): `(2.0, 4) == 2.0, 4`.
- P68 a braceless call whose argument holds an operator (`square 3 + square(4)`) warns naming both groupings.
- P143 `f x == y` is an error naming both groupings only when both type-check; types decide first (P143 refined),
  P149 an untyped parameter's type is inferred from the body first. Statement words (print, return) take the whole
  expression.
- P189 extra control-flow words stay display-only keywords; names that would be ambiguous need quoting, no guessing.
- P165 soft keywords (2026-10-06): hard keywords are control flow, declarations, literal values and module words;
  every other keyword is soft, usable as a local name, parameter, field or method with a note, never redefined at
  top level. P165b strict: `fun` is hard. P172 alias words (`fire` for emit) yield to a program's own definition.

## Numbers
- exact-decimals (2026-10-10): decimal literals are exact decimals (`0.1 + 0.2 == 0.3`), division gives exact ratios
  (`1/3+1/3` is 2/3). Floats come only from float sources (transcendental functions, sqrt of a non-square, host
  values, `as float`, float-typed places, e-notation) and are contagious; `==` warns only when a float is involved.
  Reason: exact unless the value flows into a float path; warp-perf measures the cost.
- int-list (2026-10-10): a whole decimal literal is int (`type([0.0, 0.0])` is list of int, P196b); a variable's type
  joins every write the analyzer sees.
- number-converts (2026-10-12): `number` keeps an exact value; only `float` converts.
- float-text (2026-10-10): floats print shortest-exact everywhere, like Python/JS repr (`0.30000000000000004`,
  `1.5e-7`). P225 a float joined to text uses that form.
- repeating-decimals (2026-10-11): only the overline `0.3̅` is the exact 1/3; `0.(3)` is not a repeating decimal;
  `x as decimal` writes the period overlined. Never for floats.
- D16 Int is unbounded (big-int promotion); `byte`, `int8..int64` are declared FFI types that trap on overflow; no
  `overflow` value. unknown-word (2026-10-11): `u8` = `uint8` = `byte`.
- hex-printing (2026-10-11): `255 as hex` is an int shown as 0xFF (uppercase), a byte shows so too; arithmetic gives
  a plain int (notes/hex_ints.md). binary-int (2026-10-12): `5 as binary` shows 0b101, `0b101 + 1` is 6.
- footgun-divzero (2026-10-10, Interviewer default, revisit any time): exact numbers raise divide by zero (catchable,
  `try`); computed IEEE floats give ∞/NaN (`(2 as float)/0` is ∞).
- gpu-f32 (2026-10-13): "Math is exact by default, except when doing lots of computation. GPU should be F32 too by
  default. With a little warning." Heavy float maps (327,680+ items) run on the GPU in f32 with a one-time notice
  (P240); the GPU functions default to f32 with the same warning; small computations stay exact.
- P240 (2026-10-10, revises P214): GPU math is imprecise by design (f32) and the user is told; heavy maps switch to
  the GPU automatically from 10× the measured break-even; `@cpu` and `WARP_GPU=off` keep f64; `@gpu @cpu` together
  is an error (notes/gpu.md "Precision"). Int lists fitting i32 may use the GPU with an overflow flag (P214);
  `@gpu` warns when the GPU does not apply.
- go-heavy-map (2026-10-13): a `go` map of a heavy kernel runs as the plain map, so P240 applies.
- P119 NaN canonicalization only where a float's bits are observable.
- approx-zero (2026-10-12, refines D8): `≈` is relative 1e-9 with an absolute floor 1e-12 (numpy allclose);
  `tolerance = …` overrides. P211 `~` is looser: 1% for numbers (`rough_tolerance`), texts ignoring whitespace and
  punctuation; `≈` on texts ignores case and accents. P212 a class may define `approximately` and/or `similar`.
- ± intervals: P217 `5 ± 1` is a worst-case interval ("I thought they are just interval"); Gaussian later as
  `5 ± 1σ`. P218 `x ≈ r ± 1` holds when |x−r| ≤ 1. P219 prints with 2 significant digits of the ± part. P220
  `1950 ± 50 AD` is the same ± value. P224 comparisons answer only when certain; P224b ("we don't want the
  application to crash"): known ± values need `certainly`/`possibly`/`.value` (compile error), run-time ones count
  an overlap as not certain with a one-time warning. P233 a field typed with a tolerance gives each value that
  tolerance.
- `x /= y` is exactly `x = x / y` (3/2 stays exact); floor division is `x //= y`; an `x:int` keeps a whole number.
- Trailing percent: `10%` is 1/10, `50 % of 200` is 100; infix `%` stays remainder; CSS values stay as written.
- P184 English number words are numbers when no variable has that name, with a hint toward digits.
- #11 exact reals may go into a declared float with precision loss (`float x = π`). rational-float (2026-10-10):
  `x: rational = sqrt(2)` is a compile error, `x: real = sqrt(2)` works. #21 `x=π; type(x)` is real; `2.0` is int.
- lossy-return (2026-10-12): a fraction reaching a declared int result is an error ("declare number, or truncate
  with as int"); lossless conversions still convert. float-result: `int square(x)`; `square 3.1` likewise.
- P49 a float to an int parameter is a compile error; P49b (re-asked 2026-10-12) a written `2.0` is refused too, the
  point signals float intent. P131 an int variable or field refuses a fraction loudly.
- P127 int class fields keep exact ints (no fixed-width overflow).
- P226 log glyphs: `b⌞x` is log base b of x; postfix `⌟` is a log (`ℯ⌟` 1). P226b `x⌟b` is log base b of x, `x⌟` ln.
- P57 hyperreals: `1/(1+ε)` stays an error; dual numbers and run-time hyperreals later. P56(2-4) `∞` is f64 infinity,
  ω the hyperreal infinite.
- Units: #5 units yes; #3 juxtaposition yes (`3x`), spaced only for units (`3 x` stays a list); units are written
  glued (`3km`), `3 km` works with a hint. P64 units are checked at compile time (F#-style) and kept as metadata where
  static knowledge ends ("units are very valuable in physics"). P36 `100 cm in m` / `as m` convert within a
  dimension, exact or a ratio; long unit names work. Unit sums use the finer unit. Quantities print without a space
  (`500m`); P231 print rounds quantities to a decimal, str() keeps the exact value. #31 `min` stays the function,
  `d` free.

## Text
- Text + anything concatenates in its str() form (2026-10-02, extended 2026-10-10); a function stays an error. A
  text spelling a number plus a number (`"3"+3`) warns and keeps "33" (error under `use strict`). An instance of a
  Printable type joins as its own text.
- P1 text * number repeats like Python (`"ab"*2`); a digit text warns that it is text; text*float and text*text stay
  type errors; `n times "ab"` is the explicit repeat.
- Interpolation (brace-hole, 2026-10-09): `\(…)` canonical; `\{…}`, `${…}`, `$(…)` work (all four "fine", no hint);
  bare `"$x"` is literal text (P6: "only the one with the curly braces must interpolate"); single quotes stay literal;
  `$` holes in sql/sh templates keep working.
- P126 texts inside containers print quoted, with double quotes ("Keep double quotes"); a top-level `print "a"`
  writes a.
- #35 lists join to "[1 2]"; `"x" as float` is a loud error. P27 `ord`/`ordinal`/`codepoint` give a code point,
  `int(c)` of a non-digit is invalid_number; P25 `int('5')` is 5. P74 `codepoint(c)` is the preferred name and gives
  an Int. P65 text-only arithmetic on a character is not_a_number.
- #26 upper/lower are Unicode, sort takes any comparable values, reverse works on text. Method broadcast:
  `names.upper` = `upper names`.
- Char as Text: an untyped parameter given a one-character text takes it as Text.
- P31 the Printable operation is `text(p:person)`, the one allowed type-word function name; `as text`, print and
  interpolation call it.
- P32 `print xs` of a list prints its str() text. P41 `print "x changed to " value` joins without a separator when the
  first part is a text literal; commas join with a space.
- result-keyword: after a print, `result` is the printed value.
- guillemets (2026-10-13, Interviewer's reading of "data as quasi text", undoable): «…» quotes its content as
  unevaluated warp data, a quasi-quote: `«1 2»` is the parsed node, `«x + 1»` the expression as data.
- Emoji: a single emoji is a codepoint; multi-codepoint emoji are texts, not names.

## Bool and truthiness
- bool-type (2026-10-07): "Yes is 1, no is 0", but yes/no are their own shallow type acting as 1/0; `type(yes)` is
  bool; print as yes/no; true/false accepted. `0 == false` is yes, `0 === false` is no. P195 `true + 1` is 2. P199 bool
  slots accept 1 and 0, other values are errors. A bool in an int place is 1/0, an `any` place keeps the bool; JSON
  true/false read as yes/no.
- Truthiness (2026-10-13, implied): ø + 1 stays an error, empty values are falsy in conditions, only 0 equals false.
  #38 a non-empty list is truthy, `[ø]` too. #28 `x=ø; x.size` and `[].count` are 0. `is empty` covers "", [], {}, ø.
- text-bool (2026-10-13): `as bool` and `bool(x)` are conversions and read the word, like `"80" as int`: "0", "no" and
  "false" give no, other texts yes. `if` keeps plain truthiness: any non-empty text is yes, `if "0"` runs the
  then-branch.
- in-bool (2026-10-12): `x in list` is yes/no; the position comes from `xs.index_of(x)`. `xs.has(x)` likewise.

## Types and annotations
- P203 lax versus strict (2026-10-08): annotations make it strict; unannotated code is lax (checked at run time);
  `--strict` warns on lax spots. Reason: "a general conflict between pythonic type laxity and strict typing".
- P204 a value of static type any entering an annotated place gets a run-time check. P201 `s: Shape = Circle(…);
  s.r` is a compile error with hints `match Circle(r)` / `s as Circle`; unannotated works.
- Central philosophy (2026-10-03): a value of truly unknown type is duck typed; a KNOWN different type with the right
  fields is an error teaching `image like photo` (P22); a written map is accepted structurally; `p.word` on an untyped
  parameter is a run-time "no field word".
- #30 only `is` tests types: `3 is int` yes, `3 == int` educates toward `is`. type-equal (2026-10-10): `is` is the
  subtype test, `==` between types exact; a bare type name is the type as a value.
- unknown-type (2026-10-12): an unknown declared type is always an error with the near type as fix.
- union-or (2026-10-12): in types `or` = `|` (`x: int or text`), both first-class; in values `or` is boolean.
- return-as (2026-10-12): return types are `def f(x) as int or text`; `->` works with a hint toward `as`; no `returns`.
- empty-field (2026-10-12, strict): ø needs `?` everywhere; `class Tree{left: Tree}` is an error "left can never be
  filled", fix `left: Tree?`.
- P185 `none(xs, f)`, the null spelling called with arguments, is an error naming the null.
- P125 optional parameters: `x=ø`, `maybe x`, `x?` are the same; typed `x:int?`, `maybe int`; `a ?? b` only for ø.
- P179 sum types auto-unwrap like optionals (`a:int = Some(3)`); payload fields by position `rgb(1,2,3)#1` (`.0`
  fallback); a payload-free variant is its symbol and `red is Color`. P178 enum cases with values are sealed classes.
- P45 a variable given kinds that don't mix (`x=[1]; x=5`) is a compile error ("unless we are in script mode", not
  defined yet); Int→Float widening stays. Undeclared lists stay `list any`.
- P28 `real x;` read before assignment is the zero value of its type (Go).
- P23 the type-word clash error covers every type word. P20 a function named like a type word is an error ("double
  is a type; rename your function"), P31's `text(p)` excepted.
- #9 `list of int`; plural type words (`numbers`) are lists; `list<int>` reads too. `x : 100 int` and
  `pixel:int[100]` declare typed arrays.
- `type(ø)` prints "empty". static-int: a constant call misfitting its declared int result is refused at compile time.
- D5 any noun can name a type/parameter (notes/matching.md).
- index-position (2026-10-12, user's idea): `index` (from 0, `xs[i]`) and `position` (from 1, `xs#p`) are
  dimensioned int types; index_of, argmax, argmin and insert take or return them by declared type, so
  `xs[xs.index_of(20)]` picks 20. position-arithmetic: ± int keeps the dimension, position − position is a count,
  position + position an error; a missing element is an error, never -1 (notes/index_position.md).
- schema-as (2026-10-13): `x as Config` gives x when it fits, converting field by field ("80" → 80), else an error
  naming the field; `x is Config` stays strict.
- D10/P17 dispatch on return type (`render "hello" as pdf`), ambiguous → got-it warning (notes/dispatch.md).

## Variables, binding and charged definitions
- let-reassign (2026-10-10): `let` is fully immutable (reassign, `+=`, `xs#1 = 5` are errors, fix "use var x"),
  `var` is the variable. let-mut: `let mut x` = `var x` with a loud warning.
- const-deep (2026-10-13): `const` is deep like `let`; `var` is the changeable one.
- P159 `const x=7; x=7` (same value) warns "remove the redundant assignment". P130 pi and the named constants are
  constants ("loud error, if it was declared constant before"); a class field named pi is the class's own.
- let-parallel (2026-10-13): the parallel let is only `a b = f(x) g(y)`; `let a b = …` is always a function
  definition. let-comma: `let a, b = 3, 4` unpacks.
- Data vs code (2026-10-05, P51, P62, P63, D7 revised): specified in wiki/charged.md, the single source ("def ==
  deferred", "def should behave exactly like Python … but close"). P70 late binding: a change is an error only
  where a later call sees it; `global` on the variable or in the reader; pure getters over constants may fold.
- P71 `:=` without parameters is always charged, re-evaluated at each use; assigning it after is an error. P138 Go's
  `total := 0; total += n` stays that error.
- P124 a lambda or nested `def` changing an enclosing function's variable (`n += 1`) shares it (like JS; nested def
  since cleanup-cuts-2 / interviewer option A), `nonlocal n` optional: it only shares a plain `n = 5`, which is
  otherwise a fresh local. Block arguments share captured variables too.
- P70c handlers and definitions created in a loop keep their own iteration's variable (JS `let`).
- P72 no memoization syntax; the compiler decides.
- P73 `x!` means force: a block runs, an optional unwraps, a plain value is itself. D2 (by position): `fn!` after a
  method mutates in place (`x.upper!`, `xs.sort!`), `{…}!` evaluates, await has its own word. P73's force applies
  to `x!` on a value or block; a `!` after a method call stays D2's in-place mutation (Interviewer, 2026-10-13).
- P69b a block arriving at run time is pure by default, run-time data tainted. P69c `warp compile` with run-time `!`
  warns and loads a shared compiler lazily.
- data-stays-data (2026-10-13): "data should stay data unless an explicit hole like ${} is used": a bare word in data
  stays a symbol; only `!` and `${…}` evaluate. Further levels stay open until a concrete case. wiki-full-evaluation:
  `created: now` stays a block until `!`.
- runtime-dates (2026-10-10): dates have a run-time form; `now` reads the clock at run time.
- module-global-assign (2026-10-10, default): a top-level `tempo = 90` sets the standard module's global.
- Mutating a main-level list in a function needs `global names`. P106 tasks share a variable only when declared
  `shared`. P44 `shared xs = int[n]` arrays with atomic `xs#i += v`; `atomic` is a synonym.
- range-end-once (2026-10-10): every `for` range end is evaluated once. #22 `N times {…}` evaluates N once; trailing
  `i++ while c` is a plain while; `a = 2 if c` guards the whole assignment.
- #23 a loop's `it` shadows a function's `it`, with a warning. P174 `mk(k) := { it * k }`: `it` stays the parameter,
  with a warning naming the lambda forms. P239 in a lambda naming its parameter, `it` is the outer `it`.
- #6 data as scope, warning when kebab parts are also variables. P81 the kebab-key warning offers three fixes.
- stored-visits: `stored visits default 0`; `stored visits = 0` works with a note.
- Ranges (2026-10-09): `r: 1..n` is the range itself, same as `r = 1..n`. `upto` excludes the end with a hint.

## Functions and calls
- type-library-words (2026-10-12): list/map/text library words are always available as `x.f()` and `f(x)`;
  `use list` only for qualified `list.zip`. P171 other modules' words need their `use` (error naming the module);
  `write` and `exists` are global; a file URL loads the file module.
- P141 (revised) a definition named like a builtin that would win is an error at the definition ("what's the point of
  even allowing to define it?"); a foreign export is called qualified, a bare ambiguous call is an error. P142 only
  where the builtin would win: library words may be redefined, methods always. Docs prefer broadcasting or
  `square all x` over `map`. P39 no `id`/`square` builtins; P191 `square` lives in math.
- P82 a function reference is `function add` or `&add`; a bare name that needs arguments is "add needs 1 argument".
  P83 an alias needs the reference (`g = function add`); an argument to a function parameter stays bare.
- arctan-allow (2026-10-10): `arctan := arc_tangent` names the function; partial application only with a hole
  (`add(1, _)`).
- P84 several juxtaposed arguments to a one-parameter function are one list (`f 1 2 3` = `f [1 2 3]`), "already
  done with broadcasting". `sum := fold +`.
- P50 a declared scalar parameter broadcasts over a list. P173 an unannotated parameter called with several kinds
  becomes `any`.
- P37 named arguments set parameters and free variables; no implicit capture, a missing argument is an error. P216
  named arguments run left to right as written.
- P52 a phrase-defined function is called with its own prepositions (`add 1 to 2`); elsewhere `to` is a range, `of`
  a field lookup. `f - x` with a user function is `f(-x)`.
- P24 suffix forms (`4 doubled`) call user functions. P164 a type word after a parameter name is its type
  (`phone number`); `phone-number:text` is one hyphenated name in declarations.
- P132 `{f: x => x * 2}` holds a lambda; a type word after the colon makes a typed lambda. P133 `|x| x*2` is a lambda.
- definition-cases (2026-10-13, cut by cleanup-cuts-2 2026-10-14): a second same-shape definition was "defined twice"; overloads
  by type are unaffected.
- P48 custom operators: `prefix operator ⁻ := …`, `suffix`, `infix operator ⊕ := left+right`; word infix operators
  `infix operator divides(d, n) := …`. P54 only declared operators are ranked (`has precedence above +`). P166
  element-wise operators on plain numbers act plainly.
- easy-add (2026-10-14): `x↑n` is power and `x↓b` is log only for numbers: they are typed infix operators
  (`infix operator ↑(x: number, n: number) := x ^ n`), not a fixed lowering, so other types define their own.
- values-number (2026-10-14): a system value is spelled `$mouse.x`; `$mouse_x` gives a hint naming `mouse.x`.
- D6 `|` is one operator dispatched on types: logical or, or pipe (`2|square|root`). operator-shall (2026-10-13): `&`
  and `|` of two whole numbers combine bits; `and`/`or`/`&&`/`||` stay logical; bits as a condition warn.
  amp-compose: `&` of two functions composes in reading order, `∘` alias. P158 `then` pipes only into a stage
  missing its argument with no `else`.
- `root` is an alias of sqrt. P2 tuple returns `return a, b` / `x, y = f()` as wasm multi-value. P3 closures keep
  boxing. #37 first-class functions by specialization are enough for now.
- runtime-function / print-closure (2026-10-13 defaults): function values compare by written definition and
  captures; a closure prints as written, `&f` prints the name.
- P238 a plain generator call collects the list; `iter(…)` gives the lazy object.
- check-assert: `check` asserts; float parameters stay IEEE, `==` exact.
- D4 constructors: `T{…}` of a known type constructs/validates, `k:{…}` is plain data; missing fields are errors
  unless `?` or defaulted, undeclared fields errors ("Strict"). #41 spaced `V { x: 1 }` constructs a declared V.

## Lists, maps and indexing
- #4/P40 `size` = count; bytes via `byte_size` / `number of bytes`. `len(x)` counts with a hint `#x`.
- P43 lists do not grow past the end: setting past it is a catchable `index out of range`.
- P34 a number subscript on an empty `{}` keys it as a map.
- single-entry (2026-10-12): "keep everything as it is and automatically upgrade it to a map": `{a:1}` stays a pair
  and becomes a one-entry map where a map is needed.
- stored-table-name (2026-10-14): `stored people: [Person]` names its table after the variable;
  `stored people: [Person] in "people_v2"` names it explicitly (replaces `= database.t`).
- element-named-pair (2026-10-14): a one-key pair named like an element, `{body: [{type: "image"}]}`, is markup
  (`<body type="image">`); as data it takes a second key or a quoted key. Larger maps with such keys stay data.
- #25/#36 a space before `[` never indexes; only glued `a[..]` indexes.
- P29 `pair.0` counts from 0. P30 `xs.pop()` gives the last item, `m.remove(k)` the value or ø.
- Word slices (2026-10-09): `xs from #2 to #4`, `xs starting from #2`, `xs up to #2` or ordinals, both ends inclusive;
  a bare number is an error ("up to 2 is very confusing").
- P58 `xs#a..b` keeps the range with a strong warning naming `xs#(a..b)` and `(xs#a)..b`.
- `[x]*n` / `[1 2]*2` are refused (Python repeats, NumPy multiplies); `n times [x]` repeats. `xs.insert` never
  guesses the order: `insert(x, at: i)`, kinds decide, two Ints are an error. D3 `.+` is element-wise, `xs + [4]`
  concatenates.
- list-sign (2026-10-12, refines D13/#40): in a comma-less list a space plus a glued sign starts a new item
  (`[a -1]` is [a, -1]); outside brackets `x -1` subtracts; never inside an index.
- P175 `xs where it > 1` filters with `it`; P223 filters may name fields bare (`people where age > 20`), a variable
  in scope wins with a warning. people-where (2026-10-10): no new `with`/`without` sugar ("the words have too many
  meanings"); the existing `with` filter and field `without` stay.
- P200b real references: maps and lists share like instances (`n = m; n.a = 2` changes m); `x.copy()` copies.
  From P200b, a typed list read from a field shares too. P215 covariant list assignment is an error where the alias
  is visible, else a run-time check per write.
- P176 repeated `key: value` in a named tag block are children; a plain `{…}` keeps the duplicate-key error.
- `pixels size` works like `size of pixels`.

## Control flow, matching and errors
- match-whole-value (2026-10-13): no binding syntax; inside an arm the subject's name and the word `value` are the
  whole matched value ("make it available as the matched variable or the special word value").
- match-alternatives (2026-10-13, "ideally both"): case keys are patterns, `1 | 2`, `1 or 2`, `1, 2` all mean either;
  `|` stays logical outside. match-type-union: `int or text` as a case key is the type test. match-other: `other` is
  a catch-all like `_`/`default`/`else`.
- match-exhaustive (2026-10-13): a match on a known enum or sealed class missing a case is a compile error.
- pattern-keyword (2026-10-13): named patterns only with `pattern Adult = {age: >= 18}`; a plain `Name = …` is always
  a value; names inside bind. Reason: rare in other languages, hence the visible keyword.
- #31 a switch without a match reports the run-time value (`no case for n = 4`).
- `if c {1} else 3+1`: the else takes the whole `3+1`.
- P21 `try X else Y`, Y a value or action ("I never invented the => Y syntax"). P60 `try {…} catch {…}` and Python's
  `try: … except:` are synonyms; function-level handlers later. P67 `catch e`: e is the Error value. #34 deep traps
  under `try` are caught. failed-raised (2026-10-10): a raised error stops the program; bare `try X` turns it into
  the value (`r failed`). try-finally: `try X finally Z` runs Z, X's value or Error is the result.
- P55 a while loop's value is its last body value. P213 a loop ending in `print` gives ø; loops stay expressions.
- `when` by shape: `when click {…}` = `on click`, `when x > 3 {…}` = `whenever`, `when x {… -> …}` = `match`,
  `when {cond -> …}` = an if chain; all four give the same one warning naming the word meant (user, 2026-10-13: the
  shapes are not unambiguous).
- P46 type-name and condition patterns in `for` filter with a got-it warning (`for friend in xs`, `for (it>2) in xs`).
- P61 `is` is always a comparison; with an undefined x it teaches `x be <value>`. #27 `be` is `:=`; `≤ ≥ ≠ × ÷ ¬ √`
  are accepted and normalized.
- #39 `==` never chains: `a<b<c` chains, `1<2==2` is `(1<2)==2`.

## Classes, objects and enums
- P115 bare field names read the receiver (`self`/`this` too). P116 `c.inc()` updates c. P117 a body definition
  without parentheses is a getter; `class b extends a` inherits.
- P122 static members need `static`; plain `pi = 3` is a per-instance default.
- P162 `init` is THE constructor; `value`, `constructor`, `__init__`, `initialize`, `new`, a class-named method …
  are aliases with a note; `new Point(1, 2)` works with a note.
- P123 an object's text is its constructor form `Point{x:1 y:2}`; classes are declared capitalized in docs.
- P177 `implements` / `struct Square: Shape` is a checked claim.
- P200c every object has `copy()` and `clone()`, overridable. P205 `copy()` is deep; `copy(shallow: true)` shallow.
  P206 resources inside are shared with a note (error under --strict). P207 `clone` always calls `copy`; a copy
  keeps its class. P208 `===` on objects is identity; preferred spelling `same` (`a same as b`).
- P196 `===` compares the full type; P196b `1 === 1.0` yes.
- enum-case (2026-10-13): a plain enum case is its own value (`print Coin.Dime` → Dime, `Coin.Dime as int` → 2,
  `2 as Coin` → Dime, `9 as Coin` an error, `f(5)` for a Coin parameter a type error).
- enum-int (2026-10-13, default): C functions and fields typed with an enum convert to and from its int; in warp an
  int becomes a case only explicitly. C-style enum values are on hold (WIT has no such enums).
- Reflection: `dir(x)` from compile time when known, else from WASM metadata; x.type, x.fields (alias members),
  x.methods, f.params, f.effects …; lookup order field → meta → reflection; one custom section warp.meta.
- P114 `x.meta` / `x.@key` stay; comments fill them only when activated (`use comments`). Meta on objects: `@key` in
  a literal is never a field; field first, then meta when reading `point.source`; iteration skips `@` keys.
- P230 `effects of f` gives symbols.
- field-or-minus (2026-10-12, replaces P168): `p.phone-number` without that field is the error "p has no field
  phone-number" with the fix `p.phone - number`; no silent subtraction.
- tag-assign (2026-10-10): every `=` in a tag body is a value entry like `:`.

## Signals, events and tasks
- P109 every variable can be watched; checks only where a listener watches. P111 writes through `global x` in called
  functions run the listeners. P112 no batching block; a multi-assignment notifies once.
- P163 `emit alarm{…}` runs `on alarm` handlers and continues; `raise`/`throw` are always errors; fire/trigger/signal
  alias emit with a note; `send alarm{…}` without `to` is emit. P202 an emit no handler receives warns.
- P129 `broadcast v on "chat"` / `send v to "chat"` reach `on message from "chat"` machine-wide; P129b named events
  cross programs. P155 channels: `channel()` local, `channel "chat"` machine-wide, one concept.
- P156 `whenever cond {…}` is edge-triggered. P38 listeners see later writes in their statements (current default).
- P128 `listeners of x`, `clear listeners of x`, `remove alarm from listeners of t`.
- P148 `old` (aliases previous, was, before) in `on change x`.
- P134 `on exit {…}`. P135 `on every day at 9:00 {…}`, `at 9:00 {…}` once. P136 system-value rules, one card per value.
- P120 the CLI keeps running while handlers listen ("listening: … (ctrl-c to stop)"). P121 `exit` ends the run.
- Effect handlers (2026-10-08): block-scoped `on` handlers and emitted events as typed effects; a block handler ends
  with `break value`.
- P33 real threads for `go` (WASM threads natively, Web Workers in the browser), last, as a big project. P47 a started
  job runs on its own; only a use of its value waits; `await all jobs`. P153 `await any [...]` races. P154 `await job
  within 100 ms or 0`, a compile-time note says the job stops on timeout.
- go-voice: a `go {}` task is a voice sounding along from where its starter stands.

## Modules, library and foreign code
- P169 `use math` loads a new math module with beautiful names ("we need to educate people about beautiful names");
  raw C math is `use cmath`; math may forward to cmath meanwhile.
- P182 Set, Stack, Queue, Deque, Counter only in `collections`. P183 std modules are backed only by warp, host words or
  C compiled to wasm; regex is the common Rust/JS subset; `file.append(path, text)`.
- P194 std/ merged into lib/; netbase in lib/extra/, later external in pannous/netbase (P193); long readable names.
  P192 netbase backend later as a first external-module experiment.
- D15 no automatic imports: `use folder`, `use package`, `use project`. #12 `use <file>`.
- P139 `import`/`use`/`require` only declare; `include m` runs m. P140 assigning an imported mutable global sets it.
- P146 C libraries compiled to WebAssembly are the way to reuse standard libraries (notes/stdlib_connectors.md);
  P147 the prebuilt libc.wasm lives in web/playground/lib.
- P88 for now every host grants every capability, untrusted too; the checks stay for later. P89 foreign runtimes get
  their own capability. #8 print's IO capability is granted implicitly. P26 libm counts as pure.
- P94 WIT `char` is a Codepoint.
- exe-foreign (2026-10-12): an executable using python/js starts them from PATH and fails loudly when missing.
- P228 programs using tables or JSON don't build stand-alone; the runtime stays ~1 MB; the note reads
  "<features> used runtime.".
- uniscript: entities are `\:name` in code and texts (P56(1)); an unknown entity stays as written with a warning
  (P56 revised, "must not stop the program"); bare `\alpha` is an error. P198 HTML names for letters, LaTeX for
  operators (exceptions in notes/footguns.md).
- bizarre-global (2026-10-11): play, beep, stop_sound, wait_sound are global; other sound words need `use sound`.
- #14e `download <url>` is an alias of `fetch`.

## CLI, building and tests
- P102 `warp build hello.warp` makes the native executable (`hello`, hello.exe on Windows); `--exe` accepted.
  build-should (2026-10-13 default): it writes x.wasm beside it too; `--wasm` only the module.
- no-executable-by-default (2026-10-12): a plain `warp file.warp` writes no executable; WARP_EXECUTABLE=1 turns it on.
- P105 `warp run` never writes an executable and never takes the machine-code path ("that's too slow").
- P77 an executable prints the program's output, then its value; exit 0, 1 on a trap. Float exit status: only an Int
  result is the exit status.
- P113 a ø result shows nothing. P236 `warp run` echoes the final value after the prints.
- P222 `warp serve [file] [port]` serves the page program; plain `warp app.warp` serves when the program declares a
  route/server (static check); CGI mode retired.
- build-links (2026-10-12): `warp build page.warp` makes an executable opening the page in the system web view.
- test-soft: `test C` records and continues, `check C` asserts, `test "name" {…}` is a test block. P209 test blocks run
  only under `warp test`. P210 all pass → one summary line; failures exit nonzero.
- P107 the playground offers only the .wasm download plus a `warp hello.warp` line.
- P108 Warp.sublime-syntax takes .wasp/.warp/.a/.angle.
- P229 double-clicking a .warp file opens the editor.
- Playground CI runs Chrome and Firefox in parallel; workers need not tour with --firefox.

## Web, markup and server
- P188 "Stay close to HTML and CSS; don't reinvent the web, only the basic machinery." web-keyframes: CSS's own
  `@keyframes`; warp adds only `leave:`.
- html-meta (2026-10-12): page facts are keys on html (`title:`, `description:`, `meta: {…}`) going into the head.
- key-if (2026-10-12): inside `on key {…}` bare `key` reads event.key; named keys compare as words (ESC, ENTER, UP …);
  letters stay text. held-key (2026-10-13 default): `held_key` is the held key's name.
- unknown-tag (2026-10-13 default): an unknown element is a warning with the near name; custom elements and
  components are not flagged.
- web-outside-state (2026-10-12): `counters#1.count` reads/changes a component instance; `Counter.all` hints toward it.
- component-calls (2026-10-13 default): components in loops keep the enclosing element as their hole.
- P186/P187 web defaults stand (DOM morph, identity by order). P190 page-test words stand.
- Web words: `local["k"]`, `session["k"]`, `database["k"]`, `clipboard.write`; reading the clipboard in a page is an
  error (async).
- select-shorthand: a list inside select{} gives one option per item; a map item `["a": "Apple"]` value + label.
- Form shorthand: `form post "/todos" {…}` = `form{method:"post" action:"/todos" …}`.
- P232 `$x` as the implicit subject's field only in served routes; `$a` in data references the enclosing node.
  P160 `@1` is the node with id 1, `ref 1`/`ref a` say so in words, `$<digits>` only positional.
- P221 one route line serves both sides: a route reading server data renders on the server first, then fetches data
  only ("both from your single line YES!!"); the single-page rule applies only to the playground demo.
- Served routes: a raising form route answers 400 and re-renders; missing field 400, unknown id 404. /api and /rpc
  give instances as plain objects.
- route-access: access rules later; a route gives out only what it displays.
- ORM (2026-10-08): plain classes connect to the database by registering, no inheritance; transactions optional;
  filters run any warp expression; lazy loading by smart default. Assigning a stored row's field writes through;
  `save bo` is accepted (notes/orm.md).
- site-budget (2026-10-12): the hello-world site budget is 100 KB gzipped, no ratchet.
- tour-firefox-stall: a Worker-run site ships task-workers.js.
- Hosting (2026-10-09): our own login first ("let's start with our own login and let people log in with their own
  button"), then deploy to one's own provider; three options: Cloudflare Workers, Ferron on pannous.com
  (`<name>.lambda.pannous.com`, sandboxed) and Fermyon/Akamai ("Both now"). Setup and accounts: notes/hosting.md.
- P235 WGSL is a `shader { … }` block (alias `wgsl`); P235b no capture of warp variables, but `$name` is an explicit
  hole (user's own edit). P234 `paint` takes a shader text and renders on the GPU; `use graphics` = `use draw`.

## Sound and graphics
- shader-untouched (2026-10-13, user: "I didn't mean to mess with the shader language"): WGSL in shader blocks stays
  as written, the `$name` holes are the only change; the colon result-type rewrite (87d15829e) goes. A WGSL error
  is reported as a warp error with the .warp file's line and column (card wgsl-error).
- play-dispatch (2026-10-12): `play hum for 2s` dispatches on what hum is (signal, stereo pair, block function); no
  snake case words. The phrase form is parked until prepositions-call (cleanup-cuts-2); meanwhile `play(hum, 2s)`.
- sound-at (default): `at 2 beats play C4` places sounds from the voice's first sound.
- signal-samples: samples/signal*.warp were renamed siren_sound/stereo_sound; the wiki "Signals" examples stay.
- note-value (2026-10-14): note names stay frequencies (`C4 * 2` = 523.25); `note(C4)` (alias note_value) is a Note
  that prints "C4", `note(445Hz)` "A4+20c", `n + 7` G4; `note("C#4")` now gives a Note, not 277.18.

## Old id index (superseded or historic; git has the text)
P35 → index-position. P42 → P227. P56(2-4) unknown entity → P56 revised. P63 → wiki/charged.md. P66 → footgun-divzero.
P103, P104 → no-executable-by-default + build-should. P110 → P163. P118, P214 floats → P240. P168 → field-or-minus. P170 → type-library-words. P200 → P200b.
D1 → P6 + brace-hole. D7/#33 → wiki/charged.md. D8 → approx-zero. D9, D12, D13 asks → got-it rule + list-sign. D14 →
in-bool. exact-default → exact-decimals. #40 → list-sign. P85, P87, P93, P95, P96, P98, P99, P101, P137, P144, P181,
P53, P59, P79, P80, P9, P10, P16, P19, #14b–d, #15–20, the Original questions 1–41: test edits and housekeeping done.
