use crate::is;
use warp::util::with_fuel;

// 3000 comparisons of small ratios: ~420k steps with exact_cmp's i64 shortcut, ~1.5M through the general path
const COMPARES: &str = "n = 0; x = 1/3; y = 2/7; for i in 1..3000 { if y < x { n += 1 } }; n";
const SHORTCUT_BUDGET: u64 = 800_000;

#[test]
fn small_ratios_compare_in_i64_when_the_program_calculates() {
	with_fuel(SHORTCUT_BUDGET, || is!(COMPARES, 2999));
}
