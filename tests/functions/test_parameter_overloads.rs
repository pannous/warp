// Overloading by parameter types: each definition is a variant, a call takes the one its arguments fit best
use crate::common::fails_with;
use crate::is;

const COMBINE: &str = "combine(a:float, b:float) := a+b; combine(a:int, b:int) := a*b; ";

#[test]
fn a_call_takes_the_variant_its_arguments_fit() {
	is!(&format!("{COMBINE}combine(2, 3)"), 6);
	is!(&format!("{COMBINE}combine(1.5, 2.5)"), 4.0);
	is!(&format!("{COMBINE}x=2; y=3; combine(x, y)"), 6);
	is!("f(a:int) := 1; f(a:text) := 2; f(\"x\") + f(5)", 3);
}

#[test]
fn an_int_widens_to_a_float_variant() {
	is!("combine(a:float, b:float) := a+b; combine(a:string, b:string) := a; combine(2, 3)", 5.0);
}

#[test]
fn an_argument_of_unknown_type_is_an_error() {
	fails_with(&format!("{COMBINE}def g(p){{ combine(p, p) }}; g(2)"), "fits no single variant");
}
