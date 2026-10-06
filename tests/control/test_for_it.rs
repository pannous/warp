// In `for x in xs`, `it` names the loop variable too (wiki/plural.md `for word in text: print it`, wiki/for.md); a block
// or lambda inside the body keeps its own `it`. A symbol prints its name at run time (it was "print of a Symbol has no
// runtime text yet")
use crate::is;

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

// the branches of an if in the body are no lambdas: their `it` is the loop variable too (card inside-nested)
#[test]
fn it_is_the_loop_variable_inside_an_if_block() {
	is!("xs = [1, \"a\", 2]; s = 0; for x in xs { if x is int { s += it } }; s", 3);
	is!("s = 0; for x in [1, 5, 3] { if x > 2 { s += it } else { s -= it } }; s", 7);
	is!("xs = [1, \"a\", 2]; s = 0; for int in xs { s += it }; s", 3);
	is!("s = 0; for x in [1, 2] { if x > 0 { s += sum([1, 2].map({it * 10})) } }; s", 60);
}
