//! Quantities show without a space between amount and unit (user 2026-10-08, card units-nospace): 500m, 9.81m/s², 3km.
//! The shown form reads back as the same quantity: a finite decimal amount shows as the decimal, any other fraction in
//! parentheses, `(23/18)m/s`
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

fn assert_round_trip(code: &str, expected: &str) {
	assert_eq!(shown(code), expected, "{code}");
	assert_eq!(shown(expected), expected, "{expected} reads back");
}

#[test]
fn test_constant_quantities_show_glued_and_read_back() {
	assert_round_trip("3 km", "3km");
	assert_round_trip("9.81 m/s²", "9.81m/s²");
	assert_round_trip("1.5 km", "1.5km");
	assert_round_trip("1 km/h + 1 m/s", "(23/18)m/s");
	assert_round_trip("-3 m", "-3m");
	assert_round_trip("6 m / 2 s", "3m/s");
}

#[test]
fn test_run_time_quantities_show_glued_and_read_back() {
	assert_round_trip("total = 0 m; for i in 1..3 { total += 250 m }; total", "500m");
	assert_round_trip("p = {dist: 0 km, name: \"run\"}; for i in 1..3 { p.dist += 250 m }; p", "{dist:500m name:\"run\"}");
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 2 m }; \"${d}\""), "\"4m\"");
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 2 m }; d.serialize()"), "\"4m\"");
}
