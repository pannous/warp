// An interval `x ± r` at run time as text, its parts, negated and ordered (card plus-minus-print, notes/plus_minus.md)
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

#[test]
fn intervals_order_by_their_values() {
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y < 7"), true);
	assert_eq!(eval("x = 5 ± 1; y = x + 1; y > 7"), false);
}
