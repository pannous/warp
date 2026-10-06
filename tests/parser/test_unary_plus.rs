// A leading plus glued to its operand is the unary plus: the operand itself (`+5`, `+x`, `3 + +2`), as in Python and JS
use warp::{ints, is};

#[test]
fn a_unary_plus_is_its_operand() {
	is!("+5", 5);
	is!("+2.5", 2.5);
	is!("x = +3; x", 3);
	is!("3 + +2", 5);
	is!("x=4; +x", 4);
	is!("+(2*3)", 6);
	is!("[+1 -2]", ints(vec![1, -2]));
}
