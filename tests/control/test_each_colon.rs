// wiki/iteration.md: `each [1,2,3]: print it` and `all [1,2,3]: print it` act like `for`, the item is `it`
use warp::is;

#[test]
fn each_and_all_with_a_colon_walk_the_list() {
	is!("s = 0; each [1,2,3]: s += it; s", 6);
	is!("s = 0; all [1,2,3]: s += it * 10; s", 60);
	is!("xs = [4 5]; n = 0; each xs: n += 1; n", 2);
}

#[test]
fn a_range_do_walks_it() {
	// wiki/range.md: `1…5 do print it`
	is!("s = 0; 1..4 do s += it; s", 6);
	is!("s = 0; 1…3 do s += it; s", 6);
}
