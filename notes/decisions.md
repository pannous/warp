# Decided questions (user answers)

Live decisions only, grouped by topic: the decision, then the reason when one was given. Look up a cited id (P71, D5,
#10, a slug such as gpu-f32) or grep a topic before asking the user; the code, tests and wiki are the truth.
Superseded entries are deleted (git keeps them; the old-id index at the end names the replacement). Implementation
details live in the topic notes, open and parked questions in notes/open_decisions.md, user to-dos on the board.
New answers go into their topic section with the date (pruned every 3 days, card decisions-prune).
Numbering: P = interview questions, D = wiki-survey questions (notes/wiki_features.md), #n = the first question lists
(2026-09-29/30).

## Priorities and process
- reviewer-role (2026-10-15, user: "definitely yes"): warp-reviewer reviews each merge request's diff in parallel with
  the Integrator's gate, polishes small things itself in a follow-up commit, vetoes unsound changes before the push,
  and sends structural findings to the Refactorer as cards.
- matching-first (2026-10-13): the matching roadmap (card matching-create) comes before any further self-hosting.
- self-hosting (2026-10-12): for dogfooding, independence from Rust and as a showcase; parser first, each warp part a
  twin of its Rust part with a differential test over every sample; gaps are fixed in the language (notes/self_hosting.md).
- self-hosting-postponed (2026-10-13): no further steps until the missing language features (topic parser-features)
  are in and the Rust code is cleaned up (topic cleanup); the existing twins stay.
- cleanup-only (2026-10-13): until the big Rust cleanup is done, new tasks come only from the cleanup roadmap; other
  workers finish what they hold; the Codex timer takes only cleanup cards; urgent Now cards still go out.
  Exception: use-cargo (crate components, downloads) keeps going, "important and doesn't touch the cleanup much".
- weekly-cleanup (2026-10-14, user): a fresh cleanup roadmap every week (topic `cleanup-w<week>`, e.g. cleanup-w42);
  new cleanup cards go into the current week's topic (`todo add "…" --topic cleanup-w42`). Older roadmaps stay as
  they are, solved cards Done in them. Chores that should run regularly get `todo repeat <card> <days>`, and the
  daily `todo recur` brings them back to Next; they are not copied into each new roadmap.
- topic-priority (2026-10-15, user; card todo-topics): the column is the priority, for topic cards too: a topic sits
  in Now/Next/Soon/Later beside its cards (new topics Soon); Roadmaps holds only unprioritized topics. A Priority
  field may exist only if it always matches the column. `todo move sound Next --all` raises a topic with its cards.
  The user's own cards stay where the user put them: drafts, and since 2026-10-15 every issue without the line
  "Added by <session> with `todo add`." (older ones: ~/dev/bin/probes/card_creators.py matches data/github-quota/calls.tsv).
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
- dir-lists (2026-10-15, user): built-ins become function values first, type `builtin`, a subtype of function
  (`type(function upper)` is builtin, `function upper is function` → yes); methods(x) lists them, so dir(x) and
  methods(x) keep the same names (card builtin-functions, then dir-lists).
- one-method-form (2026-10-15, user; card one-method-form): methods are just functions: a class-body `def x()` is
  `def x(_:P)`, and `p.x() == x(p)` everywhere. `methods(P)` ("functions of p" a synonym) = every function whose first
  parameter is declared P, an ancestor or a trait P implements, wherever declared. `dir(p)` = fields + methods(P);
  untyped generic words (print, str) aren't listed, though `p.print()` still works via x.f() ≡ f(x). Where a
  function is declared is only metadata (`.declared_in`), never behaviour.
- rust-style-lint (2026-10-15, user; card parked in Later): of the proposed Rust rules only doc-moved is enforced (a
  /// doc that moved onto another item, --fix moves it back); use-at-top, card-comment and blank-line-between-items
  are not.
- warp-format (2026-10-15, user; card warp-lint): the linter is for warp code and unopinionated: only clear layout
  (indentation, spacing, separators, brackets, blank lines) in one canonical form, never changing meaning; no word
  or alias rewrites, semantic rules stay compiler hints. The command is `warp format`, `warp lint` a synonym; warp
  avoids abbreviations, so no `fmt`. The linter carries that rule as a soft hint (report only, never fixed): an
  abbreviated word or name (`fmt`, `str`, `cnt`) → "warp avoids abbreviations: format / text / count", built-in
  alias words included. Layout: tabs, `name: value` (parameter types too: `area(width: int)`, also in help/serialize/docs), binary operators spaced (glued only where spacing changes the
  meaning; every edit is checked by comparing parse trees), list separators kept as written, `{` on its header's
  line, one space after `,`, no spaces inside brackets, at most one blank line.
  Later (card warp-format-strict, Soon): a second, opinionated mode rewriting code to best practices, with a preview
  (show the diff, apply nothing) before anything is applied; every rule goes to the user first. It is a verb of its
  own, `warp beautify` (synonyms canonical, idiomatic); hash-comments is its first rule.
  beautify-preview (2026-10-15 night, Interviewer's proposal, kept by the user 2026-10-16):
  `warp beautify app.warp` prints the diff; in a terminal it asks "apply? y/n", piped or scripted it applies
  nothing. Amended by the user's card yes-fix (2026-10-16): piped lines answer it, one per file (`yes y | warp fix *`),
  the end of the input is no; a terminal that cannot see the question (output piped) still applies nothing. `warp beautify yes app.warp` applies without asking (scripts), `warp beautify check app.warp` only
  previews. Never a silent rewrite: unlike format, beautify changes words. Second rule (worker default, fits the
  user's "synonyms canonical"): alias words the compiler already notes are rewritten to the warp word via their
  I-meant fixes (`__add__` → plus).
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
- type-text (2026-10-16, warp-types default under type-print; card type-values; undoable): a type value is
  `class int` in every text form, not only print: `"type is " + type 3` → "type is class int", `t as text` and
  `"\(t)"` alike (one rule, like Python's str(int)); compare types as types (`type(x) == int`), so
  `"" + type(x) == "int"` is no. `type(int)` is class type.
- now-live (2026-10-15, user): `now` stays a live clock (each read is the current instant, `now - start` measures
  time); tests pin what always holds, `now <= now` → yes, not `now == now` (card flaky-browser-suite).
- canvas-size (2026-10-14): `window.width/height`, `display.width/height`, `canvas.width/height` are writable properties,
  natively and in the playground; a bare top-level `width` works too, with a did-you-mean naming the three.
- page-ids (2026-10-15, user; card page-ids; replaces dollar-element of 2026-10-14): an id in the program's own
  markup is a name in the program's scope: plain `who` is the element (`who.focus()`), as in JavaScript but checked;
  an id clashing with a variable or function is a compile-time error naming both, with a library word only where
  the program uses that plain name (`id: "map"` with `xs.map(f)` compiles; `id: "print"` with bare `print` errs); kebab ids like `user-name`
  resolve too. `window`/`document` are plain names. `$who` is always the element's value (text, number, yes/no for a checkbox, text content otherwise) and
  `$who = "Ann"` writes it, like a route's `$title`. Dynamic or external ids go through `element("user-name")`, not
  `${…}` (string interpolation). `$` means something only in warp code: markup text, single-quoted strings, `\${`
  and files read at run time stay text. Misspelled members are compile-time errors. Principle: one meaning per
  spelling, never two things under one name silently. Later card: backtick strings with `$x` interpolation (clashes
  with `$who`, noted there).
- call-as-grouping (2026-10-15): `f x as T` is `f(x as T)` everywhere, like `f 2 + 1` = `f(2 + 1)`, with a warning
  naming `f(x as T)` and `(f x) as T`; dispatch on the result goes via `pdf p = render "x"` or `(render "x") as pdf`.
- uniscript-binary-import (2026-10-15, user: "definitely yes"): ~/dev/uniscript gets wasm-only c/ffi additions
  (malloc/free, uniscript_error, uniscript_warnings) and release v1.0.8 ships uniscript_c.wasm + uniscript_c.h;
  `use uniscript` imports the prebuilt module (1.45–2.9 s → 0.27–0.57 s per run).
  Default: warp's uniscript tests follow the released uniscript (`<:red 𓀀>` keeps red as color meta with a warning;
  any uniscript.org/vN header is read without a warning, backwards compatible).
- chars-codepoints (2026-10-14, the user's draft `['a','b','c']`): `"abc".chars` is a list of characters (codepoints),
  printing `['a' 'b' 'c']`; a literal `['a' 'b']` keeps its characters; `'a' == "a"` stays yes. A one-character text is a character, so
  `["a" "b"]` prints `['a' 'b']` and `["a" "bc"]` prints `['a' "bc"]`.
  Amended 2026-10-16 (user, card consists-characters): char is a shortcut for character, and a character is a
  grapheme (a letter with its marks), not a code point; codepoint is the separate unit. "שָלוֹם" has 4 characters
  (`s.chars`, `number of characters in s`, `for each char in s`) and 6 code points (`s.codepoints`,
  `for each codepoint in s`).
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
  mechanism (`move x from: a to: b` per labels-calls, card prepositions-call); sound uses calls until then. Version literals stay but
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
  order, `[1 2 3]+4`, `1+2 squared`) stay loud errors naming the explicit forms (D3, D9/P5); bare `a=1 2 3` is a
  list since reconsider-spaced (2026-10-15).
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
- floor-division (2026-10-16, user; card floor-division; supersedes the 2026-10-08 rule "`//` is floor division
  unless a space or line end follows"): `//` is always a comment. Floor division is `/!` (`7 /! 2` = 3, `x /!= 2`).
  Where the compiler can tell a `//` was meant as floor division (`n = 7//2`, `x//=3`), a strong warning names
  `/!`, and `warp fix` rewrites it. `div` goes to `/!` too (`7 div 2` warns, fix `7 /! 2`); `div` stays the html
  tag. All our code, samples, docs and tests move to `/!`, except one or two tests that check the hints. A comment
  hiding a closing bracket is a parse error naming the `//`.
- `#` glued to a non-space is the count expression (`#s`, `#f(x)`); `# text`, `#!`, `##` and the directives `#use`,
  `#include`, `#import` stay comments/directives.
- hash-comments (2026-10-15, user): `# text` is the preferred comment; `//` clashes with floor division and reads
  Java-esque. `warp format` has an opt-in option rewriting `// text` comments to `# text` (`/* */` untouched), and the linter gives a soft hint on every `//` comment.
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
  i64-wraps (2026-10-16, user; card fast-int) replaces the trap for 64 bits: `i64` and `int64` are the same machine
  type and wrap silently at 2^63 (`a: int64 = 2^63 - 1; a + 1` → -9223372036854775808), like `fast int`; only
  `use strict` catches the overflow. The smaller sized types (int8…int32, byte) were not asked about and keep
  trapping for now.
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
- `x /= y` is exactly `x = x / y` (3/2 stays exact); floor division is `x /!= y` (floor-division); an `x:int` keeps a whole number.
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

- dimensions-units (2026-10-15, user): dimension names are types (`3 cm is length`, `h: length = 3 s` error); the
  time dimension is `duration` (a subtype of time). Units and their connections live in lib/units.warp
  (`dimension length`, `m: length`, `cm := m/100`, `inch := 2.54 cm`). A plain `:=` stays a constant
  (`distance := 42 km` → 42km). `px` is the
  device pixel, its own dimension, converting to lengths through display.dpi at run time. A unit after a list applies
  to each element (`[3, 4] cm` = `[3 cm, 4 cm]`, `v: length[3]`, matrices alike). Predefined derived names: speed,
  area, volume, frequency.
- unit-definitions (2026-10-15 evening, Interviewer's proposal, kept by the user 2026-10-16):
  every dimension has an SI anchor unit, every other unit is a marked conversion to a known unit. Built on the user's
  sketch: `dimension length meter` declares a base dimension and its anchor; `unit m := 1 meter` (aliases use the
  same rule); `unit cm := meter / 100`, `unit furlong := 201.168 m`: the right side is an amount in a known unit,
  which fixes the dimension and the factor. Derived dimensions follow from the expression, no second path:
  `unit N := kg*m/s²`, `unit L := dm³`. Offsets: `unit °F := (it + 459.67) * 5/9 K`, `it` the amount in °F, the
  result in K; an optional reverse line only where the way back can't be derived (decibels). SI prefixes (km, mN)
  come automatically for SI-anchored and SI-derived units only, not for furlong. Unit symbols bind only after a
  number (`3 m`), never as bare names, so `m := 5` or a parameter `s` don't collide; a variable gets a unit by
  `x * 1 m` (no `x m`). lib/units.warp uses the same lines. Rejected: `length cm := …` (a type word before a name
  already declares a variable's type) and `unit meter := 100 cm` (the anchor would hang on a smaller unit).
  Second opinion: the supervisor (refinements above are its).
- pixel-lengths (2026-10-16, user via the supervisor; replaces the Interviewer's night decision dimension-bridges):
  no bridge or setting machinery in the compiler. px stays its own quantity (`unit dpi := px / inch`); a length
  becomes px by plain arithmetic, `3 cm * display.dpi`, and back by dividing. `length as pixels := …` and
  `display.dpi := 96 dpi` are gone from lib/units.warp; `3 cm as px` is a plain DimensionError. No SI prefixes on px.
- display-names (2026-10-16, user via the supervisor): display.density is pixels per length in dpi (aliases
  display.dpi, display.ppi; `unit ppi := dpi`, `unit dot := px`); display.size is width × height in px (alias
  display.dimensions, with a hint that "dimension" is ambiguous); display.diagonal in inch/cm (density derivable from
  it when the host reports none); display.resolution is an ambiguous alias with the hint "resolution: density (dpi)
  or size (px)?". Library lines only, through the common alias machinery. Default (use-reflection, undoable):
  display.resolution reads display.size, the hint adding "size is width × height", since a screen's "resolution"
  usually names its pixel size.
- unit-target (2026-10-15 night, user, told to use-reflection): a unit line may name its target the way a function
  names its return type: `unit °C as K := it + 273.15`, `unit °F as K := (it + 459.67) * 5/9`; the right side is
  then a plain amount, `it` the amount in the new unit. `(it + 273.15) K` stays accepted.
- general-conversions (2026-10-15 night, user, told to use-reflection; card general-conversions): one conversion
  mechanism for any types, `A as B := body`, e.g. `Person as text := it.name`, then `p as text`. A program's
  conversions are looked up before the built-in ones. Unit lines are its unit cases. Today
  `P as text := it.name` is silently ignored (the card fixes that).
- units-library (2026-10-16, user; card discuss-moving): partial move into warp. Units, dimensions and prefixes
  are lines in lib/units.warp, run-time quantities its Quantity class; next, conversions become warp
  `A as B := body` definitions. The compile-time dimension check (static_units.rs, P64) stays Rust until types can
  be parameterized by a dimension. Not Julia's unit values (`3km = 3*km`): they would bring back clashes with
  variables named m or s.
- litre-suffix (2026-10-16, user; card list-literal): a glued unit beats a number suffix, so `2L` and `2l` are
  2 litres (`2L in cm³` → 2000cm³). The old exact suffix L goes: exactness is `2 as exact`, with a hint where the
  suffix was meant. lib/units.warp gains `unit l := L` and `unit ha := hm²`.
- unit-after-number (2026-10-16, user; card variable-named-like): after a number a unit symbol is always the unit,
  even with a variable of that name in scope: `m = 5; 3m` is 3m (meters), with a hint "a variable m exists; write
  3*m to multiply"; `m * 3` is 15. Supersedes the agent-pinned test_variable_shadows_the_unit (`3m` → 15).
- unit-types (2026-10-16, user, via the supervisor): a unit can be used as a shallow number type:
  `display { density: dpi, size: px, diagonal: inch }`. The unit fixes the field's kind of quantity, not the exact
  unit: `118 px/cm` is accepted and converted, `3 kg` is a type error. The written unit is the default and display
  unit. A line like `dimension speed m/s` only names a kind and isn't needed for typing.
- unit-field-plain-number (2026-10-16, user, via the supervisor): a plain number assigned to a unit-typed field
  (`display.density = 96`) gives a fixable warning to use the right unit; the fix adds it (`96 dpi`).
- quantity-naming (2026-10-16, user: "Yes, perfect"): "dimension" is ambiguous (an image of 28 × 28 px is a tensor
  with 2 dimensions, and its width and height both are dimensions and have dimensions), so warp says quantity (ISO
  80000 kind of quantity) for length, speed, density (`quantity length meter`); axis for width/height; rank for the
  number of axes; shape for 28 × 28 px. `dimension` stays only as an alias with a hint asking which is meant.
  Older entries above saying "dimension" mean quantity.
- unit-prefix-lines (2026-10-16, use-reflection default; undoable): lib/units.warp spells SI prefixes `prefix k :=
  10^3` and marks the dimensions taking them `SI dimension length meter`. Conversion-only long names
  (`name hour := h`) stay for now, until durations and units are one (below).
- duration-units (2026-10-16 night, Interviewer's proposal, kept by the user 2026-10-16; card
  time-module): the time module's durations and the unit h become one thing, as dimensions-units already makes
  duration the time dimension: `unit hour := 1 h`, `2 hour + 1 h` is 3 hours (today a DimensionError), printing
  keeps the long names ("3 hours"). Amended 2026-10-16 (user, "leave as is"): the `name` form stays (`name hour := h`,
  `2 h in minutes` → 120min); step 1, where `1 h` beside a duration is a duration, is enough.

## Text
- letters (2026-10-15 evening, Interviewer's proposal, kept by the user 2026-10-16; card of-in-chain,
  low priority): `letters of "a1 b!"` → ['a' 'b'], alphabetic only, the common meaning (Unicode letter, like
  Python's isalpha and Java's isLetter); `characters` is an alias of `chars`.
- parse-json (2026-10-15, user): quoted code's `.json` is the readable operator-as-key shape
  (`parse("a * (b + 1)").json` → {"*":["a",{"+":["b",1]}]}); what it can't express is a warning or error, never `{}`.
- used-module-bare-words (2026-10-16, warp-shaders default, undoable; card json-parse-literal): after `use json`, a
  bare `parse(t)` is json's parse (a map), not warp's code parse. General rule: a used module's own qualified words
  (WARP_FORMS, today only json.parse) may also be called bare; a program's own definition wins, and without the `use`
  the word keeps its warp meaning. The alternative was to keep the code parse and warn about the ambiguity.
- json-exactness (2026-10-15 night, Interviewer's proposal, kept by the user 2026-10-16):
  following the user's "a warning, or even an error": big ints are written with all their digits as a JSON number
  (`(2^70).json` → 1180591620717411303424; JSON numbers have no size limit, only some readers do); a fraction is
  written as JSON's float with a warning naming the loss (`(1/3).json` → 0.3333333333333333, "JSON has no exact
  1/3"), an error under `use strict`; functions, ∞, NaN and complex are errors. Not text-encoded numbers ("1/3"):
  a reader would get a string where it expects a number.
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
- range-in-brackets (2026-10-16, user; reverses the Interviewer's night decision to spread): no spreading, no
  special rule: a range inside [ ] is one item, "pure logical": `[1..3, 5]` is a list whose first item is the range
  (1, 2), `[1..3]` a list holding one range. An unneeded edge case (card literal-range obsolete).

## Functions and calls
- random-forms (2026-10-15 night, Interviewer's proposal, kept by the user 2026-10-16; card
  random-should): bare `random` is its call where a value is used (a float in [0,1)). `random T` for a type is a
  random value of T, one general rule instead of special cases: `random bool` a coin flip, `random int` the full
  integer range (ids, hashes), `random codepoint`, `random float` = `random`. `random(n)` stays 0…n-1;
  `random(a, b)` with whole numbers is a whole number from a to b inclusive, the die roll `random(1, 6)`; with a
  float argument, a float in [a, b). Refined with the user's suggestion via the supervisor: random's parameter is
  declared of type Type, so `random int`, `random(int)` and `random float` take the type itself, not its
  constructor; general rule: any parameter of type Type accepts bare type words. An overload takes a range,
  and picks one of its elements: `random 1..10` → 1…9 (`..` excludes the end), `random 1...10` and
  `random 1 to 10` → 1…10, the same die as `random(1, 10)` (corrected by warp-types: `1..3` is [1 2]). The result type follows the argument
  (`int z = random 1..10`).
- function-references (2026-10-15, user): a bare name of a function that needs arguments, compared with a type
  (`floor is function`) or in `type(floor)`, means the function itself (card bare-fn-type). `.floor` in argument
  position is a lambda on it, like Swift's .member: `xs.map(.floor)`, `people.sort(.age)` (card dot-lambda).
  `&floor` and `function floor` stay the explicit references; `:floor` is not used.
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
- P52 a phrase-defined function is called with its own prepositions (`add 1 to: 2`, see labels-calls); elsewhere `to`
  is a range, `of` a field lookup. `f - x` with a user function is `f(-x)`.
- labels-calls (2026-10-15, user; revises P52 and prepositions-call): a preposition labelling an argument in a call
  always takes a colon: `save file in: folder`, `play C4 for: 1s`, `move x from: 1 to: 5`, `save sound as: "x.wav"`.
  Universal for every preposition and `as`, with/from/into/by included (they get a special meaning soon). Definitions
  keep the bare form, `play(tone, for duration)`; bare preposition labels in calls are dropped. Card labels-calls.
  Revised (user, same day): where the bare word cannot mean anything else, `play C4 for 1s`, `melody xs each 1 beat`,
  it is read as the label with a warning offering the colon (preposition_calls.rs UNAMBIGUOUS_LABELS: for, each).
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

- print-bang (2026-10-15, user): a function that gives nothing (print, show, log …) never takes D2's in-place bang:
  `print friends!` prints the forced friends and friends stays; `sort xs!` still sorts in place.

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

- reconsider-spaced (2026-10-15, user; replaces D12's error): spaced values are lists again: `xs = 1 2 3` is
  [1 2 3], `2⁻¹ 2⁰ 2¹ 2²` → 0.5 1 2 4, `print 1+1 2+2` → 2 4 (numbers, texts and number expressions, after `=` and
  as a line of their own). Only a glued sign after a space (`x = 5 -2`) still asks; list-sign stays. Calls (`f 1 2 3`),
  units (`3 cm`) and `3x` are unchanged.
- swizzles (2026-10-15, user; card compile-shaders): `.x .y .z .w` and their combinations (`.xy`, `.xyy`, `.zyx`)
  read a list of 2–4 numbers, everywhere, CPU and GPU alike. Vector math uses the dot operators (D3), the Euclidean
  length is `norm(v)` (alias `magnitude`), `length` stays the count.

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
- property-accessors (2026-10-16, user, changed from the Interviewer's night proposal `width: get{…} set{…}`; cards
  property-get, graphics-properties), refined the same morning ("polymorphism, no special syntax needed"):
  accessors are plain overloads by arity. The getter takes no argument, `canvas.width := canvas_width`; the setter
  is the same name with one parameter, `canvas.width(new_width) := canvas_width = new_width`, or with `it` as the
  implicit parameter, `canvas.width := canvas_width = it` (the same thing). `canvas.width = 5` calls the setter.
  A field with a getter and no setter is read-only: `display.width = 5` is a loud error. lib/draw.warp replaces
  the Rust table this way.
- setter-calls (2026-10-16, user): a setter overload is called four ways: `canvas.width = 12` (one general rule:
  assigning to a property with a one-parameter overload calls it; the form docs show), `canvas.width(12)`,
  `width(canvas, 12)` (falls out of uniform calls; allowed, not taught, reads awkwardly), and the English
  `set width of canvas to 12`. Two independent rules compose, no special `set … of … to` logic (user):
  1. `width of canvas = 12` is `canvas.width = 12`: an `x of y` read is also an assignment target, for ordinary
     fields and setters alike.
  2. `set x to y` is `x = y` for any assignment target. It works but gets a style hint: warp doesn't do prose,
     write `x = y`.
  So `set width of canvas to 12` = `(width of canvas) = 12` = `canvas.width = 12` (card english-setter).
- saved-words (2026-10-16 night, Interviewer's proposal, kept by the user 2026-10-16; card
  graphics-saved-words): no save-only table and no symbol overload. The general bare-call rule (random-forms: a
  function word used as a value is its call) covers it: lib/draw.warp declares `canvas()` with no arguments as the
  current Picture beside `canvas(width, height)`, so `save canvas as: "painting.png"` passes a Picture; the sound
  module likewise (`sound()` the current Recording).
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
- fail-effect (2026-10-16 night, Interviewer's proposal, kept by the user 2026-10-16; card fail-effect):
  the `Fail` effect tracks explicit raises (raise/throw/fail/panic/error "…"), division by a value that may be 0
  (`f(x) := 10/x` is Fail, `x/2` stays Pure) and library calls that answer with an Error. Index out of range,
  missing fields and failed casts are program bugs, like Rust panics, not tracked; otherwise almost every function
  would say Fail.
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
- deleted-loop (2026-10-15, user): a live_loop deleted from the source stops when the source is re-evaluated (at
  most a ~4 s fade); the sound matches the visible code, unlike Sonic Pi.

## Modules, library and foreign code
- operator-alias (2026-10-16, user; card standalone-math-units): math spellings of operators are alias lines in
  lib/math.warp, keeping the operator's precedence: `operator × := *`, `operator ≤ := <=`, `prefix operator √ :=
  sqrt`; they replace the Rust table.
- foreign-signature (2026-10-16, Interviewer's proposal, kept by the user 2026-10-16): a libm function is declared in warp by a typed foreign line, `sin(x: float): float from 'm'`,
  `atan2(y: float, x: float): float from 'm'`: the signature is visible and needs no C header. `import (…) from 'm'`
  keeps working for headers. Builtin math words use the same line with warp as the library (use-reflection
  default): `floor(x: number): int from warp`, `min(a: number, b: number, more...): number from warp`; the Rust
  signature table goes. A bare `floor` is warp's; C's float floor only qualified (`m.floor`).
  keeps working for headers. Open (card foreign-line): a program's own line still takes the header's signature;
  it should register the written one, and a disagreeing header be an error.
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

- package-forms (2026-10-15 default): packages.warp entries take short forms: a bare `uniscript` =
  github.com/pannous/uniscript, `name: owner/repo`, `name: "github:owner/repo"`; `version: latest` or none follows the
  default branch; a pin is `{repository: pannous/uniscript, version: 1.0.7}`.

## CLI, building and tests
- gate-cuts (2026-10-16, user via the Integrator; notes/test_speed.md): each sample runs once in the gate:
  test_playground_samples checks the playground menu (no error at all), test_samples_run_cleanly only the samples
  outside it (no compiler error). The self-hosting twins check every 4th source in the gate and all sources in the
  nightly run (tests/nightly.sh, cron 03:10 on origin/main; a failure files a card).
- cli-words (2026-10-15, user): warp's own command options are words, never `--flags` ("we don't like this minus
  minus style, ever"), e.g. `warp format check …`, `warp format hash-comments …`; the style guide says so. Card
  cli-words migrates the existing --fuel/--exe/--strict/… (older `--x` mentions below are superseded by it).
- fast-math (2026-10-16, warp-types default; card switch-rebinds; undoable): the polynomial math module is
  `use fast_math` (`fast` is already an alias of float); the CLI switch is the word `warp fast app.warp`.
- unicode-symbols (2026-10-16, user; card user-listed): warp parses a large range of Unicode symbol characters
  (arrows, math symbols) even when nothing defines them yet; they are free operator names, and an undefined use is a
  later "operator ⇔ is not defined" error, not "Unexpected character". Arrow spellings of existing operators are alias
  lines in lib/math.warp with a "prefer ->" hint (→ ⟶ ➜ … for ->, ⇒ ⟹ for =>, ← ⟵ ⬅ for <-, ⬆ ⇧ for ↑, ⬇ ⇩ for ↓).
  ⇔ ⟺ ↔ ⇑ ⇓ get no built-in meaning for now; an example defines one, e.g.
  `infix operator ⇔(a: bool, b: bool) := a == b`.
- default-methods (2026-10-16, user; cards library-words, method-call-bare-words): `x.word` and `x.word()` are
  identical, both resolved by multi-dispatch on the receiver's type. A library word belongs to its type: `first` is
  a list word, so an object or class with its own member `first` (`p = {first: 1}; p.first` → 1) or a class method
  `first` is the more specific match and wins. Redefining a library word for its own type (an extension changing
  `first` for lists) is not allowed, a loud error. No extra override keyword is needed for that.
- dispatch-untyped-library-words (2026-10-16, user; card one-dispatch): an untyped program function named like a
  library word (`first(x) := x * 2`) covers every value the library word doesn't: the library's `first(list)` is
  more specific, so `first(3)` → 6 but `first([1,2])` → 1, with a hint that first of a list stays the library word.
- round-halves (2026-10-16, user; card round-half-up): `round` rounds halves away from zero (`round(2.5)` → 3,
  `round(-2.5)` → -3), like school rounding, C and Rust; banker's rounding is `round_even` (2). Where the compiler
  sees an exact half (a literal or folded constant), a hint says Python would round to even. Unicode spellings
  (Interviewer's default, from the user's idea): `⌊x⌋` floor, `⌈x⌉` ceil, `⌊x⌉` round (the usual nearest-integer
  bracket). `x as int` truncates toward zero (`2.7` → 2, `-2.7` → -2), so it rounds down only for positive numbers.
- class-members-free (2026-10-16, user; card one-dispatch; limits default-methods and library-words-typed):
  width, height and every other built-in or library word are not keywords. A class field or method gets its own
  dispatch signature, with that class as the receiver, so it never conflicts with them: `class Box{width:int,
  height:int}`, `class P{count:int}` and a method `first()` in a class are all fine, and a local variable may shadow
  one (`width = 5`). The "immutable" and "loud error" rules only forbid redefining a library word for the library's
  own types (list, text, number…), never members of user classes.
- library-words-typed (2026-10-16, user; cards library-typed, one-dispatch): harden the library. Every library word
  gets a full type signature (`square(x: number): number`, `count(xs: list): int`), and typed library words are
  immutable for their types until further notice. A program's untyped definition covers only other values, with a
  hint: `count(x) := 7; count([1,2])` → 2, `square(x) := 777; square(4)` → 16, `floor(x) := x+10; floor(2.5)` → 2.
  This extends dispatch-untyped-library-words from collection words to all library words.
- type-words-reserved (2026-10-16, user; card one-dispatch): every type word is reserved as a function name,
  `double(x) := …` and `hex(x) := …` alike; the error offers "name it to_double".
- field-forms (2026-10-16, user; card one-dispatch): plain fields keep only `.` and `of` (read, and set with
  `b.w = 5` / `w of b = 5`); `w(b)`, `b.w(5)` and `w(b, 5)` stay errors for a plain field. Only properties (accessor
  definitions) answer every call form.
- is-union (2026-10-16, implied by union-return; card int-text): after `is`, `int or text` is a type, so
  `v is int or text` means `v is (int or text)`.
- fuel-metering (2026-10-16, user; card fuel-metering): runaway loops are stopped by the epoch timer with a time
  limit by default (about 1–5% overhead, stops at a slightly different step each run). Fuel, the exact step budget
  that cost 4.4× on calls (40M calls: 218 ms against 49 ms), runs only on request: `use fuel`, the CLI word
  `warp fuel N app.warp`, and tests that pin a step count.
  Numbers (warp-types default, Interviewer's amendment, undoable): `warp app.warp` gets 60 s of CPU time (waits for
  sleep or input don't count), set with WARP_TIME_LIMIT=<seconds>, and WARP_FUEL selects fuel. Compile-time folding
  of constant calls gets 2 s. `warp build` executables get no limit by default, since a built app runs as long as it
  likes. Windowed and event-loop programs aren't killed after 60 s in total. A stop after the limit says loudly what
  happened and how to raise it.
- ready-queue (2026-10-16, user via the boss and the supervisor): the board's Ready column is the merge queue. A
  worker runs its targeted tests, pushes the finished branch, makes sure it merges cleanly into origin/main, and
  hands it off with `todo ready <card> [branch]` (tip read from git); only the re-run after a clean update is dropped. `todo ready` refuses
  a branch only when it doesn't merge cleanly into current origin/main (`git merge-tree --write-tree origin/main
  <tip>`); being behind is fine, and `--force` covers carrier branches. Reason: a rebase goes stale as main moves,
  workers can't see each other's unmerged branches, and the gate re-runs the suite anyway. The Integrator merges only
  Ready cards and resolves only trivial conflicts itself (formatting, mod/use lists, adjacent additions). Logic changed
  on both sides goes back to the owner with `todo move <card> Now` plus a `todo note` holding the conflicting hunk.
  Unmerged branches without a card are only reported (`todo ready --stray`). Refinements (user, same day):
  - a batch holds at most about 8 Ready cards; wide refactors and carrier branches land alone and first;
  - the orphan watcher reports bounced cards whose agent is gone, and the Supervisor reassigns them;
  - the reviewer takes its work from Ready, and a veto moves the card back to Now.
- text-unit-plurals (2026-10-16, user; card text-unit): every plural unit word of a text is the list:
  `"👍🏽".chars` → ['👍🏽'], `.codepoints` → ['👍' '🏽'], `.graphemes`, `.bytes` → [240 159 …], like words and lines.
  Counting is `count t.codepoints` or `number of bytes in t`. Code reading `t.codepoints` as a number migrates to
  `count` (a `warp fix` rewrite where detectable).
- implicit-words-reserved (2026-10-16, user; card it-reserved): `it`, `self` and `this` are reserved. Binding one
  (`self = 3`, `it += 1`, `for this in xs`, destructuring) warns "`self` is reserved: it names the receiver of a
  method; pick another name", an error under use strict. Writing it as the explicit parameter (`f(it) := …`,
  `Point.norm(self) := …`) stays allowed.
- test-consolidation (2026-10-16, user; card probes-check, worker warp-test-consolidator): a worker turns probes
  into tests where they check decided behaviour with a stable expected value, judging case by case (not every probe
  was meant to be a test: exploratory, visual, sound, timing and undecided ones stay probes). It also dedupes and
  condenses tests, which widens the earlier "only probes condense": no distinct assertion is lost or weakened, and the
  count drops only by duplicates named in the commit. A probe is deleted only once every case is asserted.
- fast-scope (2026-10-16, user; card fast-scope): `fast` before a function or module makes everything machine
  inside it: polynomial sin/cos/exp/pow, `/` gives f64 (`fast f(x) := x/3; f(1)` → 0.3333333333333333), and plain
  ints are i64 that wrap at 2^63 (`fast g(n) := n * n; g(10^10)` wraps). `x: fast int` is i64, `x: fast float` f64;
  exact/precise is the opposite word if fast ever becomes the default.
- unix-words (2026-10-15, user): `use script` has shell words with warp results: ls pwd cd cat echo mkdir cp mv touch
  head tail wc grep which rm, plus find, size/info (du/stat), sort/uniq, open (native), curl/wget as aliases of
  fetch/download. `export a=b` clashes with the WebAssembly export, so it's left out. Not now: basename/dirname,
  chmod, ln, diff, kill/ps, tar/zip. rm: one file or an empty folder, no patterns, never / ~ or the working dir and
  above, sandbox permission, and it moves to the Trash (undoable); no recursive form. `wc` → {lines, words, bytes};
  ls hides dotfiles (list_files shows all); grep is regex over lines; unquoted paths `~/x ./x /etc`, `~` expanded on
  use. Card unix-words.
- cross-module-overload (2026-10-15, user): overloading works across modules: a module's definition doesn't shadow a
  same-named word elsewhere, the call dispatches on the argument type. `use script; sort ./names.txt` sorts the
  file's lines while `sort [3 1 2]` still sorts the list (same for unique, find, size). sort_lines/find_files/
  file_size are only interim names.
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

- uniscript-browser-wasm (2026-10-15, user): the uniscript repo commits uniscript_c.wasm/.h at each release, so
  browsers read it like other package files; warp's builds don't download release assets.
- firefox-bug (2026-10-15, user): yes, file the Firefox JSPI Worker-terminate bug at Mozilla Bugzilla
  (repro probes/firefox_jspi_terminate.html).
- deploy-hook (2026-10-15, user): watch-playground-deploy.sh asks the finished run's conclusion and failed jobs,
  retrying ~30 min; no answer gives one stderr line, no wake (card posttooluse-playground).
- browser-server (2026-10-15, user): browser checks get a dry run on pannous.com over SSH (headless Chrome and
  Firefox on the server) before anyone starts the GitHub Playground workflow, so trial runs send no "Run failed"
  mails; GitHub runs only the final check, plus Safari. Local browsers on the Mac stay banned (card browser-server).

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

- shared-buses (2026-10-15, user): a bus is a named effect chain, `room = bus() |> reverb(0.4) |> delay(0.25)`;
  a signal pipes into it (`osc(C4, 1s) |> room`), a note names it (`play(C4, 1s, bus: room)`); `room.reverb = 0.6`
  while it plays applies from that moment. No `send room` setting.
- sound-dsp (2026-10-15, user): the effects DSP is written once in Rust (src/sound/effects.rs, compiled into warp.wasm
  for the browser), lib/sound.warp calls it; a warp twin can follow with self-hosting.
- save-sound (2026-10-15, user, replaces the render_sound alias answer): writing audio to a file is the general
  `save` with an explicit object, overloaded by its type: `save sound as: "song.wav"` / `save(sound, "song.wav")`,
  likewise `save buffer as: "x.raw"` (colon per labels-calls) and other savable values. render_sound goes; its name gives an I-meant hint
  naming save.

## Old id index (superseded or historic; git has the text)
P35 → index-position. P42 → P227. P56(2-4) unknown entity → P56 revised. P63 → wiki/charged.md. P66 → footgun-divzero.
P103, P104 → no-executable-by-default + build-should. P110 → P163. P118, P214 floats → P240. P168 → field-or-minus. P170 → type-library-words. P200 → P200b.
D1 → P6 + brace-hole. D7/#33 → wiki/charged.md. D8 → approx-zero. D9, D12, D13 asks → got-it rule + list-sign. D14 →
in-bool. exact-default → exact-decimals. #40 → list-sign. P85, P87, P93, P95, P96, P98, P99, P101, P137, P144, P181,
P53, P59, P79, P80, P9, P10, P16, P19, #14b–d, #15–20, the Original questions 1–41: test edits and housekeeping done.
- fix-forward (2026-10-16, user): merges don't wait on a few failing tests. The Integrator may ignore up to 10
  failing tests per batch as `#[ignore = "fixnow: <card>: <reason>"]`, push, and card each for the breaking
  branch's owner, who fixes forward and drops the ignore. Backtracking only above 10 or when forward fixing isn't
  possible. A fixnow test pinning an old edge case may be reconsidered through the Interviewer instead of
  restored. Rules: notes/roles.md "Fix forward".
