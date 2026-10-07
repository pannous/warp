//! card global-function-inside: `global n = 5` in a function body declares n main's and sets it, as `global n; n = 5`
//! does (it silently left main's n unchanged)
use crate::is;

#[test]
fn global_with_a_value_in_a_function_changes_main() {
	is!("n = 0; f() := { global n = 5 }; f(); n", 5);
	is!("n = 0; def f() { global n = 5; n + 1 }; f() * 10 + n", 65);
	is!("n = 1; def f() {\n global n = n + 2\n}\nf(); f(); n", 5);
}
