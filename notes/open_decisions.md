# Open decisions for the user (TODO sweep 2026-09-29)

Each item blocks ignored tests or a finished-but-uncommitted change. Answer any subset; unanswered items stay parked.
Details: notes/todo_sweep_task.md (board), notes/semicolon_survey.md, notes/float_truncation_survey.md.

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

## Decided 2026-10-03 (user, multiple choice; not implemented yet)
- D7 / #33 closures: "By value + educate". Blocks keep capturing by value (`x=1; inc:={x=x+1}; do inc; x` → 1); a block
  that assigns an outer variable gets a hint: use `global x` or return the value. The wiki's lazy `:=` examples get updated.
- D3 `[1 2 3]+4`: "Ask". Like `[x]*n`: append or add to each element? Fallback Error. `.+` is element-wise,
  `xs + [4]` concatenates.
- D12 `a=1 2 3` / `a=1,2,3`: user "idk", stays parked.
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
- #30 type tests: "Only `is` tests types". `3 is int` → 1, `3 is rational` → 1; `3 == int` educates toward `is`.

## Decided 2026-10-02 (relayed by warp-f3): eat newcomer syntax, compile its intent, hint the wasp form
- Text + number concatenates, the number in its text form (`"F:" + 13` → `"F:13"`, `"5"+3` → `"53"`, JS/Kotlin), with a
  hint `str(13)`. Reversed: the 2026-09 "no implicit conversion" rule (DESIGN.md "Dangerous implicitness", wiki/Footguns.md
  "String + number"), which was there because one-character strings used to add as code points (`"5"+3` → 56). `"5"*3`
  stays a type error. Flipped tests: test_text_concat::text_plus_number_stays_an_error,
  probe_footguns::test_text_plus_number_is_a_type_error, test_text_bytes::text_plus_number_stays_a_type_error.
  Not yet: a runtime ratio (`y=2.5; "x"+y`, also `y as string`) prints garbage: list_join has no text form for ratios.
- `//` glued to its operand (`7//2`, `x//=2`) is Python floor division, lowered to `(a - a%b)/b` (`%` is Euclidean: exact
  for a positive divisor, `-7//-2` gives 4 where Python gives 3); `x // note` (space before) stays a comment.
  `a div b` is the same floor division. An index that divides (`xs[n/2]`) traps `index must be an integer` unless the
  division is exact, with the hint `n//2`.
- Spaced `a // b` — USER DECISION 2026-10-03 (fix-floor-ask-3): "just make it a warning to the user that it's read
  as a comment, don't do heuristics". A spaced `//` is always a comment, only glued `a//b` / `x//=b` divide. A `//`
  comment after code on its line educates once (diagnostic::educate_once, topic `slash-comment`): "`// …` after code
  is a comment; floor division is written glued: a//b", shown until acknowledged, never again after. The earlier
  floor-or-comment Ask and its spacing/ASCII/default heuristics (fix-floor-ask, -2) are gone.
- #28 decided (supervisor warp-f3 under the welcoming policy, reported to the user): `x=ø; x.size` and `xs=[]; xs.count`
  are 0; arithmetic on ø still needs the check. Changed line: tests/probe_footguns.rs test_null_needs_a_check
  (`x=ø; x.size` → 0); tests/test_empty_list_count.rs un-ignored.
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
    question starts with "type error: list * number:" so probe_footguns' `[1 2 3]*2` → "type error" still holds.
  Not yet: `insert 4 at 0`, `at end/start/head`, `x is 100 times [0]` (`is` compares).


## Decided 2026-10-02
- `upto` excludes the end as wiki/range.md says (`1 upto 10` = 1..9); every `upto` hints the explicit forms
  (`..<`/`..` exclusive, `to`/`...` inclusive). tests/test_loop_forms.rs `upto_excludes_the_end_unlike_to` follows.

## Original questions
## Blocking finished work
1. **should_panic test** `tests/probe_footguns.rs:56-60` pins the old compiler panic on undefined variables.
   B9(2) turns all 9 panics into `Error('undefined variable: a')`. Replace with `fails_with("a+1", "undefined variable: a")`?
   (work saved in probes/b9_part2.patch)
2. **Claude-Session trailers** in 27 pushed supervisor commits (forbidden by global CLAUDE.md): rewrite history or leave?

## Language design
3. **Juxtaposition** `3x` → `3*x` (wiki/number.md specifies it; experiment: 0 regressions, makes test_implicit_multiplication pass):
   `1/2x` = `1/(2x)` (Julia) or `(1/2)x`? ordinals `2nd` stay text? `2i` / `2e` complex / constant or product? spaced `2 km` once units exist?
4. **Lists (B8)**: `size` of a list = bytes (wiki/Footguns.md decision) or element count? assigning past the end = error or grow?
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
14b. test_wasm expectation defects (exact float compares 4.00001, 2.9999999999999996; `i=123.4;i` → 123; ø expected 0;
    object truthiness; text+text concat) — list in A14 slice 1 report, notes/todo_sweep_task.md A14.
14. `test_sin`: `eq!(sin(pi), 0.)` exact float compare — add tolerance or delete?
14c. `test_named_data_sections` ends with `exit(0)` (tests/test_wasm.rs:1447): kills the whole test process silently. Remove the line?
14d. `test_comments2` asserts `(y=0).length() == 3` (C++ model: a 3-item list); in Rust `y=0` is a Key whose length is its value's → 0. Change the expectation?
14e. `download <url>` was never implemented (only `fetch`); add as an alias of fetch?
15. `test_paint_wasm`: `w` never assigned, `(x-c)` is a kebab name — edit or delete?
16. C4 `"a".s() + 2` commented lines in test_string.rs — parked (user: don't care).

## Housekeeping (blocked by the destructive-git hook)
17. Delete duplicate test files `tests/test_footgun_application.rs`, `tests/test_footgun_list_index_bounds.rs` (approved; hook blocked).
18. Remove scratch worktrees probes/review_wt, probes/float_trunc_wt, probes/wt_before, probes/stage_check,
    ../warp-semicolon-survey; delete probes/review_target, probes/head_check; agent helper scripts in probes/*.py.
19. Old `stash@{0}: autostash` (2026-09-27, README.md + test_results.txt) — keep or drop?
20. CLAUDE.md / AGENTS.md describe `src/wit_emitter.rs`, which does not exist.

## New questions 2026-09-30 (supervisor warp-e0), none blocking
Wiki survey: the 16 questions D1–D16 are in notes/wiki_features.md section 2 (D6 `|`/`&` as pipe, D7 lazy `:=` and
D16 overflow contradict Decided rules). Found while implementing:
21. `type(2.0)` is `int` (a decimal with zero fraction normalizes to an integer), `type(1.5f)` float, `type(π)` real,
    but `x=π; type(x)` still float. OK? Add `rat` as an abbreviation of `rational`?
22. `N times {…}` re-evaluates N each round (it reuses the for loop); trailing `while` is a plain while, not do-while
    (`i=5; i++ while i<3` never runs); `a = 2 if c` guards the whole assignment. Keep?
23. `for 1..4 {x+=it}` binds `it`; inside a function with an implicit `it` parameter the loop shadows it. Keep?
24. `upto` is a global infix word (= inclusive `to`), not only inside `for`. OK?
25. `first [10, 5]` parses as the subscript `first[10, 5]` (a space before `[` still subscripts); `first [10 5]` works.
    Should a known prefix word followed by a space make `[…]` its argument?
26. Library words: upper/lower are ASCII only (error otherwise), sort ints only, reverse of a text is an error.
    Extend to Unicode / all comparable values? In-place `x.upper!` (wiki D2) not added.
27. Leftovers from earlier sessions: Unicode operators (≤ ≥ ≠ × ÷ ¬ √) and `is` for `==`: canonical or alternatives?
    `be` for `:=` (wiki/be.md) is not accepted by the parser: implement or drop?
28. DECIDED 2026-10-02 (see the top: size of ø is 0). `xs=[]; xs.size` should be 0, but `[]` and `ø` parse to the same node, and tests/probe_footguns.rs:634
    (`fails_with("x=ø; x.size", "fix: if x {")`, test_null_needs_a_check) pins the null-check error. Change or remove
    that assertion line? The fix (a small arm in check_null_use) is ready; tests/test_empty_list_count.rs waits #[ignore]d.
29. The main checkout /Users/me/dev/angles/warp is diverged: 1 local commit 6c6e9559 (a duplicate of 8bb31618, from
    warp-3f) and behind origin/main; `git merge origin/main` refuses because your staged notes/OLD/* files collide with
    files that came in from origin. Please commit or unstage them, then resolve (the local commit can be dropped).
30. DECIDED 2026-10-03 (see the top: only `is` tests types). `is` and `==` are the same operator, so a type word on the right of `==` is now also a type test (`3 == int` → 1).
    `3 is rational` → 1 (int is a special case of rational). Keep both?
31. `switch n {…}` without a match reports `no case for n` (the subject as written, not its runtime value 4).
    Units: no `min`/`d` (clash with the function `min`), `s`/`h` are now unit words. OK?
32. Stash entries left by workers (the hook blocks `git stash drop`): `stash@{0}` "On text-concat: concat-wip-tag"
    (83a40987, content is merged) plus the older autostash entries. Drop them?
33. DECIDED 2026-10-03 (see the top: by value + educate). A block function that assigns an outer variable does not change it (`inc:={x=x+1}; do inc; x` → 1), because
    closures capture by value (Decided). Should zero-parameter blocks run in the caller's scope instead?
34. `try X else Y` catches Error values and the traps directly under `try` (index, /, %, rem); a trap deeper inside X
    (`try 1 + [1 2]#5 else 0`) still ends the program. Full catching needs a host import that runs the guarded body.
    Worth it?
35. `x=[1 2]; x as string` stays a loud error because tests/test_cast_to_string.rs (added today by a worker) pins it;
    a join-based "[1 2]" for int lists is ready, a general runtime serializer would be the real fix. Allow editing
    that assertion? Also `"x" as float` → 120 (character code, like `'A' as int` → 65): OK or loud?
36. Parser: `reduce [7] (a b)->a+b` and `first [10, 5]` read `word [..]` as a subscript (see 25).
37. First-class functions (row 22) are compile-time specialisation (`apply(double2, 3)` → a copy `apply__double2`),
    not a funcref table: functions chosen at run time and capturing lambdas remain loud errors. A table needs one
    uniform (boxed) signature. Enough for now?
38. DECIDED 2026-10-03 (see the top: truthy). Is a list containing only ø falsy (`not ({[ø]})` → true, test_wasm_logic_on_objects)? Today a list is truthy
    when it has a first element.
