// P49 (user, 2026-10-05): a float passed to an int parameter is a compile error, warp never loses digits silently
use crate::common::fails_with;
use crate::is;

#[test]
fn a_float_for_an_int_parameter_is_refused() {
	fails_with("f(x:int) := x+1; f(2.5)", "2.5 is no int: write 2.5 as int");
	fails_with("f(x:int) := x+1; y=2.5; f(y)", "y is no int: write y as int");
	fails_with("fun addi(int x,int y){x+y};addi(2.2,2.2)", "2.2 is no int: write 2.2 as int");
}

#[test]
fn an_explicit_cast_or_an_int_passes() {
	is!("f(x:int) := x+1; f(2.5 as int)", 3);
	is!("f(x:int) := x+1; f(2)", 3);
	is!("fun addi(int x,int y){x+y};addi(2,2)", 4);
}
