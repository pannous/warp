# Test triage 2026-10-28 (decision night-releases-and-test-triage)

The failing lines of test/results.txt (aa60d3ddc3, 4149 ✗), sorted into three groups:
- **gap**: the test is right and warp should do it. These are grouped into cards below and not edited.
- **wrong**: the expectation is wrong. These are listed for the user to approve and not edited.
- **esoteric**: a feature we don't carry over. These are commented out with
  `# triage-held 2026-10-28: <reason> :: <the line>`; an is! test's further lines start with `# :: `.
  To restore one, remove everything up to and including ` :: `, or `# :: ` on a continuation line.

## Wrong expectations (approve to correct)

| file:line | expected now | proposed | why |
|---|---|---|---|
| test/welcoming/test_c_style.warp:26 | "class float" | "class Real" | π is the exact Real (decision exact-constants) |
| test/numbers/test_rational_type.pairs.warp:20, 21 | "real" | "class Real" | type(π) is the class Real, as above |
| test/numbers/test_float_exact_context.pairs.warp:44 | "real" | "class float" | sqrt of a float is a float; √2 is the exact one |
| test/numbers/test_implicit_libm.pairs.warp:15 | 0.8414709848078965 | 42 | `def sin(x) := 42` is the program's own sin, which wins over libm (shadowing per arity) |
| test/text/test_char_parameter_type.pairs.warp:4, 5, 6 | "codepoint" | "class text" | decisions: an untyped parameter given a one-character text takes it as Text (5 even declares t:text) |
| test/text/test_text_type_words.pairs.warp:12, 13 | π | 3.141592653589793 | `π as float` is the float, not the exact π |
| test/lists/test_element_wise_vectors.warp:28 | [-9-8] | [-9, -8] | `[-9-8]` reads as [-17]; the result [-9 -8] is right |
| test/lists/test_element_wise_lists.pairs.warp:6 | [-2-3] | [-2, -3] | the same typo |
| test/lists/test_min_max.pairs.warp:20 | 2 | 6 | `min(a,b):=a*b` is the program's own min, which wins over the prelude's |
| test/lists/test_lists.pairs.warp:61, 68, 72, 81 | [1, 2, 3] / ['a', 'b', 'c'] | 3 / 'c' | `;` separates statements: `(1;2;3)` and `{a;b;c}` give their last, as `a;b;c` gives 'c' (line 73) |
| units without `use units`, 83 lines: test/numbers/ test_decimal_units:4; test_dimension_types:4-7, 19-23; test_dynamic_units:4, 5, 8, 9, 12; test_litre_hectare:4; test_math:42; test_mixed_quantity_arguments:4, 5; test_mixed_units.warp:4, 6; test_pixel_unit:4, 5, 9, 13 and .warp:6, 8, 14; test_quantity_cast:4; test_run_time_power:6; test_time_of_day:4, 6, 9, 13; test_unit_beside_variable:10; test_unit_definitions:4-9, 11, 14, 15, 24, 26, 27, 30 and .warp:4, 7, 9, 11, 15; test_unit_functions_as_values.warp:4, 9; test_unit_literals:4-6; test_unit_prefix_lines.warp:6; test_unit_word_keys:13, 14; test_units_after_numbers.warp:30; test_units_compare:4-14; test_units_type_holes:4-7 | `1 km == 1000 m` | `use units; 1 km == 1000 m` | decision expected-type-names: without an expected type `3ms` is an error whose fix-it is `use units` |

## Held (esoteric)

| file:line | reason | delete? |
|---|---|---|
| test/welcoming/test_footguns.pairs.warp:383 | `fast` floats that reassociate math | delete? |
| test/welcoming/test_footguns.pairs.warp:395 | C float32 suffix `.1f` | delete? |
| test/welcoming/test_footguns.pairs.warp:403, 404, 425 | symbolic surds (√2·√2 exactly 2) | |
| test/welcoming/test_footguns.pairs.warp:414 | symbolic trig, sin(π/6) exactly 1/2 | |
| test/welcoming/test_footguns.pairs.warp:428 | complex exponentials (Euler's identity) | |
| test/welcoming/test_footguns.pairs.warp:442, 445, 446 | function equality by algebra (undecidable in general) | delete? |
| test/welcoming/test_footguns.warp:44 | function equality by boolean algebra | delete? |
| test/welcoming/test_footguns.pairs.warp:262, 263, 266, 267, 268 | tzdata version stamps on times (267 and 268 contradict each other) | delete? |
| test/welcoming/test_footguns.warp:9 | `law` property declarations | |
| test/welcoming/test_footguns.warp:33, 37 | effect annotations `! Pure`, `! Div` | |
| test/numbers/test_log_glyphs.pairs.warp:4-7, 10-12 | log glyphs ⌞ ⌟ (P226), hard to read beside log(x, b) | delete? |
| test/numbers/test_law.pairs.warp:4, test_law.warp:4 | `law` property declarations | |
| test/numbers/test_unit_definitions.pairs.warp:20 | a calendar era as a unit (`2 AD`) | delete? |
| test/numbers/test_unit_prefix_lines.pairs.warp:16, 17 | `name` and `prefix` as declaration words for plain functions | delete? |
| test/text/test_interpolation.pairs.warp:35 | shell-style `$0` as a function's parameter | delete? |
| test/text/test_text_type_words.pairs.warp:9 | symbolic surds, `str(√2)` as "√2" | |
| test/lists/test_element_wise_scalar.pairs.warp:5, 7, 10 | MATLAB anonymous functions `@(x)` | delete? |
| test/lists/test_where_filter.pairs.warp:13 | Swift closure arguments `$0 + $1` | delete? |

## Gap groups (cards)

### welcoming (155 gaps)
Silently wrong, which matters most:
- `for char in "abc" { n += 1 }` gives 0 (test_golf_words:12, 13).
- A big integer's `rem` gives 0 (test_footguns:362).
- `x.upper!` leaves x unchanged (test_discarded_pure_warning:5, 6, 11).
- `"a" + warning("careful") + "b"` gives "aøb" (test_warning_mode:4).

Groups:
- Dates, times and durations as values: `2024-02-29.month`, `1 hour == 60 minutes`, zoned times. About 25 lines, error "a data".
- A round group `(1 2)` as a list value, mutable and aliased. 13 lines, error "the group (…".
- Assigning to a text element, `x#1 = 'é'`. 8 lines.
- English operator words: plus, minus, equals, greater/less than, at least/most, through. 9 lines.
- Text views: `as bytes`, `as codepoints`, `as graphemes`, `byte in t`, "number of bytes in". 9 lines.
- `xs.insert(v, at: i)`. 7 lines.
- Bool arithmetic, `true + true`, `(1<2) + 1`, `v.has(1) + 1`. 5 lines.
- `round_half_even` and `round_half_up`. 6 lines.
- A method without parentheses, `xs.product`, `s.capitalize`. 6 lines.
- `sleep` with a spaced unit, `sleep(16.7 ms)` or `sleep 5 ms`. 5 lines here, many more in control.
- Float ∞ and NaN, `1.0/0.0`. 3 lines.
- `range(n)`. 3 lines.
- `if x=2` read as a comparison. 3 lines.
- `++i` and `--i`. 3 lines.
- Pairs one at a time: C-style typed declarations, `def … end`, `x <- 3`, `let mut`, `next`, `xs count`,
  shadowing pi and count, sh and sql templates, `fetch`, function equality by definition (f==g alpha-equal).

### numbers (263 gaps)
Silently wrong:
- `round(2.5)` gives 3 (test_float_to_int_range:6, test_rounding_in_functions:6). wiki/Footguns.md says round is
  half even, but notes/decisions.md round-halves says halves round away from zero: the user decides which.
- `2^62 | 2^61` gives 2^62 (test_big_int_bits:18).
- `1 << 100 == 2^100` gives no (test_shift_operators:18).
- `ε*ω` gives 2876127 (test_hyperreals).
- `use units; m = 5; str(3m)` gives "15" (test_units_arithmetic:4, test_unit_beside_variable:4, units_after_numbers:4, 7):
  a meaning question for warp-interviewer.
- The harmonic sum `s += 1/k` floors to -5 (test_unbounded_int:67).
- `sqrt(2).round(2)` ignores its argument (test_method_on_operator_call:4).

Groups:
- Unbounded ints past 64 bits, the D16 rule: they trap with cast failure or unreachable (big_int_bits, big_ratio,
  int_division, unbounded_int, ratio_gcd_warp, shift_operators). About 45 lines.
- A spaced unit after an assignment, `x = 2 km`: "a value after an assignment". 20 lines.
- The `real`, `exact` and `rational` types as parameters and variables: "a parameter of kind rational". 20 lines.
- `type([…])` of a list: "the type of a list". 15 lines.
- Repeating decimals `0.3̅` and `as decimal` (decision repeating-decimals). 15 lines.
- `fast` functions and scopes (decision fast-scope); `use fast_math` doesn't reach pi/tau and fails "checked multiply". 30 lines.
- Zero-filled arrays, `640000*int` and `x : 1000 int`. 10 lines.
- Unit definitions: `unit furlong := …`, affine `unit rankine as kelvin`, `x in minutes`, `kilometers`. 15 lines.
- `nearest(x, rational)`, the best rational approximation. 8 lines.
- ∞ needs `use reals`, though π doesn't; NaN and ∞ from float division by zero. 9 lines.
- Methods without parentheses, `x.floor`, `x.abs`. 6 lines.
- Fixed-width C types (`unsigned short`, `long long`, `int32` variables). 5 lines.
- One-offs: `log(x, base)`, `ln`, `∜`, `int("1e3")`, `else` as or-else (`false else 3`), `{a:1}(a)`, `a-b` names,
  `↑`/`↓` reaching log, "size of … in bytes", number words (`one plus two`), long decimal literals kept exact.

### text (100 gaps)
Silently wrong:
- `add "c" to x` and `x.add("c")` on a text give "ab['c']" (test_add_to_text, 4 lines).
- `try ord("ab") else 0` gives 97 (test_character_comparison:35).
- A class's `text(p:P)` isn't used by `s + p`, `x as string` or `String(x)` (test_text_plus_printable:16, test_text_type_words.warp:4, 5).
- `for byte in "äb" { byte }` gives "b" (test_text_unit_lists:7).

Groups:
- `count x in y`, `.count(x)` and `count(xs, x)`: "cannot access length code of bool". 18 lines.
- Assigning to a text element `k#1='a'`. 14 lines, the same group as in welcoming.
- sh and sql templates as lists. 8 lines.
- `byte_slice`. 6 lines.
- Selector prepositions, `length of words in t`. 6 lines.
- Backtick templates. 6 lines.
- `split` with no argument. 4 lines.
- `.string` of a list, `type([…])`, `use js` in a native run, `print()` with no argument, `join xs sep`,
  `Cursor.take_while(is_letter)`, `2nd char in`, `'world'#-1`, Python slices `t[a : b]`, text * a float count.

### lists (243 gaps)
Silently wrong:
- `[a -1]` gives [4] and `[1 -1]` gives [0]. The tests expect a sign glued to its number to start a new item (test_list_literal_sign, test_signed_operand_list, 10 lines).
- `p.@source = "gps"; #p` counts the meta field (test_map_starts:9).

Groups:
- A round group `(1 2 3)` as a list. 25 lines.
- Element-wise `.+ .* .- ./` and scalar broadcasting; `dot`; math words over lists. 45 lines.
- `linear` arrays. 17 lines.
- Swizzles `.x .xy .zyx` (decided). 16 lines.
- `where` filters and comprehensions. 14 lines.
- Lazy ranges counted without materializing (time out). 10 lines.
- `pop(i)`, `clear()`, `delete xs[i]` and their aliasing. 15 lines.
- Unpacking: `a, *rest =` and `f()` returning several values. 11 lines.
- `x : 100 int` and `int[100]` declarations. 10 lines.
- `a=1 2 3`, a bare list after `=`. 6 lines.
- `===` identity. 5 lines.
- `xs.insert(x, at: i)`. 4 lines.
- A typed list variable `xs: [P] = …`: "an assignment to `… : …`". 4 lines.
- `pixels size` and `pixels count` as postfix words. 5 lines.
- `sort_by`, `h.0` numbered fields, `map<int,int>`, `dict.fromkeys`, `tuple`, `values` of a pair.
