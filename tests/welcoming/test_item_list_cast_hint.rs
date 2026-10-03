// `[int, 3]` and `[twice, x=>x+1]` are items: no hint to write `3 as int` for a cast that was never written
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

fn cast_hints(code: &str) -> Vec<String> {
	let (_, hints) = capture_hints(|| eval(code));
	hints.into_iter().filter(|hint| hint.reason.contains("postfix 'as'")).map(|hint| hint.original).collect()
}

#[test]
fn a_type_word_item_in_a_comma_list_is_no_cast() {
	assert_eq!(cast_hints("fs=[int, 3]; 1"), Vec::<String>::new());
}

#[test]
fn a_written_constructor_call_is_still_hinted() {
	assert_eq!(cast_hints("int(3)"), vec!["int(3)".to_string()]);
}
