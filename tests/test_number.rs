use warp::Number::{Float, Int};
use warp::{eq, is, put, Number};
// use warp::Node::Number as Number;
// use warp::Number::{Float, Int};

#[test]
fn test_number() {
	let n = Int(1);
	let n2 = Int(2);
	eq!(n, 1);
	eq!(n2, 2);
	let n3 = n + n2;
	put!("n3", n3);
	eq!(n3, 3);

	// Edge cases for integers
	is!("0", 0);
	is!("1", 1);
	is!("-1", -1);
	is!("127", 127);
	is!("-128", -128);
	is!("255", 255);
	is!("256", 256);
	is!("32767", 32767);
	is!("-32768", -32768);
	is!("65535", 65535);
	is!("2147483647", 2147483647);
	is!("-2147483648", -2147483648);
}

fn approx_equal_f64(a: f64, b: f64, epsilon: f64) -> bool {
	(a - b).abs() < epsilon
}
fn approx_equal(a: Number, b: Number, epsilon: f64) -> bool {
	(a - b).abs() < epsilon
}

#[test]
fn test_number_floats() {
	let n = Float(1.1);
	let n2 = Float(2.2);
	let n3 = n + n2;
	put!("n3", n3);
	// eq!(n3, 3.3);
	assert!(
		approx_equal(n3, Float(3.3), 1e-10),
		"Left: {}, Right: {}",
		n3,
		3.3
	);
	eq!(n3, Float(3.3)); // ⚠️ 3.3000000000000003
	eq!(n3, Float(3.3));
	eq!(n3, 3.3);
	// assert!(approx_equal_f64(n3, 3.3, 1e-10));
	if let Float(val) = n3 {
		assert!(approx_equal_f64(val, 3.3, 1e-10));
	}

	// Float edge cases
	is!("0.0", 0.0);
	is!("1.0", 1.0);
	is!("-1.0", -1.0);
	is!("0.5", 0.5);
	is!("-0.5", -0.5);
	is!("0.1", 0.1);
	is!("0.01", 0.01);
	is!("0.001", 0.001);
	is!("123.456", 123.456);
	is!("-123.456", -123.456);
	// is!("1e10", 1e10); // TODO: scientific notation not yet supported by parser
	// is!("1e-10", 1e-10); // TODO: scientific notation not yet supported by parser
	is!("3.14159", 3.14159);
}

#[test]
fn test_number_mix() {
	let n = Int(1);
	let n2 = Float(2.2);
	let n3 = n + n2;
	put!("n3", n3);
	eq!(n3, 3.2);

	// Mixed integer and float edge cases
	is!("0", 0.0);
	is!("1", 1.0);
	is!("100", 100.0);
	is!("-50", -50.0);
}

#[test]
fn test_hex() {
	// eq!(hex(18966001896603L), "0x113fddce4c9b");
	is!("42", 42);
	is!("0xFF", 255);
	is!("0x100", 256);
	is!("0xdce4c9b", 0xdce4c9b);
	is!("0x113fddce4c9b", 0x113fddce4c9bi64);

	// Minimum hex value
	is!("0x0", 0);

	// Single digit hex values
	is!("0x1", 1);
	is!("0x2", 2);
	is!("0x9", 9);
	is!("0xA", 10);
	is!("0xF", 15);

	// Lowercase vs uppercase hex
	is!("0xff", 255);
	is!("0xAB", 171);
	is!("0xab", 171);
	is!("0xDeadBeef", 0xDEADBEEFi64);

	// Multi-digit hex edge cases
	is!("0xFFF", 4095);
	is!("0x1000", 4096);
	is!("0xFFFF", 65535);
	is!("0x10000", 65536);

	// Maximum 32-bit values
	is!("0xFFFFFFFF", 0xFFFFFFFFi64);
	is!("0x7FFFFFFF", 2147483647);
}
