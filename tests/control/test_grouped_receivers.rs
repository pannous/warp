//! Card dump-text: `(p).a` is `p.a`, parentheses around a variable before a dot only group it
use crate::common::fails_with;
use crate::is;

#[test]
fn a_grouped_receiver_reads_its_field() {
	is!("p = {a: 1}\n(p).a", 1);
	is!("t = \"abc\"\n(t).count", 3);
}

#[test]
fn a_grouped_text_without_the_field_names_it() {
	fails_with("for t in [\"a\" \"b\"]: print (t).dump", "no field dump");
}
