// `a or b` is a value (the first truthy operand), also as a function's result: `f() := "" or "d"` gave a WASM
// validation failure or "cannot extract a numeric value" (found writing lib/extra/netbase.warp, card netbase-package)
use crate::is;

#[test]
fn or_of_texts_in_a_function() {
	is!("s() := \"\" or \"d\"; s()", "d");
	is!("s(a) := a or \"d\"; s(\"\")", "d");
	is!("s(a) := a or \"d\"; s(\"x\")", "x");
	is!("x = \"\"; s() := x or \"d\"; s()", "d");
	is!("s(a) := a and \"yes\"; s(\"x\")", "yes");
}

#[test]
fn or_of_numbers_in_a_function_stays_a_number() {
	is!("s(a) := a or 4; s(0) + 1", 5);
	is!("s(a, b) := a > 1 and b < 2; s(3, 1)", 1);
}
