use crate::common::fails_with;
use crate::is;

// card types-constructor: a type called as a function converts as in Python, `hex(123)` is `123 as hex`
#[test]
fn a_type_called_converts_as_the_cast() {
	is!("hex(123)", 123);
	is!("text(hex(255))", "0xFF");
	is!("x = hex(255); x + 1", 256);
	is!("text(byte(65))", "0x41");
	is!("decimal(1/4)", "0.25");
	is!("int(\"12\") + 1", 13);
}

// decision type-words-reserved: a conversion word names no function either
#[test]
fn a_function_of_the_program_keeps_its_name() {
	fails_with("hex(x) := x * 2; hex(4)", "hex is a type; name your function to_hex");
}

// a braceless call is the braced one: `byte 65` is `byte(65)`, one call shape for every callable
#[test]
fn a_type_called_braceless_converts_like_braced() {
	is!("b = byte 65; text(b)", "0x41");
	is!("b = byte 65; b == byte(65)", true);
	is!("decimal 1/4", "0.25");
	is!("x = 255; h = hex x; text(h)", "0xFF");
	is!("byte b = 10; b * 3", 30);
	is!("int 3.7", 3);
	is!("text 5", "5");
	is!("0xFF", 255);
}

#[test]
fn a_function_of_the_program_keeps_its_name_braceless() {
	fails_with("hex(x) := x * 2; y = 4; hex y", "hex is a type");
}
