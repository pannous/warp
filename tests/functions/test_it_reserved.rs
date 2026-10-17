// card it-reserved (user, 2026-10-16: "we should create a warning that it is a reserved keyword"): `it` names the
// parameter of a function that names none, so assigning it warns; the program still runs (an error under use strict)
use crate::is;

fn warnings_of(code: &str) -> (warp::Node, Vec<String>) {
	warp::errors::diagnostic::take_warnings();
	let value = warp::wasm_emitter::eval(code);
	(value, warp::errors::diagnostic::take_warnings().iter().map(|warning| warning.to_string()).collect())
}

#[test]
fn assigning_it_warns_that_it_is_reserved() {
	let (value, warnings) = warnings_of("it = 5; it + 1");
	assert_eq!(value, warp::Node::int(6));
	assert!(warnings.iter().any(|warning| warning.contains("`it` is reserved")), "{warnings:?}");
	let (_, warnings) = warnings_of("square := it * it; square(3)");
	assert!(!warnings.iter().any(|warning| warning.contains("reserved")), "{warnings:?}");
	is!("square := it * it; square(3)", 9);
	let strict = warp::wasm_emitter::eval("use strict; it = 5; it").serialize();
	assert!(strict.contains("`it` is reserved"), "{strict}");
}

/// reviewer: every form that binds a name binds `it` too; a parameter written `it` is the parameter itself
#[test]
fn every_binding_of_it_warns() {
	for code in [
		"x = 1; it += 1; x",
		"for it in [1 2] { x = it }; 1",
		"let it = 3; it",
		"[it, b] = [1, 2]; b",
		"it: int = 5; it",
	] {
		let (_, warnings) = warnings_of(code);
		assert!(warnings.iter().any(|warning| warning.contains("`it` is reserved")), "{code}: {warnings:?}");
	}
	for code in ["f(it) := it * 2; f(3)", "g = it => it + 1; g(1)"] {
		let (_, warnings) = warnings_of(code);
		assert!(!warnings.iter().any(|warning| warning.contains("reserved")), "{code}: {warnings:?}");
	}
}

/// user decision implicit-words-reserved: self and this name a method's receiver, reserved as it is
#[test]
fn self_and_this_are_reserved_too() {
	for (code, name) in [
		("self = 3; self", "self"),
		("this = 1; this += 1; this", "this"),
		("for self in [1 2] { x = self }; 1", "self"),
	] {
		let (_, warnings) = warnings_of(code);
		let expected = format!("`{name}` is reserved: it names the receiver of a method");
		assert!(warnings.iter().any(|warning| warning.contains(&expected)), "{code}: {warnings:?}");
	}
	for code in [
		"class Point { x: int; norm(self) := self.x * 2 }\nPoint(x: 3).norm()",
		"class Point { x: int; norm() := this.x * 2 }\nPoint(x: 3).norm()",
	] {
		let (value, warnings) = warnings_of(code);
		assert_eq!(value, warp::Node::int(6), "{code}");
		assert!(!warnings.iter().any(|warning| warning.contains("reserved")), "{code}: {warnings:?}");
	}
}
