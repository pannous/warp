// card fourth-root: ∜ next to √ and ∛, the square root of the square root
use crate::is;

#[test]
fn fourth_root_glyph() {
	is!("∜16", 2);
	is!("∜(81)", 3);
	is!("x = 625; ∜x", 5);
	is!("∜(16*81) + ∜1", 7);
}
