// card profile-stage-programs: a list a field holds as its last use (lexer.warp `Gaps{resumes: resumes, …}` after the
// loop filling it) is filled as an array, its items read and written in constant time, and converted once there
use crate::is;

const FILLED_THEN_HELD: &str = "class Box { items: list }
def filled(n: int): int {
	items = n times [0]
	at = n - 2
	while at >= 0 {
		items[at] = items[at + 1] + 1
		at = at - 1
	}
	box = Box{items: items}
	box.items[0]
}
filled(200000)";

#[test]
fn a_list_held_last_keeps_its_items() {
	is!(FILLED_THEN_HELD, 199999);
	is!("class Box { items: list }; xs = [1, 2, 3]; xs[1] = 5; b = Box{items: xs}; b.items", warp::ints(vec![1, 5, 3]));
}

#[test]
fn a_list_changed_after_its_holder_is_shared() {
	is!("class Box { items: list }; xs = [1, 2, 3]; b = Box{items: xs}; xs[1] = 5; b.items", warp::ints(vec![1, 5, 3]));
	is!(
		"class Box { items: list }; xs = [1, 2]; for i in 1..3 { b = Box{items: xs}; xs[0] = i }; b.items",
		warp::ints(vec![2, 2])
	);
}
