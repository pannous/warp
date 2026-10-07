// Cards down-to-crash and natural-phrases: `10 down to 1` counts down (Kotlin's downTo, the counterpart of wiki/range.md
// `upto`), letters make ranges of letters (Kotlin's 'a'..'e'), and a range from an undefined name is the error naming
// it (it was the internal error "WASM validation failed")
use crate::is;
use crate::common::fails_with;

#[test]
fn down_to_counts_down() {
	is!("countdown = 10 down to 7; string(countdown)", "[10 9 8 7]");
	is!("n = 3; s = 0; for i in n down to 1 { s = s * 10 + i }; s", 321);
}

#[test]
fn letters_make_ranges() {
	let joined = |range: &str| format!("s = \"\"; for c in {range} {{ s = s + c }}; s");
	is!(&joined("'a' to 'e'"), "abcde");
	is!(&joined("'a'..'d'"), "abc");
	is!(&format!("c = 'w'; {}", joined("c to 'z'")), "wxyz");
	is!("letters = 'a' to 'e'; #letters", 5);
}

#[test]
fn a_range_from_an_undefined_name_names_it() {
	fails_with("r = x to 3; r", "undefined variable: x");
	fails_with("for i in x to 3 { print i }", "undefined variable: x");
}
