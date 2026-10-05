// `for (it>2) in xs { … }` over a list variable: its block is the body, not a construction `xs {…}` (P46 filter loops)
use warp::is;

#[test]
fn a_filter_loop_over_a_variable() {
	is!("xs=[1,5,3]; s=0; for (it>2) in xs { s += it }; s", 8);
	is!("xs=[1,2,3,4]; s=0; for (it%2==0) in xs: s += it\ns", 6);
}
