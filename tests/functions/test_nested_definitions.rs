//! card nested-def-loop: a `def` inside a braced function body ends with its own block; the lines after it, indented
//! as the def itself, are the outer body's next statements, not the nested definition's body
use crate::is;

#[test]
fn statements_after_a_nested_def() {
	is!("def total(xs: list): int {\n\tdef inner(): any {\n\t\t1\n\t}\n\tsum = 0\n\tfor item in xs { sum += item }\n\tsum\n}\ntotal([5, 6])", 11);
	is!("def outer(n) {\n\tdef twice(k) {\n\t\tk * 2\n\t}\n\ttwice(n) + 1\n}\nouter(4)", 9);
	is!("def outer(n) {\n\tdef twice(k) := k * 2\n\ttwice(n) + 1\n}\nouter(4)", 9);
	is!("f(n) := {\n\tdef twice(k) { k * 2 }\n\ttwice(n)\n}\nf(5)", 10);
}

#[test]
fn a_nested_function_keeps_its_own_map() {
	is!("def total(xs: list): int {\n\tdef inner(): any {\n\t\tfound = {}\n\t\tfound\n\t}\n\tfound = xs\n\tsum = 0\n\tfor item in found { sum += item }\n\tsum\n}\ntotal([5, 6])", 11);
}
