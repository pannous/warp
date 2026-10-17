// Card is-early-return: a function returning a text early and a list at its end returns either: its kind is known only
// at run time, so `is` tests the value, and a caller using the list never traps on a cast
use crate::is;

#[test]
fn an_early_return_of_another_kind_is_tested_at_run_time() {
	is!("f(x:any) := { if x == 1 { return \"out\" }; [1, 2] }; m = f(1); m is text", true);
	is!("f(x:any) := { if x == 1 { return \"out\" }; [1, 2] }; f(2) is list", true);
}

#[test]
fn a_word_ending_with_a_throwing_call_gives_its_other_values() {
	let code = "refuse(why: text) := { throw \"out: \" + why }\nconverted(value: any, to: text) := {\n\tif value#1 == to { return value#2 }\n\tif value#1 == \"int\" { return value#2 + [185] }\n\trefuse(\"a \" + value#1)\n}\n#(converted([\"int\", [66, 1]], \"float\") + [124])";
	is!(code, 4);
}
