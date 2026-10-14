//! P230: `effects of f` gives symbols, `(State IO)`, as a value and as the program's last statement alike

fn serialized(code: &str) -> String {
	warp::wasm_emitter::eval(code).serialize().trim().to_string()
}

#[test]
fn effects_of_gives_symbols_as_a_value() {
	assert_eq!(serialized("x=0; def f(v){ global x; x=v; print v }; e = effects of f; e"), "(State IO)");
	assert_eq!(serialized("f(x) := print x\ne = f.effects\ne"), "IO");
	assert!(matches!(warp::wasm_emitter::eval("f(x) := print x\nf.effects").drop_meta(), warp::Node::Symbol(_)));
}

#[test]
fn a_trailing_effects_of_runs_the_program() {
	assert_eq!(serialized("x=0; def f(v){ global x; x=v }; f(2); effects of f"), "State");
}
