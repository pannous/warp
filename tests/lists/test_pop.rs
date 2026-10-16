// `xs.pop()` on a list variable: the last item, removed from the variable (Python's list.pop())
use crate::is;
use warp::ints;

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

#[test]
fn a_bare_pop_or_clear_changes_the_list_as_the_call_does() {
	is!("xs=[1 2]; xs.pop; xs", ints(vec![1]));
	is!("xs=[1 2]; xs.pop", 2);
	is!("xs=[1 2]; xs.clear; count xs", 0);
	is!("q={pop:3, b:1}; q.pop", 3);
	is!("def f(xs){ xs.pop; xs }; f([4 5])", ints(vec![4]));
}
