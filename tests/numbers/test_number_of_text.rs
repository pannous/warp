//! card number-text: number(t) of a text known at run time is the number it spells, as `t as number` is
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn number_of_a_run_time_text_is_the_number_it_spells() {
	assert_eq!(printed("x = \"d4?0.3\"; number(x[3..6]) as float"), "0.3");
	assert_eq!(printed("x = \"0.3\"; y = number(x); y"), "0.3");
	assert_eq!(printed("f(t) := number(t); f(\"0.3\")"), "0.3");
	assert_eq!(printed("f(t) := number(t); f(\"42\") + 1"), "43");
}
