// In `for x in xs`, `it` names the loop variable too (wiki/plural.md `for word in text: print it`, wiki/for.md); a block
// or lambda inside the body keeps its own `it`. A symbol prints its name at run time (it was "print of a Symbol has no
// runtime text yet")
use warp::*;

#[test]
fn it_is_the_loop_variable() {
	is!("s = 0; for i in 1..4 { s += it }; s", 6);
	is!("s = 0; for x in [1, 2, 3] { s += it * 2 }; s", 12);
	is!("s = 0; for xs in [[1, 2], [3]] { s += sum(xs.map({it * 10})) }; s", 60);
}

#[test]
fn a_symbol_prints_its_name() {
	is!("x = hello; print x; 1", 1);
	is!("for friend in [foe1, friend1]: print it", 2);
}
