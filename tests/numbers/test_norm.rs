// `norm` is a synonym of `abs` (card g-1pvQ)
use crate::is;

#[test]
fn norm_is_abs() {
	is!("norm(-3)", 3);
	is!("norm -2.5", 2.5);
	is!("x = -7; norm(x) + abs(x)", 14);
}
