//! A unit field's value shows in its declared unit (card unit-value): `Run(1500 m)` stores into a `distance: km` field,
//! so the m it was written in is no display unit of the program; a loop's one-line `{ print r.distance }` and a list
//! mapped from the field (`runs.map(r => r.distance)`) keep the unit instead of showing SI amounts
use warp::wasm_emitter::eval;

const RUNS: &str = "class Run{distance: km}\nruns: [Run] = [Run(5 km), Run(1500 m), Run(12 km)]";

fn shown(rest: &str) -> String {
	eval(&format!("{RUNS}\n{rest}")).serialize().trim().to_string()
}

#[test]
fn a_field_read_shows_the_declared_unit() {
	assert_eq!(shown("runs#2.distance"), "1.5km");
	assert_eq!(shown("total = 0 km\nfor run in runs { total += run.distance }\ntotal"), "18.5km");
}

#[test]
#[cfg(feature = "native")]
fn a_one_line_loop_body_prints_the_unit() {
	let output = crate::common::printed(&format!("{RUNS}\nfor run in runs {{ print run.distance }}\n0"));
	assert_eq!(output.lines().take(3).collect::<Vec<_>>(), ["5km", "1.5km", "12km"]);
}

#[test]
fn a_list_mapped_from_a_unit_field_keeps_the_unit() {
	assert_eq!(shown("max(runs.map(r => r.distance))"), "12km");
	assert_eq!(shown("sum(runs.map(run => run.distance))"), "18.5km");
	assert_eq!(shown("distances = runs.map(r => r.distance)\ndistances#2"), "1.5km");
}
