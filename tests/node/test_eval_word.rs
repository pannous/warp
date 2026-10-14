//! Card eval: `eval(text)` runs the code a text holds, `interpret(parse(text))`, with the names of where it runs
use crate::is;

#[test]
fn test_eval_word() {
	is!("eval(\"1+1\")", 2);
	is!("eval \"2*3\"", 6);
	is!("x = 5; eval(\"x*2\")", 10);
	is!("t = \"1+2\"; eval(t)", 3);
	is!("eval(t) := 42; eval(\"1+1\")", 42); // a program's own eval stays its own
}
