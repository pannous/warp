// P158: `then` pipes when the right side is a function stage missing its argument and no `else` follows
// (`xs then sort`, `square numbers then filter(…)`); otherwise it is the condition, also without `if`
// (`x > 2 then 5`); a comparison before a function stays the condition, with a note naming `|>`
use crate::is;
use warp::ints;

#[test]
fn then_pipes_into_a_function() {
	is!("xs = [3,1,2]; xs then sort", ints(vec![1, 2, 3]));
	is!("xs = [1,6,9]; xs then filter(x => x > 5)", ints(vec![6, 9]));
	is!("square x := x*x; numbers = [1,2,3]; square numbers then filter(x => x > 5)", ints(vec![9]));
	is!("twice(x) := x * 2; 3 then twice then twice", 12);
}

#[test]
fn then_without_if_is_the_condition() {
	is!("x = 3; x > 2 then 5", 5);
	is!("x = 1; x < 2 then 1 else 2", 1);
	is!("x = 3; x < 2 then 1 else 2", 2);
	is!("twice(x) := x * 2; x = 3; x > 2 then twice(x) else 0", 6);
}
