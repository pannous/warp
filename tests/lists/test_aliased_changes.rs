// Card aliased-list: pop, pop(i) and clear change the list in place, as append and remove do: every holder sees it
use crate::is;

#[test]
fn a_pop_is_seen_through_an_alias() {
	is!("xs = [1 2 3]; ys = xs; xs.pop(); count(ys)", 2);
	is!("xs = [\"a\" \"b\"]; ys = xs; xs.pop(); count(ys)", 1);
}

#[test]
fn a_pop_at_a_position_is_seen_through_an_alias() {
	is!("xs = [5 6 7]; ys = xs; xs.pop(0); ys#1 * 10 + count(ys)", 62);
	is!("m = {a:1 b:2}; n = m; m.pop(\"a\"); count(n)", 1);
}

#[test]
fn a_clear_is_seen_through_an_alias() {
	is!("xs = [1 2 3]; ys = xs; xs.clear(); count(ys)", 0);
	is!("m = {a:1 b:2}; n = m; m.clear(); count(n)", 0);
	is!("xs = [1 2 3]; ys = xs; xs.clear(); xs.add(4); ys#1 * 10 + count(ys)", 41);
}
