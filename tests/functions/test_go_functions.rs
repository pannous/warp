// Go functions: several results `(int, int)`, taken apart with `q, r := f(…)` (Go's := in a destructuring assigns)
use crate::is;

#[test]
fn several_results() {
	is!("func divmod(a, b int) (int, int) { return a / b, a % b }; q, r := divmod(7, 2); q * 10 + r", 31);
	is!("func divmod(a int, b int) (int, int) { return a / b, a % b }; q, r = divmod(9, 4); q * 10 + r", 21);
}

#[test]
fn a_go_short_declaration_that_is_assigned_stays_an_error() {
	// P138 (user): Go's `total := 0` then `total += n` keeps the loud "charged" error, which names `total = …`
	crate::common::fails_with("def sum(xs){ total := 0; for x in xs { total += x }; total }; sum([1, 2])", "total = …");
}
