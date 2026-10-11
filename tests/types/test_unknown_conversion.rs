// card unknown-word: `as` with a word that names no type, unit or class is a compile-time error naming the word; it
// used to stay a silent `255 as hex` node, or fail as "not a joinable item" / "cannot extract a numeric value"
use warp::wasm_emitter::eval;
use crate::is;

fn unknown_conversion_error(code: &str) {
	let result = format!("{:?}", eval(code));
	assert!(result.contains("banana is no type to convert to"), "{code}: {result}");
}

#[test]
fn an_unknown_as_word_is_loud() {
	unknown_conversion_error("255 as banana");
	unknown_conversion_error("x=255; x as banana");
	unknown_conversion_error("x=255; y = x as banana; y");
	unknown_conversion_error("print(255 as banana)");
}

#[test]
fn known_as_words_still_convert() {
	is!("3.7 as int", 3);
	is!("3 as text", "3");
	is!("class Point{x:int y:int}; p = {x:1 y:2} as Point; p.y", 2);
	assert_eq!(eval("1500m as km").serialize(), "1.5km");
}

/// `255 as byte` printed itself too; a fixed width converts like `int` and traps outside its range
#[test]
fn fixed_width_casts() {
	is!("255 as byte", 255);
	is!("3.7 as int8", 3);
	is!("\"200\" as byte", 200);
	is!("x = 200 as uint8; x + 100", 300);
	let overflow = format!("{:?}", eval("256 as byte"));
	assert!(overflow.contains("byte overflow"), "{overflow}");
}

