# Brief for agents fixing Footguns.md "NOT YET" entries

> **The catalogue moved (2026-09-28):** `Footguns.md` now lives in the wiki repo `github.com/pannous/warp.wiki`
> (locally `wiki/Footguns.md`, not part of this repo). Do not recreate it here. Instead, write the new or changed entry
> text under the heading `## For wiki/Footguns.md` at the end of `notes/footguns.md` (one `### <entry title>` block each,
> ready to paste); the maintainer's session transfers it to the wiki. The Footguns.md steps below mean that block.

Repo: github.com/pannous/warp (Rust compiler for the Wasp/Warp language, emits WASM GC). Read CLAUDE.md, DESIGN.md,
Footguns.md, notes/footguns.md first. Each agent owns one work area (below); stay inside it.

## Workflow
1. Baseline: `./test.sh probes/footguns/baseline_<area>.txt` (do not commit that file; do not overwrite test_results.txt).
   `vendor/` is gitignored: if `cargo --offline` fails in your environment, build without `--offline`.
2. For each footgun: the matching `#[ignore = "next"]` test in `tests/probe_footguns.rs` is the spec. If an entry has no
   test yet, add one there first (new tests only; never modify or delete existing tests in other files).
3. Fix at the root (parser, analyzer, type inference or emitter), no special-casing of the test input
   (see wiki "Whack-a-Mole": if a fix breaks another test, redesign rather than adding logic).
4. Remove `#[ignore]` only when the test passes; run the full suite; no previously passing test may fail.
5. Move the entry in Footguns.md from NOT YET to Solved: keep its "Solved elsewhere" notes, state the verified Warp answer
   and the test name. Add the case to `probes/footguns/cases.warp`.
6. Push to main only when the full suite passes with **zero** failures on the exact commit you push (after the rebase),
   no "pre-existing failure" exemptions: every red push to main mails the maintainer. If main is already red, fix it
   first or report it instead of pushing. Without a local cargo (cloud sandboxes cannot reach crates.io), test on a
   `claude/<area>` branch: its CI job always ends green so it mails nobody, and the real outcome is the line
   `AGENT_CI_RESULT build=… tests=…` in the job log and summary. Only `tests=success` counts.
   No pull requests: merge into main yourself, then delete your `claude/<area>` branch (`git push origin --delete …`).
7. Small conventional commits (`fix:`, `feature(minor):`), no AI attribution lines, stage files by explicit path
   (never `git add -A`; note the file is `Footguns.md` with capital F), `git pull --rebase` before every push, push to main.
   Several agents push to main concurrently: rebase and resolve conflicts, never force-push.
8. If an entry needs a language design decision (not just a fix), do not decide it: write the options with a
   recommendation under the entry in Footguns.md, marked `Decision needed:`.
9. When done, append a short summary (fixed / left open / why) to notes/footguns.md.

## Work areas
- **literals**: `.1` leading-dot literal, `1e3` scientific notation, `1_000_000` digit separators, `‖x‖` norm/abs parse
  error (`‖-5‖` → Unexpected character ' '). Tests: test_scientific_notation (+ new ones).
- **exact-numbers**: 🐞 sum of quotients truncated (`1/4+1/4` → 0, result typed Int); exact rationals by default
  (`0.1+0.2==0.3`, `1/3*3==1`, DESIGN.md "Exact numbers by default", `Quotient` in src/extensions/numbers.rs);
  NaN/infinity, rounding mode, negative modulo, booleans as integers. Tests: test_sum_of_quotients_is_not_truncated,
  test_exact_decimal_arithmetic.
- **strings-equality**: 🐞 `"abc"=="abc"` panics, `0==""`, `null==false`, `if "" …`, `if [] …` panics; NFC normalization;
  char-safe `#` indexing (`'héllo'#2` → 'é'); `"abc" is "abc"`; duplicate keys. Tests: test_string_equality_is_by_value,
  test_unicode_normalization, test_character_indexing_is_unicode_safe.
- **precedence-syntax**: `-2^2` → -4; `not`/`&` bind weaker than comparisons; chained comparison `3>2>1`; assignment in a
  condition (`if x=2`); 🐞 `x++` lost; `++i` parse; 🐞 `1 + f 3` → 3. Tests: test_negative_power_precedence,
  test_logic_binds_weaker_than_comparison, test_increment_changes_variable, test_braceless_call_as_operand.
- **conversions-bounds**: `"5"+3` codepoint arithmetic, `int("12a")` → 0, `const` ignored, list `+`/`*` summing,
  index out of range / `#0` / negative index, mutation through aliases (value semantics / copy-on-write).
  Tests: test_index_out_of_bounds_is_an_error, test_mutation_through_alias_is_not_visible.

Out of scope (other agents own them): the Lean/law model (law agent), `as i64` inside functions (unbounded-Int agent),
and design decisions: truthiness of empty values, `yes`/`no`/Norway problem, null/optional types, dates, SQL, variance.
