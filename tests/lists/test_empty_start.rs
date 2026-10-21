// Card start (decisions empty-plus-text, list-append-plus): `t = []` starts a list that `+= item` appends to, `t = ø`
// starts nothing: ø + text fails as ø + 1 does, with the fix `t = ""`
use crate::common::fails_with;
use crate::is;
use warp::{ints, parse};

const GATHERED: &str = "def gathered(ps: any) { t = []; for p in ps { t += p }; t }\n";

#[test]
fn an_empty_list_start_appends_each_item() {
	is!("t = []; t += \"a\"; t", parse("[\"a\"]"));
	is!(&format!("{GATHERED}gathered([\"a\", \"b\"])"), parse("[\"a\" \"b\"]"));
	is!(&format!("{GATHERED}gathered([1, 2])"), ints(vec![1, 2]));
	is!(&format!("{GATHERED}gathered([[1], 2, [3, 4]])"), ints(vec![1, 2, 3, 4]));
}

#[test]
fn nothing_plus_a_text_fails_with_the_text_start() {
	fails_with("t = ø; t += \"a\"; t", "t = \"\"");
	fails_with("def f(ps: any) { t = ø; for p in ps { t += p }; t }\nf([\"a\"])", "ø plus a text");
	is!("t = \"\"; t += \"a\"; t", "a");
	is!("def f(a: any) { a + \"!\" }\nf(\"hi\")", "hi!");
}
