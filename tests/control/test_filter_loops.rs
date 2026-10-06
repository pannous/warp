//! P46 (user, 2026-10-05): the wiki's filtering loops (wiki/for.md) filter as written, with a got-it warning naming
//! the filter: `for friend in xs` visits only the friend instances, `for (it>2) in xs` only the items it holds for
use warp::diagnostic::take_warnings;
use crate::is;
use warp::wasm_emitter::eval;

const PEOPLE: &str = "class friend{n:int}; class foe{n:int}; xs=[friend(1), foe(2), friend(3)]; ";

#[test]
fn test_a_class_name_loop_visits_only_its_instances() {
	is!(&format!("{PEOPLE}s=0; for friend in xs {{ s += friend.n }}; s"), 4);
	is!(&format!("{PEOPLE}c=0; for friend in xs: c += 1\nc"), 2);
}

#[test]
fn test_a_condition_loop_visits_only_matching_items() {
	is!("s=0; for (it>2) in [1,2,3,4] { s += it }; s", 7);
	is!("s=0; for (it%2==0) in [1,2,3,4]: s += it\ns", 6);
}

#[test]
fn test_a_filter_warns_once_with_its_explicit_form() {
	take_warnings();
	eval("s=0; for (it>2) in [1,2,3,4] { s += it }; s");
	assert!(take_warnings().iter().any(|warning| warning.message.contains("visits only")));
}

#[test]
fn test_plain_loops_do_not_filter() {
	take_warnings();
	is!("s=0; for x in [1,2,3] { s += x }; s", 6);
	is!("s=0; for number in [1,2,3] { s += number }; s", 6); // a type word as a variable name stays a variable
	assert!(take_warnings().iter().all(|warning| !warning.message.contains("visits only")));
}
