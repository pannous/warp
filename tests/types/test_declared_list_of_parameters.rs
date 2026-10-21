//! card declared-list: a declared list of an untyped parameter's values (`ys: ints = [x, x]`) is checked when it runs,
//! not refused while compiling as a list of symbols
use crate::is;

#[test]
fn a_declared_list_of_untyped_parameters() {
	is!("h(x) := { ys: ints = [x, x]; ys }\n#h(3)", 2);
	is!("h(x) := { ys: texts = [x]; ys#1 }\nh(\"a\")", "a");
	is!("h(x: int) := { ys: ints = [x, x]; ys#2 }\nh(4)", 4);
}
