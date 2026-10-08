// Postfix ² and ³ raise the whole place they follow, as ^2 does: p.x² is (p.x)², xs#2² is (xs#2)²; prefix √ binds looser
// than them, so √x² stays √(x²), and tighter than ^, so √x^2 stays (√x)^2 (card field-square)
use crate::is;

#[test]
fn a_field_or_element_squares() {
	is!("p = {x: 3}; p.x²", 9);
	is!("xs = [1 3]; xs#2²", 9);
	is!("p = {x: 2}; p.x³", 8);
	is!("p = {x: 3}; 2 * p.x²", 18);
}

#[test]
fn a_root_takes_the_squared_place() {
	is!("x = -3; √x²", 3); // √(x²): (√-3)² has no real value
	is!("x = 9; √x^2", 9);
	is!("x = 4; √x*2", 4);
	is!("p = {x: 4}; √p.x", 2);
	is!("x = 3; -x²", -9);
}
