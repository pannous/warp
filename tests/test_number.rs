use wasp::Number::{Float, Int};
use wasp::{eq, is, put, Number};
// use wasp::node::Node::Number as Number;
// use wasp::Number::{Float, Int};

#[test]
fn test_number() {
	let n = Int(1);
	let n2 = Int(2);
	eq!(n, 1);
	eq!(n2, 2);
	let n3 = n + n2;
	put!("n3", n3);
	eq!(n3, 3);
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
}

#[test]
fn test_number_mix() {
	let n = Int(1);
	let n2 = Float(2.2);
	let n3 = n + n2;
	put!("n3", n3);
	eq!(n3, 3.2);
}

#[test]
fn test_hex() {
	// eq!(hex(18966001896603L), "0x113fddce4c9b");
	is!("42", 42);
	is!("0xFF", 255);
	is!("0x100", 256);
	is!("0xdce4c9b", 0xdce4c9b);
	is!("0x113fddce4c9b", 0x113fddce4c9bi64);
}
