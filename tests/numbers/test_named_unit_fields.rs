//! card map-units: an instance written with named fields, `Run{distance: 5 km}`, is an instance static units sees, as
//! `Run(5 km)` is: its unit fields keep their unit through a list, a map and a sum, and a field given another dimension or
//! a plain number is a DimensionError
use crate::common::fails_with;
use warp::wasm_emitter::eval;

const RUNS: &str = "class Run{distance: km}\nruns = [Run{distance: 5 km}, Run{distance: 7 km}]";

fn shown(rest: &str) -> String {
	eval(&format!("{RUNS}\n{rest}")).serialize().trim().to_string()
}

#[test]
fn a_map_over_named_instances_keeps_the_unit() {
	assert_eq!(shown("sum(runs.map(r => r.distance))"), "12km");
	assert_eq!(shown("average(runs.map(r => r.distance))"), "6km");
	assert_eq!(shown("max(runs.map(r => r.distance))"), "7km");
	assert_eq!(shown("runs#1.distance"), "5km");
}

#[test]
fn a_named_field_checks_its_dimension() {
	fails_with("class Run{distance: km}\nRun{distance: 5 kg}", "DimensionError");
	fails_with("class Run{distance: km}\nRun{distance: 5}", "DimensionError");
}
