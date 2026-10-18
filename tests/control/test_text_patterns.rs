// card text-patterns (decision pattern-roadmap, notes/matching.md step 8): a sum of text pieces and holes is a pattern;
// a hole's type reads its text (`id: int`) or names its characters' class (`n: digits`, interviewer's choice A)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_prefix_or_suffix_binds_the_rest() {
	is!(r#"s = "0xff"; match s { "0x" + rest => rest; _ => "no" }"#, "ff");
	is!(r#"s = "a.txt"; match s { name + ".txt" => name; _ => "no" }"#, "a");
	is!(r#"s = "0yff"; match s { "0x" + rest => rest; _ => "no" }"#, "no");
}

#[test]
fn pieces_and_holes_alternate() {
	is!(r#"s = "/users/7/posts/9"; match s { "/users/" + u + "/posts/" + p => u + p; _ => "no" }"#, "79");
	is!(r#"s = "(1,2)"; match s { "(" + a + "," + b + ")" => b + a; _ => "no" }"#, "21");
}

#[test]
fn a_typed_hole_reads_its_text() {
	is!(r#"s = "/users/7"; match s { "/users/" + id: int => id + 1; _ => 0 }"#, 8);
	is!(r#"s = "/users/me"; match s { "/users/" + id: int => id + 1; _ => 0 }"#, 0);
}

#[test]
fn a_class_hole_tests_its_characters() {
	is!(r#"s = "v12"; match s { "v" + n: digits => n; _ => "no" }"#, "12");
	is!(r#"s = "vx"; match s { "v" + n: digits => n; _ => "no" }"#, "no");
	is!(r#"s = "0xbeef"; match s { "0x" + h: hex => h; _ => "no" }"#, "beef");
	is!(r#"s = "a!"; match s { c: letter + "!" => c; _ => "no" }"#, "a");
}

#[test]
fn a_text_pattern_needs_a_piece_between_holes() {
	fails_with(
		r#"s = "ab"; match s { a + b + "!" => a; _ => "no" }"#,
		"a text pattern needs a piece between two holes",
	);
}

#[test]
fn a_known_name_in_a_sum_is_a_value() {
	is!(r#"p = "0x"; s = "0xff"; match s { p + "ff" => "same"; _ => "no" }"#, "same");
}
