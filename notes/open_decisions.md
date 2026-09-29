# Open decisions for the user (TODO sweep 2026-09-29)

Each item blocks ignored tests or a finished-but-uncommitted change. Answer any subset; unanswered items stay parked.
Details: notes/todo_sweep_task.md (board), notes/semicolon_survey.md, notes/float_truncation_survey.md.

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
8. **print / I/O under eval**: a print builtin needs an I/O capability, which eval does not grant.
9. **Types**: `type([1 2 3])` → `list` or generic `list<int>`? Should `Data(Vec<i32>)` equal a List of ints?
10. **Polish notation for .wat/.wast**: `(module a b)` → node `module` with children, as a ParserOptions mode (test_wast).
11. **C-style declaration blocks** `double S1 = …, S2 = …` and float literals expected equal to truncated ints (test_primitive_types).
12. **`use <file>`** module import (test_sinus_wasp_import).
13. **Web host (L1)**: `$b.ok` / externref need a real webview host (C++ WebApp.cpp; Rust: wry?).

## Test defects (can't pass unedited)
14. `test_sin`: `eq!(sin(pi), 0.)` exact float compare — add tolerance or delete?
15. `test_paint_wasm`: `w` never assigned, `(x-c)` is a kebab name — edit or delete?
16. C4 `"a".s() + 2` commented lines in test_string.rs — parked (user: don't care).

## Housekeeping (blocked by the destructive-git hook)
17. Delete duplicate test files `tests/test_footgun_application.rs`, `tests/test_footgun_list_index_bounds.rs` (approved; hook blocked).
18. Remove scratch worktrees probes/review_wt, probes/float_trunc_wt, probes/wt_before, probes/stage_check,
    ../warp-semicolon-survey; delete probes/review_target, probes/head_check; agent helper scripts in probes/*.py.
19. Old `stash@{0}: autostash` (2026-09-27, README.md + test_results.txt) — keep or drop?
20. CLAUDE.md / AGENTS.md describe `src/wit_emitter.rs`, which does not exist.
