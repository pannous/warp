// The text of a constant arithmetic expression is the text of its value, not its source: `str(1+2)` is "3"
use crate::is;

#[test]
fn a_constant_expression_converts_by_its_value() {
	is!("str(1+2)", "3");
	is!("(1+2) as string", "3");
	is!("\"\" + (1+2)", "3");
	is!("\"a\" + 2*3", "a6");
	is!("\"v\" + (2^3)", "v8");
	is!("\"h\" + 3/2", "h1.5");
}
