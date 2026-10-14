//! `type()` of a time names its kind as wiki/time.md does (card time-type-name): `type(now)` is instant, not data
use crate::is;

#[test]
fn test_time_type_names() {
	is!("type(now)", "instant");
	is!("t = now; type(t)", "instant");
	is!("type(2024-06-01T12:00Z)", "instant");
	is!("type(2024-06-01)", "date");
	is!("type(1 day)", "duration");
	is!("type(2024-01-31T10:00[Europe/Berlin])", "zoned time");
}
