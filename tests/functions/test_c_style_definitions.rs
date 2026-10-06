// C, C++, Java and C# definitions: the result type first, typed parameters, a block body
use crate::is;

#[test]
fn a_result_type_before_the_name_defines_a_function() {
	is!("int add(int a, int b) { return a + b; }; add(2,3)", 5);
	is!("int one() { return 1 }; one() + 1", 2);
	is!("float half(float x) { return x / 2 }; half(3)", 1.5);
}

#[test]
fn void_defines_a_function_without_a_result() {
	is!("void hello() { print(\"hi\") }; hello(); 3", 3);
	is!("void greet(text name) { print(\"hi \" + name) }; greet(\"bo\"); 4", 4);
	is!("x = void; x == ø", 1); // elsewhere void stays ø
}
