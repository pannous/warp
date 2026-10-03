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
