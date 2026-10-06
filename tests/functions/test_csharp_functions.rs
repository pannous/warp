// C# (and Java/C) methods: typed parameters with defaults `int b = 2`, expression-bodied members `int Add(…) => a + b`,
// modifiers before the result type
use crate::is;

#[test]
fn a_typed_parameter_with_a_default() {
	is!("int Add(int a, int b = 2) { return a + b; }; Add(1)", 3);
	is!("int Add(int a, int b = 2) { return a + b; }; Add(1, 5)", 6);
}

#[test]
fn an_expression_bodied_method() {
	is!("int Add(int a, int b) => a + b; Add(1, 2)", 3);
	is!("static int Add(int a, int b) => a + b; Add(1, 2)", 3);
	is!("public static double Half(double x) => x / 2; Half(3)", 1.5);
}
