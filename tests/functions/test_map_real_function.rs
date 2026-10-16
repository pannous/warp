// card map-sin: a real function maps over a list of floats, as a call argument and as a method
use warp::wasm_emitter::eval;

const SIN_FOUR: &str = "[-0.7568024953079283]";

#[test]
fn sin_maps_over_floats() {
	assert_eq!(eval("map([4.0], sin)").serialize(), SIN_FOUR);
	assert_eq!(eval("[4.0].map(sin)").serialize(), SIN_FOUR);
}
