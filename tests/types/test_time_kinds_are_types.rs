//! Card anchor-time: the kinds of time are types under `time`; `is` tests the subtype, `==` the type itself
use crate::is;

#[test]
fn test_time_kinds_are_types() {
	is!("now is time", true);
	is!("now is instant", true);
	is!("now is date", false);
	is!("1 day is duration", true);
	is!("1 day is time", true);
	is!("2024-06-01 is date", true);
	is!("2024-06-01 is instant", false);
	is!("2024-01-31T10:00[Europe/Berlin] is zoned time", true);
	is!("type(now) is time", true);
	is!("type(now) == time", false);
	is!("type(now) == instant", true);
	is!("3 is time", false);
}
