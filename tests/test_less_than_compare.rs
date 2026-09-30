//! `it<k` and `a<b` compare; only a type name before `<` starts a generic
use warp::wasm_emitter::eval;
use warp::*;

#[test]
fn test_a_less_than_after_a_variable_is_a_comparison() {
	assert_eq!(eval("xs=[1 2 3 4]; k=3; xs.filter {it<k}").serialize(), "[1 2]");
	is!("k=3; {it<k}(it=1)", 1);
	is!("a=1; b=2; a<b", 1);
}

#[test]
fn test_generics_still_start_after_a_type_name() {
	is!("x:list<int>=[1 2]; count x", 2);
	is!("x:list<list<int>>=[[1], [2]]; count x", 2);
}
