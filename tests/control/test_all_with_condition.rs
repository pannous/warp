// `all numbers > 2: print it` (wiki/iteration.md): `all xs op v: body` visits the items for which `it op v` holds, a
// filter loop like `for (it>2) in xs` (P46: with its got-it warning)
use warp::is;

#[test]
fn all_with_a_comparison_filters_the_items() {
	is!("numbers=[1,5,3]; s=0; all numbers > 2: s += it\ns", 8);
	is!("s=0; all [1,5,3] <= 3: s += it\ns", 4);
	is!("s=0; all [1,5,3]: s += it\ns", 9);
}
