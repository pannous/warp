// `chr(n)`, the character of a code point: the inverse of `ord(c)`
use warp::is;

#[test]
fn chr_is_the_inverse_of_ord() {
	is!("chr(97)", 'a');
	is!("chr(ord('b') + 1)", 'c');
	is!("caesar(s, k) := s.chars().map(c => chr((ord(c) - 97 + k) % 26 + 97)).join(\"\"); caesar(\"abz\", 1)", "bca");
}
