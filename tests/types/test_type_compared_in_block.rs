// card fun-int: a block of the one comparison `{ t == int }` compares t with the type value int, as `t == int` does
// elsewhere; only `is` tests the type (decision #30)
use crate::is;

#[test]
fn type_compared_in_block() {
	is!("fun g(t) { t == int }; g int", 1);
	is!("fun g(t) { t == int }; g 3", 0);
	is!("g(t) := { t == int }; g int", 1);
	is!("fun g(t) { (t == int) }; g int", 1);
	is!("fun g(t) { t is int }; g 3", 1);
}
