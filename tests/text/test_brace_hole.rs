// card brace-hole (user): `"\{x+1}"` is an interpolation hole like `\(…)`, `${…}` and `$(…)`; no hint (user,
// 2026-10-09: ${} $() \() \{} are all fine); `\u{…}` stays a unicode escape
use crate::is;
use warp::normalize::capture_hints;

#[test]
fn a_backslash_brace_is_a_hole() {
	is!("x = 1; \"a\\{x+1}b\"", "a2b");
	is!("x = 1; \"\\{x+1} \\(x+1) ${x+1} $(x+1)\"", "2 2 2 2");
	is!("\"\\u{41}\\{1+1}\"", "A2");
}

#[test]
fn a_backslash_brace_gets_no_hint() {
	let hints = capture_hints(|| warp::wasm_emitter::eval("x = 1; \"a\\{x+1}b\"")).1;
	assert!(hints.is_empty(), "{hints:?}");
}
