//! Static units, stage 3: quantities computed at run time print, join texts, convert with `as`/`in` and are returned from
//! functions with their units
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

/// The first line the warp binary prints (native only: the browser build runs no binary)
#[cfg(feature = "native")]
fn printed(code: &str) -> String {
	crate::common::printed(code).lines().next().unwrap_or_default().to_string()
}

#[test]
#[cfg(feature = "native")]
fn test_print_shows_the_unit() {
	assert_eq!(printed("total = 0 m; for i in 1..3 { total += 250 m }; print total"), "500 m");
	assert_eq!(printed("d = 0 km; for i in 1..2 { d += 10 km }; print d / 4 h"), "2.5 km/h"); // str of 5/2 is its decimal
}

#[test]
fn test_texts_join_quantities() {
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 2 m }; \"distance \" + d"), "\"distance 4 m\"");
}

#[test]
fn test_as_and_in_convert_and_check() {
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 500 m }; d as km"), "1 km");
	assert_eq!(shown("d = 0 m; for i in 1..3 { d += 50 cm }; d in m"), "1 m");
	#[cfg(feature = "native")]
	assert_eq!(printed("d = 0 m; for i in 1..3 { d += 500 m }; print d as km"), "1 km");
	fails_with("d = 0 m; for i in 1..3 { d += 500 m }; d as kg", "DimensionError");
}

#[test]
fn test_return_gives_the_unit() {
	assert_eq!(shown("half(x) := { return x / 2 }; v = half(3 m); for i in 1..2 { v = v }; v"), "3/2 m");
	fails_with("pick(x) := { if x > 1 m { return x }; return 2 kg }; v = pick(3 m); for i in 1..2 { v = v }; v", "DimensionError");
}
