// card nearest-default-limit: the prelude's own code warns no caller: nearest's exact float stop
// (`float(p1) / q1 == x`) gave "compares floats exactly … at lib/prelude.warp" to every program calling it
use crate::is;
use warp::errors::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

#[test]
fn nearest_warns_nothing_of_the_prelude() {
	take_warnings();
	is!("nearest(0.75, rational) == 3/4", true);
	let warnings: Vec<String> = take_warnings().into_iter().map(|warning| warning.message).collect();
	assert!(warnings.iter().all(|warning| !warning.contains("compares floats")), "{warnings:?}");
}

#[test]
fn the_programs_own_float_equality_still_warns() {
	take_warnings();
	eval("0.1f + 0.2 == 0.3");
	assert!(take_warnings().iter().any(|warning| warning.message.contains("compares floats exactly")));
}
