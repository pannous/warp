//! A name a match arm binds is the function's own local, never a question about a main-level variable of the same
//! name (card pattern-names)
use crate::common::warnings_of;
use warp::Node;

fn local_without_question(code: &str, expected: i64) {
	let (value, warnings) = warnings_of(code);
	assert_eq!(value, Node::from(expected), "{code}");
	assert!(!warnings.iter().any(|warning| warning.contains("make a new local")), "{code}: {warnings:?}");
}

#[test]
fn tuple_pattern_name_shadows_main_variable() {
	local_without_question("x = 3\nwhere(p) := match p { (x, 0) => x; _ => 0 }\nwhere((4, 0)) + x", 7);
}

#[test]
fn list_and_rest_pattern_names_shadow_main_variables() {
	local_without_question(
		"first = 9\nrest = 0\nhead(xs) := match xs { [first, ...rest] => first + count(rest); _ => 0 }\nhead([1, 2, 3]) + first + rest",
		12,
	);
}
