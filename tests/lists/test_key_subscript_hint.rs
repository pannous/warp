// `ages["alice"]` looks a key up, it counts no position: no hint to shift it into `#`; a position index still gets one
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

fn index_hints(code: &str) -> Vec<String> {
	let (_, hints) = capture_hints(|| eval(code));
	hints.into_iter().filter(|hint| hint.reason == "use # for indexing").map(|hint| hint.canonical).collect()
}

#[test]
fn a_text_key_subscript_gets_no_index_hint() {
	assert_eq!(index_hints("ages={alice:3}; ages[\"alice\"]"), Vec::<String>::new());
	assert_eq!(index_hints("ages={alice:3}; ages['alice']"), Vec::<String>::new());
	assert_eq!(index_hints("xs=[1,2]; i=0; xs[i]"), vec!["xs#(i+1)".to_string()]);
}

// card text-key: a variable holding a text, or a parameter typed text, is a key too
#[test]
fn a_text_variable_subscript_gets_no_index_hint() {
	assert_eq!(index_hints("l=\"en\"; m={en:\"x\"}; m[l]"), Vec::<String>::new());
	assert_eq!(index_hints("get(m, k:text) := m[k]; get({en:\"x\"}, \"en\")"), Vec::<String>::new());
	assert_eq!(index_hints("m={en:{hi:\"x\"}}; l=\"en\"; k=\"hi\"; m[l][k]"), Vec::<String>::new());
	assert_eq!(index_hints("xs=[1,2]; i=0; xs[i]"), vec!["xs#(i+1)".to_string()]);
}
