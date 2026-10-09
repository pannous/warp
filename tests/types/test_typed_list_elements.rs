// A list declared with its element type in brackets checks its items as `ints` and `list of int` do, and a
// fixed-width element type checks its range as the scalar `x: int16 = 70000` does (card typed-list-elements)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_bracketed_list_type_checks_its_items() {
	fails_with("xs: [int] = [1, \"a\"]; xs", "type mismatch");
	fails_with("xs: [text] = [\"a\", 1]; xs", "type mismatch");
	is!("xs: [int] = [1, 2]; xs#2", 2);
}

#[test]
fn a_fixed_width_list_checks_its_range() {
	fails_with("xs: [int16] = [1, 70000]; xs", "int16");
	fails_with("xs: int16s = [1, 70000]; xs", "int16");
	fails_with("xs: int16s = [1]; xs.add(70000); xs", "int16");
	fails_with("xs: [int16] = [1, 7]; xs.add(70000); xs", "int16");
	is!("xs: [int16] = [1, 32767]; xs#2", 32767);
}
