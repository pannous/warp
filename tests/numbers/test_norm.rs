// `norm` is a synonym of `abs` (card g-1pvQ)
use crate::is;

#[test]
fn norm_is_abs() {
	is!("use math; norm -2.5", 2.5);
	is!("use math; x = -7; norm(x) + abs(x)", 14);
}
