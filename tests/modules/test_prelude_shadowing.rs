//! card program-own: a program's own word named like a prelude word takes the program's calls with a hint, while the
//! library's spellings of the prelude word (`round(x, 2)` → round_to, `xs.clear()` → cleared) keep the prelude's;
//! the program's definition no longer leaves `prelude·round_to` undefined
use crate::is;

#[test]
fn library_spellings_keep_the_prelude_word() {
	is!("round_to(x, d) := 99; round(2.555, 2)", 2.56);
	is!("cleared(x) := 99; xs = [1]; xs.clear(); count(xs)", 0);
	is!("first_items(x, n) := 99; [5, 6, 7].first(2)", warp::ints(vec![5, 6]));
}

fn hints_of(code: &str) -> (warp::Node, Vec<String>) {
	let (value, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	(value, hints.into_iter().map(|hint| hint.reason).collect())
}

#[test]
fn the_program_word_takes_its_own_calls_with_a_hint() {
	for (code, expected, hint) in [
		("round_to(x, d) := 99; round_to(1, 2)", 99, "round_to replaces prelude's round_to(x, places)"),
		("first(x) := 99; first([5, 6])", 5, "first of a list or text stays the library word"),
	] {
		let (value, hints) = hints_of(code);
		assert_eq!(value, warp::Node::from(expected), "{code}");
		assert!(hints.iter().any(|said| said.contains(hint)), "no hint {hint:?} for {code}: {hints:?}");
	}
}
