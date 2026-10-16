//! card print-prints: spaced values mixing a variable and an expression keep every value, as `1+1 2+2` and `y 2` do:
//! `print y 1+1` printed only 2
use crate::is;

#[test]
fn spaced_values_with_a_variable_keep_every_value() {
	is!("y = 3\nxs = y 1+1\n[count(xs), xs#1, xs#2]", warp::ints(vec![2, 3, 2]));
	is!("y = 3\n\"\\(y 1+1)\"", "3 2");
	is!("y = 3\n\"\\(1+1 y)\"", "2 3");
	is!("x = 2\n\"\\(3 x+1)\"", "3 3");
}
