//! card field-closure-call: a field holds the function value a parameter was given, `route(path, answer) := {path:
//! path, answer: answer}` called with a lambda, so `r.answer(2)` calls it (what a plain-library router's route records
//! need, notes/server_routes.md)
use crate::is;

const ROUTE: &str = "route(path, answer) := {path: path, answer: answer}\n";

#[test]
fn a_field_given_a_lambda_parameter_is_called() {
	is!(&format!("{ROUTE}r = route(\"/x\", (q) => q + 1)\nr.answer(2)"), 3);
	is!("mk(a) := {answer: a}\nr = mk((q) => q * 2)\nr.answer(4)", 8);
}

#[test]
fn records_in_a_list_call_their_fields() {
	is!(
		&format!(
			"{ROUTE}routes = [route(\"/a\", (q) => q + 1), route(\"/b\", (q) => q * 10)]\nfor r in routes {{ if r.path == \"/b\" {{ return r.answer(3) }} }}\n0"
		),
		30
	);
}
