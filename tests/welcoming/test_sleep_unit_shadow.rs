//! card sleep-unit: a variable named like one unit (`m` meter, `mm`, `s`) leaves the other units of sleep durations
//! alone (any of them made `sleep 5 ms` 'undefined variable: ms')
use crate::is;

#[test]
fn a_unit_named_variable_keeps_other_durations() {
	is!("m = 2; sleep 5 ms; m", 2);
	is!("mm = 1; sleep 5 ms; mm", 1);
	is!("s = 1; sleep 5 ms; s", 1);
}
