//! card infers-int: a parameter that only stands in a mixed list literal (`[b, a, 22050, 0.5]`) gets no kind from the
//! literal's numbers, so the function takes a list, a text or a number there (lib/sound.warp's effected had to declare
//! `amplitudes: list` before)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_parameter_in_a_mixed_list_literal_takes_any_value() {
	let definitions = "g(xs) := xs.count\nf(a, b) := g([b, a, 22050, 0.5])\n";
	is!(&format!("{definitions}f([1 2], 3)"), 4);
	is!(&format!("{definitions}f(\"x\", 3)"), 4);
	is!(&format!("{definitions}f(1, 3)"), 4);
	is!("f(a, b) := [b, a, 22050, 0.5]\nf([1 2], 3)", parse("[3 [1 2] 22050 0.5]"));
}
