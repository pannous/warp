//! A type word stays a type in a test (`item is pair`) where no variable of that name is in scope: a parameter or a local
//! of one function shadows it only in that function's body (card static-pair: lib/wagi.warp's `let pair = …` in
//! wagi_query turned `item is pair` everywhere into `item == pair`)
use crate::is;

#[test]
fn test_local_of_another_function_leaves_the_type_word() {
	is!("q(t:any) := { let pair = split(t, \"=\"); pair#1 }; f(value:any) := { n = 0; for item in value { if item is pair { n += 1 } }; n }; f({n: 1, m: 2})", 2);
	is!("g(pair:any) := pair; f(x:any) := x is pair; p = a: 1; f(p)", 1);
}

#[test]
fn test_own_variable_still_shadows_the_type_word() {
	is!("q(pair:any) := 3 is pair; q(3)", 1);
	is!("q(t:any) := { let pair = 3; t is pair }; q(3)", 1);
}
