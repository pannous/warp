# Failing-tests triage 2026-11-06 (card test-failure-investigation)

Which of the red lines are really needed before more are fixed. Input: test/results.txt of b6287277e5 (03:46),
2630 ✗ lines in 774 files (results-cloud.txt 3057 ✗ is the older cloud lane, tests-web.txt 0 ✗). The earlier round,
notes/test_triage.md (2026-10-28, 4149 ✗), held the esoteric lines then; this one only counts and samples.

## Where the red lines are

- **2541 (97 %) are in files ported from the old Rust tests** (header `# from test_*.rs`), which are not spec unless a
  wiki page or decision backs the form (common.md "Tests are spec only when backed"). Only **89** are in files whose
  header names a card, decision or wiki page.
- By folder: types 540, control 539, functions 438, numbers 178, modules 171, lists 109, ffi 99, welcoming 92,
  parser 90, operators 80, text 77, wasm 70, programs 69, node 32, rest 19.
- By outcome: 1655 refused "outside the core … (no owner)", 489 other errors, 486 wrong values.
- Ported lines by theme (file names): types/classes 353, wasm/modules/ffi/packages 225, foreign syntax (Rust, Python,
  Ruby, JS spellings) 219, functions/lambdas/scope 164, lists/maps/text 149, numbers/operators 142, time 135,
  welcoming/footguns 112, reflection/meta/quoting 107, control/parser 83, events/tasks 47, other 801.

## Classes (a random sample of 60 lines, judged by hand, then scaled)

| class | sample | ≈ of 2630 | examples |
|---|---|---|---|
| a real gap or bug in a form users write | 20 (33 %) | ~870 | nested functions capturing an outer parameter (`outer(x) := { mid(y) := { deep(z) := k + x + y + z … } }`), `[[1,2],[3]].map{ it.map{ it*10 } }`, `company: {employees: […]}` at top level, `try xs#2 / 2.0 else -1` traps (cast failure), `x=1; if x=2 {3} else {4}` prints garbage |
| b esoteric, old-Rust or desk-invented | 30 (50 %) | ~1300 | `undo x`, `i32::MAX`, `let mut m = HashMap::new()`, `def h … end`, `pixel:int[100]`, `match alle=f(3) {…}`, `now ≈ January`, `record point{…}`, `{x+y}(x=1, y=2)`, `sqrt.signature`, `surface=height*width int`, `x = {1+2}; x!` |
| c stale expectation (a later decision says otherwise) | 9 (15 %) | ~400 | `type(now)` "instant" (now class Instant), `type(3px)` "pixels", `type(x) == nat` (decision type-of-small-local), `storage[…]` without `use storage`, `str(a)` spacing "T{n:'x'}", `dir(m)` now lists the map words, sample paths that moved |
| d duplicate | — | 29 | the same code line in two files |

## Look first: wrong values that read as plausible (the worst bugs)

- `square := it*it; numbers=[1 2 3 4]; square numbers |> filter(x => x > 5) |> sum` → 0 (expected 25)
- `def add(left:int) -> int: left; add.params` → ["xs", 'x'] (another function's parameters)
- `t = 2024-07-01T10:00; str(t in "Europe/Berlin")` → "no"
- `s = {fs: [x => x * 2]}; s.fs#1(4)` → index out of range 4 (reads `#(1(4))`)
- `names: texts = ["hi"]; ys = names; ys.add(1)` → ["hi", 1], no refusal of an int in a texts list
- `use c; isdigit(55) > 0` → "operator >= on int and char"

## Recommendation (for the user, via the Interviewer)

1. Hold class b in bulk as notes/test_triage.md did: comment the line out with `# triage-held 2026-11-06: <reason> :: …`,
   file by file, starting with the foreign-syntax files (test_rust_paths, test_rust_macros, test_python_ruby_definitions,
   test_foreign_spellings, test_operator_spellings) and the reflection/meta files. About half the red lines go.
2. Correct class c in one commit per decision, each naming its decision key (as the 10-28 table did).
3. Card the class a groups (nested closures, nested `it`, top-level key declarations, try over a trap) and the six
   wrong values above first.
4. The 89 spec-backed red lines (time patterns, regex malformed/edges, Allman braces, text signatures) are all class a
   or known decisions (regex-error-from-os keeps test_regex_malformed red on purpose).
