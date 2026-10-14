//! `mean`, also spelled `average`, is a prelude word: no `use list` (card average-undefined); the average of quantities
//! has their unit, of a list literal and of a unit column alike
use warp::wasm_emitter::eval;

fn shown(program: &str) -> String {
	eval(program).serialize().trim().to_string()
}

#[test]
fn average_needs_no_use() {
	assert_eq!(shown("mean([1, 2, 3, 4])"), "2.5");
	assert_eq!(shown("average([1, 2, 3, 4])"), "2.5");
	assert_eq!(shown("[1, 2, 3, 4].average()"), "2.5");
}

#[test]
fn the_average_of_quantities_has_their_unit() {
	assert_eq!(shown("average([2 h, 3 h])"), "2.5h");
	assert_eq!(shown("sum([5 km, 1.5 km])"), "6.5km");
	let runs = "class Run{distance: km}\nruns: [Run] = [Run(5 km), Run(1.5 km), Run(2.5 km)]";
	assert_eq!(shown(&format!("{runs}\naverage(runs.map(r => r.distance))")), "3km");
}

#[test]
fn a_program_may_define_its_own_mean() {
	assert_eq!(shown("mean(xs) := 42\nmean([1, 2])"), "42");
}

/// A function's float variable captured by a closure stays a float in it
#[test]
fn a_captured_float_stays_a_float() {
	assert_eq!(shown("g() := { m = float(2.5); s(x => x * m) }\ns(f) := f(1)\ng()"), "2.5");
	assert_eq!(shown("m = float(2.5)\ns(f) := f(1)\ns(x => x * m)"), "2.5");
}
