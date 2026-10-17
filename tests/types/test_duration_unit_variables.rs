//! Card time-unit: a time-unit quantity held by a variable meets a duration at run time (`y = 1 h; x + y`), a parameter
//! some call passes a time is a time inside its function, and each function sees its own names (reviewer)
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_quantity_variable_meets_a_duration() {
	assert_eq!(shown("x = 2 hours; y = 1 h; x + y"), "3 hours");
	assert_eq!(shown("x = 2 hours; y = 30 min; y + x"), "2 hours 30 minutes");
	assert_eq!(shown("s = 3; x = 2 hours; y = 1 h; x + y"), "3 hours");
	assert_eq!(shown("y = 1 h; y + 2 h"), "3h");
}

#[test]
fn a_parameter_passed_a_time_is_a_time() {
	assert_eq!(shown("f(x) := x + 1 h; f(now) > now"), "yes");
	assert_eq!(shown("f(x) := x + 1 h; f(2 hours)"), "3 hours");
}

#[test]
fn a_function_sees_its_own_names() {
	assert_eq!(shown("g() := { d = 1 day; d }\nh(d) := d + 2 h\nh(3 h)"), "5h");
}
