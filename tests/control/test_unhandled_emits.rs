// P202 (user 2026-10-08): an emit that no handler anywhere receives gives a got-it warning "no handler for event X"; it
// still does nothing (P163) and is no error
use crate::is;

fn reasons_of(code: &str) -> Vec<String> {
	warp::normalize::clear_shown_hints();
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	hints.iter().map(|hint| hint.reason.clone()).collect()
}

fn warns_unhandled(code: &str) -> bool {
	reasons_of(code).iter().any(|reason| reason.contains("no handler for event"))
}

#[test]
fn an_emit_nobody_handles_warns() {
	assert!(warns_unhandled("emit nobody listens; 5"));
	is!("emit nobody listens; 5", 5);
}

#[test]
fn a_handled_emit_does_not_warn() {
	assert!(!warns_unhandled("n = 0; on ping { n += 1 }; emit ping; n"));
	assert!(!warns_unhandled("compute() := emit ask; on ask { 2 } in { compute() }"));
}
