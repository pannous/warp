use crate::is;

#[test]
fn test_words_read_as_not() {
	is!("a=ø; if a empty {1} else {2}", 1);
	is!("a=3; if a empty {1} else {2}", 2);
	for word in ["empty", "missing", "absent", "unknown", "undefined"] {
		is!(&format!("a=ø; if a {word} {{1}} else {{2}}"), 1);
		is!(&format!("a=3; if a {word} {{1}} else {{2}}"), 2);
	}
}

#[test]
fn test_words_work_in_other_conditions() {
	is!("a=ø; unless a missing {1} else {2}", 2);
	is!("i=0; x=5; while x missing {i++}; i", 0);
	is!("a=ø; b=1; b=5 if a missing; b", 5);
}

#[test]
fn is_with_a_test_word() {
	is!("a=ø; a is empty", true);
	is!("a=ø; a is missing", true);
	is!("a=3; a is missing", false);
	is!("a=3; a is absent", false);
}

// wiki/null.md: the empty keyword checks missing or empty, so `x is empty` holds for "", [] and {} as for ø (a number
// or a bool is never empty); `x is ø` stays the comparison with ø
#[test]
fn is_empty_holds_for_an_empty_text_list_or_map() {
	is!("\"\" is empty", true);
	is!("t=\"\"; if t is empty {1} else {2}", 1);
	is!("xs=[]; xs is empty", true);
	is!("m={}; m is empty", true);
	is!("t=\"a\"; t is empty", false);
	is!("xs=[1]; xs is empty", false);
	is!("0 is empty", false);
	is!("no is empty", false);
	is!("f(r: any) := r.title is empty; f({title: \"\"})", true);
	is!("t=\"\"; t is ø", false);
}
