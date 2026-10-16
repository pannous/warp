// decision units-library (card standalone-math-units): the SI prefixes, which dimensions take them and the long names of
// conversion targets are lines of lib/units.warp's unit_definitions block, not Rust tables
use crate::common::fails_with;
use crate::is;

#[test]
fn each_prefix_line_is_a_power_of_ten() {
	is!("1 Gs == 1000000000 s", true);
	is!("1000 µg == 1 mg", true);
	is!("1 dam == 10 m", true);
}

#[test]
fn only_an_si_dimension_takes_prefixes() {
	is!("1 kK == 1000 K", true);
	fails_with("3 kpx", "undefined: kpx");
}

#[test]
fn a_name_line_is_a_conversion_target() {
	is!("2 hours in minutes == 120 min", true);
	is!("3 feet in inches == 36 inch", true);
}
