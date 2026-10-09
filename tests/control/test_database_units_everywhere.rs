// card browser-unit-migrations: the playground's table store migrates a unit column like src/database.rs does: km → m
// keeps the SI amounts, km → kg and km → float are loud errors, plain numbers given a unit are read as it, loudly.
// The same program runs natively and in the browser (tests/control/test_database_tables.rs pins the native run)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn runs(class: &str, table: &str, rest: &str) -> String {
	format!("class Run{{{class}}}\nruns: [Run] = database.{table}\n{rest}")
}

fn value(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_unit_column_migrates_within_its_quantity() {
	eval(&runs("distance: km", "units_everywhere", "runs.add(Run(5 km))\nruns.add(Run(1500 m))"));
	assert_eq!(value(&runs("distance: km", "units_everywhere", "runs#2.distance")), "1.5km");
	assert_eq!(value(&runs("distance: m", "units_everywhere", "runs#1.distance")), "5000m");
	fails_with(&runs("distance: kg", "units_everywhere", "runs#1.distance"), "different quantities");
	fails_with(&runs("distance: float", "units_everywhere", "runs#1.distance"), "the unit would be lost");
}

#[test]
fn plain_numbers_given_a_unit_are_read_as_it() {
	eval(&runs("distance: int", "plain_everywhere", "runs.add(Run(3))"));
	warp::diagnostic::take_runtime_warnings();
	assert_eq!(value(&runs("distance: km", "plain_everywhere", "runs#1.distance")), "3km");
	let warnings = warp::diagnostic::take_runtime_warnings();
	assert!(warnings.iter().any(|warning| warning.contains("read as km")), "{warnings:?}");
}
