// cards std-module-swallowed, end-indented, def-indented: an `end` in a comment closes no block; `for … do` reads its
// lines up to `end`, or the lines indented below it; `def f(n)` takes the lines indented below it by spaces or tabs
use crate::is;

/// `code` is written with tabs; it gives `value` indented by tabs and by spaces alike
fn indented_either_way(code: &str, value: i64) {
	is!(code, value);
	is!(code.replace('\t', "  ").as_str(), value);
}

#[test]
fn an_end_in_a_comment_closes_no_else() {
	is!("t(n) := if n == 1 then \"a\" else n\n\n// this would end it\nt(2)", 2);
	is!("t(n) := if n == 1 then \"a\" else n\n\n/* the end */\nu(n) := n + 1\nu(2)", 3);
}

#[test]
fn a_for_do_reads_its_lines_up_to_end() {
	indented_either_way("s = 0\nfor i in 1 to 3 do\n\ts += i\nend\ns", 6);
	indented_either_way("s = 0\nfor i in 1 to 3 do\n\ts += i\ns", 6);
}

#[test]
fn a_def_takes_its_indented_lines() {
	indented_either_way("def f(n)\n\twhile n > 1\n\t\tn = n / 2\n\tn\nf 8", 1);
	is!("def f(x)\n  x + 1\nend\nf(1)", 2);
}
