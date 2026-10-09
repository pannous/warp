// card unit-field: an optional unit field `distance: km?` is a nullable NUMERIC km column: a run without a distance
// stores ø, one with a distance its unit amount, natively and in the browser's table store
use warp::wasm_emitter::eval;

fn runs(rest: &str) -> String {
	format!("class Run{{label: text; distance: km?}}\nruns: [Run] = database.optional_units\n{rest}")
}

fn value(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn an_optional_unit_field_holds_a_distance_or_nothing() {
	eval(&runs("runs.add(Run(\"park\", 5 km))\nruns.add(Run(\"rest\", ø))"));
	assert_eq!(value(&runs("runs#1.distance")), "5km");
	assert_eq!(value(&runs("runs#2.distance == ø")), "yes");
	assert_eq!(value(&runs("count(runs where it.distance == ø)")), "1");
}
