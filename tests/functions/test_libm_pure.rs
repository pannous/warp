// P26 (user, 2026-10-05): libm (sin, exp, …) counts as pure: no Ffi capability, allowed in eval_untrusted and inside
// `! Pure` functions. Other foreign functions keep the FFI effect.
use crate::common::effects_of;
use crate::is;
use warp::wasm_emitter::eval_untrusted;
use warp::Node;

#[test]
fn libm_calls_are_pure() {
	assert_eq!(effects_of("v(x) := sin(x)", "v"), "Pure");
	assert_eq!(effects_of("use m; down(x) := floor(x)", "down"), "Pure");
	assert_eq!(effects_of("wave(x) := exp(x) + cos(x)", "wave"), "Pure");
	assert_eq!(effects_of("import strlen from \"c\"; size(s) := strlen(s)", "size"), "FFI");
}

#[test]
fn libm_runs_untrusted_and_in_pure_functions() {
	assert_eq!(eval_untrusted("exp(0)"), Node::int(1));
	// P88 (user 2026-10-05): untrusted code gets every capability for now, libc included
	assert_eq!(eval_untrusted("import strlen from \"c\"; strlen(\"ab\")"), Node::int(2));
	is!("wave(x) := exp(x) ! Pure\nwave(0)", 1);
}
