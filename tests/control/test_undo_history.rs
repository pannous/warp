// card web-stores: `undo x` gives x the value it had before its last change, `redo x` the one undone; a new change
// after an undo drops what was undone (lowering/undo_history.rs)
use crate::is;

#[test]
fn undo_steps_back_through_the_changes() {
	is!("x = 1; x = 2; x = 3; undo x; x", 2);
	is!("x = 1; x = 2; x = 3; undo x; undo x; x", 1);
	is!("x = 1; undo x; x", 1);
	is!("x = 1; x = 1; x = 2; undo x; undo x; x", 1); // a write of the same value is no change
}

#[test]
fn redo_takes_back_an_undo_until_a_new_change() {
	is!("x = 1; x = 2; undo x; redo x; x", 2);
	is!("x = 1; x = 2; undo x; x = 5; redo x; x", 5);
}

#[test]
fn any_value_and_any_writer_has_a_history() {
	is!("t = \"a\"; t = \"b\"; undo t; t", "a");
	is!("xs = [1]; xs = [1,2]; undo xs; string(xs)", "[1]");
	is!("n = 0; def bump() { global n; n += 1 }; bump(); bump(); undo n; n", 1);
	is!("stored kept_undo = 1; kept_undo = 2; kept_undo = 3; undo kept_undo; kept_undo", 2);
}
