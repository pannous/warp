//! Property access on objects: `o.a`, `a of o`, `o's a` and `o["a"]` are one lookup; a missing key is a loud error value
use crate::common::fails_with;
use crate::is;

#[test]
fn test_dot_on_an_object_literal() {
	is!("{a:1 b:2}.a", 1);
	is!("{a:1 b:2}.b", 2);
}

#[test]
fn test_dot_on_a_variable_and_nested() {
	is!("p={a:1 b:2}; p.a", 1);
	is!("p={a:1 b:{c:3}}; p.b.c", 3);
	is!("p={name:\"Joe\"}; p.name", "Joe");
}

#[test]
fn test_of_and_possessive() {
	is!("p={name:\"Joe\"}; name of p", "Joe");
	is!("p={name:\"Joe\"}; p's name", "Joe");
	is!("p={a:1 b:{c:3}}; c of b of p", 3);
}

#[test]
fn test_bracket_lookup_by_text_never_traps() {
	is!("p={name:\"Joe\"}; p[\"name\"]", "Joe");
	is!("p={name:\"Joe\"}; p[name]", "Joe");
	is!("{a:1 b:2}[\"b\"]", 2);
}

#[test]
fn test_missing_key_is_a_loud_error() {
	fails_with("p={a:1}; p.x", "no field x");
	fails_with("p={a:1}; x of p", "no field x");
	fails_with("p={a:1}; p[\"x\"]", "no field x");
	fails_with("p={a:1 b:2}; p.b.c", "no field c");
	fails_with("{a:1}.zzz", "no field zzz");
}

#[test]
fn test_field_names_win_over_library_words_on_an_object() {
	is!("p={first:\"A\" last:\"B\"}; p.last", "B");
	is!("p={count:7}; p.count", 7);
	is!("p={a:1 b:2}; p.length", 2); // no field of that name: the counting word
}

#[test]
fn test_non_objects_keep_the_undefined_function_rule() {
	fails_with("x=5; x.foo", "undefined function: foo");
}

#[test]
fn test_object_data_stays_data() {
	is!("p={a:1 b:2}; p.a + p.b", 3);
	assert_eq!(warp::wasm_emitter::eval("{a:1 b:2}").serialize(), "{a:1 b:2}");
}

#[test]
fn test_of_lookup_is_an_assignment_target() {
	is!("p={width:3}; width of p = 12; p.width", 12);
	is!("square of a number = number*number; square 3", 9);
}

#[test]
#[ignore = "dropped: rust-cleanup-drops: the set x to y phrase; warp writes x = y (set x of y to z may come back as library code)"]
fn test_set_to_phrases_assign() {
	is!("p={width:3}; set width of p to 12; p.width", 12);
	is!("p={width:3}\nwidth of p = 12\nset width of p to 7\np.width", 7);
	is!("x=1; set x to 5; x", 5);
	is!("f := { n = 1; set n to 5; n }; f()", 5);
	is!("class Box { w = 1; set size(v) { w = v * 2 } }; b = Box(); set size of b to 5; b.w", 10);
}

/// `set A to B` works but is prose: the hint names the assignment (card english-setter)
#[test]
#[ignore = "dropped: rust-cleanup-drops: the set x to y phrase; warp writes x = y (set x of y to z may come back as library code)"]
fn test_set_to_hints_the_assignment() {
	let (_, hints) =
		warp::normalize::capture_hints(|| warp::wasm_emitter::eval("p={width:3}; set width of p to 7; p.width"));
	let hint = hints.iter().find(|hint| hint.canonical == "p.width=7").expect("a no-prose hint");
	assert_eq!(hint.original, "set p.width to 7");
	assert!(hint.reason.contains("doesn't do prose"), "{}", hint.reason);
}

#[test]
fn test_print_of_lookup_prints_the_field() {
	is!("p={w:1}; print w of p; w of p + 1", 2);
}
