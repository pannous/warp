//! card runtime-text-ratio: a text known only at run time that spells a ratio is the exact ratio as number, as the
//! literal `'1/3' as number` is; a ratio of no integers is invalid number
use crate::common::fails_with;
use crate::is;

const RUN_TIME: &str = "f(x) := x\n";

#[test]
fn a_run_time_ratio_text_is_exact() {
	is!(&format!("{RUN_TIME}t = f(\"1/3\")\n(t as number) * 3"), 1);
	is!(&format!("{RUN_TIME}t = f(\"1/3\")\nn = t as number\nn == 1/3"), true);
	is!(&format!("{RUN_TIME}t = f(\"-6/4\")\n(t as number) == -3/2"), true);
	is!(&format!("{RUN_TIME}t = f(\"4/2\")\nt as number"), 2);
}

#[test]
fn a_run_time_ratio_of_no_integers_fails() {
	fails_with(&format!("{RUN_TIME}t = f(\"1/x\")\nt as number"), "invalid number");
}
