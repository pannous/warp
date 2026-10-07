// card compile-path: an undefined name joined to a text is the undefined variable, at the name, as in number arithmetic
// (`1 + cont`); not a type error "text + symbol" at the operator (seen in the warp dev overlay for p{ "clicks " + cont })
use crate::common::fails_with;

#[test]
fn an_undefined_name_in_a_text_sum_is_an_undefined_variable() {
	fails_with("\"clicks \" + cont", "undefined variable: cont at 1:13");
	fails_with("cont + \" clicks\"", "undefined variable: cont at 1:1");
	fails_with("p{ \"clicks \" + cont }", "undefined variable: cont at 1:16");
}
