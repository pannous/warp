//! The fields of a time at run time come from lib/prelude.warp time_field_of (card time-fields-warp): the emitter's
//! time_field finds the wall clock position, the warp word its calendar
use crate::is;

/// The prelude word itself: form 1 is a date, its position in days since 1970; 0 an instant in nanoseconds
#[test]
fn test_time_field_of_a_position() {
	is!("time_field_of(1, 19782, 0, 0)", 2024); // 2024-02-29
	is!("time_field_of(1, 19782, 0, 1)", 2);
	is!("time_field_of(1, 19782, 0, 2)", 29);
	is!("time_field_of(1, 19782, 0, 3)", 4); // a Thursday
	is!("time_field_of(1, 19782, 0, 4)", 60);
	is!("y = time_field_of(1, -1, 0, 0); y * 10000 + time_field_of(1, -1, 0, 1) * 100 + time_field_of(1, -1, 0, 2)", 19691231);
	is!("time_field_of(1, -1, 0, 3)", 3); // a Wednesday
	// 2024-01-31T10:30:15Z
	is!("t = 1706697015000000000; time_field_of(0, t, 0, 5) * 3600 + time_field_of(0, t, 0, 6) * 60 + time_field_of(0, t, 0, 7)", 37815);
	is!("time_field_of(0, 1706697015000000000, 0, 8)", 1706697015);
	is!("time_field_of(0, -1000000000, 0, 5)", 23); // 1969-12-31T23:59:59
}

/// The calendar's edges through a date at run time: century years, the last day of a leap year, before 1970
#[test]
fn test_calendar_edges_at_run_time() {
	is!("d = 1900-03-01; print d; d.day_of_year", 60); // 1900 is no leap year
	is!("d = 2000-12-31; print d; d.day_of_year", 366);
	is!("d = 1969-12-31; print d; d.year * 10000 + d.month * 100 + d.day", 19691231);
	is!("d = 1600-02-29; print d; d.weekday", 2); // a Tuesday
}
