//! `for _ in 0..3 { … }` loops with an unused loop variable; `_` is no placeholder of a partial application there
//! (it made the loop a lambda that never ran: samples/particles.wasp spawned no particles)
use crate::is;

#[test]
fn a_loop_over_underscore_runs() {
	is!("x = 0; for _ in 0..3 { x += 1 }; x", 3);
	is!("x = 0; for _ in [5, 6] { x += 1 }; x", 2);
	is!("global xs = []\ndef add(n) { for _ in 0..n { xs.push(1) } }\nadd(3)\ncount(xs)", 3);
	is!("add(a, b) := a + b; plus1 = add(1, _); plus1(4)", 5);
}
