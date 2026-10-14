// A then or else branch is a statement: a braceless call takes a variable as its argument there, as `x = count xs` does;
// `then count xs` took `count` alone and handed xs to the else branch
use crate::is;

#[test]
fn a_braceless_call_with_a_variable_in_a_branch() {
	is!("f(xs, n) := { if n == 0 then count xs else f(xs, n - 1) }; f([1,2], 2)", 2);
	is!("xs = [1, 2, 3]; if 1 then count xs else 0", 3);
	is!("xs = [1, 2, 3]; if 0 then 0 else count xs", 3);
	is!("xs = [1, 2]; y = 0; if 1 then y = count xs else y = 5; y", 2);
	is!("a = 1; b = 2; if 1 then a else b", 1);
}
