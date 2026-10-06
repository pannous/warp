// `xs.find(f)`: the first element f holds for (ø when none); `xs.any(f)`, `xs.all(f)`: 1 or 0
use crate::is;
use warp::*;

#[test]
fn find_any_and_all_test_the_elements() {
	is!("[0, 1, 2].find(el => el > 0)", 1);
	is!("[0, 1].find(el => el == 2)", Empty);
	is!("[1 2 3].any(x => x > 2)", 1);
	is!("[1 2 3].any(x => x > 5)", 0);
	is!("[1 2 3].all(x => x > 0)", 1);
	is!("[1 2 3].all(x => x > 1)", 0);
	is!("words = [\"ab\" \"cde\"]; words.find(w => #w > 2)", "cde");
}
