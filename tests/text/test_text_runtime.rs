use warp::wasm_emitter::eval;
use crate::is;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn join_and_concatenation_take_non_ascii_characters() {
	is!("join [\"ä\" \"ö\"] \"\"", "äö");
	is!("join [\"ä\" \"ö\"] \"-\"", "ä-ö");
	is!("\"ä\"+\"b\"", "äb");
	is!("x=\"ö\"; y=\"b\"; x+y", "öb");
}

#[test]
fn upper_and_lower_map_latin_greek_and_cyrillic() {
	is!("\"abc\".upper", "ABC");
	is!("\"äb\".upper", "ÄB");
	is!("\"ÀÉ\".lower", "àé");
	is!("\"αβγ\".upper", "ΑΒΓ");
	is!("\"привет\".upper", "ПРИВЕТ");
	is!("\"ПРИВЕТ\".lower", "привет");
	is!("\"straße\".upper", "STRASSE");
	is!("\"ÿ\".upper", "Ÿ");
	is!("\"ā\".upper", "Ā");
	is!("x=\"éa\"; x.upper", "ÉA");
}

#[test]
fn reverse_of_a_text_is_by_characters() {
	is!("reverse \"abc\"", "cba");
	is!("\"abc\".reverse", "cba");
	is!("reverse \"äb\"", "bä");
	is!("reverse [1 2 3]", warp::ints(vec![3, 2, 1]));
	is!("x=\"a\"; reverse x", "a");
}

#[test]
fn the_characters_of_a_runtime_text() {
	assert_eq!(text_of("x=\"ab\"; x as list"), "['a' 'b']");
	assert_eq!(text_of("x=\"äb\"; x as list"), "['ä' 'b']");
	assert_eq!(text_of("chars \"ab\""), "['a' 'b']");
	assert_eq!(text_of("x=\"hi\"; chars x"), "['h' 'i']");
	is!("\"ab\".chars", 2);
	is!("x=\"ab\"; x.chars", 2);
}
