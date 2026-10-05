# Open decisions for the user

Only the Interviewer asks the user (notes/roles.md). Nothing here blocks: each question names the assumption the code
already follows. Answers move to a Decided section with the date and the user's words.
Details: notes/todo_sweep_task.md (board), notes/semicolon_survey.md, notes/float_truncation_survey.md.

## Pending questions (ordered by impact; recommended option first)
- P28 `real x;` (declared without a value) read before an assignment: loud error "x is declared without a value"
  (Java/Kotlin) / zero value of the type (Go). Assumed: error (analyzer::check_unassigned_declarations). Night
  session 2026-10-04.
- P29 `pair.0`: counts from 0 like `pair[0]` (Rust/Swift tuples) / from 1 like `pair#1`. Assumed: from 0
  (library_words method_call). Night session 2026-10-04.
- P30 `xs.pop()` gives the last item and removes it from the variable (Python); `xs.pop()!` style mutation markers
  are not required. Assumed: Python semantics. Night session 2026-10-04.
- P35 `m.remove(k)` on a map and `xs.index_of(x)` on a list: remove the key from the variable and give its value
  (Python dict.pop) / give a new map; `index_of` 0-based (Python) / 1-based like `x in xs` and `in` on texts.
  Implemented defaults (supervisor, night 2026-10-04): remove takes the entry out of m and gives its value (ø when
  absent), analyzer::removed_key + the runtime map_without; index_of is the 1-based position, 0 when absent
  (a synonym of collection_position). Test: tests/lists/test_remove_and_index_of.rs.
- P44 Shared arrays across tasks (user P33 step 6, notes/threads.md): `shared xs = int[n]` makes n Ints (zeros) in a
  memory every task of the run shares; `xs#i` reads, `xs#i = v` writes, `xs#i += v` / `-=` add atomically (the value
  is the new one), `#xs` / `count(xs)` is n; `go f(xs)` passes the same array, the one exception to copying, marked by
  `shared`; a parameter given a shared array is shared inside its function. Ints only for now. Implemented default;
  alternatives: `atomic` instead of `shared`, an explicit `xs.add(i, v)` instead of atomic `+=`. Night 2026-10-04.
- P46 Type-name patterns in `for` (wiki/for.md): `for friend in [foe1, friend1, foe2, friend2]: print it` walks only
  the items whose name matches the loop variable's type name, and `for (it>2) in [1,2,3,4]` the items a condition
  holds for. Proposed default (parked, supervisor): `for NAME in xs` stays every item; a filtered walk is written
  `for x in xs where cond` (or `xs.filter(…)`), and a type-named walk `for friend in xs` filters by the declared type
  `friend` only when one is declared. Not implemented. Night 2026-10-04.
- P47 Lists of tasks: a job used as a value awaits it (the auto-cast, P33), so `jobs.add(j)` awaits j at once and
  tasks cannot be collected without running them one after the other (200 tasks of 200 ms: 41 s). Proposed default
  (supervisor): adding a job to a list keeps the handle unawaited, the auto-cast applies only where a value is needed
  (arithmetic, print, a call taking a number), and `await all jobs` waits for every job of a list, giving their
  results. Not implemented. 2026-10-05.
- P48 Custom operators (wiki/operator.md): `prefix operator ⁻ := it*-1`, `suffix operator ³ := it*it*it`,
  `suffix operator ⁰ := 1`. Proposed syntax (parked, supervisor): exactly the wiki's, `prefix operator SYMBOL := body`
  and `suffix operator SYMBOL := body` with `it` the operand, plus `infix operator SYMBOL := a op b`-style bodies naming
  `left` and `right`; prefix and suffix operators bind tighter than any infix one (like `-x` and `x²` today), a new
  infix operator binds like `+` unless declared `infix operator SYMBOL precedence of *`. The parser learns the symbol
  from the declaration before the rest of the file (one pre-scan), so a use before the declaration works. Not
  implemented. 2026-10-05.
  Implemented as the assumption (worker warp-90, branch custom-operators): prefix/suffix/infix declarations existed
  (src/wasp_parser.rs `scan_user_operators`); added superscript glyphs (`suffix operator ³`, `prefix operator ⁻`) that
  override the built-in power, and the short `suffix ⁰ := 1`. Marked `ASSUMPTION P48` in the code, test file
  tests/operators/test_superscript_operator_declarations.rs. Precedence declarations stay refused.
  test_wasm::test_custom_operators stays ignored: its `.5³` line expects Rust's `1 / 8` (= 0), the value is 0.125.
- P51 An unknown word applied to a value, `cube 3` (no `cube` defined): today data with a got-it warning
  ("cube is no function … fix: [cube 3]", notes/unknown_prefix_word.md). Should it be a loud error like `cube(3)`
  (Decided #7)? Then call-shaped data in code needs brackets: `[cube 3]`, `(cube 3)`, which contradicts rule 3 of
  notes/unresolved_call_survey.md (`print 3` as data). Proposed: keep the warning (wasp is a data notation; strict
  mode makes it an error). Asked by worker warp-90, branch unknown-word. 2026-10-05.
  User 2026-10-05, undecided: "i'm not certain probably the second one [loud error] or we need to declare data in
  the beginning of the file or I don't know let's discuss it later". Keep the warning until discussed.
- P52 Calling a `to` phrase with its prepositions: `to add number a to number b: a+b` is called `add(1, 2)` today;
  should `add 1 to 2` work too, and `square of 4` for `square of a number`? Today `1 to 2` is a range and `x of y` a
  field lookup, so `add 1 to 2` is add(1..2). Proposed: when the name is a phrase-defined function, its own
  prepositions win; everywhere else the operators keep their meaning. Asked by warp-90, branch type-name-params.
  2026-10-05.
- P53 test_wasm::test_custom_operators stays ignored only because its `.5³` line expects Rust's `1 / 8` (integer
  division, 0) while the value is 0.125. Options: change that line to 0.125 and un-ignore (recommended) / leave it
  ignored. Assumed: ignored. Asked by warp-90, branch custom-operators 28ee01c2. 2026-10-05.
- P54 tests/operators/test_operator_declarations.rs::test_precedence_declarations_are_refused pins
  `infix operator ⊕ := a+b; operator ⊕ has precedence above +` as "not supported yet"; the new feature makes it work
  (X must be a declared operator; built-ins are never re-ranked). Options: replace the assertion with the positive
  result (recommended) / keep the refusal, drop the feature. Assumed: replaced in its own commit. Asked by warp-90,
  branch operator-precedence. 2026-10-05.
- P55 test_while_nop_issue (ignored) expects `x=0;while x++<11: nop;` to be 11, but a while loop's value is its last
  body value (Decided 2026-09-30). Options: rewrite the test to `…;x` (gives 11 today, recommended) / a loop whose
  body has no value gives the condition variable. Assumed: stays ignored. Asked by the Fixer warp-2d. 2026-10-05.
- P56 Uniscript entities (row 37, branch fix-uniscript-entities, all four assumed as recommended): (1) `\name` inside
  double-quoted texts: no, code only (`"\nat"` stays newline+"at", not ℕ); (2) `\epsilon`/`\phi` are ε U+03B5 and
  φ U+03C6 (not LaTeX's lunate ϵ/ϕ); (3) an unknown `\name` is the loud error "unknown entity \name"; (4) `∞` has no
  value yet: f64 infinity now, or wait for the hyperreals (ω)? Asked by the Fixer warp-2d. 2026-10-05.
- P57 Hyperreals (branch hyperreals, notes/hyperreals.md; ε exact with integer exponents, compile time only):
  (1) `1/(1+ε)` (an infinite series): keep the error (recommended) / truncated series marked approximate
  (HyperApproxInv.lean); (2) dual-number mode (ε² = 0) via `use dual`: later (recommended) / now; (3) hyperreals at
  run time (a WASM GC representation): later with exact reals at run time (recommended) / now. Asked by warp-90.
  2026-10-05.
Parked: #10 Polish notation for .wat/.wast, user "Keep parked" 2026-10-03.
Dropped as answered: code quality 7 (Node operators return Node::Error: Decided #1, errors as values); #14 (test_math
uses near!), #14c (exit(0) commented out), #15 (decided: delete), #17/#18 (done), #20 (AGENTS.md fixed; CLAUDE.md → P12),
#24 (upto decided exclusive 2026-10-02), #29 (checkout is only behind now), D5 detail (notes/matching.md accepted).

## User to-dos (not questions)
- Branch `claude/shared-kind-constants` (only unmerged commit: the dropped cloud pilot report); the hook refuses, run
  `git -C /Users/me/dev/angles/warp push origin --delete claude/shared-kind-constants`. From BOSS-cheeky-shannon.
- Cloud-Microsoft environment setup script needs `rustup target add wasm32-wasip1` (claude.ai/code → chevron next to
  the session title → Edit cloud environment). From BOSS-cheeky-shannon.

## Decided 2026-10-05 (user, multiple choice, all as recommended)
- P50 a declared scalar parameter broadcasts over a list too (`foo(x:int):=x+1; foo([1 2 3])` → [2 3 4], the wiki's
  `square number = …; square [1 2 3]`); test_argument_kinds and test_parameter_call_kinds change to a text argument
  (user decision). `print [1 2 3]` keeps printing the list (not asked; the recommended default stands).
- P49 a float passed to an int parameter is a compile error ("2.2 is no int: write 2.2 as int"); the ignored
  test_function_argument_cast is edited accordingly (user decision).
- P22 `p:photo = pic{width:3}` with pic a known other type: an error that teaches `pic like photo`. The warning
  default on claude/like-keyword is undone.
- P23 the type-word clash error covers every type word, generic ones (`number := …`) included (as assumed).
- P24 the suffix form `4 doubled` calls a user function too (like `4.square`, wiki D9 `1+2 squared`). Not done yet.
- P26 libm (sin, exp, …) counts as pure: no Ffi capability, allowed in eval_untrusted and `! pure` functions.
  The Ffi assumption (effects.rs) is undone.

## Decided 2026-10-05 (user, own words)
- P45 a variable given values of two kinds that do not mix (`x=[1]; x=5`): user: "compile error unless we are in
  script mode, which is not defined yet". The implemented Node fallback is undone outside a future script mode;
  the error names the variable and suggests another name. Int→Float widening stays. Script mode: a new open topic.
  Done (branch p45-kind-change): analyzer::check_kind_changes on the program as written, before lowering, compares
  the kinds evident from literals (`x = [1]; x = 5` → "x was a List, is given an Int: use another name"). Where a
  kind is only inferred (a call's result, a loop variable reusing a list's name) the Node fallback still holds the
  value: an error there would rest on guessed kinds (function results default to Int before inference).

## Decided 2026-10-04 (user; moved out of the pending queue 2026-10-05)
- P31 DECIDED (user, 2026-10-04): the Printable operation is `text(p:person)`, the one allowed exception to type words as function names; `as text`, print and interpolation call it. Printable trait: the operation that gives an instance's text for interpolation, `as text` and print. `text` is a
  type word (P20 forbids it as a function name). Options: `show(p:person)` (Haskell) / `description(p:person)` (Swift)
  / allow `text(p:person)` as the one exception. Not implemented yet (todo.md "Traits: Printable"). Night 2026-10-04.
- P32 DECIDED (user, 2026-10-04): `print xs` of a list prints the str(xs) text; the pinned test in tests/welcoming/test_welcoming_print.rs may change for it. `print xs` of a list variable: allow it with the text `str(xs)` gives ("[1 2]", nested lists too) / keep the error
  "print of a List has no runtime text yet" that tests/welcoming/test_welcoming_print.rs pins. Recommended: allow
  (the text exists now); needs the edit of that pinned test. Night 2026-10-04.
- P33 DECIDED (user, 2026-10-04): REAL threads for `go` (WASM threads with shared memory natively, Web Workers in the browser); last, as a big project: notes/threads.md first. Real concurrency for `go`: tasks now finish where they start (one thread), so pause/stop handlers never run (a
  warning). Options: keep it / wasm threads + shared memory natively and Web Workers in the browser / an event loop
  with explicit yields (`await` points). Assumed: keep it. Night 2026-10-04.
- P34 DECIDED (user, 2026-10-04): a number subscript on an empty `{}` keys it as a map (`d={}; d[1]="a"` → {1:"a"}). `d = {}; d[1] = "a"`: a number subscript of an empty `{}` indexes it as a list (index out of range) / keys it as a
  map like a text subscript does (Lua tables, JS objects). Assumed: list indexing (today's behaviour, a loud error).
  Night 2026-10-04.
- P36 DECIDED (user, 2026-10-04): implement unit conversion as proposed below. Unit conversion: `100 cm in m`, `2 h in minutes`, `3 km as m` are undefined today (`in` places a time in a zone,
  `as` casts to a type). Proposed default: `quantity in unit` and `quantity as unit` give the quantity in that unit of
  the same dimension, exact when it divides (`100 cm in m` → `1 m`), else a ratio (`150 cm in m` → `3/2 m`); the unit
  words take their long names too (minute(s), hour(s), meter(s)); a unit of another dimension is the DimensionError.
  Not implemented (not in the wiki). Night 2026-10-04.
- P37 DECIDED (user, 2026-10-04): named arguments f(name=value)/f(name:value) set parameters and free variables of the body; no implicit capture of same-named variables, a missing argument stays an error. Implemented (src/lowering/named_arguments.rs): `name:{…}` stays an ad-hoc instance (it parses like `name{…}`), so an object passed by name is written `name={…}`; `s:shape` in a signature is a declaration. Gap filling (wiki/gap-filling.md, binding.md, inventions.md): `f y := y*y+v; f(y=2, v=3)` → 7, `fun={x*y};
  fun(x:2 y:3)` → 6, `x=7; f(x):=x*x; f()` → 49: named arguments bind a function's free variables, and a missing
  argument takes the variable of its name. Proposed default: named arguments `f(name=value)`/`f(name:value)` may set
  parameters and free variables of the body; a missing argument stays an error (implicit capture of a same-named
  variable is too surprising). Not implemented. Night 2026-10-04.
- P38 DECIDED (user, 2026-10-04): keep the current default. Variable listeners (wiki/signal.md Todo: "shall event listeners be registrable post-hoc?"): `once x==5 {…}` and
  `whenever cond {…}` are implemented (src/lowering/variable_signals.rs) as checks after each later write of a variable
  the condition reads, in the statements after the listener and their loop bodies; writes before it, in functions
  called later, or in an outer block are not seen, and the condition holding when the listener is declared does not
  fire it. Alternatives: hoist listeners to the top of their block (post-hoc registration), or watch writes in called
  functions too (needs global flags). `on set x {…}` (value = the new value) works the same way, and `after tested:` /
  `before test {…}` (a defined function, also named in the past tense) run after/before each later statement that
  calls it, once per statement even if it calls it twice. `during` is not implemented. Night 2026-10-04.
- P39 DECIDED (user, 2026-10-04): neither `id` nor `square` becomes a builtin; the 5 tests stay ignored ("user: no id/square builtins"). Builtins `id` and `square` (C++ wasp's test runtime words): five ignored tests use them (test_comparison_id,
  test_comparison_id_precedence, test_wasm_function_calls, test_wasm_stuff, test_squares); braceless user functions
  already bind the same way (`f 3+4`). Should they be global library words (a user definition winning), given `square`
  is also a shape type in the trait tests? Parked for the user. Night 2026-10-04.
- P40 DECIDED (user, 2026-10-04): size = count; `byte_size` gives bytes; test_array_constructor stays ignored (P40). `size` of a typed array: test_array_constructor (ignored) wants `size(640000*int)` = 2560000 bytes, the passing
  test_array_length says `size` is a synonym for count. Which is right (bytes as `byte_size`?)? Parked for the user.
- P41 DECIDED (user, 2026-10-04): no separator, as implemented. Juxtaposed print arguments: `print "x changed to " value` prints the parts joined without a separator ("x changed
  to 3"), only when the first part is a text literal (`print first xs` stays a call); commas still join with a space.
  Implemented default (wiki/signal.md example). Alternative: join with a space like the comma form. Night 2026-10-04.
- P42 DECIDED (user, 2026-10-04): keep the parse (comma looser than ==) and warn strongly when a tuple is compared to a bare comma expression `(…) == 2.0, 4`, suggesting parentheses. Comma against `==`: `(2 as float, 4.3 as int) == 2.0, 4` (ignored test_emit_cast_tuple) wants the right side
  to be the tuple (2.0, 4); today the comma binds looser than `==`, so it is `((…) == 2.0), 4`. Tuples in parentheses
  compare element by element now (`(…) == (2.0, 4)` → 1). Change the precedence (or only for a tuple on the left)?
  Not changed. Night 2026-10-04.
- P43 DECIDED (user, 2026-10-04): lists do not grow when an item is set past the end, the empty list included;
  setting past the end is a loud, catchable `index out of range` error, as the existing tests have it
  (`x=(1 2 3);x#4=0`, test_footguns). An experimental growth (night 2026-10-04) was dropped before commit; the
  ignored tests expecting growth (test_array_creation) stay ignored.

## Decided by the user (2026-09-29) — implementation: notes/cloud_tasks.md
- #1 yes: avoid panics everywhere, errors as values. #2 leave the trailers. #3 juxtaposition yes, spaced only if the unit exists.
- #4 `size` = count; bytes via `byte count` / `number of bytes` / `#bytes in list`. #5 units yes.
- #6 data as scope yes, warning when the kebab parts are also variables. #7 unresolved calls are errors.
- #8 print gets an IO capability that eval grants implicitly (hidden). #9 `list of int`, plural type words (`numbers`) are lists.
- #10 no opinion (parked). #11 exact reals may be assigned to a declared float with precision loss: `float x = π` allowed.
- #12 `use <file>` yes. #13 web host later. #14 not-implemented errors unless easy (shifts: implement). Rest: cleanup.

## Decided 2026-09-30
- Keep: unit sums use the finer unit (3010 m); `f - x` with a parameterized user function is `f(-x)`; untyped parameters take
  the kind all call sites agree on, else a loud error.
- Assignment past the end of a list stays an error. `x : 100 int` AND `pixel:int[100]` declare typed arrays.
- A while loop's value is its last body value. `pixels size` (property word after a name) works like `size of pixels`.
- Both `list<int>` and `list of int` in code. Delete test_paint_wasm. Vendor refresh automated (free, only on Cargo.lock change).

## Decided 2026-10-03 (relayed by BOSS-cheeky-shannon to the fixer, playground `print greeting*2 print(g, g) print g, g`)
- `print a    print b` on one line: "Error with hint". Loud error "two statements on one line? separate them with `;`
  or a newline" (wasp_parser.rs grouped_list, tests/parser/test_one_line_statements.rs).
- text * number: first "Always ask", then superseded (Asks are being replaced by got-it warnings, warp-b8): repeat the
  text, with an educate_once "got it" warning naming `n times text` (and `int("5")*3` for a number-like text).
  Confirmed by the user as "Python repeat" (P1 below); implemented on fix-text-repeat (tests/text/test_text_repeat.rs).
  text * float and text * text stay type errors; `n times "ab"` is the explicit repeat. This replaces "`"5"*3` stays a
  type error" from 2026-10-02.

## Decided 2026-10-03 (user, multiple choice; not implemented yet)
- Non-digit characters (P27, asked by warp-d2), user to warp-d2, verbatim: "use ord ordinal codepoint() to get the
  code point". So `ord(c)` / `ordinal(c)` / `codepoint(c)` give the code point; `c as int` / `int(c)` of a non-digit
  character is invalid_number, digits stay digits. Revises #35's `'A' as int` → 65. Branch int-of-char edits
  test_text_casts and test_cast_bugs accordingly (user decision).
- int of a character (P25, asked by warp-d2): user to warp-d2, verbatim: "obviously one of five is five". `int('5')`
  is 5 (main does this since a13b4fd6; pinned by tests/text/test_character_comparison.rs, branch int-of-char).
- Text quotes in printed output (asked by warp-6c; user WIP commit ec968e18 expected single quotes in
  tests/test_method_words.rs): "Keep double quotes". The printer keeps `"HELLO"`; the WIP test edits are reverted to
  double quotes (user decision) and the 5 ignored tests un-ignored.
- `try` syntax (P21, asked by warp-bc, branch claude/try-exits-and-naming), user verbatim: "try X else
  otherValueOrAction     I never invented the => Y syntax". The form is `try X else Y`, Y a value or an action;
  the named binding `else e => Y` is not wasp syntax and is removed (the question how e binds is moot).
- Objects as arguments (asked by warp-bc for Cloud-Microsoft, branch claude/object-arguments). CENTRAL PHILOSOPHY,
  to be written into the wiki and implemented. User, verbatim: "if it's truly unknown then this is a duct typing like a
  python if it has all the fields but is a known different type then we should create an error and get the user about
  a new keyword image like photo so the image will be treated like a photo and judged by users we should write that as
  a very central philosophy which needs to be implemented". So: a value of truly unknown type passed to `p:photo` is
  duck typed (judged by the fields it is used with); a value of a KNOWN different type (an `image` with all of photo's
  fields) is an error that teaches `image like photo`, which declares image usable as a photo, judged by uses.
  Keyword: "`image like photo`" (chosen over `as`/`is`). Written map `{width:7}` for `p:photo`: "Accept
  structurally". `p.word` typo on an untyped parameter: "Field lookup, runtime error" ("no field word").
- libm FFI (warp-12 experiment: headers serve all of libm on macOS incl. hypot; glibc's __MATHCALL macros defeat the
  header parser on Linux): "Headers first, table fallback". Header-driven linking always; LIBM_UNARY/LIBM_BINARY link
  only when the headers give nothing for "m". Patch: warp.worktrees.noindex/ffi-libm-experiment.patch.
- tests/ folders (asked by warp-1a, branch tests-tidy, notes/tests_layout.md): "OK as listed" (17 topic folders,
  welcoming one folder). Duplicates: "Only probes condense". Probe lines condense into topic files; assertions in
  regular test files stay even when duplicated.
- Wiki remote `main` (asked by warp-d0, notes/wiki_branch.md): "Delete remote main". GitHub wikis serve only master;
  master stays the only branch, agents push `HEAD:master`. The hook blocks agents, so the user runs
  `git -C /Users/me/dev/angles/warp/wiki push origin --delete main`.
- #10 Polish notation for .wat/.wast (P18): "Keep parked". test_wast stays ignored.
- #16 commented `"a".s() + 2` lines in tests/text/test_string.rs (P19): "Delete the lines" (approved test-file edit).
- tests/ layout (P14, code quality 3), verbatim: "The official policy was that probes are can be turned into a real
  tests by condensing that what really matters. We should start a new agent to sort that all out and just grouped into
  folders". A new agent condenses the `probe_*.rs` files into real tests (keeping what matters) and groups tests/ into
  topic folders.
- CLAUDE.md (P15, code quality 8): "Symlink to AGENTS.md".
- Root clutter (P16, code quality 9): "Move to notes/OLD" (the dangling `wasp`/`warp` links are deleted).
- D10 return-type polymorphism (P17): "Dispatch on return type" (un-parked). `render "hello" as pdf` /
  `docx example = render "x"` pick the overload by the expected type; ambiguous → got-it warning (assumed under the
  Asks-become-warnings rule).
- smarty.rs + tests/test_asts.rs (P10, code quality 2): "Delete both" (with smarty's asserts in tests/numbers/test_angle.rs).
- Wisp format (P11, code quality 4): "Keep + add a roundtrip test".
- GC reading API (P12, code quality 5): "GcObject". The gc_traits wrappers go.
- libm table (P13, code quality 6): "Keep the table", then verbatim: "Mark the FFI deliberately S. examples and maybe
  quickly try if it works without the hard coding". So `LIBM_UNARY`/`LIBM_BINARY` get a comment saying they are deliberate
  examples of hand-linked FFI, and an experiment checks whether the header-driven FFI serves the same calls without
  them (result decides whether they become example-only).
- Type-word shadowing (P20, asked by warp-bc, branch claude/type-word-user-function): "Clash error". A user function
  named like a type word (`double := it*2`) is an error: "double is a type; rename your function". Reverts the
  shadow-with-warning default.
- cdylib (P7, code quality 10), verbatim: "what is that it doesn't mean that didn't need that before can we gate it".
  Answer: the cdylib is the browser-wasm output, and web/playground/build.sh already asks for it itself
  (`cargo rustc --crate-type cdylib`), so it is already gated. Cargo.toml keeps only `rlib` (since 2024-02 it listed
  `["cdylib", "rlib"]`); the per-worktree crate-type patch is no longer needed.
- vendor/ (P8), verbatim: "currently we don't need it but maybe we want to run an off-line agent later again so let's
  just note that it's currently deactivated". The docs (Cargo.toml, AGENTS.md) say vendoring is deactivated for now;
  `--offline` builds from the registry cache.
- Stale C++ feature flags (P9, code quality 1): "Remove them". The `#[cfg]` branches in tests/wasm/test_wasm.rs and
  tests/web/test_web.rs go, keeping the branch that runs today (approved test edit).
- Asks become got-it warnings (user to BOSS-cheeky-shannon, verbatim): "I really love the got it mechanism for the
  warnings, the Ask mechanism is not what I expected. I thought it would rewrite the code to whatever the user pics we
  don't want context, sensitive execution, lol instead turn all the Ask into a warning with the got it feature plus an
  extra feature for later as an intelligent intent to change the code." Assigned to warp-b8 (ask-to-warning).
- Error-fallback Asks (`[x]*n`, `insert(0, 4)` order, `[1 2 3]+4`, bare `a=1 2 3`, `1+2 squared`): "Stay loud errors".
  They name the explicit forms, no default reading is taken. Asked by warp-b8; its assumption stands.
- Text * number (P1, asked by warp-35): first "actually, why not use the python app approach? Is it really a foot gun",
  then "Python repeat". `"ab"*2` → "abab"; digit text `"5"*3` → "555" with a got-it warning that it is text, not 15.
  Replaces the type error with the repeat hint (also the earlier "text * number: Always ask").
- Typed lists follow-up (P4, asked by warp-5e): "Yes, continue". Float typed arrays, then whole-op Sum/Map ListOps.
- D9 fallback (P5): "Error". An unanswered `1+2 squared` is an error naming both groupings (assumption stands).
- D1 `$` (P6), verbatim: "sorry, I don't know what a hole means but only the one with the curly braces must interpolate
  the other is text like dollar money". So `"${expr}"` interpolates, bare `"$x"` stays the literal text `$x`.
  Revises D1's "also `$x`": tests/text/test_interpolation.rs `dollar_holes_interpolate_too` follows (user decision). Swift
  `"\(expr)"` was not asked about and stays.
- Tuple returns (P2, asked by warp-d7/warp-5e): user "yes" (answer "no yes ?" in warp-5e's session, second item).
  `return a, b` and `x, y = f()`, compiled to wasm multi-value without allocating a list. Not built yet.
- Closure Int->Int fast path (P3, asked by warp-5e): user "no" (same answer, first item). Closure calls keep boxing.
- D7 / #33 closures: "By value + educate". Blocks keep capturing by value (`x=1; inc:={x=x+1}; do inc; x` → 1); a block
  that assigns an outer variable gets a hint: use `global x` or return the value. The wiki's lazy `:=` examples get updated.
- D3 `[1 2 3]+4`: "Ask". Like `[x]*n`: append or add to each element? Fallback Error. `.+` is element-wise,
  `xs + [4]` concatenates.
- D12 `a=1 2 3` / `a=1,2,3`: first "idk", then "Ask on bare forms": an unbracketed `a=1,2,3` / `a=1 2 3` asks list or
  separate statements, fallback Error; bracketed lists never ask.
- #39 chained comparison: "== never chains". `a<b<c` chains as before, but `1<2==2` is `(1<2)==2` (wiki/Footguns.md
  "Chained comparison" and test_chained_comparison's `1<2==2` line need updating; ask before editing that test).
- D1 interpolation: "Both". Double-quoted text accepts Swift `"\(expr)"` and `"${expr}"` (also `$x`); the normalizer
  picks one canonical form. Single quotes stay literal. Open detail: `$` holes in sql/sh templates must keep working.
- D6 pipe: "The pipe operator is just an operator that behaves differently with different types unified". So `|` is one
  operator dispatched on its operand types: truth values → logical or, a value and a function → pipe (`2|square|root`).
- D14 `x in list`: "The position one index would behave as a truth but maybe it's a foot gun let's try it with a
  warning". `3 in [1 2 3]` gives the position (truthy when found), with a warning. Caveat for implementation: a 0-based
  position makes the first element falsy, so the position must be 1-based (or found-at-0 still truthy).
- #38 `[ø]`: "Truthy". A non-empty list is truthy; the test_wasm_logic_on_objects expectation may be edited.
- D16 numbers: "Unbounded + FFI types". Int stays unbounded (BigInt promotion); `byte`, `int8..int64` exist as declared
  types for FFI that trap on overflow. No `overflow` value.
- D2 `!` (evaluate / mutate / await): "let's think about this later", parked.
- D13 `1 -1`: "ask and assume list". An Ask (signed operand glued after a space) whose default reading is the list
  `[1 -1]` (fallback Warning, taking the list); the arithmetic reading is written `1 - 1`.
- D9 suffix precedence (`1+2 squared`): "Ask". An ungrouped mix asks `1+(2 squared)` or `(1+2) squared`.
- D5 matching by type name: "General rule". Any noun can name a type/parameter (wiki matching.md); open detail: the
  rule for unknown words (`photo`) and multi-word class names.
- D4 constructor vs data: "Distinguish". `T{…}` with a known type constructs/validates, `k:{…}` is plain data, not equal.
- D8 `≈` / `~` / `circa`: "Relative 1e-9 + override". Default relative tolerance 1e-9, settable via `tolerance = …`.
- D15 auto-imports: "Later", parked. D10 return-type polymorphism: "Park", parked.
- #25 / #36 `first [10, 5]`, `reduce [7] …`: "if by subscript you mean index then we already have a rule that space
  disabled index". So a space before `[` never indexes; reading these as an index is a bug against that rule:
  `word [..]` passes the list as an argument, only glued `a[..]` indexes.
- #22 loops: "N once, keep rest". `N times {…}` evaluates N once; trailing `i++ while c` stays a plain while (with a
  hint); `a = 2 if c` keeps guarding the whole assignment.
- #21 type(): "x=π stays real" (bug: `x=π; type(x)` must be real like `type(π)`). Not chosen: `2.0` stays int
  (current), no `rat` alias.
- #27: "Accept Unicode + `be`". `≤ ≥ ≠ × ÷ ¬ √` are accepted alternatives normalized to ASCII; `be` is implemented
  as `:=` (wiki/be.md).
- #26 library: "Extend all". Unicode upper/lower, sort any comparable values, reverse works on text.
- #23: "Warn on `it` shadowing". A loop's `it` still shadows a function's `it`, with a warning.
  #31 (switch message shows value; `min`/`d` units): neither option chosen, stays open.
- #35: "Lists join to "[1 2]"" (the tests/text/test_cast_to_string.rs assertion may be edited; a general runtime serializer
  later) and ""x" as float is loud" (error with a hint; only single-quote codepoints convert to numbers).
- #34 deep traps under `try`: "Later". #37 first-class functions by specialisation: "Enough for now".
- #14e: `download <url>` is an alias of `fetch`.
- #31: "Show runtime value, min/d stay non-units". `no case for n = 4`; `min` stays the function, `d` free.
- #14b test_wasm expectation defects: "Yes, minimal edits" (tolerance compares, expectations per current decisions,
  each edit listed in the commit).
- #19 / #32 stashes: "Drop them if they contain nothing valuable otherwise merge". Checked: concat-wip-tag, the global
  compound-assignment autostash and the README/Footguns autostash were already on main; pre-history-rewrite held only
  .DS_Store and test_results.txt. All 4 dropped. The one unmerged line, the user's 2026-09-27 note on chained
  comparison "BUT NOT WITH == etc !!", became #39 below.
- D4 fields (asked again 2026-10-03, wiki/constructor.md says optional + extendible): "Strict". A missing field is an
  error unless `?` or defaulted (`!` = explicit required); an undeclared field is an error with the hint `T:{…}`.
- Meta information on objects (user idea 2026-10-03: "Think of a special syntax to extend known objects with meta
  information"): existing `@name(value) X` prefix and `X["@name"]` stay; chosen additions: `@key` inside a literal
  (`point{x:1 y:2 @source:"gps"}`, never a field, not counted, ignored by == and by strict validation), `x.@key`
  read/write, glued postfix `x@key`, and attributes survive wasm emission/eval. User: "Can we also access them
  normally as a fallback? point.source first, check if it's a real attribute, then check if it's a meta. Or is that
  too complicated and overkill?" Answer (warp-43): not overkill, one fallback in the field lookup: field first, then
  meta; when both exist the field wins and a hint names `.@source`; neither → the strict "has no field" error.
- Round 3 (2026-10-03, multiple choice):
  - #40 `x -1`: "Keep subtracting". Only a number literal on the left asks (`1 -1`); variables, calls, words subtract.
  - D2 `!`: "By position". `fn!` after a function/method mutates in place (`x.upper!`), `{…}!` evaluates a block,
    await gets its own word `await job` (no `job!`).
  - D14 `in` warning: "Never warn". `3 in [1 2 3]` gives the position silently (the impl-sem warning goes).
  - D5 type-name matching rule (notes/matching.md): "Accept".
  - D15 auto-imports: "Yes, folder scope" (REVISED the same day: opt-in `use folder/package/project`, see above).
  - Meta keys in iteration: "Skip @ keys". Iteration, keys, values and count ignore `@` entries.
  - #34 deep traps under `try`: "Do it now".
  - 14d test_comments2: user "what? 0.length() ???"; explained: the test parses two lines, `y=0` is one Key whose
    length is 0 (C++ counted the 3-item list `[y = 0]`). Correct for the current model, closed.
- D15 REVISED (user to warp-b8, 2026-10-03): "revision of my previous decision create special keyword use folder and
  use package and use Project to automatically include everything but not by default anymore". No automatic folder
  scope; `use folder` (the program's folder), `use package` (below the nearest folder holding <name>.wasp),
  `use project` (below the nearest .git), lazy per-name lookup, same conflict rules.
- #30 type tests: "Only `is` tests types". `3 is int` → 1, `3 is rational` → 1; `3 == int` educates toward `is`.

## Decided 2026-10-02 (relayed by warp-f3): eat newcomer syntax, compile its intent, hint the wasp form
- Text + number concatenates, the number in its text form (`"F:" + 13` → `"F:13"`, `"5"+3` → `"53"`, JS/Kotlin), with a
  hint `str(13)`. Reversed: the 2026-09 "no implicit conversion" rule (DESIGN.md "Dangerous implicitness", wiki/Footguns.md
  "String + number"), which was there because one-character strings used to add as code points (`"5"+3` → 56). `"5"*3`
  stays a type error. Flipped tests: test_text_concat::text_plus_number_stays_an_error,
  test_footguns::test_text_plus_number_is_a_type_error, test_text_bytes::text_plus_number_stays_a_type_error.
  Not yet: a runtime ratio (`y=2.5; "x"+y`, also `y as string`) prints garbage: list_join has no text form for ratios.
- `//` glued to its operand (`7//2`, `x//=2`) is Python floor division, the Euclidean quotient that goes with `%`
  (`floor_quotient(a, b)` since 2026-10-04, same results as the earlier `(a - a%b)/b`: floor for a positive divisor,
  `-7//-2` gives 4 where Python gives 3); `x // note` (space before) stays a comment, and one whose comment hides a
  closing bracket (`(col // 3)`) is a parse error naming the `//` (no reinterpretation).
  `a div b` is the same floor division. An index that divides (`xs[n/2]`) traps `index must be an integer` unless the
  division is exact, with the hint `n//2`.
- Spaced `a // b` — USER DECISION 2026-10-03 (fix-floor-ask-3): "just make it a warning to the user that it's read
  as a comment, don't do heuristics". A spaced `//` is always a comment, only glued `a//b` / `x//=b` divide. A `//`
  comment after code on its line educates once (diagnostic::educate_once, topic `slash-comment`): "`// …` after code
  is a comment; floor division is written glued: a//b", shown until acknowledged, never again after. The earlier
  floor-or-comment Ask and its spacing/ASCII/default heuristics (fix-floor-ask, -2) are gone.
- #28 decided (supervisor warp-f3 under the welcoming policy, reported to the user): `x=ø; x.size` and `xs=[]; xs.count`
  are 0; arithmetic on ø still needs the check. Changed line: tests/welcoming/test_footguns.rs test_null_needs_a_check
  (`x=ø; x.size` → 0); tests/lists/test_empty_list_count.rs un-ignored.
- `#` directly followed by a non-space starts an expression (count): `#s`, `#a-1`, `#f(x)`, also at line start
  (fix-sugar-4; before only `#name` as a whole statement counted). Comments: `# text` (space or tab), `#!` (shebang),
  `##` (doc comment) and the directives in wasp_parser.rs HASH_DIRECTIVES: `#use`, `#include`, `#import`.
  Side effect: commented-out code written `#code` in samples (samples/raylib_*.wasp `#while(1>0){`, `#sleep(2000)`,
  samples/main.wasp `#print …`, `#fun …`, samples/lib.wasp `#fun ok(){`, tests/wasp/ffi/*/*.wasp) is now live code
  when run; test_all_samples still parses all 72 samples.
- `let x = …` / `var x = …` declare a variable in any block. USER DECISION (fix-sugar-2): `let` is immutable as
  wiki/variable.md says: `let x=1; x=2` (also `+=`, `++`, `x#i=`) → "x is let (immutable), cannot assign it again;
  fix: declare it with var or plain `x =` if it changes" (check_constants, like const). `var` stays mutable. The `let`
  style hint carries the education "in wasp `let` is immutable (unlike JS) …" (one hint, test_normalization pins one).
  fix-sugar-3: the note is `diagnostic::educate_once("let", …)`: shown once per run until the user acknowledges it,
  then remembered as `ack:let` (.wasp-answers) and never shown again.
- A bare word statement that names nothing (`x=1; foo; x`, `foo x = 3`) is `undefined variable: foo` (was silently dropped).
- `len(x)` counts like `#x` (hint `#x`); `n times [x]` fills a list; `b=[]; b.count` is 0.
- Rule (user, via warp-f3): newcomer forms are eaten only where they do not clash with a known footgun (text + number
  is the one exception). So, revised in fix-sugar-2:
  - `[x]*n` / `n*[x]` / `[1 2]*2` are refused (wiki/Footguns.md "Lists and arithmetic": Python repeats, NumPy multiplies):
    "ambiguous: Python repeats the list, NumPy multiplies each element; write `n times [x]` to repeat, or map to multiply".
  - `xs.insert(a, b)` never guesses the order (Footguns "Guessing intent"): `insert(x, at: i)` names the position;
    otherwise the kinds decide (the one Int is the position, `insert(0, "z")`, `insert("z", 1)`); two Ints
    (`insert(0, 4)`, `insert(i, v)`) are an error listing both readings, `insert(v, at: i)` / `insert(i, at: v)`.
    So the ignored wasp test form `pixel.insert(4,0)` is refused too. Positions are 0-based slots, past the end or
    negative appends. `xs.insert(v)` appends.
  - fix-sugar-3: both are Asks with fallback Error (src/diagnostic.rs). Topic `list-times` (analyzer::lower_list_times,
    a list literal times a number): answers "repeat the list" → `n times [x]`, "multiply each element" →
    `[x].map(x => x*n)`; a list variable times a number stays the plain type error. Topic `insert-order`
    (list_emitter insert_position_and_value, two Int arguments): "position first, as Python" / "value first, as wasp".
    Unanswered (tests, CI, pipes): "<question> (too ambiguous to guess); fix: <both explicit forms>". The list-times
    question starts with "type error: list * number:" so test_footguns' `[1 2 3]*2` → "type error" still holds.
  Not yet: `insert 4 at 0`, `at end/start/head`, `x is 100 times [0]` (`is` compares).


## Decided 2026-10-02
- `upto` excludes the end as wiki/range.md says (`1 upto 10` = 1..9); every `upto` hints the explicit forms
  (`..<`/`..` exclusive, `to`/`...` inclusive). tests/control/test_loop_forms.rs `upto_excludes_the_end_unlike_to` follows.

## Original questions
## Blocking finished work
1. **should_panic test** `tests/welcoming/test_footguns.rs:56-60` pins the old compiler panic on undefined variables.
   B9(2) turns all 9 panics into `Error('undefined variable: a')`. Replace with `fails_with("a+1", "undefined variable: a")`?
   (work saved in probes/b9_part2.patch)
2. **Claude-Session trailers** in 27 pushed supervisor commits (forbidden by global CLAUDE.md): rewrite history or leave?

## Language design
3. **Juxtaposition** `3x` → `3*x` (wiki/number.md specifies it; experiment: 0 regressions, makes test_implicit_multiplication pass):
   `1/2x` = `1/(2x)` (Julia) or `(1/2)x`? ordinals `2nd` stay text? `2i` / `2e` complex / constant or product? spaced `2 km` once units exist?
4. **Lists (B8)**: `size` of a list = bytes (wiki/Footguns.md decision) or element count? assigning past the end = error (DECIDED, user 2026-10-04, see P43).
   typed array declarations `x : 100 int`, `pixel:int[100]`, `640000*int`? value of a `while` loop (C++: 0, Rust test: 11)?
5. **Units**: `1 m + 1km`, `1950 ± 50`, `1900 - 2000 AD` (wiki/unit.md) — unit values as identifiers, so `3km` = `3*km`?
6. **Data as scope**: does `a-b:2 c-d:4 a-b` resolve the symbol to its key's value (2)?
7. **Unresolved calls**: `print(3)`, `square(3.0)` without a definition silently become data `(print 3)`. Error in code position?
   Survey (notes/unresolved_call_survey.md): rule `name(` without space in emitted code → `undefined function: name`, data stays data;
   0 test changes. Sub-decisions: `P(1)` type constructor, `x(3)` variable callee (error or multiplication), add `min`/`max`/`print`?
8. **print / I/O under eval**: a print builtin needs an I/O capability, which eval does not grant.
9. **Types**: `type([1 2 3])` → `list` or generic `list<int>`? Should `Data(Vec<i32>)` equal a List of ints?
10. **Polish notation for .wat/.wast**: `(module a b)` → node `module` with children, as a ParserOptions mode (test_wast).
11. **C-style declaration blocks** `double S1 = …, S2 = …` and float literals expected equal to truncated ints (test_primitive_types).
12. **`use <file>`** module import (test_sinus_wasp_import).
13. **Web host (L1)**: `$b.ok` / externref need a real webview host (C++ WebApp.cpp; Rust: wry?).

14a. **Shift operators**: `2 << 1` silently gives 0 (parsed as `<` + angle group), `8 >> 1` a cryptic error.
    Add exact-Int `<<` / `>>`, or make the parser reject them loudly?

## Test defects (can't pass unedited)
14b. DECIDED 2026-10-03 (minimal edits). test_wasm expectation defects (exact float compares 4.00001, 2.9999999999999996; `i=123.4;i` → 123; ø expected 0;
    object truthiness; text+text concat) — list in A14 slice 1 report, notes/todo_sweep_task.md A14.
14. RESOLVED (near! in tests/numbers/test_math.rs). `test_sin`: `eq!(sin(pi), 0.)` exact float compare — add tolerance or delete?
14c. RESOLVED (line commented out). `test_named_data_sections` ends with `exit(0)` (tests/wasm/test_wasm.rs:1447): kills the whole test process silently. Remove the line?
14d. CLOSED 2026-10-03 (correct for the Key model). `test_comments2` asserts `(y=0).length() == 3` (C++ model: a 3-item list); in Rust `y=0` is a Key whose length is its value's → 0. Change the expectation?
14e. DECIDED 2026-10-03 (alias of fetch). `download <url>` was never implemented (only `fetch`); add as an alias of fetch?
15. DECIDED 2026-09-30 (delete). `test_paint_wasm`: `w` never assigned, `(x-c)` is a kebab name — edit or delete?
16. C4 `"a".s() + 2` commented lines in test_string.rs — parked (user: don't care).

## Housekeeping (blocked by the destructive-git hook)
17. DONE. Delete duplicate test files `tests/test_footgun_application.rs`, `tests/test_footgun_list_index_bounds.rs` (approved; hook blocked).
18. DONE. Remove scratch worktrees probes/review_wt, probes/float_trunc_wt, probes/wt_before, probes/stage_check,
    ../warp-semicolon-survey; delete probes/review_target, probes/head_check; agent helper scripts in probes/*.py.
19. DECIDED 2026-10-03 (dropped). Old `stash@{0}: autostash` (2026-09-27, README.md + test_results.txt) — keep or drop?
20. RESOLVED for AGENTS.md (CLAUDE.md: pending P15). CLAUDE.md / AGENTS.md describe `src/wit_emitter.rs`, which does not exist.

## New questions 2026-09-30 (supervisor warp-e0), none blocking
Wiki survey: the 16 questions D1–D16 are in notes/wiki_features.md section 2 (D6 `|`/`&` as pipe, D7 lazy `:=` and
D16 overflow contradict Decided rules). Found while implementing:
21. DECIDED 2026-10-03 (see the top). `type(2.0)` is `int` (a decimal with zero fraction normalizes to an integer), `type(1.5f)` float, `type(π)` real,
    but `x=π; type(x)` still float. OK? Add `rat` as an abbreviation of `rational`?
22. DECIDED 2026-10-03 (see the top). `N times {…}` re-evaluates N each round (it reuses the for loop); trailing `while` is a plain while, not do-while
    (`i=5; i++ while i<3` never runs); `a = 2 if c` guards the whole assignment. Keep?
23. DECIDED 2026-10-03 (warn). `for 1..4 {x+=it}` binds `it`; inside a function with an implicit `it` parameter the loop shadows it. Keep?
24. OBSOLETE (upto excludes the end, decided 2026-10-02). `upto` is a global infix word (= inclusive `to`), not only inside `for`. OK?
25. DECIDED 2026-10-03 (see the top: a space never indexes). `first [10, 5]` parses as the subscript `first[10, 5]` (a space before `[` still subscripts); `first [10 5]` works.
    Should a known prefix word followed by a space make `[…]` its argument?
26. DECIDED 2026-10-03 (extend all). Library words: upper/lower are ASCII only (error otherwise), sort ints only, reverse of a text is an error.
    Extend to Unicode / all comparable values? In-place `x.upper!` (wiki D2) not added.
27. DECIDED 2026-10-03 (Unicode + be). Leftovers from earlier sessions: Unicode operators (≤ ≥ ≠ × ÷ ¬ √) and `is` for `==`: canonical or alternatives?
    `be` for `:=` (wiki/be.md) is not accepted by the parser: implement or drop?
28. DECIDED 2026-10-02 (see the top: size of ø is 0). `xs=[]; xs.size` should be 0, but `[]` and `ø` parse to the same node, and tests/welcoming/test_footguns.rs:634
    (`fails_with("x=ø; x.size", "fix: if x {")`, test_null_needs_a_check) pins the null-check error. Change or remove
    that assertion line? The fix (a small arm in check_null_use) is ready; tests/lists/test_empty_list_count.rs waits #[ignore]d.
29. OBSOLETE (checkout now only behind origin). The main checkout /Users/me/dev/angles/warp is diverged: 1 local commit 6c6e9559 (a duplicate of 8bb31618, from
    warp-3f) and behind origin/main; `git merge origin/main` refuses because your staged notes/OLD/* files collide with
    files that came in from origin. Please commit or unstage them, then resolve (the local commit can be dropped).
30. DECIDED 2026-10-03 (see the top: only `is` tests types). `is` and `==` are the same operator, so a type word on the right of `==` is now also a type test (`3 == int` → 1).
    `3 is rational` → 1 (int is a special case of rational). Keep both?
31. DECIDED 2026-10-03 (see the top). `switch n {…}` without a match reports `no case for n` (the subject as written, not its runtime value 4).
    Units: no `min`/`d` (clash with the function `min`), `s`/`h` are now unit words. OK?
32. DECIDED 2026-10-03 (dropped). Stash entries left by workers (the hook blocks `git stash drop`): `stash@{0}` "On text-concat: concat-wip-tag"
    (83a40987, content is merged) plus the older autostash entries. Drop them?
33. DECIDED 2026-10-03 (see the top: by value + educate). A block function that assigns an outer variable does not change it (`inc:={x=x+1}; do inc; x` → 1), because
    closures capture by value (Decided). Should zero-parameter blocks run in the caller's scope instead?
34. DECIDED 2026-10-03 (now, round 3). `try X else Y` catches Error values and the traps directly under `try` (index, /, %, rem); a trap deeper inside X
    (`try 1 + [1 2]#5 else 0`) still ends the program. Full catching needs a host import that runs the guarded body.
    Worth it?
35. DECIDED 2026-10-03 (see the top). `x=[1 2]; x as string` stays a loud error because tests/text/test_cast_to_string.rs (added today by a worker) pins it;
    a join-based "[1 2]" for int lists is ready, a general runtime serializer would be the real fix. Allow editing
    that assertion? Also `"x" as float` → 120 (character code, like `'A' as int` → 65): OK or loud?
36. DECIDED 2026-10-03 (see 25). Parser: `reduce [7] (a b)->a+b` and `first [10, 5]` read `word [..]` as a subscript (see 25).
37. DECIDED 2026-10-03 (enough for now). First-class functions (row 22) are compile-time specialisation (`apply(double2, 3)` → a copy `apply__double2`),
    not a funcref table: functions chosen at run time and capturing lambdas remain loud errors. A table needs one
    uniform (boxed) signature. Enough for now?
38. DECIDED 2026-10-03 (see the top: truthy). Is a list containing only ø falsy (`not ({[ø]})` → true, test_wasm_logic_on_objects)? Today a list is truthy
    when it has a first element.
39. DECIDED 2026-10-03 (== never chains). Chained comparison and `==` (user's note from a dropped 2026-09-27 stash on wiki/Footguns.md: "Intended: mathematical
    chaining as in Python, `3>2>1` → `true`. BUT NOT WITH == etc !!"). Today all comparisons share one level, so
    `1<2==2` chains (`1<2 and 2==2` → true). Should `==`/`!=` stop chaining with `<`/`>`?
40. (impl-ask, 2026-10-03; assumption taken, not blocking) D13 `1 -1` asks only when the left operand is a number
    literal (`1 -1`, `1 +1`, `[1 -1]`); `x -1`, `abs -3`, `return -1` keep subtracting/negating, so
    tests/operators/test_negated_call.rs:10 `is!("x=5;x -1", 4)` stays. Should `x -1` ask too (that test line would change)?
    Also assumed: D9's suffix-precedence Ask falls back to Error.
41. (samples, 2026-10-03; assumption taken, not blocking) Spaced construction: `V { x: 1, y: 2 }` with a space now
    constructs a declared type `V` exactly like the glued `V{x: 1 y: 2}` (D4); a word that names no declared type
    before a block stays data (`Person { name: "Alice" }`). Also: `v.x` reads the field at run time when `x` is a field
    of any declared type, even if v's type is unknown at compile time (a missing field fails "no field x").
    OK, or should the spaced form stay data?
