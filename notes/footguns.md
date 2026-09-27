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
- Fixed: the Lean exporter now models Warp Int as unbounded `Int`; `law square(x) >= 0` is proved consistently with the runtime
  (`test_proof_model_matches_unbounded_int`). Explicit `as i64` proof terms remain future work.

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

## Work area "variance" (2026-09-27)
- Decided (recorded under Footguns.md NOT YET → Variance): variance is inferred, not annotated; immutable collections
  covariant, collections mutated through a widened view invariant (type error, never a runtime store check).
- No subtyping exists yet, so nothing to implement. Added `test_no_array_store_exception` (ignored, unverified).
- Left open: could not build or run tests in this session (crates.io blocked by the network policy, `vendor/` absent),
  so the entry stays in NOT YET until the test is run and passes; then move it to Solved.

## Work area "dates-time" (2026-09-27)
- Decided (recorded under Footguns.md NOT YET → Dates and time zones): distinct `date`, `local time`, `instant`, `zoned time`;
  RFC 3339 / RFC 9557 literals only; 1-based months; no implicit zone (`now` is an instant, `in "Zone"` places it);
  `+ 1 month` rejects overflow, `add(…, overflow: clamp)` clamps.
- Spec tests added (`#[ignore = "next"]`): test_months_are_one_based, test_calendar_overflow_is_explicit,
  test_no_implicit_time_zone, test_date_and_time_types_are_distinct.
- Left open: the implementation. The session could not build: index.crates.io is denied by the environment's network
  policy and there is no vendor/ or ~/.cargo cache, so no compiler code was pushed. Open questions for the implementer:
  date literal lexing in `parse_number` (`2024-01-31` currently parses as `2024-1-31`), duration units (`1 month`, `24 hours`),
  and where the tz database lives (host import vs embedded table).

## strings-equality work area (2026-09-27)
- Fixed: `==`/`!=`/`is` compare by value at runtime (`src/wasm_emitter/equality.rs`: `values_equal`, used when either
  operand is text, list, ø, an indexed element or a ref-typed variable); `is` parses as `==`; conditions on text/list/ø
  use `is_truthy` (same rule as `Node::is_falsy`) instead of panicking; parser normalizes source to NFC; `#` on text
  decodes UTF-8 code points (bounds checked); `{a:1 a:2}` is a parse error (curly objects only; code may redefine).
- Found on the way: `matches_keyword` treated `_` as a word boundary, so `is_prime` would have lexed as `is` + `_prime`.
- Left open: truthiness of empty values (Decision needed in Footguns.md, recommendation: only bool is a condition);
  grapheme-level `#`/`size`/`length`; `x#i='é'` still writes a single byte (string_set_char_at); duplicate-key error has no span.
- test.sh parsing: WASI tests print to stdout, which sometimes interleaves with `test … ok` lines, so e.g.
  `test_wasi_putf` can go missing from test_results.txt although it passes.

## Work area "data-formats" (2026-09-27)
- Fixed: Norway problem, numbers-that-are-not-numbers, data that executes (tests test_norway_problem_in_data,
  test_data_keeps_number_literals, test_data_does_not_execute).
- Mechanism: `ParserOptions::data()` / `parse_data` / CLI `warp data <file>`; in data mode only `true`/`false`/`null`
  are word literals, and lossy number literals keep their source text as Meta key `literal` (serialize prints it).
  `eval_untrusted` refuses any program whose EffectReport resolves an external (empty capability set).
- Decided: code keeps `yes`/`no` aliases (so `warp eval 'country: NO'` is still `country:0`); cases.warp shows code-mode output.
- Side fix: space-separated lists serialized with double spaces (`[a  b]`).
- Left open: code-mode `NO`; literal preservation only in data mode (code-mode Meta would ride into the analyzer);
  `warp data` has no JSON output option.

## Work area "closures-defaults" (2026-09-27)
- Fixed: closures capture outer variables by value at the definition point (`test_closures_capture_values`,
  `test_closures_in_loop_capture_each_iteration`). `analyzer::captured_variables` finds free variables bound in main;
  each gets a mutable WASM global per function, set where the definition appears (`emit_closure_capture`), and the body
  reads it through `user_globals` while it is compiled. Ref-kind (list/text) globals are nullable `ref $Node`.
- Fixed: default arguments are evaluated at every call (`test_default_argument_is_fresh_per_call`). Parameters take the
  kind of their default (`analyzer::param_kind`), so list/text/float defaults no longer panic in numeric emission.
- Decided: capture by value (DESIGN.md immutable bindings) over by-reference (wiki/assignment.md `z := y*y` sketch).
- Left open: functions are not first-class, so "a list of closures from a loop" is not expressible; a name is its latest
  definition. Functions still only see main-level variables (no nested functions capturing a function's locals);
  a call before the definition reads zero/null globals. `a.add(1)` on a list does not mutate it (lists area).

## Work area "truthiness-null-errors" (2026-09-27)
- Fixed: declared types enforced (`x:int=5;x="five"` → Error with position and fix-it; `x:float=5` widens), null-use check
  (`x=ø; x+1` → Error, narrowed inside `if x {…}`), `a and b or c` lint (stderr warning + `analyzer::lint`), float and
  >32-bit conditions (`if 0.5`, `if 2^32` were falsy), parser now consumes `ø` in `parse_atom` (`if ø {…}` was garbage).
- Mechanism: `src/diagnostic.rs` (`Diagnostic::at(node,…).fix(…)` → "msg at line:col; fix: …"), `analyzer::diagnose` runs
  before emission in `eval_parsed`, `analyzer::lower_declarations` turns `x:T=v` into a typed local (type kept as Meta on x).
  Truthiness rides on the strings-equality agent's `emit_condition`/`is_truthy`; I added the float/i64 != 0 branch.
- Decided: empty values falsy, uniformly (Footguns.md → Empty values, with alternatives).
- Left open: `T?` syntax (`x:int?=ø` parse error) and locals first assigned `ø` panic in the emitter
  (`test_optional_local_runs`, ignored); declared-type check only sees literal values; other emitter panics
  (`Undefined variable`, pinned as should_panic by an existing test); runtime traps have no span; `failed_run` still returns
  the parsed program for link/instantiation failures; `x=0.5;x=0` → WASM validation error (int stored into float local).

## exact-numbers work area (2026-09-27)
- Fixed: `1/4+1/4` → 0 (analyzer::arithmetic_kind). Exact rationals by default (src/wasm_emitter/exact.rs): an Int handle
  may point at a `$Ratio{num,den}` of Int payloads in the same heap as `$BigInt`; integer fast paths unchanged, only the
  slow paths dispatch on `is_ratio`. `0.1+0.2==0.3`, `1/3*3==1`, `2^-2` → 0.25 (used to trap), `1/0` → ∞, `0/0` → NaN (den 0).
- Decimal literals with ≤ 15 significant digits are exact (`Number::is_exact_decimal`); π, sqrt, FFI floats stay f64.
- Left open (Decision needed in Footguns.md): negative modulo (existing tests pin C semantics), rounding-mode naming,
  bool kind, and `test_float_plus_int_type_upgrading` which requires a Float where the exact result is now a Quotient.
- Other gaps: ratios with parts beyond i64 read back as f64 (Quotient is (i64,i64)); `3 == 3.0000000000000001` still true
  (literal parsed to f64 first); `law`/Lean export still treats `/` as unsupported ("yields a Float" comment in law/lean.rs is stale).
## Work area "injection" (2026-09-27)
- Fixed (verified via CI on claude/footguns-injection): `src/injection.rs` lowers `sql "…"` / `sh "…"` before effects and
  emission. SQL holes (`$name`, `${expr}`, `$$` = `$`) become `?` parameters, the value is `(query param…)`; shell templates
  become an argv `(program arg…)` where a hole is one whole argument. Diagnostics: non-literal template, hole inside SQL
  quotes, shell operators/quotes, hole glued into a word, `execute`/`exec` of anything but a template of its language.
  `execute` (Capability::Sql) and `exec` (Capability::Process) are trusted IO externals; `eval_parsed` refuses modules
  needing a capability outside `Capability::GRANTED_BY_EVAL`.
- Decided (recorded in Footguns.md → Solved → SQL and shell injection): tag-prefixed literal templates, `?` placeholders,
  argv without a shell, runners gated by capability.
- Left open: no host grants sql/process (nothing really runs); template language is tracked per variable by the lowering
  pass, not the type system; no typed pipes/redirection. Braceless `execute sql "…"` parses left-nested
  (`((execute sql) "…")`), the lowering flattens it.
## Work area "conversions-bounds" (2026-09-27)
- Fixed: index bounds (`index_out_of_range` trap → error value; eval turns any wasm trap into `Error`, other failures
  still fall back to the program text); `int("12a")`/`float("1.5x")` → `Error('invalid number')` (`number_in_text`);
  value semantics for `x#i=v` (`node_with_at`: lists copy the prefix and share the tail, texts copied into memory grown
  past the string table, so deduplicated literals are safe); `x#i op= v`; `"5"+3`, `[1 2 3]*2` → compile-time type error
  (`arithmetic_kind` → `Kind::Error`, `emitter.type_error()`); `list + list` → `list_concat`; `const` single assignment
  (`check_constants`); `x.add(v)` → `x = x + [v]`; `[4]` stays a list.
- Mechanism worth knowing: `emit_call` of a runtime function the analyzer did not require records it and emission reruns
  once with it required (types are only known during emission). Recursive runtime functions compute their own index
  as `import_count + code_count` before `runtime_function` registers them.
- Left open: `x#i='é'` writes one byte (reads are char-safe); errors carry no span; `#-1` from the end not implemented;
  `()` is ø so `a=();a.add(1)` hits the null-use diagnostic; `pixel + 4` (append scalar) is a type error; text
  concatenation `"a"+"b"` not implemented (type error); no uniqueness analysis, every element write copies; runtime
  texts are never freed; `::=` does not parse. `while i<=3 {…}` with braces returns a strange list (seen while probing,
  not investigated). `test_float_plus_int_type_upgrading` fails since 2b74cd0f (exact rationals), not this area.
