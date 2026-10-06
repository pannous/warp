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
