// P174 (user, 2026-10-07): `mk(k) := { it * k }` keeps `it` = the parameter (mk(3) is 9) and warns, naming both
// readings: the parameter, or the parameter of a returned lambda (Kotlin/Swift)
use crate::is;
use warp::diagnostic::take_warnings;
use warp::wasm_emitter::eval;

fn warnings_of(code: &str) -> Vec<String> {
	take_warnings();
	eval(code);
	take_warnings().iter().map(|warning| warning.to_string()).collect()
}

#[test]
fn a_braced_it_body_is_the_parameter_and_warns() {
	is!("mk(k) := { it * k }; mk(3)", 9);
	let warnings = warnings_of("mk(k) := { it * k }; mk(3)");
	assert!(warnings.iter().any(|warning| warning.contains("mk(k) := k*k or mk(k) := x => x*k")), "{warnings:?}");
}

#[test]
fn other_bodies_do_not_warn() {
	for code in ["f(k) := { k * 2 }; f(3)", "sq := { it * it }; sq(3)", "f(k) := it * k; f(3)"] {
		assert!(warnings_of(code).iter().all(|warning| !warning.contains("braced body")), "{code}");
	}
}
