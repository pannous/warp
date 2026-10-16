// `//` always starts a comment; floor division is `/!`. Where `//` was clearly meant as floor division (glued between
// operands, `7//2`, or `x //= 3`) a severe warning names `/!` and `warp fix` rewrites it
// (user decision floor-division 2026-10-16, notes/decisions.md; it replaced the 2026-10-08 rule "glued divides")
use crate::is;
use warp::errors::diagnostic::{take_warnings, Level};

/// The severe warnings of running `code` that name the floor division spelling `/!`
fn floor_division_warnings(code: &str) -> usize {
	take_warnings();
	warp::wasm_emitter::eval(code);
	take_warnings().iter().filter(|warning| warning.level == Level::Severe && warning.message.contains("/!")).count()
}

#[test]
fn slash_slash_glued_is_a_comment_too() {
	is!("x = 7; x //= 3\nx", 7);
	is!("x = 7; x//=3\nx", 7);
	is!("7 //2", 7);
	is!("a = 7; a //2", 7);
	is!("7//2", 7);
}

#[test]
fn slash_slash_with_space_is_a_comment() {
	is!("x = 7 // note\nx", 7);
	is!("x = 7 // 2\nx", 7);
	is!("x = 7 //\nx", 7);
	is!("//note\n3", 3);
}

#[test]
fn slash_slash_meant_as_floor_division_warns_naming_bang() {
	assert_eq!(floor_division_warnings("n = 7//2\nn"), 1);
	assert_eq!(floor_division_warnings("x = 9\nx //= 3\nx"), 1);
	assert_eq!(floor_division_warnings("x = 7 // note\nx"), 0);
	assert_eq!(floor_division_warnings("u = \"http://x.org\"; 1"), 0);
	assert_eq!(floor_division_warnings("//note\n3"), 0);
}

#[test]
fn warp_fix_writes_the_floor_division_bang() {
	let fixed = warp::formatting::beautify("n = 7//2\nx = 9\nx//=3\nm = 7 div 2\nn + x + m\n").unwrap().text;
	assert_eq!(fixed, "n = 7 /! 2\nx = 9\nx /!= 3\nm = 7 /! 2\nn + x + m\n");
	is!(fixed.as_str(), 9);
}
