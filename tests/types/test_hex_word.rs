// The hex-word literal rule (`hex 1010` read as 0x1010) is retired (user-approved, 2026-10-11): `hex x` is the hex
// constructor like `hex(x)` and `x as hex`, through the one call shape (analyzer::applied_word). Hex literals are 0x…
use warp::wasm_emitter::eval;
use crate::is;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

fn same(braceless: &str, braced: &str) {
	assert_eq!(shown(braceless), shown(braced), "{braceless} vs {braced}");
}

#[test]
fn hex_word_is_the_hex_call() {
	same("hex 255", "hex(255)");
	same("hex 255", "255 as hex");
	same("hex 1010", "hex(1010)");
	same("x = 255; hex x", "x = 255; hex(x)");
	same("hex ff", "hex(ff)");
	assert_eq!(shown("hex 255"), "0xFF");
	assert_eq!(shown("hex 1010"), "0x3F2");
	is!("hex 255", 255);
}

#[test]
fn hex_literals_are_written_0x() {
	is!("0xFF", 255);
	is!("0x1010", 4112);
	assert_eq!(shown("hex 0x1010"), "0x1010");
}
