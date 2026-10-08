// A variable given values of two kinds (a list, then an Int) holds what it was given last: it is held as a Node and
// computes by the kind it has at run time (P45). It kept its first kind: `x = 5; x = [1]; x` was 1, a text then an Int
// a cast failure
use crate::is;
use warp::*;

#[test]
fn a_variable_holds_its_last_value_of_another_kind() {
	// P45 (user, 2026-10-05): written kinds that do not mix are a compile error (test_variable_kind_clash.rs)
	assert!(matches!(warp::wasm_emitter::eval("x = 5; x = [1]; x"), Node::Error(_)));
	assert!(matches!(warp::wasm_emitter::eval("x = [1,2]; x = 5; x + 1"), Node::Error(_)));
	assert!(matches!(warp::wasm_emitter::eval("x = \"a\"; x = 5; x + 1"), Node::Error(_)));
}

#[test]
fn a_loop_variable_may_reuse_the_name_of_a_list() {
	is!("xs = [1 2]; s = 0; for xs in [5 6] { s += xs }; s", 11);
}

#[test]
fn a_loop_variable_may_reuse_the_name_of_a_number_and_read_fields() {
	// samples/natural.wasp: `item = 2` earlier, then `for each item in basket { … item.price }`
	is!("item = 2; basket = [{price: 2} {price: 3}]; t = 0; for item in basket { t = t + item.price }; t", 5);
	is!("item = 2; basket = [{price: 2} {price: 3}]; t = 0; for each item in basket { t = t + item.price }; t", 5);
	is!("p = 2; people = [{age: 7}]; p = people#1; p.age", 7);
}
