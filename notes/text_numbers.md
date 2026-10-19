# Texts as numbers in warp (card card-4, sprint 2026-10-19, written without a build or test run)

lib/compiler/runtime.warp holds the twins of Rust's run-time text casts:
- `text_as_int(t)`: sign + decimal digits, nothing else (Rust: node_arithmetic.rs TEXT_AS_INT)
- `text_as_float(t)`: sign, digits with an optional point, optional exponent; the digits go through
  `decimal_digits` (first 18 significant ones exact in an int, the rest counted) and `decimal_float`, which reuses
  float_text's double-double `float_scaled`: one exact power up to 10^22 is one rounding (Clinger's fast path), more
  steps stay within 2^-104 before the last rounding. Rust multiplies ten by ten beyond 10^±22 and accumulates the
  digits in an f64, so it can be an ulp off there; warp's is the correct one (twins-never-bounce: accepted difference).
- `text_as_number(t)`: "a/b" the exact ratio (exact_ratio), a whole float below 2^63 an int, else the float.

lib/compiler/emitter.warp routes `t as int`, `t as float` (text_number_words) and `t as number` of a text value to
them; `x as number` of an int or float is the value itself; any other kind is refused (Rust takes it).

Differences from Rust, each a later step:
- no number traps (`fail()`), where Rust raises invalid_number that `try` catches: needs a runtime error word in
  runtime.warp.
- text_as_int is 64 bits; Rust's grows a big int ("99999999999999999999" as int) with the int runtime.
- results beyond 10^290 or below 10^-290 are scaled by 10^20 apart from float_scaled and back, a second rounding.
- a one-character text is a char in the emitter ("7" as int): still refused, Rust gives the digit.
- the Rust functions stay until a tested step switches emission over.

Tests: tests/parser/test_self_text_numbers.rs over probes/self_hosting/emitter/text_numbers (emitter twin); its
probes avoid the forms where Rust's scaling is off.
