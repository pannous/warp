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
