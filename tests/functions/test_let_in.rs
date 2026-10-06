// OCaml / F# / Haskell: `let f x = body in expr` defines f for expr; `let x = 5 in x + 1` binds x
use crate::is;

#[test]
fn let_function_in() {
	is!("let twice x = x * 2 in twice 4", 8);
	is!("let add a b = a + b in add 1 2", 3);
	is!("let twice x = x * 2; twice 4", 8);
}

#[test]
fn let_value_in() {
	is!("let x = 5 in x + 1", 6);
	is!("let x = 5; x", 5);
}
