// card serve-var: a `let` or `const` binds its name in its own function only; another function's variable of the same
// name may change without the note teaching `var` (lib/markup.warp's `let name` taught it for lib/router.warp's
// `var name` whenever a served page had a route parameter) and without the const error
use crate::is;

fn hint_texts(code: &str) -> Vec<String> {
	warp::normalize::clear_shown_hints();
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	hints.iter().map(|hint| format!("{} → {}: {}", hint.original, hint.canonical, hint.reason)).collect()
}

#[test]
fn a_let_in_one_function_leaves_another_alone() {
	let code = "def a(){ let name = 1; name }\ndef b(){ name = 2; name = name + 1; name }\na() + b()";
	let hints = hint_texts(code);
	assert!(hints.iter().all(|hint| !hint.contains("let name")), "{hints:?}");
	is!(code, 4);
}

#[test]
fn a_const_in_one_function_leaves_another_alone() {
	is!("def a(){ const x = 1; x }\ndef b(){ x = 2; x = 3; x }\na() + b()", 4);
}
