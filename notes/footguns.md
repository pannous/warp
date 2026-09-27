# Footguns catalogue — how it is maintained

- The file is `Footguns.md` (capital F). The checkout is case-insensitive (`core.ignorecase=true`):
  `git add footguns.md` silently stages nothing for the untracked file; use the real name.
- `probes/footguns/footguns.sh` rebuilds warp and evaluates every case in `probes/footguns/cases.warp`
  (cases separated by `---` lines) into `probes/footguns/results.txt`. Rerun it after compiler changes and diff.
- `tests/probe_footguns.rs`: passing tests = Solved entries; `#[ignore = "next"]` = NOT YET entries with a clear answer.
  When a NOT YET bug gets fixed, un-ignore its test and move the entry to Solved.
- `warp eval` and `is!` use the same `wasm_emitter::eval`, so CLI output is representative.

## Plain bugs found while probing (2026-09-26), good agent tasks
- `1/4+1/4` → 0: comparisons see 0.5, the returned sum is typed Int and truncated.
- `"abc"=="abc"`, `0==""`, `null==false`, `if "" …`, NFC vs NFD compare → compiler panic `Cannot extract numeric value`.
- `x=1;x++;x` → 1 (increment lost); `++i` parse error.
- `f := it*10; 1 + f 3` → 3 (should be 31).
- `x=[1 2 3]; x[3]` returns unevaluated program text; `x#0`, `x[-1]` → 1.
- `'héllo'#2` → 'Ã' (byte index despite wiki promising char-safe `#`).
- strings mutate through aliases: `x="ab";y=x;y#1="z";x` → 'zb'.
- `country: NO` → `country:0` (YAML Norway problem) — needs a design decision on `yes`/`no` aliases.

- `x as i64` panics inside a function body (`f(x) := (x*x) as i64`), works at top level.
- Since fcbd300b (unbounded Int) the Lean export's `BitVec 64` model rejects true laws: `law square(x) >= 0` →
  lean counterexample x=-4611686018427388111 (`test_proof_model_matches_unbounded_int`).

## Inspiration notes ("Solved elsewhere")
- Research agents write one file per topic group to `probes/footguns/inspiration/<group>.md` (brief: `BRIEF.md`),
  never Footguns.md directly; `python3 probes/footguns/merge_inspiration.py` appends them under matching headings
  (idempotent: entries that already have "Solved elsewhere:" are skipped). Diff for deleted lines after merging.
- Group `injection-time-effects` was sent to the cloud session "footgun.md (cloud)" on 2026-09-26: delivery reported
  success but the route is one-way and the session stayed idle (likely awaiting approval). Once it pushes
  `probes/footguns/inspiration/injection-time-effects.md`, rerun the merge script.

## Work area "literals" (2026-09-27, 9682281d)
- Fixed in `src/wasp_parser.rs` `parse_number`: `1e3` → exact Int 1000 (`MAX_INTEGER_EXPONENT` 4096 digits, beyond → error),
  mantissa with `.` or negative exponent → Float; `_` accepted only between two digits; `.5`/`-.5` start a number
  (`number_starts_at`), a space before `.5` makes it a new list item instead of `Op::Dot`.
- `‖x‖`: the closing bar used to be re-read as a new prefix Abs with an empty operand ("Unexpected character"); now
  `parse_norm_bars` parses a full expression up to the closing `‖` (like parentheses). `‖3‖-1` → 2 (was the list `3 1`).
- Left open: nested bars without spaces (`‖‖x‖‖`) are ambiguous; `1e`, `1_`, `2em` still parse as lists (not numbers).


- Fixed: `-2^2` → -4 (prefix `-` is always `Op::Neg` at bp 155, folded into a literal via `impl Neg for Number`);
  `not` bp (0,105): weaker than comparisons, tighter than and/or; chained comparisons (all comparisons one level 120,
  parse_expr rewrites `a<b<c` → `a<b and b<c`, the middle operand is duplicated, so side effects in it run twice);
  `=` inside `if`/`while` conditions parses as `==` (`WaspParser::equals_compares`, off again inside `{}` and after `:`);
  `x++` as a statement was emitted as data (key_emitter skipped Inc/Dec), `++i`/`--i` parse as `i++`/`i--`;
  braceless call as operand of `+ - * /` (MAX_BP_FOR_APPLICATION 130 → 151).
- `warp parse <code>` prints the parse tree as s-expressions: `(op left right)`, lists as `(items`.
- Left open, Decision needed in Footguns.md: `3 & 4 == 4` (test wants Python's bitwise `false`, wiki says `&` is `and`);
  braceless call argument extent (`1 + f 3-1` → 30 vs statement-level `f 3-1` → 20) and the recursive `fib it-1` case.
- Unrelated, noticed: `2^-2` returns the unevaluated program (negative exponent, exact-numbers area).
