// `g(a, b) := { $0 - $1 }`: a definition with parameters takes the block as its body, `$0` the first parameter, no
// closure (it gave closure_lambda_1: the body's position meta hid the block from closures.rs)
use crate::is;

#[test]
fn dollar_parameters_in_a_body_block() {
	is!("g(a, b) := { $0 - $1 }; g(5, 2)", 3);
	is!("g(a, b) := { a - b }; g(5, 2)", 3);
	is!("mk(n) := { x => x + n }; h = mk(3); h(4)", 7);
}
