//! Class fields typed with a unit (card unit-fields): `class Run{distance: km}` holds its amount like any quantity, a
//! value of another quantity or a plain number is a DimensionError
use crate::common::fails_with;
use warp::wasm_emitter::eval;

const RUN: &str = "class Run{distance: km; note: text}";

fn shown(rest: &str) -> String {
	eval(&format!("{RUN}\n{rest}")).serialize().trim().to_string()
}

#[test]
fn a_unit_field_holds_a_quantity() {
	assert_eq!(shown("Run(5 km, \"park\").distance"), "5km");
	assert_eq!(shown("r = Run(5 km, \"park\")\nr.distance += 500 m\nr.distance"), "5500m");
}

#[test]
fn a_unit_field_refuses_another_quantity_or_a_plain_number() {
	fails_with(&format!("{RUN}\nRun(5 s, \"x\").distance"), "Run.distance holds m");
	fails_with(&format!("{RUN}\nRun(5, \"x\").distance"), "Run.distance holds m");
}

#[test]
fn unit_fields_add_up_in_loops_over_lists_of_the_class() {
	assert_eq!(shown("runs: [Run] = [Run(1 km, \"a\"), Run(2 km, \"b\")]\ntotal = 0 m\nfor r in runs { total += r.distance }\ntotal"), "3000m");
	assert_eq!(shown("rs = [Run(3 km, \"a\"), Run(2 km, \"b\")]\nrs#1.distance"), "3km");
}
