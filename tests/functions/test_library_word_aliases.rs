// Every spelling of a library word goes through the one synonym table (lowering/library_words.rs SYNONYMS): the
// containment words, a phrase's `sorted`; print's println and writeln (wiki/Features.md, card print-aliases), read as
// print by the parser; and a name the program gives a module function again (card arctan-alias-library)
use crate::is;

#[test]
fn print_spellings_print() {
	is!("println(\"hi\"); 3", 3);
	is!("writeln 42; 3", 3);
	is!("println 42", warp::Node::Empty);
	is!("println \"\" || 3", warp::Node::Empty);
	is!("def println(x) := x + 1; println(2)", 3);
}

#[test]
#[ignore = "cleanup-edge: cleanup-park-phrases: `sorted by` phrase parked until card prepositions-call"]
fn containment_and_phrase_spellings() {
	for contains in ["contains", "has", "includes"] {
		is!(&format!("xs = [1, 2]; xs {contains} 2"), true);
	}
	is!("todos = [3, 1, 2]; first(todos sorted by it)", 1);
}

/// math.warp's own `arctan=&arc_tangent` and the program's `arctan := arc_tangent` name the same function
#[test]
fn renaming_a_module_function_as_the_module_does() {
	is!("use math; arctan := arc_tangent; arctan(0)", 0.0);
	is!("use math; arcsin := arc_sine; arcsin(0)", 0.0);
}

/// a note on another language's word points at the word itself, not at a node read before or after it
#[test]
fn parser_alias_notes_point_at_the_word() {
	warp::normalize::clear_shown_hints();
	let code = "x = 1\nprintln x\n  writeln 2\nconstant y = 3";
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	let position = |written: &str| hints.iter().find(|hint| hint.original == written).map(|hint| hint.position.clone());
	assert_eq!(position("println"), Some("2:1".to_string()), "{hints:?}");
	assert_eq!(position("writeln"), Some("3:3".to_string()), "{hints:?}");
	assert_eq!(position("constant"), Some("4:1".to_string()), "{hints:?}");
}
