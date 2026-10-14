//! Time zones at run time (card run-time-dates, stage B3, notes/dates_at_run_time.md): the offset of a zone under its
//! daylight saving rules, `t in "Zone"` of a time only the run knows, a wall time a transition skips or repeats, and
//! arithmetic on a zoned time. Each answer is the one the compile-time evaluator gives for the same constants.
use crate::common::fails_with;
use crate::is;

/// `t` placed in `zone` shows as `expected`
fn placed(instant: &str, zone: &str, expected: &str) {
	is!(&format!("t = {instant}; print t; str(t in \"{zone}\")"), expected);
}

#[test]
fn test_offset_follows_the_europe_rules() {
	placed("2024-01-15T10:00:00Z", "Europe/Berlin", "2024-01-15T11:00+01:00[Europe/Berlin]");
	placed("2024-07-01T10:00:00Z", "Europe/Berlin", "2024-07-01T12:00+02:00[Europe/Berlin]");
	placed("2024-03-31T00:59:59Z", "Europe/London", "2024-03-31T00:59:59+00:00[Europe/London]");
	placed("2024-03-31T01:00:00Z", "Europe/London", "2024-03-31T02:00+01:00[Europe/London]");
	placed("2024-10-27T00:59:00Z", "Europe/Berlin", "2024-10-27T02:59+02:00[Europe/Berlin]");
	placed("2024-10-27T01:00:00Z", "Europe/Berlin", "2024-10-27T02:00+01:00[Europe/Berlin]");
}

#[test]
fn test_offset_follows_the_united_states_rules() {
	placed("2024-03-10T06:59:00Z", "America/New_York", "2024-03-10T01:59-05:00[America/New_York]");
	placed("2024-03-10T07:00:00Z", "America/New_York", "2024-03-10T03:00-04:00[America/New_York]");
	placed("2024-11-03T05:59:00Z", "America/New_York", "2024-11-03T01:59-04:00[America/New_York]");
	placed("2024-11-03T06:00:00Z", "America/New_York", "2024-11-03T01:00-05:00[America/New_York]");
	placed("2024-07-01T12:00:00Z", "Asia/Kolkata", "2024-07-01T17:30+05:30[Asia/Kolkata]");
}

#[test]
fn test_a_zoned_time_changes_zone_keeping_its_instant() {
	is!(
		"z = 2024-07-01T12:00+02:00[Europe/Berlin]; print z; str(z in \"America/Los_Angeles\")",
		"2024-07-01T03:00-07:00[America/Los_Angeles]"
	);
	is!("t = 2024-07-01T10:00:00Z; z = t in \"Asia/Kolkata\"; print z; z.offset", 19800);
}

#[test]
fn test_a_local_time_in_a_zone_resolves() {
	is!("t = 2024-07-01T10:00; print t; str(t in \"Europe/Berlin\")", "2024-07-01T10:00+02:00[Europe/Berlin]");
}

#[test]
fn test_a_skipped_wall_time_is_rejected_unless_chosen() {
	fails_with("t = 2024-03-31T02:30; print t; t in \"Europe/Berlin\"", "does not exist");
	is!(
		"t = 2024-03-31T02:30; print t; str(zoned(t, \"Europe/Berlin\", disambiguation: earlier))",
		"2024-03-31T01:30+01:00[Europe/Berlin]"
	);
	is!(
		"t = 2024-03-31T02:30; print t; str(zoned(t, \"Europe/Berlin\", disambiguation: later))",
		"2024-03-31T03:30+02:00[Europe/Berlin]"
	);
}

#[test]
fn test_a_repeated_wall_time_is_rejected_unless_chosen() {
	fails_with("t = 2024-10-27T02:30; print t; t in \"Europe/Berlin\"", "ambiguous");
	is!(
		"t = 2024-10-27T02:30; print t; str(zoned(t, \"Europe/Berlin\", disambiguation: earlier))",
		"2024-10-27T02:30+02:00[Europe/Berlin]"
	);
	is!(
		"t = 2024-10-27T02:30; print t; str(zoned(t, \"Europe/Berlin\", disambiguation: later))",
		"2024-10-27T02:30+01:00[Europe/Berlin]"
	);
}

#[test]
fn test_a_calendar_day_keeps_the_wall_time_across_a_transition() {
	is!("z = 2024-03-30T12:00[Europe/Berlin]; print z; str(z + 1 day)", "2024-03-31T12:00+02:00[Europe/Berlin]");
	is!("z = 2024-03-31T12:00[Europe/Berlin]; print z; str(z - 1 day)", "2024-03-30T12:00+01:00[Europe/Berlin]");
	is!(
		"z = 2024-01-31T10:00[Europe/Berlin]; n = 6; print n; str(z + n * 1 month)",
		"2024-07-31T10:00+02:00[Europe/Berlin]"
	);
}

#[test]
fn test_exact_time_keeps_the_elapsed_time_across_a_transition() {
	is!("z = 2024-03-30T12:00[Europe/Berlin]; print z; str(z + 24 hours)", "2024-03-31T13:00+02:00[Europe/Berlin]");
	is!("z = 2024-03-31T01:30[Europe/Berlin]; print z; str(z + 1 hour)", "2024-03-31T03:30+02:00[Europe/Berlin]");
}

#[test]
fn test_a_day_landing_on_a_skipped_wall_time_is_rejected() {
	fails_with("z = 2024-03-30T02:30[Europe/Berlin]; print z; z + 1 day", "does not exist");
}

#[test]
fn test_zoned_times_subtract_to_elapsed_time() {
	is!("a = 2024-03-31T12:00[Europe/Berlin]; b = 2024-03-30T12:00[Europe/Berlin]; print a; str(a - b)", "23 hours");
	is!("a = 2024-07-01T12:00[Europe/Berlin]; b = 2024-07-01T12:00[Europe/London]; print a; str(a - b)", "-1 hours");
}
