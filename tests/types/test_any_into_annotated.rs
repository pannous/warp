//! P204 (card int-unchecked): a value of static type any going into an annotated int place is checked when it runs: an
//! int passes, anything else is the error "not an int" (it was read as its code point, 97 for "a")
use crate::common::fails_with;
use crate::is;

#[test]
fn any_into_an_annotated_int_is_checked_at_run_time() {
	fails_with("y: any = \"a\"; x: int = 0; x = y; x", "not an int");
	fails_with("y: any = \"a\"; x: int = y; x", "not an int");
	fails_with("xs = [1, \"a\"]; x: int = xs#2; x", "not an int");
	fails_with("f(n: int) := n; xs = [1, \"a\"]; f(xs#2)", "not an int");
	is!("y: any = 3; x: int = 0; x = y; x", 3);
	is!("y: any = 3; x: int = y; x + 1", 4);
	is!("xs = [1, \"a\"]; x: int = xs#1; x", 1);
	is!("f(n: int) := n + 1; xs = [1, \"a\"]; f(xs#1)", 2);
	is!("y: any = true; x: int = y; x + 1", 2);
}

#[test]
fn unannotated_places_stay_lax() {
	is!("c = 'a'; c >= 'a'", true);
	is!("xs = [1, \"a\"]; x = xs#2; x", "a");
}
