// card fun-x-loud (decision foreign-spellings: OCaml's `fun x -> …` is dropped): the dropped form is an error naming
// warp's lambda, never a lambda that runs silently wrong
use crate::common::fails_with;
use crate::is;
use warp::parse;

#[test]
fn ocaml_fun_is_an_error_naming_the_arrow_lambda() {
	fails_with("map (fun x -> x * 2) [1, 2, 3]", "x -> …");
	fails_with("xs = [1, 2, 3]; xs.map(fun x -> x * 2)", "x -> …");
	fails_with("f = fun a b -> a * b; f(3, 4)", "(a, b) -> …");
}

#[test]
fn warps_lambda_and_kotlins_fun_still_work() {
	is!("map (x -> x * 2) [1, 2, 3]", parse("[2 4 6]"));
	is!("fun square(x) = x * x; square(3)", 9);
}
