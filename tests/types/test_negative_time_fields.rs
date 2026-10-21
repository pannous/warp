//! Card type-zoned: a time's negative amounts (a zone west of UTC, a duration back in time) keep their kind, made
//! by the compiler or at run time
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_zone_west_of_utc_is_a_zoned_time() {
	is!("t = 2024-03-10T06:59Z; type(t in \"America/New_York\")", "zoned time");
	is!("type(now in \"America/New_York\")", "zoned time");
	is!("z = now in \"America/New_York\"; type(z + 1 day)", "zoned time");
	assert_eq!(shown("t = 2024-03-10T06:59Z; t in \"America/New_York\""), "2024-03-10T01:59-05:00[America/New_York]");
	assert_eq!(
		shown("z = 2024-03-10T06:59Z in \"America/New_York\"; z + 1 day"),
		"2024-03-11T01:59-04:00[America/New_York]"
	);
}

#[test]
fn a_duration_back_in_time_is_a_duration() {
	is!("type(0 days - 2 months)", "duration");
	is!("f(x) := x - 5 months; type(f(3 days))", "duration");
	assert_eq!(shown("0 days - 2 months - 3 days"), "-2 months -3 days");
	assert_eq!(shown("f(x) := x - 5 months; f(3 days)"), "-5 months 3 days");
	assert_eq!(shown("2024-05-01 + (0 days - 2 months)"), "2024-03-01");
}
