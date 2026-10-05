//! P50 (user, 2026-10-05): a function whose one parameter is declared with a scalar type broadcasts too
//! (wiki/broadcasting.md `square number = number*number; square [1 2 3] == [1 4 9]`).
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_declared_scalar_parameters_broadcast() {
	assert_eq!(printed("square number = number*number; square [1 2 3]"), "[1 4 9]");
	assert_eq!(printed("inc(x:int):=x+1; inc [1 2]"), "[2 3]");
	assert_eq!(printed("half(x:float):=x/2; half [1.0 3.0]"), "[0.5 1.5]");
	assert_eq!(printed("inc(x:int):=x+1; inc [a:1 b:2]"), "[a:2 b:3]");
}

#[test]
fn test_declared_list_parameters_take_the_list() {
	assert_eq!(printed("total(xs:list):=count(xs); total [1 2 3]"), "3");
	fails_with("inc(x:int):=x+1; inc \"abc\"", "inc needs an Int for parameter x");
}
