//! A text computed at run time converts to a float: `s as float`, `float(s)` (samples/calculator.warp)
use crate::is;
use crate::common::fails_with;

#[test]
fn test_text_as_float() {
	is!("s=\"12.5\"; s as float", 12.5);
	is!("s=\"-0.25\"; s as float", -0.25);
	is!("s=\"1.5e3\"; s as float", 1500.0);
	is!("s=\"2E-2\"; s as float", 0.02);
	is!("s=\".5\"; s as float", 0.5);
	is!("s=\"12.5x\"; float(s.slice(0, 4))", 12.5); // was 4
	is!("s=\"7.25\"; x = s as float; x * 2", 14.5);
	fails_with("s=\"12.5x\"; s as float", "invalid number");
}
