// The text of a constant arithmetic expression is the text of its value, not its source: `str(1+2)` is "3"
use crate::is;

#[test]
fn a_constant_expression_converts_by_its_value() {
	is!("str(1+2)", "3");
	is!("(1+2) as string", "3");
	is!("\"\" + (1+2)", "3");
	is!("\"a\" + 2*3", "a6");
	is!("\"v\" + (2^3)", "v8");
	is!("\"h\" + 3/2", "h1.5");
}

/// card string-global: a global is a variable, not the source text of its name (`string(s)` was "s")
#[test]
fn the_text_of_a_global_is_its_value() {
	is!("global s = [1]; string(s)", "[1]");
	is!("global s = int[3]; def f() { s[1] = 4 }; f(); str(s)", "[0 4 0]");
	is!("global s = [1]; string({s: 2})", "{s:2}");
}
