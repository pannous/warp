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
