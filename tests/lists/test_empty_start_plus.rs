// Card local-fails (decisions empty-plus-text, list-append-plus): a local given `[]` is a list to `+` too, as it is to
// `+=`; `xs += [3]` of an evident item gets the hint that the brackets are not needed
use crate::is;
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;
use warp::{ints, parse};

const NO_BRACKETS: &str = "an item is appended without brackets";

fn bracket_hints(code: &str) -> Vec<String> {
	let (_, hints) = capture_hints(|| eval(code));
	hints.into_iter().filter(|hint| hint.reason == NO_BRACKETS).map(|hint| hint.canonical).collect()
}

#[test]
fn an_empty_list_start_plus_an_item_is_a_list() {
	is!("t = []; u = t + \"a\"; u", parse("[\"a\"]"));
	is!("t = []; u = t + 3; u", ints(vec![3]));
	is!("t = []; t + [1, 2]", ints(vec![1, 2]));
	is!("def f() { t = []; t + \"a\" }\nf()", parse("[\"a\"]"));
}

#[test]
fn a_bracketed_item_appended_gets_the_bracket_hint() {
	assert_eq!(bracket_hints("xs = [1]; xs += [3]; xs"), vec!["xs += 3".to_string()]);
	assert_eq!(bracket_hints("xs = [\"a\"]; xs += [\"b\"]; xs"), vec!["xs += \"b\"".to_string()]);
	assert_eq!(bracket_hints("xs = [1]; xs += 3; xs"), Vec::<String>::new());
	assert_eq!(bracket_hints("xs = [1]; xs += [2, 3]; xs"), Vec::<String>::new());
	// an item of unknown kind keeps its brackets: a list item would be concatenated without them
	assert_eq!(bracket_hints("def f(p: any) { xs = [1]; xs += [p]; xs }\nf(2)"), Vec::<String>::new());
	assert_eq!(bracket_hints("xs = [1]; xs.add(2); xs"), Vec::<String>::new());
}
