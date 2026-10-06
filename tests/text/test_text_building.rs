// Building a text by appending: the left text grows in place when it ends where the heap starts, so a long loop of
// `t += "x"` is linear (it ran out of memory at 200000 appends) and no other text sharing those bytes ever changes
use crate::is;

#[test]
fn appending_in_a_loop_is_linear() {
	is!("t = \"\"; for i in 0..200000 { t += \"x\" }; count(t)", 200000);
}

#[test]
fn a_text_grown_in_place_leaves_its_earlier_value() {
	is!("a = \"x\"; for i in 0..2 { a += \"y\" }; b = a + \"c\"; c = a + \"d\"; b + \" \" + c", "xyyc xyyd");
	is!("a = \"x\"; for i in 0..2 { a += \"y\" }; b = a + \"c\"; a", "xyy");
}
