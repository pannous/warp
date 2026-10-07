//! `x.string`, `t.int`, `x.float()`: a conversion written as a method is its call `string(x)` (card string-str)
use crate::is;

#[test]
fn conversions_as_methods() {
	is!("x = 1; x.string + \"!\"", "1!");
	is!("x = \"4\"; x.int + 1", 5);
	is!("x = 3; x.float / 2", 1.5);
	is!("x = 3; x.str() + \"a\"", "3a");
	is!("p = {int: 4}; p.int", 4);
}
