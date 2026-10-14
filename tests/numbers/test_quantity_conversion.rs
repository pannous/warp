//! card quantity-falls: a run-time quantity (`5 m ± 1 cm` the program needs at run time, `quantity("5 km")`) converts
//! with `as` and `in` like a static one, and joins a text as its text
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().trim_matches('"').to_string()
}

#[test]
fn a_run_time_quantity_converts_with_as() {
	assert_eq!(shown("rope = 5 m ± 1 cm\nrope as cm"), "500.0 ± 1.0cm");
	assert_eq!(shown("rope = 5 m ± 1 cm\n(rope / 2 s) as km/h"), "9.000 ± 0.018km/h");
	assert_eq!(shown("q = quantity(\"5 km\")\nq in m"), "5000m");
	assert_eq!(shown("q = quantity(\"5 km\")\n(q * 2) as km"), "10km");
}

#[test]
fn a_run_time_quantity_joins_a_text() {
	assert_eq!(shown("q = quantity(\"5 km\")\n\"q: \" + q"), "q: 5km");
	assert_eq!(shown("rope = 5 m ± 1 cm\n\"r: \" + (rope as cm)"), "r: 500.0 ± 1.0cm");
	assert_eq!(shown("q = quantity(\"5 km\")\nw = q.to(\"m\")\n\"q: \" + w + \"!\""), "q: 5000m!");
}
