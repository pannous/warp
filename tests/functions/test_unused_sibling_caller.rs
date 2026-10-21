// card unused-sibling: a second caller nobody calls leaves the used caller's calls as they are (warp-packages ciphers)
use crate::is;

#[test]
fn an_unused_sibling_caller_changes_nothing() {
	let helpers =
		"shift_letter(c, shift) := { if c >= 'a' and c <= 'z' { return chr((ord(c) - 97 + shift) % 26 + 97) }; c }\n\
		caesar(t, shift) := join([shift_letter(c, shift) for c in t], \"\")\n";
	is!(&format!("{helpers}uncaesar(t, shift) := caesar(t, -shift)\nrot13(t) := caesar(t, 13)\nrot13(\"abc\")"), "nop");
	is!(&format!("{helpers}rot13(t) := caesar(t, 13)\nrot13(\"abc\")"), "nop");
}
