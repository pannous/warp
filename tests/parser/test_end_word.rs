//! The word end: Ruby's/Lua's block end, unless the program names something end (card end-variable)
use crate::is;

#[test]
fn a_name_end_stays_a_name() {
	// was: else {ø}, printed nothing
	is!("f(a, end) := { if a == 0 then 1 else end }; f(1, 5)", 5);
	is!("end = 4; if end > 3 then end else 0", 4);
}

#[test]
fn end_still_closes_blocks() {
	is!("x = if 1 > 2 then 1 else 2 end; x", 2);
	is!("def f(x)\n  if x > 1 then\n    \"big\"\n  else\n    \"small\"\n  end\nend\nf(3)", "big");
}
