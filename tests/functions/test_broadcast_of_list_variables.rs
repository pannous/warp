//! card round-floor-xs: a broadcast over a list variable is a list for the word around it and for the variable it is
//! assigned to
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_word_around_a_broadcast_of_a_list_variable_maps_too() {
	assert_eq!(printed("xs = [1.5 2.5]; round(floor(xs))"), "[1 2]");
	assert_eq!(printed("xs = [1 9]; round(log(xs, 3))"), "[0 2]");
}

#[test]
fn a_broadcast_of_a_list_variable_assigned_is_a_list_variable() {
	assert_eq!(printed("xs = [1.5 2.5]; ys = floor(xs); round(ys)"), "[1 2]");
	assert_eq!(printed("xs = [1 9]; ys = log(xs, 3); round(ys)"), "[0 2]");
	assert_eq!(printed("add(a, b) := a + b; xs = [1 2]; ys = add(xs, 10); ys.count"), "2");
}
