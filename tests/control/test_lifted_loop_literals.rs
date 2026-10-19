// card loop-literal-list: a loop walking a literal of constants inside another loop builds it once, before the outer loop
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_walked_literal_is_built_once_before_the_outer_loop() {
	let code = "t = 0\nfor i in 0..3 { for k in [16, 8, 0] { t += k } }\nt";
	is!(code, 72);
}

#[test]
fn lifted_literals_keep_each_loop_its_own() {
	is!(
		"out = []\nfor i in 0..2 { for s in [1, 2] { out.add(s) }; for s in [3, 4] { out.add(s * 10) } }\nout",
		parse("[1 2 30 40 1 2 30 40]")
	);
	is!("f(n) := { t = 0; for i in 0..n { for k in [1, 2, 3] { t += k } }; t }\n[f(2), f(3)]", parse("[12 18]"));
	// a literal of lists may change through its items: built each round
	is!(
		"out = []\nfor i in 0..3 { for row in [[1], [2]] { row.add(i); out.add(count(row)) } }\nout",
		parse("[2 2 2 2 2 2]")
	);
}
