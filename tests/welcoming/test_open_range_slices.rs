// card golf-open: a range slice may leave out its start or end, `s[2…]` and `s[…2]`, as `s[2:]` and `s[:3]` do
// (user 2026-10-09: "s[2…] is much better than python s[2:]")
use crate::is;

#[test]
fn a_slice_to_the_end() {
	is!("s = \"hello\"; s[2…]", "llo");
	is!("s = \"hello\"; s[2...]", "llo");
	is!("s = \"hello\"; s[2..]", "llo");
	is!("xs = [1, 2, 3, 4]; count(xs[1…])", 3);
	is!("s = \"hello\"; s#(2…)", "ello");
}

#[test]
fn a_slice_from_the_start() {
	is!("s = \"hello\"; s[…2]", "hel");
	is!("s = \"hello\"; s[...2]", "hel");
	is!("s = \"hello\"; s[..2]", "he");
	is!("xs = [1, 2, 3, 4]; count(xs[…1])", 2);
}
