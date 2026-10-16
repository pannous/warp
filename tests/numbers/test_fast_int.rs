//! A fast function's Int expressions are machine i64 arithmetic as a whole: nested operations and comparisons wrap
//! at 2^63 without the unbounded-Int checks between them (card fast-int); `x: fast int`, `i64` and `int64` wrap too,
//! and trap under `use strict` (decision i64-wraps)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_fast_loop_counts_as_the_exact_one() {
	is!("fast f(n) := { t = 0; i = 0; while i < n { t = t + i % 7; i = i + 1 }; t }; f(1000)", 2997);
}

#[test]
fn nested_fast_operations_wrap() {
	is!("fast g(a, b) := (a * b + 1) * 2; g(2^62, 2)", 2);
	is!("fast h(n) := n + 1 < n; h(2^63 - 1)", true);
}

#[test]
fn a_fast_int_variable_wraps() {
	is!("x: fast int = 2^63 - 1; x + 1", -9223372036854775808i64);
}

#[test]
fn int64_wraps_and_traps_only_when_strict() {
	is!("a: int64 = 2^63 - 1; a + 1", -9223372036854775808i64);
	is!("x: i64 = 2^63; x", -9223372036854775808i64);
	fails_with("use strict\nint64 x = 9223372036854775807; x+1", "int64 overflow");
	fails_with("use strict\nx: i64 = 2^63 - 1; x + 1", "int64 overflow");
}
