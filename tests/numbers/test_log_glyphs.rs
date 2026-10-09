//! P226 (user, 2026-10-09): the log glyphs differ. `b⌞x` is log base b of x, `x⌟` the natural log, and (default until
//! P226b) `x⌟b` log base b of x.
use crate::is;

#[test]
fn the_lower_corner_before_a_number_is_its_log_to_that_base() {
	is!("10⌞100", 2);
	is!("2⌞8", 3);
	is!("use math; 10⌞100", 2);
	is!("10⌞100 + 1", 3);
}

#[test]
fn the_lower_corner_after_a_number_is_its_log() {
	is!("ℯ⌟", 1);
	is!("x = ℯ*ℯ; x⌟", 2);
	is!("100⌟10", 2);
}
