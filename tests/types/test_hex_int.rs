// card hex-printing (user, 2026-10-11): `255 as hex` is an int that behaves exactly like an int but shows as 0xFF; so
// does a byte (`byte`, `u8`, `uint8`), a plain wasm integer otherwise. Only the shown text differs: arithmetic on it
// gives a plain int
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn hex_shows_as_hex() {
	assert_eq!(shown("255 as hex"), "0xFF");
	assert_eq!(shown("x = 4660 as hex; x"), "0x1234");
	assert_eq!(shown("-1 as hex"), "-0x1");
	assert_eq!(shown("0 as hex"), "0x0");
	assert_eq!(shown("(255 as hex) as text"), "\"0xFF\"");
	assert_eq!(shown("\"value: \" + (255 as hex)"), "\"value: 0xFF\"");
}

#[test]
fn hex_acts_as_its_int() {
	is!("255 as hex", 255);
	is!("x = 255 as hex; x + 1", 256);
	is!("(255 as hex) == 255", true);
	assert_eq!(shown("x = 255 as hex; x + 1"), "256");
}

#[test]
fn a_byte_shows_as_hex() {
	assert_eq!(shown("255 as byte"), "0xFF");
	assert_eq!(shown("byte b = 10; b"), "0xA");
	assert_eq!(shown("u8 b = 200; b"), "0xC8");
	is!("byte b = 10; b * 3", 30);
}

#[test]
#[cfg(feature = "native")]
fn print_shows_hex() {
	assert_eq!(crate::common::printed("print(255 as hex)").trim(), "0xFF");
	assert_eq!(crate::common::printed("byte b = 171\nprint b").trim(), "0xAB");
}
