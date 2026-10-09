//! Unit columns as a user edits samples/orm_units.warp (card units-stress): parentheses around a unit field, a column's
//! sum and maximum, and a filter on it keep the unit
use warp::wasm_emitter::eval;

const RUNS: &str = "class Run{distance: km; note: text}\nruns: [Run] = [Run(5 km, \"park\"), Run(1500 m, \"track\"), Run(12 km, \"river\")]";

fn shown(rest: &str) -> String {
	eval(&format!("{RUNS}\n{rest}")).serialize().trim().to_string()
}

#[test]
fn parentheses_keep_a_unit_field() {
	assert_eq!(shown("(runs#2.distance) as m"), "1500m");
	assert_eq!(shown("r = runs#1\n(r.distance + 1 m) as m"), "5001m");
	assert_eq!(shown("d = (runs#3.distance)\nd"), "12km"); // the declared unit (card unit-value)
}

#[test]
fn a_column_sums_and_peaks_in_its_unit() {
	assert_eq!(shown("sum(runs.map(r => r.distance)) as m"), "18500m");
	assert_eq!(shown("distances = runs.map(r => r.distance)\nmax(distances) as km"), "12km");
}

// a number field read off a row is an int or a float only at run time: arithmetic on it widens an exact variable
#[test]
fn a_number_field_adds_into_an_exact_total() {
	assert_eq!(eval("class T{load: number}\nt = T{load: 0.5 as float}\ntotal = 1\ntotal = 1 + t.load\ntotal").serialize().trim(), "1.5");
}

// the stored table keeps SI amounts (5 km as 5000): a where on the column compares in SI, a quantity of another kind
// is a loud error, an exact amount (500 g) is stored as a real
#[cfg(feature = "native")]
#[test]
fn a_stored_unit_column_filters_and_sums() {
	let stored = |rest: &str| eval(&format!("class Run{{distance: km; note: text}}\nruns: [Run] = database.runs_stressed\n{rest}")).serialize().trim().to_string();
	stored("runs.add(Run(5 km, \"park\"))\nruns.add(Run(1500 m, \"track\"))\nruns.add(Run(12 km, \"river\"))");
	assert_eq!(stored("count(runs where it.distance > 5 km)"), "1");
	assert_eq!(stored("count(runs where it.distance > 4000 m)"), "2");
	assert_eq!(stored("count(runs where it.distance * 2 > 5 km)"), "2");
	assert_eq!(stored("sum(runs.map(r => r.distance)) as m"), "18500m");
	crate::common::fails_with("class Run{distance: km; note: text}\nruns: [Run] = database.runs_stressed\ncount(runs where it.distance > 5 s)", "compared with");
	let parcels = |rest: &str| eval(&format!("class Parcel{{weight: kg}}\nparcels: [Parcel] = database.parcels_exact\n{rest}")).serialize().trim().to_string();
	parcels("parcels.add(Parcel(3 kg))\nparcels.add(Parcel(500 g))");
	assert_eq!(parcels("sum(parcels.map(p => p.weight)) as g"), "3500g");
	let loads = |rest: &str| eval(&format!("class T{{load: number}}\nts: [T] = database.loads_mixed\n{rest}")).serialize().trim().to_string();
	loads("ts.add(T(3))\nts.add(T(0.5))");
	assert_eq!(loads("total = 0\nfor t in ts { total = total + t.load }\ntotal"), "3.5");
}

// `as` takes a compound unit whole: `speed as km/h` (card compound-unit)
#[test]
fn a_conversion_takes_a_compound_unit_whole() {
	assert_eq!(eval("x = 10 km / 50 min\nx as km/h").serialize().trim(), "12km/h");
	assert_eq!(shown("r = runs#1\n(r.distance / 25 min) as km/h"), "12km/h");
}
