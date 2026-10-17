// Card nested-two: a function nested two or more deep reads the variables of every function around it, as in Python
use crate::is;

#[test]
fn a_function_nested_two_deep_reads_the_outermost_variables() {
	is!("outer(x) := { mid(y) := { deep(z) := x + y + z; deep(1) }; mid(10) }; outer(100)", 111);
	is!("a(w) := { b(x) := { c(y) := { d(z) := w + x + y + z; d(1) }; c(10) }; b(100) }; a(1000)", 1111);
	is!("k = 5; outer(x) := { mid(y) := { deep(z) := k + x + y + z; deep(1) }; mid(10) }; outer(100)", 116);
}
