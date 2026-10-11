//! Stage B4 of card run-time-dates (notes/dates_at_run_time.md): no program is answered whole at compile time; its
//! constant time expressions are folded and the rest runs, a printing program as a constant one
use crate::is;

#[test]
fn a_zoned_times_rules_version_is_read_at_run_time() {
	is!("t = 2030-07-01T10:00[Europe/Berlin]; print t; t.tzdata", "warp-2026a");
}

#[test]
fn a_duration_in_a_unit_is_its_quantity_in_a_running_program() {
	is!("print 1; str(1 day in h)", "24h");
}

#[test]
fn a_placement_in_parentheses_has_fields() {
	is!("t = 2030-07-01T10:00Z; print t; (t in \"Europe/Berlin\").hour", 12);
}
