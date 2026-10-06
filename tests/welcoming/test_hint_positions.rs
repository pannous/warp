// card hint-positions: a hint said by a lowering pass names the line and column of the node it is about, not the
// parser's position at the end of the program
use warp::normalize::{capture_hints, clear_shown_hints};
use warp::wasm_emitter::eval;

/// The line of the hint preferring `canonical`
fn line_of_hint(code: &str, canonical: &str) -> usize {
	clear_shown_hints();
	let (_, hints) = capture_hints(|| eval(code));
	hints.into_iter().find(|hint| hint.canonical == canonical).map(|hint| hint.line_and_column().0).unwrap_or_else(|| panic!("no hint preferring {canonical} for {code}"))
}

#[test]
fn a_lowering_hint_names_its_node() {
	assert_eq!(line_of_hint("n = 3\nlinear xs = int[n]\nxs#1 = 7\nprint 1\nprint 2\nxs#1", "xs = int[n]"), 2);
	assert_eq!(line_of_hint("x = 1\n\nfunc greet(person name: String) -> String { \"Hello \" + name }\ngreet(person: \"Swift\")", "person:String"), 3);
}
