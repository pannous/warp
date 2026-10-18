# Match/pattern lowering in Rust: fewer, cleaner passes (card match-pattern)
User direction 2026-10-17 night: trim the Rust core, more elegant, fewer lines; each merge request states its net
src/ delta. Survey 2026-10-17 (src/pipeline.rs order): 15 passes read matches, patterns, destructuring, type tests.

## Passes (position in src/pipeline.rs)
SUGAR: rust_options::lower (2), welcome_forms::lower_match_arms (3, warp twin), parameter_patterns::lower (21),
plain_enums (25), sum_variants (26), match_exhaustive::check (27), named_patterns::lower (32),
switch::check_unit_names (33), tree_walks (34), quasi_quote (35). DECLARATION: class_methods destructurings (15),
welcome_forms::lower (when_chain, destructured_parameters; 23). FUNCTION: tuples::lower (16). MEANING:
type_tests::lower (5), switch::lower (28, the real matcher, `Matched`).

## Duplications
- D1 two matchers: class_methods/destructuring.rs 145-233 (own Matched) re-does switch.rs Matched; class_arm builds
  `if test then parts·from: body`, which switch::split_guard takes apart again.
- D2 six match-shape recognisers: switch::split_switch, welcome_forms::with_match_cases, match_exhaustive::
  subject_and_arms, rust_options::option_match, destructuring::is_match, function_values.rs 237-250.
- D3 four catch-all tables: switch::DEFAULT_KEYS, match_exhaustive::CATCH_ALLS, welcome_forms WILDCARD/ELSE_WORD/
  DEFAULT_CASE, rust_options::WILDCARD (+ literal "_" in destructuring.rs).
- D4 three binding-name listers: switch::pattern_bindings, tuples::pattern_names/is_pattern,
  property_accessors::destructured_names.
- D5 Kotlin `when(x)` lowered by its own when_chain (no ranges, bindings, `is T`); match_exhaustive special-cases it.
- D6 inside switch.rs: alternatives twice (condition / pattern_test), type_named re-inlined, item_pattern duplicated
  in named_patterns::is_open_part.
- D7 three walkers of pattern positions: named_patterns::substituted, quasi_quote::lower, check_unit_names.
- D8 two parameter destructurings (JS `({a,b}) =>` vs parameter_patterns): differ for instances, leave as is.

## Steps, lowest risk first (estimated src/ lines saved)
1. one CATCH_ALLS + is_catch_all in src/words.rs (~8)
2. switch.rs internal: type_named everywhere, item_pattern shared, pattern_test via condition, fix comment :572 (~15)
3. tuples::pattern_names over switch::pattern_bindings, drop property_accessors::destructured_names (~25)
4. nodes::match_parts(items) -> (head, subject, cases) + with_cases, replacing the six recognisers (~50)
5. check_unit_names folded into named_patterns::lower; optional shared pattern_positions visitor (one pass, ~20)
6. Kotlin `when(s)` with a subject → nodes::matched(s, arms via colon_case) in DECLARATION (~35, gains ranges)
7. rust_options after lower_match_arms: `Some(v):` arm → guard `v if v != ø`, `None` → `ø:`, add `default: ø` (~25)
8. class patterns into switch's Matched; class_methods only rewrites positional Point(a, 0) to Point{x: a, y: 0};
   delete destructuring.rs class_arms/class_arm/Matched/matched_pattern/is_match (~80, riskiest)
Steps 1-5 keep behaviour; 6-8 change what arms do: run tests/control and tests/types after each.
Tests per step: control/test_match_*, test_switch_*, test_named_patterns, test_pattern_unit_name, test_quasi_quote,
test_rust_option, test_when_signals, types/test_match_exhaustive*, test_class_destructuring, test_variant_payloads,
lists/test_tuples, functions/test_parameter_patterns, parser/test_self_passes (twin of lower_match_arms).
