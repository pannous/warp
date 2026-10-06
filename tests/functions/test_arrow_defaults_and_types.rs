// JS/TS arrow functions with defaults or types: `(a, b = 2) => a * b`, `(a: number): number => a * 2` said
// "cannot extract a numeric value"; they define the function as `f(a, b = 2) := …` and `f(a:number):number := …` do
use crate::is;

#[test]
fn an_arrow_function_takes_defaults_and_types() {
	is!("const f = (a, b = 2) => a * b; f(4)", 8);
	is!("f = (a, b = 2) => a * b; f(4, 3)", 12);
	is!("let f = (a: number): number => a * 2; f(3)", 6);
	is!("f = (a: int, b = 1): int => a + b; f(3)", 4);
}
