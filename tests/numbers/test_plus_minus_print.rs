// An interval `x ± r` at run time as text, its parts, negated and ordered (card plus-minus-print, notes/plus_minus.md)
use crate::common::fails_with;
use warp::diagnostic::take_runtime_warnings;
use warp::wasm_emitter::eval;

#[test]
fn an_interval_prints_as_it_reads_back() {
	assert_eq!(eval("x = 5 ± 1; str(x + 1)"), "6.0 ± 1.0");
	assert_eq!(eval("x = 6 ± 0.3; y = 2 ± 0.2; str(x / y)"), "3.00 ± 0.50");
	assert_eq!(eval("x = 1000 ± 123; str(x * 1)"), "1000 ± 120");
	assert_eq!(eval("x = -0.5 ± 0.0012; str(x + 0)"), "-0.5000 ± 0.0012");
	assert_eq!(eval("x = 5 ± 0; str(x + 0)"), "5 ± 0");
	assert_eq!(eval("x = 5 ± 1; str([x + 0, 2])"), "[5.0 ± 1.0 2]");
}

#[test]
fn the_parts_of_an_interval() {
	assert_eq!(eval("x = 5 ± 1; y = x * 2; y.value"), 10.0);
	assert_eq!(eval("x = 5 ± 1; y = x * 2; y.uncertainty"), 2.0);
	assert_eq!(eval("x = 5 ± 1; y = x * 2; y.low"), 8.0);
	assert_eq!(eval("x = 5 ± 1; y = x * 2; y.high"), 12.0);
}

#[test]
fn an_interval_negated() {
	assert_eq!(eval("x = 5 ± 1; str(-(x + 1))"), "-6.0 ± 1.0");
	assert_eq!(eval("f(y) := -y; str(f(2 ± 1))"), "-2.0 ± 1.0");
	assert_eq!(eval("f(y) := -y; f(2.5)"), -2.5);
}

// decision P224b: an ordering of intervals answers only when certain, and never crashes

#[test]
fn certainly_and_possibly_compare_the_whole_interval() {
	// y is 5..7
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y certainly < 8"), true);
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y certainly < 7"), false);
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y certainly >= 5"), true);
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y possibly < 6"), true);
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y possibly > 7"), false);
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y.value < 7"), true);
	assert_eq!(eval("x = 5 ± 1; certainly(x + 1 < 8)"), true);
	assert_eq!(eval("2 certainly < 3"), true);
	assert_eq!(eval("area = 5 ± 1; big = area certainly > 10; big"), false);
	assert_eq!(eval("x = 5 ± 1; if (x certainly < 8) then 1 else 2"), 1);
}

#[test]
fn a_known_interval_says_how_it_compares() {
	fails_with("x = 5 ± 1; y = x + 1; y < 7", "y certainly < 7");
	fails_with("y = 6 ± 1; y > 4", "y certainly > 4");
}

#[test]
fn an_interval_known_only_at_run_time_compares_certainly_with_a_warning() {
	take_runtime_warnings();
	assert_eq!(eval("f(y) := y < 8; f(6 ± 1)"), true);
	assert!(take_runtime_warnings().is_empty());
	assert_eq!(eval("f(y) := y < 7; f(6 ± 1)"), false);
	let warnings = take_runtime_warnings();
	assert_eq!(warnings.len(), 1, "{warnings:?}");
	assert!(warnings[0].contains("certainly"), "{warnings:?}");
	assert_eq!(eval("f(y) := y < 7; [f(6 ± 1), f(6 ± 1)]"), eval("[no, no]"));
	assert_eq!(take_runtime_warnings().len(), 1, "once per run");
}
