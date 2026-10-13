// card int-text: a text spelling a number with an exponent converts like the literal, for int and float alike
use crate::is;

#[test]
fn int_of_an_exponent_text() {
	is!("int(\"1e3\")", 1000);
	is!("int(\"2.5e2\")", 250);
	is!("int(\"2.5E2\")", 250);
}

#[test]
fn float_of_an_exponent_text() {
	is!("float(\"2.5e-2\")", 0.025);
	is!("float(\"1e3\")", 1000.0);
	is!("\"1.5e-7\" as float", 1.5e-7);
}
