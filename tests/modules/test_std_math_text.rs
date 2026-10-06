//! The standard library modules math and text (notes/stdlib.md)
use crate::is;

#[test]
fn use_math_brings_libm_and_the_module() {
	is!("use math; gcd(12, 18)", 6);
	is!("use math; lcm(4, 6)", 12);
	is!("use math; clamp(15, 0, 10) + clamp(-2, 0, 10) + clamp(5, 0, 10)", 15);
	is!("use math; sign(-3)", -1);
	is!("use math; floor(2.7)", 2);
}

#[test]
fn use_text_brings_its_words() {
	is!("use text; \"[\" + pad_left(\"ab\", 5) + \"]\"", "[   ab]");
	is!("use text; \"[\" + pad_right(\"ab\", 5) + \"]\"", "[ab   ]");
	is!("use text; repeat(\"ab\", 3)", "ababab");
}

#[test]
fn use_text_formats_a_template() {
	is!("use text; format(\"{} has {} items\", [\"cart\", 3])", "cart has 3 items");
	is!("use text; format(\"{}{}\", [1, 2])", "12");
	is!("use text; format(\"none\", [])", "none");
}
