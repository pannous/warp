// card errors-needs: a lambda among comma-separated arguments is an argument, never applied to the next one
use crate::is;

#[test]
fn a_lambda_argument_of_an_immediately_called_lambda() {
	is!("((f, x) -> f(x))(y -> y + 3, 7)", 10);
	is!("((f, x) -> f(f(x)))(y -> y + 3, 7)", 13);
}

#[test]
fn a_juxtaposed_lambda_still_applies() {
	is!("((x -> x * 2) 7)", 14);
	is!("(x -> x * 2)(7)", 14);
}
