# Decided questions

One line per live decision, grouped by topic, keyed by its id (P71, D5, #10 or a slug such as gpu-f32) so cited ids can
be looked up. The code, tests and wiki are the truth; this file is for lookups and for checking before asking the user,
not for reading front to back. Superseded entries are deleted (git keeps them; the old-id index at the end names the
replacement). Open and parked questions: notes/open_decisions.md. New answers go into their topic section as one line.
Numbering: P = interview questions, D = wiki-survey questions, #n = the first question lists.

## Process
- reviewer-role: warp-reviewer reviews each merge request's diff in parallel with the gate, polishes small things in a follow-up commit, vetoes unsound changes before the push, sends structural findings to the Refactorer as cards.
- ready-queue: the board's Ready column is the merge queue (`todo ready <card>`, refuses only a branch that doesn't merge cleanly into origin/main); batches of ≤ ~8 cards, wide refactors and carrier branches alone and first; the Integrator resolves only trivial conflicts, others go back to the owner (card to Now + note with the hunk); a reviewer veto moves the card back to Now.
- urgent-direct: warp-urgent pushes and merges each of its cards to main itself, in any area, without the merge queue; the Integrator's next full run is the check (notes/roles.md "Urgent").
- fix-forward: the Integrator may ignore up to 10 failing tests per batch as `#[ignore = "fixnow: <card>: <reason>"]`, push, and card each for the owner (notes/roles.md "Fix forward").
- crash-fixes-first: crash fixes merge with the highest priority. small-changes-on-main: small or non-conflicting changes may go on main without a worktree. session-names: a session is named for its current task.
- self-hosting: parser first, each warp part a twin of its Rust part with a differential test over the samples' kept forms (exclusion list, self-hosting-minimal); gaps are fixed in the language.
  self-hosting-resumed: self-hosting is the fleet's next big task; workers
  finish their current work, then switch to it, starting with a readiness survey (warp-self-survey: are all the
  language parts there, or should macros or a cheaper feature come first so the warp compiler doesn't bloat).
- self-hosting-back-end (user 2026-10-18 night): the fleet's only focus is the analyzer, the emitter and the runtime in warp (lib/compiler), wired into the WARP_SELF_HOSTED pipeline; other cards wait in Soon, except important bug fixes (crashes, regressions on main) and the final Rust cleanup. Owners: notes/self_hosting.md "Back-end focus".
- whole-pipeline-in-warp (user 2026-10-19 morning; takes effect only once every sprint and current worktree branch is integrated, the supervisor then messages the user on iMessage): the goal becomes the whole pipeline in warp (parser, passes, analyzer, emitter, runtime, no Rust fallback), not raising the self-hosted suite count above 70%; only tests that exercise the warp pipeline count for now.
- passes-first-principles / ignore-rust (user 2026-10-19, with whole-pipeline-in-warp: "ignore rust"): the warp compiler is derived from first principles, reaching the same goals (the programs' results) by very different means; the Rust compiler's passes and internal shapes are ignored, not ported, triaged or twinned. No new twins; a warp stage is judged by the suite's program results, not by matching a Rust tree, kind table or layout. The approved target shape (notes/self_hosting.md) is a starting point, not a mapping of Rust passes. Where a warp stage still hands its result to a Rust stage, only that hand-over must fit, and that ends once the pipeline is warp end to end. Effective at once for the analyzer: analyzer.warp feeds emitter.warp directly (card emitter-takes), no Rust kind comparison; crutches recent commits added only to match Rust may be undone.
- sprint-merge-as-is (user 2026-10-19 ~11:30): the remaining sprint branches are merged as they are, errors fixed later. Must still build (native + wasm32, clippy); each test a branch breaks gets #[ignore = "card sprint-fixes: …"] instead of a bounce, collected on card sprint-fixes; only a branch breaking widely (the prelude or unrelated areas) loses the offending hunk.
- warp-is-tests (user 2026-10-19): the Rust tests' `is!("code", expected)` assertions also live as warp files, written `is! "3+3" 6`, where `is!` is a small warp function over eval: `def is!(warp: text, expected: any) := eval(warp) == expected` (failures report the code, got, expected). Needs names ending in `!` to be definable words (today `is!` reads as `is` + `!`). Run with `warp test` through any pipeline switch (WARP_SELF_HOSTED=pure): the fast lane for the self-hosted stages. Card warp-is-tests. Test pairs as data (user 2026-10-19 evening): the test files drop the repeated `is!` and hold code/expected pairs, either as a map `test_pairs = { "1+1" 2 \n "√9" 3 }` or as the bare pairs with no wrapping, the file loaded as data. Any runner can use them (`warp test`, `is!`, a pipeline switch). Matching is permissive: a result counts if warp's `==` says yes (1.0 ≈ 1 ≈ yes) or both sides are empty/falsy (0 ≈ ø ≈ [] ≈ no). Warp's own `==` stays as it is: today 0 == ø is no. The `is! "3+3" 6` line form stays too: it is helpful for a test whose pair would exceed the 80-character line limit, Two kinds of files, not mixed: a pairs file of pure data (the short tests), and ordinary warp test files beside it with the `is!` lines and any other tests.
- wasm-instruction-body (user 2026-10-19): a function or operator whose body is one wasm instruction says so in warp source: `prefix operator √(a: float): float := wasm f64.sqrt`, `floor(a: float): float := wasm f64.floor`, `min(a: float, b: float): float := wasm f64.min`. The arguments are evaluated onto the wasm stack in parameter order, the instruction consumes them and leaves the result; the parameter names and types only give the signature (and pick the overload). Overloads by type: other types keep a warp body. Replaces emitter.warp's hard-coded intrinsics table and FFI calls (libm `from 'm'`) wherever wasm has the instruction. Multi dispatch (user 2026-10-19 afternoon, replacing a brief "hard-code them" scale-back): √ is declared per type, `prefix operator √(a: float): float := wasm f64.sqrt` and `prefix operator √(a: exact): exact := a^(1/2)`, and the argument's type picks the body. So √ of a float is the f64 root (`√2.0` → 1.414…), and √ of an exact number stays exact (`√16` → 4, `√2` → √2, `√-4` → 2ⅈ). Other little builtins and library words may stay hard-coded until they need such overloads.
- no-primitive-rewrites (user 2026-10-19, at the first pure run): never compromise on the syntax to make something pass. No program, test, sample or lib/compiler code is rewritten into more primitive syntax so a warp stage (pure lane, emitter core, analyzer) can handle it; the stage grows to the language, or refuses loudly and the form gets a card.
- core-pipeline-first (user 2026-10-19): the core warp pipeline (parser, passes, analyzer, emitter, the runtime words ordinary programs need, running the module) comes first; library features such as time, dates, durations, calendars, units, sound wait in Later, including the sprint branches' follow-ups (wiring, Rust removal, their ignored tests). The planned "finishing" phase after the sprint integration is cut to what the core needs.
- forward-crunch (user 2026-10-19, standing rule for every branch, extends sprint-merge-as-is): move fast, merge unless really broken. Really broken = doesn't build (native, wasm32, clippy) or breaks tests outside the branch's own area (then only the offending hunk goes, not the branch). Failures inside its own area: #[ignore = "card <owner's card>: …"] and merged, no bounce; the owner fixes forward. Cap: an owner with more than ~20 ignored tests fixes them before taking new work.
- call-takes-right (user 2026-10-19, standing rule restated; answers rule-implicit gh #2069): a function consumes everything to its right if it can: `square 3 + 1` → square(4) = 16, `2 * square 3` → 18, `print 1 + 2` prints 3. No spacing- or type-based tightening; parentheses split it: `square(3) + 1`.
- operators-are-functions (user 2026-10-19, higher priority than √ multi dispatch): an infix, prefix or suffix operator is a normal function with a flag for its fixity. It shares everything else with functions: arguments, signatures, types, overloads by parameter type, `:= wasm` bodies, names in the wasm name section. No separate operator table or declaration path. The one difference besides the flag is spacing: a function call needs spaces between the name and its arguments (`square 3`), while an operator doesn't (`√2`, `a+b`, `3!`).
- restructuring-regressions (user 2026-10-19, with forward-crunch): a regression that comes with a clean restructuring is fine. Fix it the clean way on the new structure, or #[ignore] it onto the owner's card; never bring back the old shape or add a special case just to make a test green again.
- outcome-tests-only (user 2026-10-19): tests check only outcomes: given warp code, the value, printed output, error or effect it produces. Tests that pin intermediates (tree shapes, pass output, twins comparing with Rust, kind tables, wasm bytes or layouts, internal Rust APIs) are removed or rewritten as outcome tests; this overrides "never remove tests" for those. New tests are outcome tests. Card outcome-tests, worker warp-test-cleanup.
- compiler-typed (user 2026-10-18): every function of the warp-written compiler and runtime declares its parameter and result types; generic helpers are typed generically (T, element of xs), not left untyped. Untyped functions stay a language feature for scripts.
- compiler-node-enum (user 2026-10-18): the warp compiler uses the built-in node, no parallel class and no conversion: one warp declaration (prelude or lib/compiler/node.warp) IS the built-in node's definition (emitter.warp derives the $Node struct from it; until Rust goes a twin test checks src/node/mod.rs and type_manager.rs against it), with the cases of src/node/mod.rs (True, False, Empty, Number, Char, Text, Symbol, Error, Key, List, Data, Meta, Type; Op, Bracket, Separator as enums), so `Node` works as a type and in `match n { Key(left, op, right): … }` (cards node-enum, node-rewrite). Declared in lib/node.warp, read by the compiler (cases by name → Kind, fields by position → node parts). Op/Bracket/Separator cases print their name (enum-case: `print Op.Add` → Add); the spelling is `Op.Add as text` → "+", `"+" as Op` → Add; op_of/bracket_of/separator_of return the case once node-rewrite moves lib/compiler over (Interviewer's decision).
- compiler-high-level (user 2026-10-18): the warp compiler is written at the highest level the language has (classes, enums, match, for, comprehensions); a missing or weak feature is fixed in the language, not worked around with low-level code. Only runtime.warp's intrinsics stay instruction-level.
- self-hosting-minimal: the warp compiler is no blind copy of the Rust one; it drops experimental
  bloat and inelegant passes, cleanest minimal implementation first. Failing fringe/edge forms are fine if each dropped
  form is recorded with how to bring it back (notes/self_hosting_dropped.md); twins measure the kept forms via an
  exclusion list. Sizeable syntax drops go to the user as questions.
- self-parser-drops: lib/compiler/parser.warp leaves out foreign syntax (Rust macros, Python
  generators/yield from, Ruby blocks), music `go {…}`, phrase definitions (`to get price of item := …`), CSS in style;
  silently also WGSL shader holes and error nodes (`$a` pattern names stay: pattern-roadmap). Code is data: music and CSS need no parser syntax and
  must still work as library data DSLs (`tune d:data := play d` → `tune { c4 e4 g4 }`, note names are sound-library
  constants; `style {…}` reads its block as data).
- sugar-twins-drops: lib/compiler/passes.warp leaves out Rust Option forms (`if let Some`, `match o {Some/None}`,
  `unwrap_or`, `expect`, `is_some`; warp has `o != ø`, `o ?? d`, `o ?? raise(m)`) and the `set x to y` pass;
  setter-calls stands, `set x of y to z` comes back as a library code template.
- sugar-triage: web + markup, media + host, help + reflection passes stay out of the warp compiler and return as library
  code. `of`/`in` are right-associative infix operators (`length of characters in text` = `length of (characters in
  text)`), so the of extras are no special passes; forms that aren't a right-hand infix application are dropped or carded.
  `in`/`from` are a lookup only after a selector word (chars, words, lines, keys, …; one table), membership otherwise
  (in-bool); `print words in t` walks and `byte count of t` are dropped (Interviewer's decision).
- rust-cleanup-drops (user): the Rust compiler drops them too (Option forms, `set x to y` pass, scoped_handlers, print
  walk, `byte count of t`); same language on both compilers, their tests retired as decision edits.
- rust-texts (Interviewer's decision): `t += x` builds a rope flattened once when read; test_gc_struct's
  read_string_field reads the `$Text` byte array.
- weekly-cleanup: a fresh cleanup roadmap per week (topic `cleanup-w<week>`); regular chores use `todo repeat <card> <days>`.
- topic-priority: the column is the priority, for topic cards too (new topics Soon); Roadmaps holds only unprioritized topics; the user's own cards (drafts, issues without "Added by <session> with `todo add`.") stay where the user put them.
- cleanup-means-fewer-lines: the cleanup's metric is lines removed; each cleanup merge request states its net src/ delta. long-functions: splitting an overly long function by concept is a card of its own even when it adds lines.
- cleanup-may-drop-edge-cases / cleanup-bold: a simplification may break edge-case tests, marked `#[ignore = "cleanup-edge: <card>: <what>"]` (assertions untouched) and listed; clearly superfluous code goes without a question; samples and the tour must keep running.
- edge-cases-general: special-cased border cases belong in the general machinery or go.
- restructuring: every function lives in the file of its concept; moves are pure moves; true duplicates merge (notes/restructuring.md).
- consistency (card #376): an agent regularly checks the app against the docs; clear mismatches fixed, ambiguous ones via the Interviewer.
- Questions to the user are multiple-choice popups. Every major feature ships with a sample in samples/. Un-ignoring a test that passes unedited needs no question; editing its assertions does.
- test-consolidation: probes that check decided behaviour with a stable value become tests, case by case; tests are deduped without losing a distinct assertion; a probe goes only once every case is asserted; probes the user touched are never deleted, their wish lines become cards.
- gate-cuts: each sample runs once in the gate (playground menu samples in test_playground_samples, the rest in test_samples_run_cleanly); self-hosting twins check every 4th source in the gate, all in the nightly run (tests/nightly.sh).
- cargo-fmt: the gate enforces `cargo fmt --check` (`use_small_heuristics = "Max"`). emit-rows: emitter instruction lists one logical step per row like WAT, kept by `#[rustfmt::skip]`. rust-style-lint: of the Rust lint rules only doc-moved is enforced.
- git-hook restore: agents may restore only Cargo.toml/Cargo.lock (the build tweak); the hook backs up the diff first.
- #1 avoid panics, errors as values. #14 not-implemented errors unless easy.
- P151 the crates.io package is `warp-lang`, the runtime `warp-runtime`. P150 license: the compiler AGPL-3.0-or-later, warp-runtime MIT (programs built with warp stay their authors'), contributor agreement in CONTRIBUTING.md so warp can be dual-licensed commercially. release-channels: next a PyPI wheel wrapping the binary, named warp-language (`pip install warp-language`; NVIDIA's Warp owns warp-lang there), Linux .deb/.rpm (warp-lang), AUR (warp-lang-bin).
- package-reexport (card module-own): a package's own `use` lines reach the programs that use it.
  `use webgl` (which says `use draw`) alone gives draw's words too: `canvas(200, 100)`, `star(…)`, `save "x.png"`.
- laws-file: the human-owned laws live in samples/laws/project_laws.warp
  (samples/laws/laws.warp already exists), keeping the "AI agents must not edit this file" header; test_bend_laws runs
  `warp verify samples/laws/project_laws.warp`.
- Language name: warp everywhere; `.wasp` files still read; URLs/domains and wisp unchanged.
- P180 split-debuginfo "unpacked" plus the hourly sweep. P90 ./test.wasm is written at every compile. P91 one shared analysis per compile, refreshed only when a pass changed the program.
- Repo layout: tests/ in topic folders (P14); CLAUDE.md is a symlink to AGENTS.md (P15); wisp format stays with a roundtrip test (P11); GcObject is the GC reading API (P12) with gc_struct!/wasm_struct!/wasm_object! as sugar (P97); test macros in tests/common (P100).
- P8 vendoring deactivated, kept for a later offline agent. P7 the cdylib is built only by web/playground/build.sh. warp-ast.wit lives at samples/wit/warp-ast.wit.
- Bee/wasp entity names go into uniscript and are regenerated into src/text/uniscript_entities.tsv, never by hand.
- P13 libm FFI: header-driven linking first, the LIBM tables only as fallback and as examples of hand-linked FFI.
- uniscript-binary-import: `use uniscript` imports the released prebuilt uniscript_c.wasm (+ .h), committed in the uniscript repo at each release (uniscript-browser-wasm); warp's tests follow the released uniscript.
- browser-server: browser checks dry-run on pannous.com over SSH first; GitHub runs only the final check plus Safari; no local browsers on the Mac. firefox-bug: the Firefox JSPI Worker-terminate bug is filed at Bugzilla (probes/firefox_jspi_terminate.html).
- deploy-hook: watch-playground-deploy.sh reports the finished run's conclusion and failed jobs, one stderr line on no answer.
- Playground CI runs Chrome and Firefox in parallel; workers need not tour with --firefox.

## Tooling: format, beautify, CLI
- warp-format: `warp format` (synonym `warp lint`, no `fmt`) only fixes layout in one canonical form, never meaning: tabs, `name: value`, spaced binary operators (glued only where spacing changes meaning; checked by comparing parse trees), list separators as written, `{` on its header's line, one space after `,`, none inside brackets, ≤ 1 blank line. Abbreviated words get a soft hint ("warp avoids abbreviations"), never a fix.
- beautify (also beautify-preview): opinionated rewrites are `warp beautify`: prints the diff, asks "apply? y/n" in a terminal (piped lines answer it, one per file; end of input is no), `beautify yes` applies, `beautify check` previews; rules: hash-comments, alias words to their warp word via the I-meant fixes.
- cli-words: warp's command options are words, never `--flags` (`warp format check …`); card cli-words migrates the old ones.
- P102 `warp build hello.warp` makes the native executable; build-should: it writes x.wasm beside it, `--wasm` only the module. build-links: `warp build page.warp` opens the page in the system web view.
- no-executable-by-default: plain `warp file.warp` writes no executable (WARP_EXECUTABLE=1). P105 `warp run` never writes an executable nor takes the machine-code path.
- P77 an executable prints the output, then its value; exit 0, 1 on a trap; only an Int result is the exit status. P113 a ø result shows nothing. P236 `warp run` echoes the final value after the prints.
- P222 `warp serve [file] [port]` serves a page program; plain `warp app.warp` serves when the program declares a route/server; CGI mode retired.
- test-soft: `test C` records and continues, `check C` asserts, `test "name" {…}` is a test block. P209 test blocks run only under `warp test`. P210 all pass → one summary line; failures exit nonzero.
- fuel-metering: runaway loops are stopped by the epoch timer: 60 s CPU time by default (WARP_TIME_LIMIT), not for `warp build` executables or windowed/event-loop programs; the stop says how to raise it. Fuel (exact step budget, 4.4× on calls) only on request: `use fuel`, `warp fuel N app.warp`, WARP_FUEL, and compile-time folding of constant calls.
- fast-math: the polynomial math module is `use fast_math`; the CLI word is `warp fast app.warp`.
- P107 the playground offers only the .wasm download plus a `warp hello.warp` line. P108 Warp.sublime-syntax takes .wasp/.warp/.a/.angle. P229 double-clicking a .warp file opens the editor.

## Welcoming, diagnostics and newcomer forms
- Newcomer syntax is eaten, its intent compiled and the warp form hinted, only where it does not clash with a known footgun.
- Asks become got-it warnings; error-fallback cases (`[x]*n`, insert order, `[1 2 3]+4`, `1+2 squared`) stay loud errors naming the explicit forms (D3, D9/P5).
- Got-it scope: `[y = this one, a = all of this kind, n]`; an expression is remembered by its text; a `// got it` comment silences the line.
- warning-levels / hints-are-warnings: one Diagnostic with a level hint < weak < normal < severe; `--no-hints` and `use strict` are thresholds (notes/welcoming.md).
- canvas-zero: a missing setting takes a sensible default; errors only for an ambiguity too dangerous to guess.
- P75 fix buttons read "I meant: <replacement>", the meaning as a tooltip.
- julia-compat (user, 2026-10-17): stay close to Julia; its syntax is beautiful and its semantics often near perfect (unlike Rust). Julia forms read with Julia's meaning; deliberate departures listed in wiki/Julia.md.
  - julia-floor-divide: `÷` is floor division as in Julia (synonym of `/!`, `7 ÷ 2` → 3), with a hint for newcomers expecting `/`; replaces #27's normalization of `÷` to `/`.
  - julia-pair-range: `1:4` stays a pair; iterated (for, sum, comprehensions) a pair of two numbers is the range.
  - julia-text-times-hint: `"a" * "b"` concatenates with a got-it warning naming `+`; an error only when a text holds a number (`"3" * "4"`).
  - julia-dollar-interpolation: `"x is $x"` only as a toggled feature after the self-hosting migration (`$million`, `$0` clash).
- P78 modifiers from other languages (`public`, `static`, `inline`, …) are skipped with a got-it note; words with a warp meaning keep it.
- Word choices are not asked: the recommended word is canonical, synonyms are aliases with a note.
- rust-forms-welcome: warp may read Rust's forms (`<T>` included) as long as each has a nicer warp form, which stays canonical.
- foreign-spellings: foreign names are cheap aliases of existing words, not lowering code. Kept: Rust `<T>`/`let mut`, Enum.map/List.map/Seq.sum, Python lambda, variadic lambdas, JS destructured parameters, Ruby lambda/proc/blocks/to_s, C# LINQ, C++ `auto x`. Dropped: OCaml let-in and `fun`, Elixir `def …, do:`, C++ `[](…){…}`. R/PHP forms live in `use R` / `use PHP`. Kotlin `when x { 1 -> … }` = `match` with a warning; `match` takes `->` arms and `else`; `case` is optional before any arm.
- P157 no generic syntax: ported `fn id<T>(x: T)` compiles untyped with a note. P161 no Swift argument labels; ported labels compile with a note.
- P167 Go's `Point{1, 2}` builds `Point(1, 2)` for a known class with a note; unknown names stay tagged data.
- P145 R's `x <- 3` assigns with a note; `x<-3` is an error naming both readings; docs never use `<-`.
- P241 no vector type: `Vec<int>`, `vector<int>`, `Array<int>` read as `list of int` with the note "warp says list".
- Error highlighting: CLI carets under the word; web red/amber wavy underlines with the message on hover.
- #7/P92 unresolved calls are errors: `foo()` is "undefined function: foo"; a bare word statement naming nothing is "undefined variable: foo"; a bare `foo` in data stays a symbol.
- constant-words (leave as is): null nul none nil nill nix nada nothing empty void true yes false no are constants in any case; `empty = 3` is `ø = 3`.
- function-words-consume (user): every function word, not only print, takes its arguments without parentheses, by its parameters (arity and types), no special rules: `show a, b` = show(a, b), `add 1 2` = add(1, 2), `print ord c` = print(ord(c)), `[twice 1, twice 2]` = [twice(1), twice(2)], `count xs > 3` = count(xs) > 3; a function word where the parameter takes a function is a value (`map square xs` = map(square, xs)); non-function words are unchanged. An argument reaches right as long as the expression fits the parameter type (`sqrt 4 + 5` = sqrt(9); `xs > 3` is a bool, so count stops). Varargs/defaults take what fits to the statement end, overloads the most arguments that type-check, a function value of unknown arity one atom per blank (Interviewer's decision).
- P86 adding values of unknown kinds builds `a + b` lazily; evaluated operands need Addable, else a type error.

## Comments, separators and lexing
- floor-division: `//` is always a comment; floor division is `/!` (`7 /! 2` = 3, `x /!= 2`); where `//` was clearly meant as division (`n = 7//2`), a strong warning names `/!` and `warp fix` rewrites it; `div` warns with the fix `/!` (`div` stays the html tag). A comment hiding a closing bracket is a parse error naming the `//`.
- hash-comments: `# text` is the preferred comment; `warp format hash-comments` rewrites `//` comments; the linter hints on `//`. `#` glued to a non-space is the count expression (`#s`); `#!`, `##`, `#use`/`#include`/`#import` stay comments/directives.
- `print a    print b` on one line is an error asking for `;`.
- P227 a comma next to `==` without parentheses is an error asking for them.
- P68 a braceless call whose argument holds an operator (`square 3 + square(4)`) warns naming both groupings.
- P143 `f x == y` is an error naming both groupings only when both type-check; P149 an untyped parameter's type is inferred from the body first. Statement words (print, return) take the whole expression.
- call-as-grouping: `f x as T` is `f(x as T)`, with a warning naming `f(x as T)` and `(f x) as T`.
- P189 extra control-flow words stay display-only keywords; ambiguous names need quoting.
- P165 soft keywords: hard keywords are control flow, declarations, literal values and module words (`fun` is hard, P165b); every other keyword is usable as a local name, parameter, field or method with a note. P172 alias words (`fire` for emit) yield to a program's own definition.
- unicode-symbols: warp parses Unicode symbol characters (arrows, math) as free operator names; an undefined use is "operator ⇔ is not defined". Arrow spellings of existing operators are alias lines in lib/math.warp with a "prefer ->" hint.
- operator-alias: math spellings of operators are alias lines in lib/math.warp keeping the precedence (`operator × := *`, `prefix operator √ := sqrt`).
- implicit-words-reserved: `it`, `self`, `this` are reserved: binding one warns (error under use strict); as the explicit parameter (`f(it) := …`) it is allowed.
- data-reserved: `data` is a keyword (the `data …` quoting prefix); a variable named data is an error. test_data_quoting::a_variable_named_data_is_no_prefix changes accordingly.

## Numbers
- exact-decimals: decimal literals are exact (`0.1 + 0.2 == 0.3`), division gives exact ratios; floats come only from float sources (transcendentals, sqrt of a non-square, host values, `as float`, float-typed places, e-notation) and are contagious; `==` warns only with a float involved.
- int-list: a whole decimal literal is int (`type([0.0, 0.0])` is list of int, P196b); a variable's type joins every write. #21 `x=π; type(x)` is real.
- number-converts: `number` keeps an exact value; only `float` converts.
- float-text: floats print shortest round-trip digits (Python/JS repr) everywhere; P225 also joined to text.
- repeating-decimals: only the overline `0.3̅` is the exact 1/3; `x as decimal` writes the period overlined; never for floats.
- D16 Int is unbounded; `byte`, `int8..int32` are declared FFI types that trap on overflow; `u8` = `uint8` = `byte`. i64-wraps: `i64`/`int64` wrap silently at 2^63 like `fast int`; `use strict` catches it.
- fast-scope: `fast` before a function or module makes it machine arithmetic: polynomial sin/cos/exp/pow, `/` gives f64, ints are wrapping i64; `x: fast int` is i64.
- hex-printing: `255 as hex` is an int shown as 0xFF; arithmetic gives a plain int. binary-int: `5 as binary` shows 0b101.
- footgun-divzero: exact numbers raise divide by zero (catchable); computed IEEE floats give ∞/NaN.
- gpu-f32 / P240: heavy float maps (from 10× the measured break-even) run on the GPU in f32 with a one-time notice; `@cpu` and `WARP_GPU=off` keep f64; `@gpu @cpu` together is an error; `@gpu` warns when the GPU doesn't apply; int lists fitting i32 may use the GPU with an overflow flag. go-heavy-map: a `go` map of a heavy kernel runs as the plain map.
- P119 NaN canonicalization only where a float's bits are observable.
- approx-zero: `≈` is relative 1e-9 with absolute floor 1e-12; `tolerance = …` overrides. P211 `~` is looser: 1% for numbers, texts ignoring whitespace and punctuation; `≈` on texts ignores case and accents. P212 a class may define `approximately` and/or `similar`.
- ± intervals: P217 `5 ± 1` is a worst-case interval (Gaussian later as `5 ± 1σ`). P218 `x ≈ r ± 1` holds when |x−r| ≤ 1. P219 prints 2 significant digits of the ± part. P220 `1950 ± 50 AD` is the same ± value. P224/P224b comparisons answer only when certain: known ± values need `certainly`/`possibly`/`.value` (compile error), run-time ones count an overlap as not certain with a one-time warning. P233 a field typed with a tolerance gives each value that tolerance.
- `x /= y` is exactly `x = x / y`; an `x:int` keeps a whole number.
- Trailing percent: `10%` is 1/10, `50 % of 200` is 100; infix `%` stays remainder; CSS values stay as written.
- P184 English number words are numbers when no variable has that name, with a hint toward digits.
- #11 exact reals may go into a declared float with precision loss. rational-float: `x: rational = sqrt(2)` is a compile error, `x: real = sqrt(2)` works.
- lossy-return (also float-result): a fraction reaching a declared int result is an error ("declare number, or truncate with as int"). P49/P49b a float, even a written `2.0`, to an int parameter is a compile error. P131 an int variable or field refuses a fraction loudly. P127 int class fields keep exact ints.
- round-halves: `round` rounds halves away from zero; banker's rounding is `round_even`; an exact half seen at compile time gets a hint about Python. `⌊x⌋` floor, `⌈x⌉` ceil, `⌊x⌉` round. `x as int` truncates toward zero.
- nearest-limit: `nearest(x, rational)` limits the denominator to 1,000,000; `limit = 0` gives the exact fraction of the float. float-as-rational: `x as rational` of a float is the exact fraction of its shortest printed digits.
- P226/P226b log glyphs: `b⌞x` and `x⌟b` are log base b; postfix `x⌟` is ln. easy-add: `x↑n` power and `x↓b` log are typed infix operators for numbers, other types define their own.
- P57 hyperreals: `1/(1+ε)` stays an error; dual numbers later. P56(2-4) `∞` is f64 infinity, ω the hyperreal infinite.
- real-runtime: real.rs (compile-time symbolic reals) stays until symbolic reals are a run-time number type; the general run-time exact mechanism is parked (notes/real_runtime.md).

## Units and quantities
- #5 units yes; #3 juxtaposition `3x` multiplies; units are written glued (`3km`), `3 km` works with a hint. P64 units are checked at compile time and kept as metadata where static knowledge ends. P36 `100 cm in m` / `as m` convert within a quantity. Unit sums use the finer unit. Quantities print without a space (`500m`); P231 print rounds quantities, str() keeps the exact value. #31 `min` stays the function.
- quantity-naming: warp says quantity (length, speed, density), axis for width/height, rank for the number of axes, shape for 28 × 28 px; `dimension` is only an alias with a hint.
- dimensions-units: quantity names are types (`3 cm is length`; `h: length = 3 s` errs); the time quantity is `duration`; units live in lib/units.warp; a unit after a list applies to each element (`[3, 4] cm`). Predefined derived names: speed, area, volume, frequency.
- unit-definitions: `quantity length meter` declares a base and its SI anchor; `unit cm := meter / 100`, `unit N := kg*m/s²` (derived quantities follow from the expression); offsets `unit °F := (it + 459.67) * 5/9 K`; SI prefixes only for SI-anchored/derived units. unit-target: `unit °C as K := it + 273.15`. unit-prefix-lines: `prefix k := 10^3`, `SI dimension length meter`.
- Unit symbols bind only after a number, never as bare names; a variable gets a unit by `x * 1 m`.
- unit-after-number (also variable-named-like): `m = 5; 3m` is 3 meters with a hint "write 3*m to multiply".
- litre-suffix: a glued unit beats a number suffix (`2L` is 2 litres); exactness is `2 as exact`. Ambiguous glued units (L, l, a unit named like a variable) need `use units`, else an error with a fix adding it; unambiguous units work without.
- pixel-lengths: px is its own quantity (`unit dpi := px / inch`); a length becomes px by `3 cm * display.dpi`; `3 cm as px` is a plain DimensionError.
- display-names: display.density (aliases dpi, ppi) in dpi; display.size width × height in px (alias dimensions, with a hint); display.diagonal in inch; display.resolution is an ambiguous alias reading display.size, with a hint.
- unit-types: a unit can type a field (`display { density: dpi }`): it fixes the kind of quantity, converts other units, refuses other quantities. unit-field-plain-number: a plain number there warns with a fix adding the unit.
- general-conversions: one conversion mechanism `A as B := body` (`Person as text := it.name`); a program's conversions come before built-in ones; unit lines are its unit cases.
- units-library: units, quantities and prefixes are lib/units.warp lines, run-time quantities its Quantity class; the compile-time check (static_units.rs, P64) stays Rust until types take a quantity parameter.
- duration-units: `1 h` beside a duration is a duration; `name hour := h` stays (`2 h in minutes` → 120min).
- canvas-size: `window`, `display` and `canvas` `.width/.height` are writable properties natively and in the playground; a bare top-level `width` works with a did-you-mean.

## Text
- Text + anything concatenates in its str() form; a function stays an error; `"3"+3` warns and gives "33" (error under `use strict`); a Printable instance joins as its own text.
- P1 text * number repeats (`"ab"*2`); a digit text warns; text*float and text*text are type errors; `n times "ab"` is the explicit repeat.
- brace-hole: `\(…)` canonical interpolation; `\{…}`, `${…}`, `$(…)` work; bare `"$x"` is literal (P6); single quotes stay literal; `$` holes in sql/sh templates keep working.
- P126 texts inside containers print double-quoted; a top-level `print "a"` writes a.
- #35 lists join to "[1 2]"; `"x" as float` is a loud error. P74 `codepoint(c)` (aliases ord, ordinal, P27) gives an Int; P25 `int('5')` is 5; `int(c)` of a non-digit is invalid_number; P65 arithmetic on a character is not_a_number.
- #26 upper/lower are Unicode, sort takes any comparable values, reverse works on text; `names.upper` = `upper names`.
- An untyped parameter given a one-character text takes it as Text. P31 the Printable operation is `text(p:person)`, the one allowed type-word function name. P32 `print xs` prints its str(). P41 `print "x changed to " value` joins without a separator after a text literal; commas join with a space.
- result-keyword: after a print, `result` is the printed value.
- chars-codepoints: `"abc".chars` is a list of characters printing `['a' 'b' 'c']`; a one-character text is a character; `'a' == "a"`. A character is a grapheme; code points are `.codepoints` ("שָלוֹם": 4 characters, 6 code points). text-unit-plurals: every plural unit word is the list (`.chars`, `.codepoints`, `.graphemes`, `.bytes`); counting is `count t.codepoints`; `.bytes` is the byte list of any value, size `x.bytes.count` / `byte_size(x)`.
- letters: `letters of "a1 b!"` → ['a' 'b'] (Unicode letters); `characters` is an alias of `chars`.
- guillemets: «…» quotes its content as unevaluated warp data (`«x + 1»` the expression as data).
- Emoji: a single emoji is a codepoint; multi-codepoint emoji are texts, not names.
- parse-json: quoted code's `.json` is the operator-as-key shape (`{"*":["a",{"+":["b",1]}]}`); what it can't express warns or errs, never `{}`. json-exactness: big ints all digits; a fraction as a JSON float with a warning (error under strict); functions, ∞, NaN, complex are errors.
- used-module-bare-words: a used module's qualified words (WARP_FORMS, today json.parse) may be called bare after its `use`; a program's own definition wins.
- uniscript: entities are `\:name` in code and texts; an unknown entity stays as written with a warning; bare `\alpha` is an error. P198 HTML names for letters, LaTeX for operators.

## Bool and truthiness
- bool-type: yes/no are their own shallow type acting as 1/0; `type(yes)` is bool, prints yes/no, true/false accepted; `0 == false` yes, `0 === false` no; P195 `true + 1` is 2; P199 bool slots accept 1 and 0 only; JSON true/false read as yes/no.
- Truthiness: ø + 1 stays an error; empty values are falsy; only 0 equals false; #38 a non-empty list (even `[ø]`) is truthy; #28 `x=ø; x.size` and `[].count` are 0; `is empty` covers "", [], {}, ø.
- text-bool: `as bool` / `bool(x)` read the word ("0", "no", "false" → no); `if` keeps plain truthiness.
- in-bool: `x in list` is yes/no (also `xs.has(x)`); the position comes from `xs.index_of(x)`.

## Types and annotations
- P203 annotations make code strict; unannotated code is lax (checked at run time); `--strict` warns on lax spots. P204 an `any` value entering an annotated place gets a run-time check. P201 `s: Shape = Circle(…); s.r` is a compile error with hints `match Circle(r)` / `s as Circle`.
- Duck typing: a truly unknown type is duck typed; a known different type with the right fields is an error teaching `image like photo` (P22); a written map is accepted structurally; `p.word` on an untyped parameter is a run-time "no field word".
- #30 only `is` tests types (`3 == int` educates toward `is`). type-equal: `is` is the subtype test, `==` between types exact; a bare type name is the type as a value. is-union: `v is int or text` means `v is (int or text)`.
- type-text (also type-print): a type value is `class int` in every text form; `type(int)` is class type. time-word: every type value prints `class X`, which reads back as a reference.
- unknown-type: an unknown declared type is an error with the near type as fix.
- union-or: in types `or` = `|` (`x: int or text`); in values `or` is boolean.
- return-as (also union-return): return types are `def f(x) as int or text`; `->` works with a hint; no `returns`.
- error-type / error-optional: a function that can fail has type `T or error`, never Result; `T?` is T, ø or an error. Writing an Error into a field or list element raises; `x is ø` missing only, `x failed` failed only, `if no x` / `x or 0` both; at a WIT export: `result`, `option` or `result<option<T>>`; an Error propagates where a caller can take it.
- empty-field: ø needs `?` everywhere (`class Tree{left: Tree}` errs, fix `left: Tree?`).
- P185 `none(xs, f)`, the null spelling with arguments, is an error naming the null.
- P125 optional parameters: `x=ø`, `maybe x`, `x?` alike; typed `x:int?`; `a ?? b` only for ø.
- P179 sum types auto-unwrap like optionals; payload fields by position `rgb(1,2,3)#1`; a payload-free variant is its symbol and `red is Color`. P178 enum cases with values are sealed classes.
- P45 a variable given kinds that don't mix (`x=[1]; x=5`) is a compile error; Int→Float widening stays; undeclared lists stay `list any`.
- P28 `real x;` read before assignment is its type's zero value.
- P23/P20/type-words-reserved: every type word is reserved as a function name (`double(x) := …` errs, "name it to_double"), except P31's `text(p)`. type-word-local-warns: binding a variable to a type word (`symbol = ":"`) warns, since it shadows the type in `is`/`as` (Interviewer's decision).
- #9 `list of int`; plural type words (`numbers`) are lists; `list<int>` reads too; `x : 100 int` and `pixel:int[100]` declare typed arrays.
- `type(ø)` prints "empty". static-int: a constant call misfitting its declared int result is refused at compile time.
- D5 any noun can name a type/parameter (notes/matching.md).
- index-position: `index` (from 0, `xs[i]`) and `position` (from 1, `xs#p`) are int types; index_of, argmax, argmin, insert use them by declared type; position ± int stays a position, position − position is a count, position + position an error; a missing element is an error, never -1 (notes/index_position.md).
- schema-as: `x as Config` converts field by field or errs naming the field; `x is Config` stays strict.
- D10/P17 dispatch on return type (`render "hello" as pdf`); ambiguous → got-it warning (notes/dispatch.md).
- function-type: `type(gcd)` prints `function: (a, b) -> int`. parse-type / body-code: quoted program parts (`parse("1+1")`, `f.body`) have type `code`, print their source, keep `.op`.

## Variables, binding and charged definitions
- let-reassign: `let` is fully immutable (fix "use var x"); `var` is the variable; `let mut x` = `var x` with a loud warning. const-deep: `const` is deep like `let`.
- P159 `const x=7; x=7` warns "remove the redundant assignment". P130 pi and named constants are constants; a class field named pi is the class's own.
- let-parallel: the parallel let is only `a b = f(x) g(y)`; `let a b = …` is a function definition; `let a, b = 3, 4` unpacks.
- Data vs code (P51, P62, P63, D7): specified in wiki/charged.md. P70 late binding: a change is an error only where a later call sees it; pure getters over constants may fold.
- P71 `:=` without parameters is charged, re-evaluated at each use; assigning it after is an error (P138: also Go's `total := 0; total += n`).
- P124 a lambda or nested `def` changing an enclosing variable (`n += 1`) shares it; `nonlocal n` only makes a plain `n = 5` shared. P70c handlers and definitions created in a loop keep their iteration's variable.
- watched-cells: watched variables use the run-time cell, except main-level variables no function writes.
- P72 no memoization syntax; the compiler decides.
- P73 / D2 `!`: `x!` forces (a block runs, an optional unwraps, a value is itself); `!` after a method mutates in place (`xs.sort!`). print-bang: a function that gives nothing never takes the in-place bang (`print friends!` prints the forced friends).
- P69b a block arriving at run time is pure by default. P69c `warp compile` with run-time `!` warns and loads a shared compiler lazily.
- data-stays-data: a bare word in data stays a symbol; only `!` and `${…}` evaluate; `created: now` stays a block until `!`.
- runtime-dates / now-live: `now` reads the clock at run time at each use; tests pin `now <= now`, not `now == now`.
- time-word (also print-time, time-now, anchor-time): bare `time` is the type; instant, date, local time, zoned time, duration are its subtypes; `now.time` is the time of day, `now.date` the date; `now > 17h` / `now >= October` compare the matching part, `==` stays exact; `time > 18h` is an error with the fix `now > 18h`.
- module-global-assign: a top-level `tempo = 90` sets the standard module's global.
- Mutating a main-level list in a function needs `global names`. P106 tasks share a variable only when declared `shared`. P44 `shared xs = int[n]` arrays with atomic `xs#i += v` (`atomic` synonym).
- range-end-once: every `for` range end is evaluated once. #22 `N times {…}` evaluates N once; `i++ while c` is a plain while; `a = 2 if c` guards the whole assignment.
- #23 a loop's `it` shadows a function's `it`, with a warning. P174 `mk(k) := { it * k }`: `it` stays the parameter, with a warning. P239 in a lambda naming its parameter, `it` is the outer `it`.
- #6 data as scope, warning when kebab parts are also variables; P81 that warning offers three fixes.
- stored-visits: `stored visits default 0`; `stored visits = 0` works with a note.
- Ranges: `r: 1..n` is the range itself; `upto` excludes the end with a hint. range-in-brackets: a range inside [ ] is one item, never spread.

## Functions and calls
- reflection: function objects have .name, .signature, .return_type, .parameters, .doc; printing one shows its doc above its typed .dump; `methods(x)` returns the function objects; `use reflection` adds compilation unit, compiler, origin, wasm index, effects. A bare parameterless function inside print/help/dump/type is called, with a warning and hint `function now`.
- function-in-list: in a list or text a function prints as its name; only alone it shows doc + definition.
- dir-lists: built-ins are function values of type `builtin` (a subtype of function); methods(x) lists them.
- one-method-form: methods are functions: a class-body `def x()` is `def x(_:P)`, `p.x() == x(p)`; `methods(P)` = every function whose first parameter is P, an ancestor or a trait P implements; `dir(p)` = fields + methods(P); where a function is declared is only metadata (`.declared_in`).
- default-methods: `x.word` ≡ `x.word()`, resolved by multi-dispatch on the receiver's type; a member or method of the object's own class is more specific and wins.
- class-members-free: built-in and library words are not keywords; class fields and methods dispatch on their class and never conflict; locals may shadow them.
- receiver-field-assign: `fresh(f: F) := { body = 99 }` writes f.body, same as in a class-body method (one-method-form, P115/P116); `let body = …` makes a local; any warning applies equally in both places (Interviewer's decision).
- library-words-typed / dispatch-untyped-library-words: every library word has a full type signature and is immutable for its types; a program's untyped definition of the same name covers only other values, with a hint (`count(x) := 7; count([1,2])` → 2, `first(3)` → the program's).
- field-forms: plain fields take only `.` and `of` (read and set); call forms (`w(b)`, `b.w(5)`) are only for properties.
- cross-module-overload: definitions in different modules overload by argument type, no shadowing (`use script; sort ./names.txt` sorts lines, `sort [3 1 2]` the list).
- random-forms: bare `random` is a float in [0,1) where a value is used; `random T` is a random value of type T (`random bool`, `random int`); `random(n)` 0…n-1; `random(a, b)` whole numbers inclusive, floats [a, b); `random 1..10` picks from the range. Any parameter of type Type accepts bare type words.
- function-references: a bare function name compared with a type or in `type(f)` is the function itself; `.floor` in argument position is a lambda (`xs.map(.floor)`, `people.sort(.age)`); `&floor` and `function floor` are the explicit references.
- type-library-words: list/map/text library words are always available as `x.f()` and `f(x)`; `use list` only for qualified `list.zip`. P171 other modules' words need their `use` (error naming the module); `write` and `exists` are global.
- P141/P142 a definition named like a builtin that would win is an error at the definition; library words may be redefined only where the builtin wouldn't win, methods always; a foreign export is called qualified. P39 no `id`/`square` builtins; P191 `square` lives in math.
- builtins-name (user 2026-10-18): library words are called builtins (shorter to say and write), in notes, cards, messages and new code names; where the old split still matters, the Rust-only ones are "Rust builtins" until retired. Old names (library_words.rs, retire-library-words) are renamed along with the Rust cleanup.
- music-plain-library (user 2026-10-18): music (lib/sound.warp and the music words) is a plain library: no compiler magic at all, no Rust pass, no rewrite rules, no module pass. It uses only core language features. music_words.rs goes away: each form becomes an ordinary function/class/operator call; a form that can't be written that way either needs a general core feature (card, useful beyond music) or is dropped/simplified. Rephrasing the syntax of some calls is fine; most forms should be doable with the general patterns (definition clauses, function-word patterns, notes/matching.md).
- domains-plain-libraries (user 2026-10-18): the same holds for web routes, markup and styles, graphics/window properties, shader holes and reflection/help. Whatever isn't expressible in pure warp yet goes to the user as a question, since the language may be extended for it.
- markup-plain-library (user 2026-10-18): glued attributes are dropped (`label{ for: name; "Name" }`); `form` is a plain function `form(method: symbol, action: text, body)`, so `form post "/todos" {…}` comes from function-words-consume; core rule: `name = value` inside any data block is a key, as `name: value` (`input{type="text"}`).
- rewrite-general (user 2026-10-18): `rewrite PATTERN => TEMPLATE [where $c = SHAPE]` stays as a general user-macro feature (`rewrite double $x => $x * 2`); the arrow may later become `as`/`with`. Channels are a plain Channel class (send/receive/close); `for v in ch` uses an iteration protocol for classes (core feature).
- block-lazy-body (user 2026-10-18): a `{…}` block passed to a function-typed parameter is its lazy body, a function of no parameters or of `it` when called with one (`get "/echo" { it.body }`); general core rule.
- serve-plain (user 2026-10-18): path parameters via text patterns (`get "/users/" + id: int { users#id }`, card route-path-patterns; Rust keeps `:id` until then); `$field` dropped (`it.body.title`); default routes of `warp deploy` move into the library via a reflection word `functions()`.
- data-parameter (user 2026-10-18): a parameter typed `data` takes its argument as an unevaluated tree (`style(sheet: data) := css_of(sheet)`, `style{ ul > li { color: red } }`); any library can take a DSL block.
- transitions-data (user 2026-10-18): markup tags take their body as `data`, so `li{ transition: opacity 200ms ease-out }` and the library kinds `fade/scale/slide 200ms` stay bare words, joined into CSS by the library; transitions.rs retires.
- data-names (Interviewer's decision, as P92 and markup values): inside a data argument a name the program binds is its value, spliced in at the call site (`color: theme` → red, `p{ "count " + n }` shows n); an unbound word stays a word (`ul > li`); quote it (`"red"`) to keep a word that is also a variable. The made-up transition kinds lose their compile-time hint.
- reflection-plain (user 2026-10-18, hybrid as the Interviewer read "builtin meta() and typed extraction via lib"): one builtin `meta(x): map` gives what the compiler knows about a class, module or function; lib/reflection.warp builds typed views (`.name .fields .methods .parent`, `.words .doc`) and dir/fields/methods/help on it. `help(x)` gives the text; the REPL/program value prints it, mid-program `print help(x)`.
- markup-elements-functions (user 2026-10-18): HTML elements are library functions taking a data body (`ul(body: data)`), so `ul { … }` and `div{ p "hi" }` = p("hi") need no special rule (function-words-consume); plain words are untouched (`friends{peter "x"}`).
- self-hosting-focus (user 2026-10-18): "later, everything later": non-self-hosting design questions are parked with their leave-as-is defaults.
- letters-only (Interviewer's decision, from 2026-10-15): `letters` stays Unicode letters only; `.length` counts characters.
- same-as-result (user 2026-10-18): a result typed like an argument is spelled `same as <param>` (`shuffled(xs): same as xs`); an element of an argument is `element of xs`, `value of m`, `key of m` in type position (`first(xs): element of xs`; Interviewer's decision, no type variables per P157).
- music-plain-details (user 2026-10-18): live loops take a function (`live_loop(drums, 4)`), a block once block-into-go lands; in-program redefinition dropped, the playground handover between runs stays. Sharps: constants `F♯4`, alias `Fs4`, note texts "F#4", and `F#4` itself by overloading the index of the note object F (no pass). Block-wise sound functions: interim `play_blocks(hum, linear_address(hum_block), 0.1s)` with a linear array; goal: `play` picks the block filler by the function's parameter type (`hum(block: samples, t)`; Interviewer's decision).
- P82 a function reference is `function add` or `&add`; a bare name that needs arguments is "add needs 1 argument". P83 an alias needs the reference; an argument to a function parameter stays bare.
- arctan-allow: `arctan := arc_tangent` names the function; partial application only with a hole (`add(1, _)`).
- P84 several juxtaposed arguments to a one-parameter function are one list (`f 1 2 3` = `f [1 2 3]`). `sum := fold +`.
- P50 a declared scalar parameter broadcasts over a list. P173 an unannotated parameter called with several kinds becomes `any`.
- P37 named arguments set parameters and free variables; no implicit capture; a missing argument is an error. P216 named arguments run left to right.
- labels-calls: a preposition labelling an argument in a call takes a colon (`save file in: folder`, `move x from: 1 to: 5`), for every preposition and `as`; definitions keep the bare form `play(tone, for duration)`; where the bare word can mean nothing else (for, each) it is read as the label with a warning.
- P52 elsewhere `to` is a range, `of` a field lookup; `f - x` with a user function is `f(-x)`.
- P24 suffix forms (`4 doubled`) call user functions. P164 a type word after a parameter name is its type (`phone number`); `phone-number:text` is one hyphenated name in declarations.
- P132 `{f: x => x * 2}` holds a lambda; a type word after the colon makes it typed. P133 `|x| x*2` is a lambda.
- P48 custom operators: `prefix operator ⁻ := …`, `suffix`, `infix operator ⊕ := left+right`, word operators `infix operator divides(d, n) := …`. P54 only declared operators are ranked (`has precedence above +`). P166 element-wise operators on plain numbers act plainly.
- values-number: a system value is `$mouse.x`; `$mouse_x` hints `mouse.x`.
- D6 `|` is one operator dispatched on types: logical or, or pipe (`2|square|root`). operator-shall: `&`/`|` of two whole numbers combine bits; `and`/`or`/`&&`/`||` stay logical. amp-compose: `&` of two functions composes in reading order, `∘` alias. P158 `then` pipes only into a stage missing its argument with no `else`.
- `root` is an alias of sqrt. P2 tuple returns `return a, b` / `x, y = f()` as wasm multi-value. P3 closures keep boxing.
- runtime-function / print-closure: function values compare by definition and captures; a closure prints as written, `&f` prints the name.
- check-assert: `check` asserts; float parameters stay IEEE, `==` exact.
- D4 constructors: `T{…}` of a known type constructs and validates, `k:{…}` is plain data; missing fields are errors unless `?` or defaulted, undeclared fields errors. #41 spaced `V { x: 1 }` constructs a declared V.
- use-script: `use script` = `use relaxed` (no hints) plus the os, file and net words and `command("…")`, `arguments`, `cwd`, `glob`, `stdin`/`input`, `sleep`, `exit n`. unix-words: it adds shell words with warp results (ls pwd cd cat echo mkdir cp mv touch head tail wc grep which rm find size/info sort/uniq open, curl/wget = fetch/download); rm takes one file or an empty folder, never / ~ or the working dir and above, and moves to the Trash; unquoted paths `~/x ./x /etc`.
- sleep-seconds: a bare number in `sleep 0.3` means seconds, with a "needs a unit" warning (none under `use script`).
- cleanup-cuts: dropped compile-time lambdas, generators/yield, type-name parameters `fib int i`, scoped handlers `on … in {…}`, the defined-twice diagnostic, all phrase calls (until prepositions-call: `move x from: a to: b`). Kept: return-type overloads, tuple returns, list element checks, instance destructuring, object-shaped parameters, dispatch by class, version literals (rebuilt small). Traits are a thin alias of classes with default methods; ORM keeps only `stored people: [Person]`; `a of b` ≡ `b.a`.

## Lists, maps and indexing
- #4/P40 `size` = count; bytes via `byte_size` / `number of bytes`; `len(x)` counts with a hint `#x`.
- P43 lists do not grow past the end: setting past it is a catchable `index out of range`.
- P34 a number subscript on an empty `{}` keys it as a map. single-entry: `{a:1}` stays a pair and becomes a one-entry map where a map is needed.
- stored-table-name: `stored people: [Person]` names its table after the variable; `… in "people_v2"` explicitly.
- element-named-pair: a one-key pair named like an element (`{body: [...]}`) is markup; as data it takes a second key or a quoted key.
- #25/#36 a space before `[` never indexes; only glued `a[..]`.
- P29 `pair.0` counts from 0. P30 `xs.pop()` gives the last item, `m.remove(k)` the value or ø.
- Word slices: `xs from #2 to #4`, `xs starting from #2`, `xs up to #2` or ordinals, both ends inclusive; a bare number is an error. P58 `xs#a..b` keeps the range with a strong warning naming `xs#(a..b)` and `(xs#a)..b`.
- `[x]*n` / `[1 2]*2` are refused; `n times [x]` repeats. `xs.insert` never guesses: `insert(x, at: i)`, two Ints are an error. D3 `.+` is element-wise, `xs + [4]` concatenates.
- list-append-plus (user 2026-10-19, answers the 2026-10-03 `[1 2 3]+4` question): `xs + item` and `xs += item` append an item that is no list (`xs = [1]; xs += 3` → `[1 3]`, `[] += 3` → `[3]`); a list on the right still concatenates, and a one-item list (`xs += [3]`) gets a hint that the brackets are not needed.
- reconsider-spaced: spaced values are lists (`xs = 1 2 3`, `print 1+1 2+2` → 2 4); only a glued sign after a space (`x = 5 -2`) asks. list-sign: in a comma-less list a space plus a glued sign starts a new item (`[a -1]`); outside brackets `x -1` subtracts; never inside an index.
- P175 `xs where it > 1` filters; P223 filters may name fields bare (`people where age > 20`), a variable in scope wins with a warning. people-where: no new `with`/`without` sugar.
- P200b maps and lists share like instances (`n = m; n.a = 2` changes m); `x.copy()` copies; a typed list read from a field shares. P215 covariant list assignment is an error where the alias is visible, else a run-time check per write.
- P176 repeated `key: value` in a named tag block are children; a plain `{…}` keeps the duplicate-key error.
- `pixels size` works like `size of pixels`.
- swizzles: `.x .y .z .w` and combinations (`.xy`, `.zyx`) read a list of 2–4 numbers everywhere; Euclidean length is `norm(v)` (alias `magnitude`), `length` stays the count.
- curly-group: `{a; b}` is a list at every depth; `{a: b}` is a map literal at every depth, inside a parse() result too (`parse("f {a: b}")#2 is map`); walkers test groups with `n.bracket != ø`.
- empty-group-node (leave as is): an empty group in code is ø (`parse("{}") == ø`, `bracketed([], "{")` is ø); self-hosting twins compare with ø ≈ {} or the bridge maps ø back to the Rust bracket.
- node-part-clash: part readers `key_of(n)`, `value_of(n)`, `op_of(n)`, `name_of(n)`, `bracket_of(n)`, `separator_of(n)` always read the node part at any depth; `.key` etc. keep field → meta → reflection for data.

## Control flow, matching and errors
- match-whole-value: no binding syntax; inside an arm the subject's name and `value` are the whole matched value.
- match-alternatives: `1 | 2`, `1 or 2`, `1, 2` as case keys all mean either; `int or text` as a key is the type test; `other` is a catch-all like `_`/`default`/`else`.
- match-exhaustive: a match on a known enum or sealed class missing a case is a compile error.
- pattern-keyword: named patterns only with `pattern Adult = {age: >= 18}`; a plain `Name = …` is a value.
- #31 a switch without a match reports the run-time value (`no case for n = 4`).
- `if c {1} else 3+1`: the else takes the whole `3+1`.
- P21 `try X else Y`. P60 `try {…} catch {…}` and `try: … except:` are synonyms. P67 `catch e`: e is the Error value. #34 deep traps under `try` are caught. failed-raised: a raised error stops the program; bare `try X` turns it into the value (`r failed`). try-finally: `try X finally Z`.
- P55 a while loop's value is its last body value. P213 a loop ending in `print` gives ø.
- `when` by shape: `when click {…}` = `on click`, `when x > 3 {…}` = `whenever`, `when x {… -> …}` = `match`, `when {cond -> …}` = an if chain; each warns naming the word meant.
- P46 type-name and condition patterns in `for` filter with a got-it warning (`for friend in xs`, `for (it>2) in xs`).
- P61 `is` is always a comparison; with an undefined x it teaches `x be <value>`. #27 `be` is `:=`; `≤ ≥ ≠ × ÷ ¬ √` are accepted and normalized (÷ now floor division: julia-floor-divide).
- #39 `==` never chains: `a<b<c` chains, `1<2==2` is `(1<2)==2`.
- fail-effect: the `Fail` effect tracks explicit raises, division by a value that may be 0, and library calls answering with an Error; index out of range, missing fields and failed casts are untracked bugs.
- pattern-roadmap (user 2026-10-17, notes/matching.md ★): pattern-clauses: functions whose parameters are patterns are
  clauses tried in order (`simplify(data($a * 1)) := a; simplify(n) := n`); binding-alternatives: `|` branches bind when
  every branch binds the same names; whole-binding: `as` inside a pattern names the (sub)pattern
  (`data($f($inner) as call)`), outside it casts; text-patterns: `"0x" + rest`, at most one hole per side, plus
  prefix/suffix and character classes. A class is the type of a hole, like a typed hole that converts (`"0x" + h: hex`,
  `"v" + n: digits`, `c: digit`, `"/users/" + id: int`); class words digit(s), letter(s), hex, space, word are hole
  types only in a text pattern's hole position, nothing reserved (a variable `digits` stays plain; Interviewer's decision).

## Classes, objects and enums
- property-accessors: accessors are overloads by arity: getter `canvas.width := canvas_width`, setter `canvas.width(new_width) := …` or with `it`; a getter without setter is read-only (assigning is a loud error).
- setter-calls: a setter is called by `canvas.width = 12`, `canvas.width(12)`, `width(canvas, 12)`; `width of canvas = 12` is `canvas.width = 12`; `set x to y` is `x = y` with a style hint.
- saved-words: lib/draw.warp declares `canvas()` (no arguments) as the current Picture, so `save canvas as: "painting.png"` works through the bare-call rule; likewise `sound()`.
- P115 bare field names read the receiver (`self`/`this` too). P116 `c.inc()` updates c. P117 a body definition without parentheses is a getter; `class b extends a` inherits.
- P122 static members need `static`; plain `pi = 3` is a per-instance default.
- P162 `init` is the constructor; `constructor`, `__init__`, `initialize`, `new`, a class-named method … are aliases with a note; `new Point(1, 2)` works with a note.
- P123 an object's text is its constructor form `Point{x:1 y:2}`; classes are capitalized in docs.
- P177 `implements` / `struct Square: Shape` is a checked claim.
- P200c every object has `copy()` and `clone()`. P205 `copy()` is deep; `copy(shallow: true)` shallow. P206 resources inside are shared with a note (error under --strict). P207 `clone` calls `copy`; a copy keeps its class. P208 `===` on objects is identity, preferred `same`. P196 `===` compares the full type; P196b `1 === 1.0` yes.
- enum-case: a plain enum case is its own value (`print Coin.Dime` → Dime, `as int` → 2, `2 as Coin` → Dime, `9 as Coin` errs). enum-int: C functions and fields typed with an enum convert to and from its int; in warp only explicitly.
- Reflection: `dir(x)` from compile time when known, else from WASM metadata; x.type, x.fields (alias members), x.methods, f.params, f.effects; lookup order field → meta → reflection; one custom section warp.meta. P230 `effects of f` gives symbols.
- P114 `x.meta` / `x.@key` stay; comments fill them only with `use comments`; `@key` in a literal is never a field; iteration skips `@` keys.
- field-or-minus: `p.phone-number` without that field is "p has no field phone-number" with the fix `p.phone - number`.
- tag-assign: every `=` in a tag body is a value entry like `:`.
- page-ids: an id in the program's own markup is a name in scope (`who.focus()`); a clash with a variable or function is a compile error naming both; kebab ids resolve; `$who` is the element's value and `$who = "Ann"` writes it; dynamic ids via `element("user-name")`; `$` means something only in warp code.

## Signals, events and tasks
- P109 every variable can be watched; checks only where a listener watches. P111 writes through `global x` in called functions run the listeners. P112 no batching block; a multi-assignment notifies once.
- P163 `emit alarm{…}` runs `on alarm` handlers and continues; `raise`/`throw` are errors; fire/trigger/signal alias emit with a note; `send alarm{…}` without `to` is emit. P202 an emit no handler receives warns.
- P129 `broadcast v on "chat"` / `send v to "chat"` reach `on message from "chat"` machine-wide; P129b named events cross programs. P155 channels: `channel()` local, `channel "chat"` machine-wide.
- P156 `whenever cond {…}` is edge-triggered. P38 listeners see later writes in their statements.
- P128 `listeners of x`, `clear listeners of x`, `remove alarm from listeners of t`. P148 `old` (aliases previous, was, before) in `on change x`.
- P134 `on exit {…}`. P135 `on every day at 9:00 {…}`, `at 9:00 {…}` once. P136 system-value rules, one card per value.
- P120 the CLI keeps running while handlers listen. P121 `exit` ends the run.
- P33 real threads for `go` (WASM threads natively, Web Workers in the browser). P47 a started job runs on its own; only a use of its value waits; `await all jobs`. P153 `await any [...]` races. P154 `await job within 100 ms or 0`.
- go-voice: a `go {}` task is a voice sounding along from where its starter stands. deleted-loop: a live_loop deleted from the source stops on re-evaluation (≤ ~4 s fade).

## Modules, library and foreign code
- foreign-signature: a libm function is declared by a typed foreign line `sin(x: float): float from 'm'`; builtin math words use the same line with `from warp`; a bare `floor` is warp's, C's only qualified (`m.floor`); `import (…) from 'm'` keeps working for headers. Open (card foreign-line): a program's own line should win over the header's signature.
- P169 `use math` is the module with readable names; raw C math is `use cmath`.
- P182 Set, Stack, Queue, Deque, Counter only in `collections`. P183 std modules are backed only by warp, host words or C compiled to wasm; regex is the common Rust/JS subset; `file.append(path, text)`.
- P194 std/ merged into lib/; netbase in lib/extra/, later external (P193, P192).
- D15 no automatic imports: `use folder`, `use package`, `use project`. #12 `use <file>`. P139 `import`/`use`/`require` only declare; `include m` runs m. P140 assigning an imported mutable global sets it.
- P146 C libraries compiled to WebAssembly are the way to reuse standard libraries; P147 the prebuilt libc.wasm lives in web/playground/lib.
- P88 every host grants every capability for now; the checks stay. P89 foreign runtimes get their own capability. #8 print's IO capability is granted implicitly. P26 libm counts as pure.
- P94 WIT `char` is a Codepoint. exe-foreign: an executable using python/js starts them from PATH and fails loudly when missing.
- P228 programs using tables or JSON don't build stand-alone; the runtime stays ~1 MB.
- bizarre-global: play, beep, stop_sound, wait_sound are global; other sound words need `use sound`. #14e `download <url>` is an alias of `fetch`.
- package-forms: packages.warp entries take short forms (`uniscript`, `name: owner/repo`, `"github:owner/repo"`); `version: latest` follows the default branch; a pin is `{repository: pannous/uniscript, version: 1.0.7}`.

## Web, markup and server
- P188 stay close to HTML and CSS; only the basic machinery. web-keyframes: CSS's own `@keyframes`; warp adds only `leave:`.
- html-meta: page facts are keys on html (`title:`, `description:`, `meta: {…}`).
- key-if: inside `on key {…}` bare `key` reads event.key; named keys compare as words (ESC, ENTER, UP); held-key: `held_key` is the held key's name.
- unknown-tag: an unknown element warns with the near name; custom elements and components are not flagged.
- web-outside-state: `counters#1.count` reads/changes a component instance. component-calls: components in loops keep the enclosing element as their hole.
- P186/P187 DOM morph, identity by order. P190 page-test words stand.
- Web words: `local["k"]`, `session["k"]`, `database["k"]`, `clipboard.write`; reading the clipboard in a page is an error.
- select-shorthand: a list inside select{} gives one option per item; `["a": "Apple"]` value + label. Form shorthand: `form post "/todos" {…}`.
- P232 `$x` as the implicit subject's field only in served routes; `$a` in data references the enclosing node. P160 `@1` is the node with id 1, `ref 1`/`ref a` in words, `$<digits>` only positional.
- P221 one route line serves both sides: server render first, then data fetches.
- Served routes: a raising form route answers 400 and re-renders; missing field 400, unknown id 404; /api and /rpc give instances as plain objects. route-access: a route gives out only what it displays.
- ORM: plain classes connect to the database by registering, no inheritance; filters run any warp expression; assigning a stored row's field writes through; `save bo` accepted (notes/orm.md).
- site-budget: the hello-world site is ≤ 100 KB gzipped. tour-firefox-stall: a Worker-run site ships task-workers.js.
- Hosting: our own login first, then deploy to one's own provider: Cloudflare Workers, Ferron on pannous.com (`<name>.lambda.pannous.com`), Fermyon/Akamai (notes/hosting.md).
- P235 WGSL is a `shader { … }` block (alias `wgsl`); P235b no capture of warp variables, `$name` is an explicit hole; shader-untouched: WGSL stays as written otherwise; WGSL errors are reported at the .warp line and column. P234 `paint` takes a shader text; `use graphics` = `use draw`.
- gpu-multipass: `waves = render(height_field, 256, 256)` paints off-screen, the image stays on the GPU (alias `texture`); another painted function reads it as a function, `waves(uv)`, like `$previous(at)` (Interviewer's decision; notes/gpu.md "Several passes").

## Sound
- play-dispatch: `play` dispatches on what it plays (signal, stereo pair, block function); `play(hum, 2s)` until prepositions-call.
- sound-at: `at 2 beats play C4` places sounds from the voice's first sound.
- note-value: note names are frequencies (`C4 * 2` = 523.25); `note(C4)` is a Note printing "C4", `note(445Hz)` "A4+20c".
- shared-buses: a bus is a named effect chain `room = bus() |> reverb(0.4)`; `osc(C4, 1s) |> room`, `play(C4, 1s, bus: room)`; `room.reverb = 0.6` applies from that moment.
- sound-dsp: the effects DSP is Rust (src/sound/effects.rs, also in warp.wasm); lib/sound.warp calls it.
- save-sound: audio goes to a file through the general `save` (`save sound as: "song.wav"`); render_sound gives an I-meant hint.
- signal-samples: samples/signal*.warp are siren_sound/stereo_sound.

## Old id index (superseded or historic; git has the text)
P35 → index-position. P42 → P227. P56(2-4) unknown entity → P56 revised. P63 → wiki/charged.md. P66 → footgun-divzero.
P103, P104 → no-executable-by-default + build-should. P110 → P163. P118, P214 → P240. P168 → field-or-minus.
P170 → type-library-words. P200 → P200b. P52 bare labels → labels-calls. D1 → P6 + brace-hole. D7/#33 → wiki/charged.md.
D8 → approx-zero. D9, D12, D13 → got-it rule + reconsider-spaced + list-sign. D14 → in-bool. exact-default →
exact-decimals. #40 → list-sign. dimension-bridges → pixel-lengths. dollar-element → page-ids. definition-cases → cut.
P85, P87, P93, P95, P96, P98, P99, P101, P137, P144, P181, P53, P59, P79, P80, P9, P10, P16, P19, #2, #14b–d, #15–20,
the original questions 1–41: test edits and housekeeping done.
