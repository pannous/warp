// card constant-alias (user, 2026-10-07: "constant is an alias of const"): `constant x = 3` declares the constant x as
// `const x = 3` does, with a got-it note naming const (the alias mechanism); a variable named constant stays one
use crate::common::fails_with;
use crate::is;

/// The notes (written → wasp word) with a fix that a program's compilation gives
fn alias_notes(code: &str) -> Vec<(String, String)> {
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	hints.iter().filter(|hint| hint.fix().is_some()).map(|hint| (hint.original.clone(), hint.canonical.clone())).collect()
}

#[test]
fn constant_is_an_alias_of_const() {
	is!("constant x = 3; x", 3);
	fails_with("constant x = 3; x = 4", "x is const");
	assert_eq!(alias_notes("constant x = 3; x"), vec![("constant".to_string(), "const".to_string())]);
	assert_eq!(alias_notes("const x = 3; x"), vec![]);
	is!("constant = 5; constant + 1", 6);
	assert_eq!(alias_notes("constant = 5; constant + 1"), vec![]);
}
