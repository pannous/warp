//! lib/time.warp's time zones (card time-zones-warp): zone_offset and zone_instant in warp, twins of calendar.rs
//! Zone::offset_at and Zone::resolve
use crate::common::fails_with;
use crate::is;
use warp::time::calendar::{Date, ZONES};

const DAY_SECONDS: i64 = 86_400;

const DAY_COUNTS: [(i64, i64, i64); 4] = [(1970, 1, 1), (2026, 3, 29), (2026, 7, 1), (2026, 11, 1)];
const SECONDS_INTO_DAY: [i64; 3] = [0, 3 * 3600, 23 * 3600];

#[test]
fn zone_offsets_match_the_rust_calendar_in_every_zone() {
	for (year, month, day) in DAY_COUNTS {
		for into_day in SECONDS_INTO_DAY {
			let seconds = Date { year, month, day }.days() * DAY_SECONDS + into_day;
			let offsets = ZONES.iter().map(|zone| zone.offset_at(seconds as i128 * 1_000_000_000) as i32).collect();
			is!(&format!("use time; [zone_offset(zone, {seconds}) for zone in time_zones]"), warp::ints(offsets));
		}
	}
}

#[test]
fn daylight_saving_starts_on_the_last_sunday_of_march_at_one_utc() {
	is!("use time; zone_offset(zone_named(\"Europe/Berlin\"), days_from_date(2026, 3, 29) * 86400 + 3599)", 3600);
	is!("use time; zone_offset(zone_named(\"Europe/Berlin\"), days_from_date(2026, 3, 29) * 86400 + 3600)", 7200);
	is!("use time; zone_offset(zone_named(\"America/New_York\"), days_from_date(2026, 7, 1) * 86400)", -14400);
	is!("use time; zone_offset(zone_named(\"Asia/Tokyo\"), days_from_date(2026, 7, 1) * 86400)", 32400);
}

#[test]
fn zone_instant_resolves_wall_times() {
	let berlin = "use time; berlin = zone_named(\"Europe/Berlin\")";
	is!(&format!("{berlin}; zone_instant(berlin, 43200, Disambiguation::Reject)"), 39600);
	// 02:30 on 2026-03-29 is skipped: earlier and later are the wall time read in summer and in winter time
	let skipped = "days_from_date(2026, 3, 29) * 86400 + 9000";
	is!(&format!("{berlin}; zone_instant(berlin, {skipped}, Disambiguation::Earlier) - ({skipped})"), -7200);
	is!(&format!("{berlin}; zone_instant(berlin, {skipped}, Disambiguation::Later) - ({skipped})"), -3600);
	fails_with(&format!("{berlin}; zone_instant(berlin, {skipped}, Disambiguation::Reject)"), "does not exist");
	// 02:30 on 2026-10-25 occurs twice
	let repeated = "days_from_date(2026, 10, 25) * 86400 + 9000";
	is!(&format!("{berlin}; zone_instant(berlin, {repeated}, Disambiguation::Earlier) - ({repeated})"), -7200);
	is!(&format!("{berlin}; zone_instant(berlin, {repeated}, Disambiguation::Later) - ({repeated})"), -3600);
	fails_with(&format!("{berlin}; zone_instant(berlin, {repeated}, Disambiguation::Reject)"), "ambiguous");
}

#[test]
fn an_unknown_zone_is_named_in_the_error() {
	fails_with("use time; zone_named(\"Mars/Olympus\")", "unknown time zone Mars/Olympus");
}
