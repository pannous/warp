// card def-moved (particles sample): a list variable reassigned from a broadcast over itself stays a list, so its
// broadcasts map it: `ps = moved ps` was Error "not a number"
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_list_reassigned_from_its_own_broadcast_stays_a_list() {
	assert_eq!(printed("def moved(p){p+1}; ps=[1,2]; ps = moved ps"), "[2 3]");
	assert_eq!(printed("def moved(p){p+1}; ps=[1,2]; ps = moved(ps); ps"), "[2 3]");
	assert_eq!(printed("def moved(p){p+1}; ps=[1,2]; ps = moved ps; ps = moved ps; ps"), "[3 4]");
	assert_eq!(printed("def moved(p){p+1}; ps=[1,2]; for i in 0..3 { ps = moved ps }; ps"), "[4 5]");
}

#[test]
fn a_broadcast_over_a_variable_alone_does_not_make_it_a_list() {
	assert_eq!(printed("def moved(p){p+1}; n = 1; n = moved n; n"), "2");
	assert_eq!(printed("s=\"ab\"; up(x) := { x = upper x; x }; up(s)"), "\"AB\"");
}
