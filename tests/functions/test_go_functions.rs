// Go functions: several results `(int, int)`, taken apart with `q, r := f(…)` (Go's := in a destructuring assigns)
use crate::is;

#[test]
fn several_results() {
	is!("func divmod(a, b int) (int, int) { return a / b, a % b }; q, r := divmod(7, 2); q * 10 + r", 31);
	is!("func divmod(a int, b int) (int, int) { return a / b, a % b }; q, r = divmod(9, 4); q * 10 + r", 21);
}
