// `xs.pop()` on a list variable: the last item, removed from the variable (Python's list.pop())
use warp::{ints, is};

#[test]
fn pop_gives_the_last_item() {
	is!("s=[1 2 3]; s.pop()", 3);
	is!("s=[1 2 3]; s.pop() + s.pop()", 5);
}

#[test]
fn pop_removes_the_last_item() {
	is!("s=[1 2 3]; s.pop(); s", ints(vec![1, 2]));
	is!("def f(xs){ xs.pop(); xs }; f([4 5])", ints(vec![4]));
}
