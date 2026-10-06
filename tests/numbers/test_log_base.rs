// `log(x, base)`: the logarithm to a base, ln(x)/ln(base); the second argument used to be dropped (log(100, 10) was ln 100)
use crate::is;

#[test]
fn log_takes_a_base() {
	is!("log(100, 10)", 2.0);
	is!("log(8, 2)", 3.0);
	is!("x = 1000; log(x, 10) > 2.99", 1);
	is!("ln(1)", 0);
}
