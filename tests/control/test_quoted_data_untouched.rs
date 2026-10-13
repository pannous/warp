// card data-quote-lowered: quoted data stays as written, the lowering passes do not rewrite inside it, so it equals
// the run-time parse of the same text and code patterns with dots match
use crate::is;

#[test]
fn a_member_read_stays_a_dot() {
	is!("data (a.warp) == parse_code(\"a.warp\")", true);
	is!("x = data(a.b.c); x.op", ".");
	is!("match data(a.warp) { data($x.warp) => x }", "a");
}

#[test]
fn library_sugar_stays_as_written() {
	is!("data(a ?? b) == parse_code(\"a ?? b\")", true);
	is!("data(x is int) == parse_code(\"x is int\")", true);
	is!("data(√4) == parse_code(\"√4\")", true);
}
