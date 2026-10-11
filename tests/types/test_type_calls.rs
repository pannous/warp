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

#[test]
fn a_function_of_the_program_keeps_its_name() {
	is!("hex(x) := x * 2; hex(4)", 8);
}
