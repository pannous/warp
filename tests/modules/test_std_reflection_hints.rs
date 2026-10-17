use warp::normalize::capture_hints;

fn hints_of(code: &str) -> Vec<warp::normalize::CapturedHint> {
	capture_hints(|| warp::wasm_emitter::eval(code)).1
}

#[test]
fn reflecting_on_standard_module_functions_gives_no_hints_from_the_module() {
	for code in ["function repeat", "methods(\"hi\")"] {
		let hints = hints_of(code);
		assert!(hints.is_empty(), "{code}: {hints:?}");
	}
}

#[test]
fn reflection_still_gives_hints_for_the_programs_own_code() {
	let hints = hints_of("methods(\"hi\"); xs = [1, 2]; xs[0]");
	assert!(hints.iter().any(|hint| hint.original == "xs[0]"), "{hints:?}");
}
