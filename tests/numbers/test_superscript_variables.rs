//! A superscript letter as an exponent is the variable of that letter: `n=3;2ⁿ` → 8 (wiki operator.md superscripts)
use crate::is;

#[test]
fn test_a_superscript_variable_exponent() {
	is!("n=3;2ⁿ", 8);
	is!("n=3.0;2.0ⁿ", 8.0);
	is!("k=2; 3ᵏ", 9);
	is!("n=2; m=3; 2ⁿ⁺ᵐ", 32);
	is!("n=3; (1+1)ⁿ", 8);
	is!("n=1; 2ⁿ⁺¹", 4);
}

#[test]
fn test_a_negative_superscript_variable_exponent() {
	is!("n=1; 2.0⁻ⁿ", 0.5);
}
