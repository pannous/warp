# Footguns catalogue — how it is maintained

- The file is `Footguns.md` (capital F). The checkout is case-insensitive (`core.ignorecase=true`):
  `git add footguns.md` silently stages nothing for the untracked file; use the real name.
- `probes/footguns/footguns.sh` rebuilds warp and evaluates every case in `probes/footguns/cases.warp`
  (cases separated by `---` lines) into `probes/footguns/results.txt`. Rerun it after compiler changes and diff.
- `tests/probe_footguns.rs`: passing tests = Solved entries; `#[ignore = "next"]` = NOT YET entries with a clear answer.
  When a NOT YET bug gets fixed, un-ignore its test and move the entry to Solved.
- `warp eval` and `is!` use the same `wasm_emitter::eval`, so CLI output is representative.

## Plain bugs found while probing (2026-09-26), all fixed — each entry names its regression test
- `1/4+1/4` → 0: fixed, pinned by `test_sum_of_quotients_is_not_truncated` (tests/probe_footguns.rs).
- `"abc"=="abc"`, `0==""`, `null==false`, `if "" …`, NFC vs NFD compare: fixed, pinned by `test_string_equality_is_by_value`,
  `test_equality_across_kinds_is_structural`, `test_empty_values_are_falsy`, `test_unicode_normalization` (tests/probe_footguns.rs).
- `x=1;x++;x` and `++i`: fixed, pinned by `test_increment_changes_variable`, `test_prefix_increment`.
- Already fixed before the 2026-09-29 sweep, now in wiki/Footguns.md: `f := it*10; 1 + f 3` → 31 (`test_braceless_call_as_operand`,
  9d54997d; tests/numbers/test_angle.rs covers `1+square 2+3`) and list index errors: `x[3]`, `x#0`, `x[-1]` → `Error('index out of range')`
  (`test_index_out_of_bounds_is_an_error`, `test_negative_index_is_an_error`).
- `'héllo'#2`: fixed, pinned by `test_character_indexing_is_unicode_safe`.
- String mutation through aliases: fixed, pinned by `test_mutation_through_alias_is_not_visible`.
- `country: NO` → `country:0` is the decided behaviour for `warp eval` (data keeps the symbol: `test_norway_problem_in_data`); see wiki/Footguns.md "The Norway problem".

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

## Work area "dates-time", implementation (2026-09-27)
- Fixed: all four date specs pass (plus `test_date_literal_needs_strict_form`), validated on CI (branches claude/footguns-dates, claude/footguns-dates-rebased;
  no local build: crates.io is blocked in the cloud sandbox; `src/time/calendar.rs` has no dependencies and was unit-checked with plain rustc).
- Answers to the open questions: date literals are lexed at the top of `parse_number` (strict RFC 3339 / RFC 9557 shape only,
  4-2-2 digits, word boundary after it, so `2024-1-31` stays arithmetic); durations are lexed after an integer followed by a
  unit word (`1 month`, `24 hours`), months/days counted, hours and below exact; the tz database is an embedded rule table
  (`calendar::ZONES`, current EU/US DST rules), a host import of the IANA database is future work.
- Programs that mention a date literal, duration, `now` or `date(…)` are evaluated at compile time by `time::answer`
  (called first in `eval_parsed`): assignments, `;`/newline sequences, `.field`, `+ - == < …`, `in`, `date()`, `add()`.
- Left open: no WASM runtime representation of dates (functions/loops over dates are an explicit "not supported yet" error);
  `now` is the compiler's clock and is not an effect yet; no historical or southern-hemisphere zone rules; the parse-only
  data path (`data_mode`) does not lex dates or durations yet (it round-trips source text and has no date serialization).
- Noticed: `Node == bool` ignores `Node::True`/`Node::False` (`PartialEq<bool>` in src/node.rs has no arm for them), so
  `is!(…, true)` only works for Int 1/0 results; the time evaluator returns Int 1/0 like the rest of eval.

## Work area "number decisions" (2026-09-28)
- Fixed/decided: Negative modulo (`%` stays truncating, pinned by tests/numbers/test_unbounded_int.rs; new infix `mod` is floored,
  lowered in the parser to `(a % b + b) % b`); Rounding mode (`round` = half even, `round_half_even`, `round_half_up`
  in list_emitter, half up = floor(x) + (frac ≥ ½) via a scratch local holding the f64 bits); Booleans are integers
  (analyzer `check_boolean_arithmetic` in `diagnose`: arithmetic on true/false, comparisons or prefix `not` is an error with
  an `int(…)` fix-it). Validated on CI (branch claude/footguns-numbers2; no local build, crates.io blocked).
- Left open: a distinct `Kind::Bool` (True/False are Int 1/0 at the Node boundary, pinned by tests/node/test_node_operators.rs);
  the boolean check does not follow variables; `false == 0` still true; `mod` evaluates its divisor three times;
  rounding is via f64, no half-away-from-zero; no `mod=`.

## Graphemes agent (bytes vs graphemes, multi-byte index assignment)
- Fixed: `#`, `count`, `length` on text count grapheme clusters (`'👍🏽'#1` → "👍🏽", `count "🇩🇪🇫🇷"` → 2); `size` counts
  bytes; `.bytes`, `.chars`, `.graphemes` name the unit; `x#i='é'` splices the UTF-8 of the character over the grapheme.
  One table (`strings::GRAPHEME_EXTEND`) drives the Rust `grapheme_clusters` and the emitted `grapheme_end` runtime; no new crate.
- Decided: graphemes by default, `size` = bytes (wiki/string.md, wiki/char.md, wiki/ABI.md); alternatives recorded in Footguns.md.
- Left open: `[]` as byte access (it is `#` shifted by one), typed iteration `for byte in text`, `count bytes of x` phrasing,
  full UAX #29 (Hangul jamo, Indic conjuncts, Prepend), assigning a multi-code-point grapheme (`x#1='👍🏽'`).

## Work area "optionals and errors" (2026-09-28)
- Fixed (verified via CI on claude/footguns-errors): a local first bound to `ø` or declared `T?` is held as a Node and read
  as a number through `get_int_value` (`x=ø; if x {x+1} else {2}` → 2); `T?` parses as a suffix on a type name
  (`x:int?=ø`); `x:int=ø` is a declared-type error with fix-it; `a=();a.add(1)` → `[1]` (the null check lets append and
  list `+` through, since ø is the empty list). Emitter "Cannot extract numeric/float value" panics are now `Error` values
  with a source position (`emit_not_a_number`); `failed_run` returns an `Error` for link/instantiation failures instead of
  the parsed program. `x=0.5;x=0` already worked after exact decimals; pinned.
- Decided: `()` stays ø and ø is the empty list; `T?` suffix syntax (Footguns.md → Null: optional types).
- Left open: `Undefined variable` panic (pinned by an existing should_panic test), spans on runtime traps (needs a
  code-offset → source map), `x!` unwrap, typed null, optional floats (read through the Int path), literal `ø+[1]` is
  still a type error (infer_type(ø) defaults to Int).

## Work area "syntax decisions" (2026-09-28)
- Decided and fixed: `&`/`|` vs comparison. `&`/`|` stay logical and/or (wiki/&.md); the parser turns a single-char `&`/`|`
  next to an ungrouped comparison into an error value with both groupings as fix-it (`logic_mixed_with_comparison` in
  src/wasp_parser.rs). `and`/`or`/`&&`/`||` are not affected. Alternatives recorded in Footguns.md.
- Decided and fixed: braceless calls. The argument takes arithmetic and stops at ranges/comparisons (ARGUMENT_BP 140), in operand
  position and, for functions of the implicit `it`, at statement level: `1 + f 3-1` → 21, `f 3-1 > 15` → true. This matches the
  legacy wasp tests (`3 + id 3+3` → 9, test_wasm.rs, ignored). A braceless call inside a braceless argument is rejected by
  `check_ambiguous_calls` (analyzer `diagnose`) with both readings (wiki/precedence.md), which turns Bad.md's
  `fib it-1 + fib it-2` from `Undefined variable: it` into a fix-it. Functions defined with `:=` take identifier arguments in any
  position (`it * fac it-1`). Validated on CI (branch claude/footguns-syntax2; no local build, crates.io blocked).
- Left open: `fac := it<=1 ? …` fails to parse (`Unexpected character '='`), `it<2` works; probably `it<` read as a generic,
  not investigated. `print 1 + f 3` is also reported as ambiguous. Multi-parameter functions at statement level keep the
  list form `add 3 4`, so `add 3 4 > 5` still passes the comparison as the last argument.

## Work area "Euclidean modulo" (2026-09-28)
- Decided by the maintainer: `%` is Euclidean as in mathematics (`0 ≤ r < |b|`): `-7 % 3` → 2, `7 % -3` → 1. Built on the
  "number decisions" run (floored `mod`, merged in 1bc4d2b8): `mod` is now a plain alias of `%` (no parser rewrite, divisor
  evaluated once), `rem` (new Op::Rem) is the truncated remainder. Fixnum fast path adds `|b|` to a negative `i64.rem_s`,
  the BigInt/ratio slow path is `exact_mod`; integer `x /= y` is the matching quotient `(x - x % y) / y`, `x %= y` follows `%`.
  Lean exports `%` as `Int.emod`, `rem` as `Int.tmod`. The analyzer lints `%` with a negative literal or negated operand.
- Tests changed with maintainer authorization: test_unbounded_int.rs (`-7 % 3` → 2, big `% 1000` → 110) and
  probe_footguns.rs test_modulo_and_remainder_are_both_named. Validated on CI (branch claude/footguns-euclid).
- Left open: float `%=` falls back to multiplication in the compound-assign float path (pre-existing, `_ => F64Mul`);
  float `%` goes through the integer path. No `rem=`.

## Equality does not chain (user decision 2026-09-28)
- Only `< <= > >=` chain (`3>2>1` → true). `==`/`!=` bind weaker (115 vs 120) and compare results: `1<2 == 2<3` → true.
- An ungrouped `a==b==c` / `a==b!=c` is a diagnostic with fix-it (as in Rust), grouped `(1==1)==1` is allowed.
- Footguns.md "Chained comparison" still says `1<2==2` chains: update it when the user's edit of that file is committed.

## Work area "fast and lawful floats" (2026-09-28)
- Decided by the maintainer: literals stay exact, IEEE floats are opt-in. One alias table, `type_kinds::canonical_type_name`:
  `real`→`exact` (Kind::Int, which may hold a ratio), `fast`/`f64`/`double`→`float`; `builtin_type_kind`, casts and the
  Lean kind map all go through it. `T x=v` (T a number type) lowers to `x:T=v` in `lower_declarations`, both parse shapes
  (`(T x)=v` and the statement pair `T`, `x=v`); `double(x) := …` stays a function definition.
- Fixed: `as` now binds (152,153), tighter than `* +`, weaker than unary minus; `v as T` works as an operand
  (`infer_type`, `emit_numeric_value`, `emit_float_value`): `0.1 as float + 0.2 as float` → 0.30000000000000004,
  `x=3.3 as float; x` → 3.3. Behaviour change: `2 * 1.5 as int` is now 2, `'2.1' as real` is the ratio 21/10.
- Tests: test_addition_is_associative_by_default, test_fast_floats_are_not_associative, test_as_binds_tighter_than_arithmetic,
  test_number_declaration_spellings_agree. Validated on CI (branch claude/float-decl).
- Left open: `as exact` of a runtime f64 is a type error (no f64→ratio conversion yet); `'2.5' as float` inside arithmetic
  is not a number (text casts only at top level); `int x=…`/`string x=…` prefix declarations are not lowered (only number
  types); `real` excludes irrationals until a symbolic/constructive real exists.

## `as` is loose, typed literals are tight (user decision 2026-09-28)
- The float-syntax run had put `as` at bp 152 (tighter than `*`, like Rust/Kotlin) on my instruction "tighter than +":
  `2 * 1.5 as int` → 2. Now `as` is (125, TYPE_OPERAND_BP=250): it converts the whole arithmetic expression to its left
  (C#, TypeScript), `2 * 1.5 as int` → 3, and its target type is one atom. An ungrouped mix is linted:
  fix `(2*1.5) as int or 2 * 1.5:int`.
- Tight conversion is written on the literal (`literal_type_suffix` in wasp_parser.rs, code only, not data mode):
  `0.1:float`, `1.5:int` and the C/Java/C# suffixes `0.1f`/`F`, `0.1d`/`D` (double) → float, `0.1l`/`L` (long double) → exact.
- Word operators serialize with spaces (`0.1 as float`, was `0.1asfloat`).
- /usr/local/bin/warp is a symlink to target/debug/warp: every cargo build/test updates it.

## Work area "exact reals" (2026-09-28)
User decision 2026-09-28: exact numbers beyond Q with π, ℯ and square/cube roots now; ε/ω (hyperreals) later.
- One mechanism: `src/extensions/reals.rs` `Exact` = sparse polynomial, rational (BigInt) coefficients × monomials over
  `Generator {Pi, Euler, Imaginary, SquareRoot(r), CubeRoot(r)}`, normal form by construction (√a·√b = √(ab) reduced,
  ⅈ² = -1, π/ℯ any integer exponent). Mirrors ~/dev/script/lean4/hyper `HyperGeneral`. `Real = Exact | Approx(f64)`,
  carried as `Number::Real(&'static Real)` (leaked like BigInt to keep Number Copy). Rationals never take this form.
- `src/real.rs` evaluates constant programs that mention a generator (π, pi, τ, ℯ, euler, ⅈ, √, sqrt, ∛, cbrt,
  sin/cos/tan/ln/exp) at compile time, like time.rs does for dates. Anything else it does not understand (functions, loops,
  imports, `global`) falls back to the WASM path, where exact reals are lowered to f64 exactly as before (π was always
  float(PI) in the parser). So `f(x):=x*x; f(√2)` is still f64: a WASM GC representation of `Exact` is the next step.
- sin/cos/tan exact at multiples of π/12 (sin(π/12) = (√6-√2)/4); exp(n + qⅈπ) exact for integer n, q·12 integer;
  ln(ℯ^n) = n; x^(k/2), x^(k/3) via roots. Everything else is `Approx`, printed `≈…`.
- `<`/`>`: interval arithmetic in BigInt fixed point (Machin for π, series for ℯ, isqrt/icbrt for roots), 64 → 4096 bits;
  still straddling zero → Error('undecidable …'). `==` on normal forms; with an approximation only a clear difference
  decides, else the same Error.
- Radicands are reduced by trial division up to 10^4 plus a perfect-power test: exact for radicands below 10^12 (square)
  / 10^16 (cube); larger radicands with big repeated prime factors may not be fully reduced (then == could miss).
- Left open: rationalizing sums in denominators (1/(1+√2) is approximated), general algebraic roots, runtime `Exact`
  in WASM, ⅈ outside constant expressions (lowering reports an error), `log` (base ambiguous, not taken).
- Next: a Lean reference model of the same normal form (extend `HyperGeneral` with named generators) as a
  differential-testing oracle for `Exact` arithmetic and printing.

## Work area "termination-determinism" (2026-09-28)
- Implemented: canonical NaNs. `util::deterministic_config()` (GC, function references,
  `cranelift_nan_canonicalization`) is the one Config of the project's engines: `gc_engine`, `run_wat`, `run_wasm`.
  Test helpers under tests/ that build their own Config were left unchanged. Test: test_nan_bits_are_canonical (0/0 is 0x7ff8… on x86 too).
- Implemented: fuel. `gc_engine` consumes fuel; every store comes from `util::fueled_store`, with the budget
  `util::DEFAULT_FUEL` = 10^10 steps, overridable by `WARP_FUEL=<steps>`, `warp --fuel <steps>` or `util::with_fuel` (per thread).
  Running out is the error `out of fuel after N steps: the program may not terminate …` (`failed_run`, also `run_wat`).
  `while 1 {}` used to fail at compile time (`cannot extract a numeric value from ø`): an empty body `{}` (parsed as ø)
  is now a spinning loop. Tests: test_infinite_loop_runs_out_of_fuel, test_fuel_budget_can_be_raised. The CI test step
  took about as long as before (5m07s, the same as the preceding green runs).
- Implemented: `Div` effect (src/effects.rs). A function has Div if its body has a `while` (unless the condition is literally
  false), if it recurses through another function (mutual recursion), or if its self-recursion has no measure: a parameter
  that each recursive call moves by a positive literal toward a literal bound known from its guard (`n<2 ? n : f(n-1)`,
  `n>0 ? f(n-1) : 0`, `n>=10 ? n : f(n+1)`), and that the body never assigns. Div propagates to callers; `f ! Pure` then
  fails with "performs Div via f" and a reason; `effects of f` answers `Div`. Tests: test_shrinking_recursion_is_total,
  test_unproven_recursion_may_diverge, test_while_loop_may_diverge, test_pure_rejects_divergence.
- Decision: the default budget is 10^10 steps (several seconds), not 10^9 (alternatives: 10^9, which ends a hang in about
  a second but cuts off legitimate long runs such as fib(35); no default, i.e. unlimited unless asked). A budget per thread
  lets tests use small budgets while running in parallel.
- Decision: the measure check assumes finite numbers. A float NaN or ∞ argument can still make `n<2 ? n : f(n-1)` recurse
  forever; the fuel budget catches it at runtime (alternatives: Div for every function whose parameter may be a float,
  which would make most numeric code Div; excluding NaN via the type once parameters have inferred types).
- Decision: trusted externals (puts, fetch, FFI) do not carry Div, although a host call can block (alternatives: Div on
  every IO/FFI signature, which makes Div indistinguishable from IO). A `for` loop is not emitted yet; once it is, a loop
  over a range or list is total by construction.
- Left open: `while` loops with a provably shrinking counter (`while i<n { i++ }`) are still Div; lexicographic measures
  (Ackermann) and measures that shrink a list are Div; recursion through a function value passed as an argument is not
  tracked (a function symbol used as a value counts as unknown only for self-recursion); `n<=1` in a guard still does not
  parse (`Unexpected character '='`, see "syntax decisions" above).

## For wiki/Footguns.md

### Exact real numbers
(keep the user's lines of the entry verbatim; as of the last copy in this repo, f3fe42a^:Footguns.md, they were:)
**Richardson's theorem**: equality of real expressions built from `π`, `exp`, `sin`, … is undecidable, so `√2 * √2 == 2` cannot hold for every real computation.  
Warp: rationals are exact, algebraic numbers, π,e etc could be kept symbolic! 
see Hyperreal numbers for pragmatic extensions of Q (Also needed for law proofs )
beyond that the result is an approximation and its type says so.

(append:)
Warp now (2026-09-28): `√2*√2 == 2`, `sqrt(8) == 2*√2`, `∛27 == 3`, `sin(π/6) == 1/2`, `cos(π) == -1`, `ℯ^(ⅈ*π) == -1`,
`ln(ℯ) == 1`, `π > 3.14`, `π < 355/113`; `π+ℯ`, `√2+√3`, `π/2`, `2√2` print symbolically; `sin(1)` prints `≈0.8414709848078965`;
`π as float` is 3.141592653589793. A bare `e` or `i` stays a free name; the constants are `ℯ`/`euler` and `ⅈ`.
Constant expressions are evaluated exactly at compile time; inside functions and loops exact reals are still f64 (next: WASM GC form).
Tests: tests/probe_footguns.rs test_square_roots_multiply_exactly … test_euler_identity.  
Decision: an exact real is a sparse polynomial with rational coefficients over named generators (π, ℯ, ⅈ, √r, ∛r) in a
normal form; ε and ω join later as generators ordered by lowest ε power.  
Decision: π and ℯ are treated as algebraically independent (Schanuel's conjecture, unproven), so equal normal forms ⇔ equal values.  
Decision: `==` compares normal forms, `<`/`>` use interval arithmetic up to 4096 bits; what cannot be decided is a loud
Error('undecidable …'), never a guess (`sin(1) == sin(1)` is such an error).  
Decision: a value that is only an approximation prints with `≈`; `as float`/`as fast` converts an exact value to f64 explicitly.

### Nondeterministic NaN bits
Solved in Warp. Every engine the compiler creates canonicalizes NaNs (`cranelift_nan_canonicalization`), so a NaN
produced by arithmetic is always 0x7ff8000000000000 on x86 and ARM and float results are bit-identical across CPUs.
Test: test_nan_bits_are_canonical.

### Termination (halting problem)
Truly impossible to decide in general; Warp handles both sides without a proof assistant.
Runtime: every run has a fuel budget (default 10^10 steps, `WARP_FUEL=<steps>` or `warp --fuel <steps>`); `while 1 {}` ends
with `Error('out of fuel after N steps: the program may not terminate …')` instead of hanging.
Compile time: the effect system has Koka's `Div`. Recursion that moves one parameter by a positive literal toward a guarded
literal bound (`fib(n) := n<2 ? n : fib(n-1)+fib(n-2)`) is total; `while`, unguarded or unbounded recursion
(`f(n) := n==0 ? 1 : n*f(n-1)` diverges for n<0) and mutual recursion are Div. `f ! Pure` rejects a function that may
diverge, `effects of f` reports `Div`. Conservative: when unsure, Div.
Tests: test_infinite_loop_runs_out_of_fuel, test_fuel_budget_can_be_raised, test_shrinking_recursion_is_total,
test_unproven_recursion_may_diverge, test_while_loop_may_diverge, test_pure_rejects_divergence.

## Work area "civil-time" (2026-09-28)
- Implemented: repeated wall times (`2030-10-27T02:30[Europe/Berlin]`) are an error listing both instants as RFC 9557
  fix-its; skipped ones keep their error and now list both candidates. `zoned(local, "Zone", disambiguation: earlier|later|reject)`
  and `add(t, 1 day, disambiguation: …)` spell the choice. A zoned time is `Zoned {date, clock, offset, zone, rules}`
  (src/time/calendar.rs): wall time plus zone, instant derived. The rules are a `TzRules {version, published, zones}`,
  built-in `warp-2026a`, swappable per thread with `time::with_rules` (tests simulate a rule change). `t.tzdata` names the
  version; times after the rules' publication print `[_tzdata=version]`. `1 day == 24 hours` is an error, not false.
  Tests: test_repeated_local_time_needs_disambiguation, test_skipped_local_time_can_be_chosen_explicitly,
  test_zoned_time_records_its_rules_version, test_rule_change_is_not_silent, test_calendar_day_is_not_24_hours.
- Decision: `reject` is the default for literals, `zoned(…)` and `add` (alternatives: Temporal's `compatible`, which takes
  the earlier instant in an overlap and shifts forward in a gap; `earlier` like Python fold=0). `compatible` is not offered:
  it resolves silently.
- Decision: the rules version is recorded as the RFC 9557 elective suffix `[_tzdata=…]`, printed only for predictions
  (instant after the rules' publication date) (alternatives: always print it; a separate field outside the literal; no record).
- Decision: an offset that disagrees with the zone is an error with both fix-its, "keep the wall time" and "keep the instant",
  naming both versions when the literal carries an older `_tzdata` (alternatives: warning and keep the wall time, which is
  Temporal's `offset: 'prefer'`/`'ignore'`; keep the instant). No warning channel exists for compile-time evaluation.
  An agreeing offset under newer rules is accepted silently: the value did not change.
- Decision: durations are equal if all fields are, unequal if exactly one field differs, otherwise an error
  (`1 day` vs `24 hours`, `1 month` vs `30 days`) (alternatives: structural `false`, Temporal's 24-hour days without relativeTo).
- Left open: `Zoned - Zoned` is always an exact duration (no `until(…, largest: days)`); the embedded rule table only models
  current rules (no historical transitions, no real IANA versions); no WASM representation; `_tzdata` is not critical
  (`[!_tzdata=…]` is not parsed); `in` has no options (use `zoned`).

## For wiki/Footguns.md

### Future civil time
What UTC instant is `2030-03-31 02:30 Europe/Berlin`? Time zone rules change by political decision after the code is written,
and each autumn one wall hour happens twice.  
Python: `datetime(2030,3,31,2,30,tzinfo=ZoneInfo("Europe/Berlin"))` silently becomes 01:30 UTC; the repeated 02:30 in October
silently takes `fold=0`. JavaScript Temporal: `disambiguation: 'compatible' | 'earlier' | 'later' | 'reject'`.  
Warp: a zoned time is its wall time plus zone; the offset and instant are derived under a recorded rules version.
`2030-03-31T02:30[Europe/Berlin]` → Error "does not exist … skipped by a daylight saving transition", listing
`01:30+01:00` (earlier) and `03:30+02:00` (later); `2030-10-27T02:30[Europe/Berlin]` → Error "occurs twice", fix-its
`2030-10-27T02:30+02:00[Europe/Berlin] (earlier)` / `+01:00 (later)`. The choice is spelled out:
`zoned(2030-10-27T02:30, "Europe/Berlin", disambiguation: later)`, `add(t, 1 day, disambiguation: earlier)`; reject is the default.
Future times print the rules they were resolved with, `2030-07-01T10:00+02:00[Europe/Berlin][_tzdata=warp-2026a]` (`t.tzdata`);
read back under rules where the offset changed, it is an Error naming both versions, with fix-its "keep the wall time" and
"keep the instant". `+ 1 day` keeps the wall time, `+ 24 hours` the elapsed time (23 hours apart across the March switch), and
`1 day == 24 hours` is an Error, not false.  
Still impossible: knowing tomorrow's politics. Warp only notices when the rules changed.
Tests: test_repeated_local_time_needs_disambiguation, test_skipped_local_time_can_be_chosen_explicitly,
test_zoned_time_records_its_rules_version, test_rule_change_is_not_silent, test_calendar_day_is_not_24_hours.

## Work area "string length" (2026-09-28)
- Implemented: `number of X in t` for the text units `byte(s)`, `char(s)`, `codepoint(s)`, `grapheme(s)` (singular or
  plural, `count of`/`length of` too), and the unit views `#(byte in t)`, `#(t as bytes)`; on literals and variables.
  `number of chars in "héllo"` → 5, `number of graphemes in "👍🏽"` → 1, `number of bytes in "héllo"` → 6,
  `number of codepoints in "👍🏽"` → 2. Lowered in `analyzer::lower_declarations` (`counting_phrase`, `unit_count`) onto
  the existing `t.bytes`/`t.chars`/`t.graphemes` methods; `number of pixels` keeps lowering to `count pixels`.
  The parser no longer takes `#(` at line start as a shell comment (`#x` already counted after a statement).
- Decided: a `char` is a code point (as `x.chars`, the `char` type, Rust's `chars()`); the user-perceived character is a
  grapheme. Bare `#t`, `count`, `length`, `number of t` count graphemes, consistent with grapheme indexing `t#i`;
  `size`/`size of t` counts bytes (memory).
- Decided: case mapping is locale-independent, Unicode default (`"i".upper` → "I", `"İ".lower` → "i̇", `"straße".upper`
  → "STRASSE"), as JS `toUpperCase` and Rust `to_uppercase`. Locale-aware mapping (Turkish/Azeri dotted İ/ı, Lithuanian)
  is left open: no locale syntax exists yet.
- Tests: probe_footguns.rs test_number_of_chars_in_text, test_number_of_graphemes_in_text, test_number_of_bytes_in_text,
  test_number_of_codepoints_in_text, test_count_of_text_without_unit, test_case_mapping_is_locale_independent.
- Left open: locales for case mapping and collation; `number of words/lines in t`; `number of X in list` (count matches).

## For wiki/Footguns.md

### String length: bytes, code points or graphemes?
`"👍🏽"` is 8 bytes (Go `len`), 4 UTF-16 units (JS `.length`), 2 code points (Python `len`), 1 grapheme (Swift `.count`).
**Warp (solved):** the unit is named when it matters. `#t`, `count t`, `length t`, `number of t` count graphemes, the same
unit `t#i` indexes by; `size` is a synonym of `count` (2026-09-29, was: bytes). Bytes only by explicit unit: `byte count of t` / `number of bytes in t` / `#(byte in t)` / `#(t as bytes)` /
`t.bytes`; `number of chars in t` = `number of codepoints in t` = `t.chars`; `number of graphemes in t` = `t.graphemes`.
`number of chars in "héllo"` → 5, `number of bytes in "héllo"` → 6, `number of codepoints in "👍🏽"` → 2,
`number of graphemes in "👍🏽"` → 1. A `char` is a code point; a user-perceived character is a grapheme.
Tests: probe_footguns.rs test_number_of_{chars,graphemes,bytes,codepoints}_in_text, test_count_of_text_without_unit.

### Case mapping and the Turkish İ
`"i".toUpperCase()` in a Turkish locale is "İ" in Java (default locale), breaking identifiers and keyword matching.
**Warp (decided):** case mapping is locale-independent Unicode default mapping, as JS `toUpperCase` and Rust
`to_uppercase`: `"i".upper` → "I", `"ı".upper` → "I", `"İ".lower` → "i̇" (i + combining dot), `"straße".upper` → "STRASSE".
Test: test_case_mapping_is_locale_independent.
**Open:** locale-aware mapping (Turkish/Azeri, Lithuanian) and collation need a way to name a locale; not designed yet.

## Work area "remote calls" (2026-09-28)
- Implemented: `fetch` yields the body as Text or an Error node carrying the reason (DNS/connection, timeout, HTTP status
  >= 400), never "". The host marks a failure by a negative length, the emitter picks the Error or Text kind from it.
  Errors are falsy (`Node::is_falsy`, wasm `is_truthy`), so `if x {…}` is the check.
- Using `x = fetch …` unchecked in arithmetic or member access, or `fetch …` directly as an operand, is a diagnostic
  with fix-it (`check_null_use`, generalized to an `Unchecked::{Null, Error}` reason).
- Default timeout `host::FETCH_TIMEOUT` (10 s); `fetch URL timeout SECONDS` overrides it (host.fetch_within) and a
  timeout is an Error "timeout after N ms".
- Runtime `if … else` with a text/list/error branch now yields a Node (was numeric only: `if x {x} else {"offline"}`
  failed with "cannot extract a numeric value").
- Tests (no external network: `http://127.0.0.1:9` and a local stub): test_failed_fetch_is_an_error_value,
  test_fetch_result_needs_a_check, test_fetch_times_out_loudly. Validated on CI (branch claude/remote-calls).
- Decided: the checked form is `if x {…}` (same as ø), not a new `try`/`?` syntax; the timeout is per call by keyword.
- Left open: no retry policy; no access to the HTTP status or headers of a successful response; a fetch result passed to
  a function parameter is not tracked by the check; the error kind is only in the message text (no structured reason).

## For wiki/Footguns.md

### Remote call failures are silent values
JS wrappers resolve to "", Go drops `err`, PHP `file_get_contents` returns `false`, browsers/curl/Python requests have no
default timeout. Solved elsewhere: Rust/Swift force unwrapping a Result.
Warp: solved. `fetch "http://127.0.0.1:9/"` → Error "fetch http://127.0.0.1:9/ failed: …"; HTTP 404 → Error "HTTP status 404";
`x = fetch …; x + "!"` → diagnostic "x may be an error (fetch can fail)", fix `if x { x + "!" }`; errors are falsy so
`if x {x} else {"offline"}` → "offline"; default timeout `FETCH_TIMEOUT` (10 s), `fetch URL timeout 0.5` → Error
"timeout after 500 ms". Tests: test_failed_fetch_is_an_error_value, test_fetch_result_needs_a_check, test_fetch_times_out_loudly.

## Work area "function equality" (2026-09-28)
- Implemented (src/function_equality.rs, runs in `eval_parsed` before analysis): `f == g` / `f != g` on two named
  top-level functions is decided in fragments, in this order:
  1. structure up to renaming: parameters become positions, self-reference becomes `$self`, the content hash of the
     normalized body plus parameter domains is compared (Unison). `f(x):=x+1; g(y):=y+1; f==g` → true, also recursive
     `fib`/`fibo`.
  2. polynomials over exact numbers: a small sparse normal form (exponent vector → BigInt ratio, expanded and collected;
     `+ - * ^ ² ³`, unary minus, division by a constant). `(x+1)^2` vs `x^2+2*x+1` → true (was a compiler panic),
     `x*x` vs `x+x` → false.
  3. finite domains: all parameters `:bool` (≤ 10) are enumerated through the law machinery (`law::check_instance`).
  4. everything else: `Error("undecidable: f == g (…)")`; a quick property test (`law::property_test`, 16 inputs) adds
     `they differ: counterexample x=…` when it finds one.
- Decision: different arity or different declared parameter types → false, the domains differ (alternatives: an error,
  or compare on the common domain).
- Decision: float parameters never use the polynomial normal form, IEEE arithmetic is not a ring; they fall to 4
  (alternatives: normalize anyway and call it "equal as real functions").
- Decision: a counterexample found in step 4 still yields the undecidable error (as specified), not `false`
  (alternative: `false`, since a reproducible counterexample proves inequality; recommended as a follow-up once
  evaluation of both sides is known to be total and deterministic).
- Left open: no polynomial code from the "exact pi, e and roots" run had landed on main, so the normalizer here is
  kept small; merge it into that sparse-polynomial normal form once it lands (one normal form, not two). Rational
  functions (`x/x`), `%`, conditionals, calls to helper functions and lambdas are outside the decided fragments.
  Finite domains are only `bool`, not enums or small integer ranges. Comparisons of anonymous lambdas or functions
  defined in nested blocks are not rewritten (top-level definitions only). Tests: test_function_equality_up_to_renaming,
  test_polynomial_function_equality, test_finite_domain_function_equality,
  test_undecidable_function_equality_is_an_error.

## For wiki/Footguns.md

### Function equality
Truly impossible in general (Rice's theorem: extensional equality of arbitrary functions is undecidable), but decidable
in useful fragments, and Warp answers only there: never a guess, never a panic.
- **Solved elsewhere:** Unison compares definitions by the hash of their normalized AST; computer algebra systems
  (Mathematica `Expand`, SymPy `simplify(f-g)==0`) compare polynomials by normal form; Lean/Coq need a proof
  (`funext`). JavaScript/Python/Java compare function *identity* (`f == g` is false for two identical lambdas).
- **Warp:** `f == g` on named functions is decided by (1) structure up to renaming (content hash):
  `f(x):=x+1; g(y):=y+1; f==g` → true; (2) polynomial normal form over exact numbers:
  `f(x):=(x+1)^2; g(x):=x^2+2*x+1; f==g` → true, `x*x` vs `x+x` → false; (3) enumeration of finite (bool) domains:
  De Morgan holds; different arity or parameter types → false. (4) Otherwise an error
  `undecidable: f == g …`, with `they differ: counterexample x=2` when a quick property test finds one
  (`f(x):=x%2; g(x):=x%3`). For more, state a `law` or prove it in Lean.
- Tests: test_function_equality_up_to_renaming, test_polynomial_function_equality,
  test_finite_domain_function_equality, test_undecidable_function_equality_is_an_error (tests/probe_footguns.rs).

## Decision update (2026-09-29): `size` = `count`
`size` (function, `size of x`, `x.size`) counts elements, of a text its characters (graphemes), exactly like `count`. This replaces
the earlier "size counts bytes" decision (wiki/Footguns.md is not part of this repository; the record is here). Bytes are counted
only by an explicit unit: `byte count of x`, `number of bytes in x`, `#bytes in x`, `x.bytes` (8 per list element).
Tests still pinning the old rule (not edited, supervisor decides): tests/probe_footguns.rs lines ~366, 370, 411; tests/sweeps/test_todo.rs:103.

## Work area "globals" (2026-10-02)
- Fixed: a function changes a main-level variable declared `global`: `global n=0; def f(x){n+=1;x}; f(3); n` → 1, for
  fn/def/:= bodies, `=`, `+=`, `++` and element assignment (`counts[i]+=1`), ints, floats, lists and texts.
  Mechanism: `allocate_declared_globals` gives every `global` of the program its WASM global before any function is
  compiled; `Scope::globals` holds each declaration's kind and type (`Scope::binding`), so `xs[1]*2` on a global list
  knows its element type; function scopes start with `ctx.declared_globals`, so assignments never create a shadowing local.
- Fixed: `print "called"` as a statement in a numeric function body trapped (cast of the printed text to an Int);
  `x=10; x=floor(x/2)` failed validation (the rounding builtins build an Int, but libm's signature said f64).
- Left open then, fixed in fix-print: `print` of a runtime value and of `print("c")`, and `import floor from "m";
  x=10.0; x=floor(2.5)` (see Work area "print").

## Work area "print" (2026-10-02)
- Fixed: `print(x)` of any number, text or character prints its runtime text form (runtime `print_value`: list_join of
  `[x]`, as `x as string` does), the value is x; `print("c")` is the text "c" (analyzer::held_kind), `print x` is worth x
  in infer_type. Lists and floats stay "print of a List has no runtime text yet", like `xs as string`.
- Fixed: exact numbers that are no fixnum have a runtime text form (`exact_text` in wasm_emitter/exact.rs, used by
  list_join): ratios with a terminating decimal as decimals (`2.5`, `-0.125`), others as `1/3`, ±∞ and NaN as the host
  prints them, big integers in full (`int_text`, digits by int_quot/int_rem). Before, the handle printed as
  -9223372036854775807. Required with list_join whenever the int runtime is (text_builtins::add_dependencies).
- Fixed: an exact variable later assigned an f64 (`import floor from "m"; x=10.0; x=floor(2.5)`, `x=1; x=2.5 as float`)
  holds an f64 throughout (analyzer `widen_to_float`), as expressions mixing in an f64 do; before, WASM validation failed.
- Tests: tests/test_welcoming_print.rs, probes/print/.

## For wiki/Footguns.md

### Assigning a main-level variable from a function
- **Elsewhere:** Python makes `n += 1` inside a function a new local and fails with UnboundLocalError (or silently
  shadows with `n = 5`); JavaScript/Ruby blocks silently mutate the outer variable; PHP sees no outer variables at all.
- **Warp:** a function reads main-level variables by value (captured where it is defined, see Closures). Changing one
  needs the `global` declaration (a State effect, wiki/effects.md): `global n=0; def f(x){n+=1;x}`. Without it,
  `n=0; def f(x){n+=1;x}` is an error that educates:
  `n is a main-level variable: declare it `global n` to change it from a function, or use a new local name; fix: global n`.
  The error applies when the function reads or updates the name before (or without) binding it: `n += 1`, `n++`,
  `xs#i = v`, `y = n; n = y+1`. A body whose first mention is a fresh `name = value` not reading it (`primes = []`)
  is ambiguous and asks (topic `local-or-global`, analyzer::resolve_main_variable_assignments): "a new local of f"
  (default, explicit form `let n = …`) or "the main-level n" (`global n`, which turns main's first `n = …` into the
  global declaration). Unanswered it warns and takes the local, as Python does (samples/sieve_idiomatic.wasp relies on
  this); `use strict` makes it an error. `let`/`var n = …`, a parameter of the same name, or a local whose name main does
  not use, is the function's own without a question.
- Tests: tests/test_welcoming_globals.rs.
