//! A program's last value of a class with text() shows that text bare, as print does: `V(1)` is v1, not "v1"
//! (card program-last)
use warp::wasm_emitter::eval;

#[test]
fn a_final_instance_shows_its_text_bare() {
	assert_eq!(eval("class V{x:int; text() := \"v\" + str(x)}; V(1)").serialize(), "v1");
	assert_eq!(eval("quantity(\"5 km\")").serialize(), "5km");
	assert_eq!(eval("x = 5 m ± 1 cm; x * 2").serialize(), "10.000 ± 0.020m");
	assert_eq!(eval("class V{x:int; text() := \"v\" + str(x)}; str(V(1))").serialize(), "\"v1\"");
}
