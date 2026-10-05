// `xs#(a…b)` / `xs#(a..b)`: a 1-based slice, like `xs#i` is a 1-based index (`xs[a-1..b]`); the parentheses keep it apart
// from `xs#a..b`, the range from the value xs#a to b
use warp::is;

#[test]
fn hash_slices_are_one_based() {
	is!("xs=[1,2,3,4,5]; xs#(2…4)", warp::ints(vec![2, 3, 4]));
	is!("xs=[1,2,3,4,5]; xs#(2..4)", warp::ints(vec![2, 3]));
	is!("s=\"hello\"; s#(2…4)", "ell");
	is!("xs=[1,2,3,4,5]; i=2; j=3; count(xs#(i…j))", 2);
	is!("xs=[2,5]; r = xs#1..6; count(r)", 4); // still the range from the value xs#1
}
