// Default parameter values as Python, Kotlin, Swift, C#, JS/TS, Ruby and Julia write them: a missing trailing argument
// takes its default, a default may read earlier parameters, a typed parameter may have one, named arguments skip them
use crate::is;

#[test]
fn a_missing_argument_takes_its_default() {
	is!("f(a, b=2) := a + b; f(1)", 3); // Julia f(a, b=2) = a + b
	is!("f(a, b=2) := a + b; f(1, 5)", 6);
	is!("def f(a=1, b=2){a*10+b}; f()", 12); // Python
	is!("def greet(name, greeting=\"Hello\"){greeting + \" \" + name}; greet(\"Bob\")", "Hello Bob");
	is!("f(x, y=10) = x + y; f(1)", 11);
}

#[test]
fn a_default_may_read_earlier_parameters() {
	is!("def f(a, b=a*2){a+b}; f(3)", 9); // Kotlin fun f(a: Int, b: Int = a*2), JS function f(a, b=a*2)
	is!("def f(a, b=a*2){a+b}; f(3, 1)", 4);
}

#[test]
fn a_typed_parameter_may_have_a_default() {
	is!("def f(a, b:int=2) {a+b}; f(1)", 3); // TS function f(a, b: number = 2), Swift func f(_ a: Int, b: Int = 2)
	is!("def f(a:int, b:int=2) {a+b}; f(1, 4)", 5);
	is!("f(a:int, b:int=2) := a+b; f(1)", 3);
	is!("def f(b:int=2) {b}; f()", 2);
	is!("def f(x:float=1.5){x*2}; f()", 3.0);
	is!("fun area(w:int, h:int=w){w*h}; area(4)", 16); // Kotlin
	is!("def label(s:text=\"none\"){s}; label()", "none");
}

#[test]
fn named_arguments_skip_defaults_and_mix_with_positional_ones() {
	is!("def f(a=1, b=2){a*10+b}; f(b=5)", 15); // Python f(b=5), C# f(b: 5)
	is!("def greet(name, greeting=\"Hello\"){greeting + \" \" + name}; greet(\"Bob\", greeting=\"Hi\")", "Hi Bob");
	is!("def f(a, b, c){a*100+b*10+c}; f(1, c=3, b=2)", 123);
	is!("def f(a, b, c){a*100+b*10+c}; f(c:3, a:1, b:2)", 123); // Swift f(c: 3, a: 1, b: 2) labels in any order
	is!("def f(a, b=5, c=7){a*100+b*10+c}; f(1, c=3)", 153);
	is!("def f(a, b:int=5, c:int=7){a*100+b*10+c}; f(1, c=3)", 153);
}

#[test]
fn calls_without_parentheses_take_defaults_too() {
	is!("def f(a, b=2){a+b}; f 1", 3);
	is!("def f(a, b:int=2){a+b}; f 1", 3);
}
