// card instance-final: a program whose value is an instance of a class with text() shows that text, as str() does
use crate::is;

#[test]
fn a_final_instance_shows_its_text() {
	is!("class V{x:int; text() := \"v\" + str(x)}; V(1)", "v1");
	is!("class V{x:int; text() := \"v\" + str(x)}; v = V(2); v", "v2");
	is!("x = 5 m ± 1 cm; x * 2", "10.000 ± 0.020m");
	is!("quantity(\"5 km\")", "5km");
}

#[test]
fn a_final_instance_without_text_stays_a_record() {
	is!("class P{x:int}; P(1).x", 1);
	assert_eq!(warp::wasm_emitter::eval("class P{x:int}; P(1)").serialize(), "P{x:1}");
}
