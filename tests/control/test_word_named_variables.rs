//! card codepoint-loop-variable: a loop variable or parameter named like a library word (`codepoint`, `ord`, `upper`)
//! is that value in its body, also read bare (`x = ord`), not the library function
use crate::is;

#[test]
fn a_bound_name_shadows_the_library_word() {
	is!("xs = []; for each codepoint in \"ab\" { x = codepoint; xs = xs + [x] }; xs", warp::parse("['a' 'b']"));
	is!("n = 0; for ord in [1 2] { x = ord; n = n + x }; n", 3);
	is!("f(upper) := { x = upper; x }; f(2)", 2);
}
