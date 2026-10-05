// `int i = π*1000000`: a float computed from literals is a type mismatch like the float literal `int i = 3.5`, not
// invalid wasm ("WASM validation failed: expected i64, found f64"); exact decimals stay exact (`2.0*3` is 6)
use warp::is;

#[test]
fn an_int_declaration_refuses_a_float_expression() {
	crate::common::fails_with("int i=π*1000000; i", "i is declared int, cannot assign float");
	crate::common::fails_with("int i=π+1", "i is declared int, cannot assign float");
	is!("int i=2.0*3; i", 6);
	is!("int i=2*3; i", 6);
	is!("float f=π*2; f > 6", true);
}

// card int-exact: a non-whole exact value computed from literals is the same mismatch as `int i=3.5`; a whole one is fine
#[test]
fn an_int_declaration_refuses_a_non_whole_exact_value() {
	crate::common::fails_with("int i=3.3*2; i", "i is declared int, cannot assign");
	crate::common::fails_with("int i=7/2; i", "i is declared int, cannot assign");
	is!("int i=3.5*2; i", 7);
	is!("int i=0.1+0.9; i", 1);
	is!("int i=8/2; i", 4);
}
