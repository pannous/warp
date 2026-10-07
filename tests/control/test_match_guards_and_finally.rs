// The control flow sample (samples/control_flow.wasp) as newcomers write it: a guard `n if n < 0 =>` binds n to the
// subject and tests the condition (Rust, Scala), an assigned `d = match v {…}` takes `=>` arms and `_` too, and
// `finally {…}` after `try … catch …` runs either way while the value stays the try's or the catch's
use crate::is;

const SIGN: &str = "describe(value) := match value {\n 0 => \"zero\"\n n if n < 0 => \"negative\"\n n if n > 100 => \"large\"\n _ => \"other\" }\n";

#[test]
fn a_guard_binds_and_tests() {
	is!(&format!("{SIGN}describe(-5)"), "negative");
	is!(&format!("{SIGN}describe(150)"), "large");
	is!(&format!("{SIGN}describe(0)"), "zero");
	is!(&format!("{SIGN}describe(50)"), "other");
	is!("match [1, 5] { [a, b] if a > b => \"down\"\n [a, b] if a < b => b\n _ => 0 }", 5);
}

#[test]
fn an_assigned_match_takes_arrow_cases() {
	is!("value = 1; d = match value {\n 0 => \"zero\"\n _ => \"other\" }; d", "other");
	is!("value = 0; d = match value {\n 0 => \"zero\"\n 1 => \"one\" }; d", "zero");
}

#[test]
fn finally_runs_and_keeps_the_value() {
	is!("ran = 0; v = try { 1/0 } catch { -1 } finally { ran = 1 }; v * 10 + ran", -9);
	is!("ran = 0; v = try 5 else 0 finally ran = 1; v * 10 + ran", 51);
	is!("try:\n    x = 1/0\nexcept:\n    x = 5\nfinally:\n    x = x + 1\nx", 6);
}
