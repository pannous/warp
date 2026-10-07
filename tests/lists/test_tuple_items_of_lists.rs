// samples/neural_net.wasp: an element of a list of tuples keeps its items' kinds. `p = xs#1; p#1` of `[("a", 2)]` read
// the text "a" as the Int 97, and a list item of a pair was "not an int"
use crate::is;

#[test]
fn a_tuple_from_a_list_keeps_its_items() {
	is!("xs = [(\"ab\", 2)]; p = xs#1; a = p#1; a", "ab");
	is!("xs = [([1, 5], 2)]; p = xs#1; a = p#1; #a", 2);
	is!("f(xs) := xs#2 * 3; ex = [([0, 1], 0)]; s = 0; for (a, b) in ex { s = s + f(a) }; s", 3);
	is!("pairs = [(0, 1), (1, 2)]; s = 0; for (r, c) in pairs { s = s + r * 10 + c }; s", 13);
}
