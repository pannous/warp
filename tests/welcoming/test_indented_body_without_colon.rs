// cards golf-indented, golf-bare, golf-indented-while, golf-indented-swap: the lines indented below a loop head or a
// `:=` that ends its line are its body, as below a trailing `:` (notes/code_golf.md)
use crate::is;

/// `code` is written with tabs; it gives `value` indented by tabs and by spaces alike
fn indented_either_way(code: &str, value: i64) {
	is!(code, value);
	is!(code.replace('\t', "  ").as_str(), value);
}

#[test]
fn a_bare_for_takes_its_indented_lines() {
	indented_either_way("s = 0\nfor 1 to 3\n\ts += it\ns", 6);
}

#[test]
fn a_for_in_takes_its_indented_if() {
	indented_either_way("s = 0\nfor n in 1 to 3\n\tif n > 1 then s += n\ns", 5);
}

#[test]
fn a_while_takes_all_its_indented_lines() {
	indented_either_way("n = 8\nk = 0\nwhile n > 1\n\tn = n / 2\n\tk += 1\nk", 3);
}

#[test]
fn an_indented_comma_line_swaps() {
	indented_either_way("x = 0; y = 1\nfor i in 0 to 2\n\tx, y = y, x + y\nx", 2);
	indented_either_way("x = 0; y = 1\nfor i in 0 to 2:\n\tx, y = y, x + y\nx", 2);
}

#[test]
fn a_definition_ending_its_line_takes_the_indented_lines() {
	indented_either_way("f(n):=\n\twhile n > 1\n\t\tn = n / 2\n\tn\nf 8", 1);
}
