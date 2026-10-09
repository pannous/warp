// card number-param: a parameter typed `number` in a program with ± values takes a ± value (notes/plus_minus.md)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_number_parameter_takes_a_plus_minus_value() {
	assert_eq!(shown("lo(a:number) := a.low; lo(5 ± 1)"), "4");
	assert_eq!(shown("inc(a:number) := a + 1; inc(5 ± 1)"), "6.0 ± 1.0");
	assert_eq!(shown("inc(a:number) := a + 1; x = 5 ± 1; inc(2)"), "3");
	fails_with("inc(a:number) := a + 1; x = 5 ± 1; inc(\"x\")", "inc needs a number for parameter a");
}
