//! card function-ref-compare: `h == &g` and `h == k` of an alias `k = &g` compare the function g itself, not its call
use crate::is;

const CHOSEN: &str = "f(x) := x + 1; g(y) := y * 2; h = if clock() > 0 then &f else &g; ";

#[test]
fn a_function_value_compares_with_a_reference() {
	is!(&format!("{CHOSEN}h == &f"), true);
	is!(&format!("{CHOSEN}h == &g"), false);
	is!(&format!("{CHOSEN}h != &g"), true);
}

#[test]
fn a_function_value_compares_with_an_alias() {
	is!(&format!("{CHOSEN}k = &f; h == k"), true);
	is!(&format!("{CHOSEN}k = &g; h == k"), false);
}
