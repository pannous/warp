//! A function testing its parameter's kind (`x is number`) handles every kind itself: a list argument is passed whole,
//! not broadcast over (card function-whose, found by self-parser)
use crate::is;

#[test]
fn a_kind_tested_parameter_takes_a_list_whole() {
	let neg = r#"def neg(x) { if x is number { return 0 - x }; ["key", "-", ø, x] }; "#;
	is!(&format!(r#"{neg}#neg(["symbol", "x"])"#), 4);
	is!(&format!(r#"{neg}neg(["symbol", "x"])#4#2"#), "x");
	is!(&format!("{neg}neg(3)"), -3);
}
