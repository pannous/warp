// card binary-int (user, 2026-10-12): `5 as binary` is an int that shows as 0b101, the way `255 as hex` shows as 0xFF;
// `bin` is an alias; the literal 0b101 reads as 5; arithmetic on it gives a plain int
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn binary_literals_read_as_ints() {
	is!("0b101", 5);
	is!("0b101 + 1", 6);
	is!("-0b11", -3);
	is!("0B1111_0000", 240);
}

#[test]
fn binary_shows_as_binary() {
	assert_eq!(shown("5 as binary"), "0b101");
	assert_eq!(shown("x = 10 as binary; x"), "0b1010");
	assert_eq!(shown("0 as binary"), "0b0");
	assert_eq!(shown("-2 as binary"), "-0b10");
	assert_eq!(shown("(5 as binary) as text"), "\"0b101\"");
	assert_eq!(shown("5 as bin"), "0b101");
	assert_eq!(shown("binary(6)"), "0b110");
}

#[test]
fn binary_acts_as_its_int() {
	is!("5 as binary", 5);
	assert_eq!(shown("x = 5 as binary; x + 1"), "6");
	is!("(5 as binary) == 0b101", true);
	assert_eq!(shown("255 as hex"), "0xFF");
}
