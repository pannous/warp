// P26 (user, 2026-10-05): libm (sin, exp, …) counts as pure: no Ffi capability, allowed in eval_untrusted and inside
// `! Pure` functions. Other foreign functions keep the FFI effect.
use warp::effects::{effects_of, Effect::FFI, EffectSet};
use warp::is;
use warp::wasm_emitter::eval_untrusted;
use warp::Node;

#[test]
fn libm_calls_are_pure() {
	assert_eq!(effects_of("sin(1.0)", "main"), Some(EffectSet::PURE));
	assert_eq!(effects_of("use m;floor(4.5)", "main"), Some(EffectSet::PURE));
	assert_eq!(effects_of("wave(x) := exp(x) + cos(x)", "wave"), Some(EffectSet::PURE));
	assert_eq!(effects_of("import strlen from \"c\"; strlen(\"ab\")", "main"), Some(EffectSet::of(&[FFI])));
}

#[test]
fn libm_runs_untrusted_and_in_pure_functions() {
	assert_eq!(eval_untrusted("exp(0)"), Node::int(1));
	// P88 (user 2026-10-05): untrusted code gets every capability for now, libc included
	assert_eq!(eval_untrusted("import strlen from \"c\"; strlen(\"ab\")"), Node::int(2));
	is!("wave(x) := exp(x) ! Pure\nwave(0)", 1);
}
