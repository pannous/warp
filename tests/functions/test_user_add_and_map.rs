// A user function named like a list method (`add`, `append`) leaves map and filter alone: their loops appended with
// `out.add(…)`, which then called the user's add ('lists only concatenate with lists')
use warp::wasm_emitter::eval;

#[test]
fn map_and_filter_beside_a_user_add() {
	assert_eq!(eval("def add(a, b){ a + b }; map([1,2], x => x * 10)").serialize(), "[10 20]");
	assert_eq!(eval("def add(a, b){ a + b }; map([1,2], x => add(x, 10))").serialize(), "[11 12]");
	assert_eq!(eval("def append(a, b){ a }; filter([1,2,3], x => x > 1)").serialize(), "[2 3]");
}
