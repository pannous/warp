//! Card task-name: a function's own name (a loop variable, a local) is its own, also when main names a task or a
//! variable the same; found with `c = go nap(1000)` in a site program under std/markup.wasp's `for c in chars(name)`
use crate::is;

#[test]
fn a_loop_variable_named_like_a_task_is_the_loop_variable() {
	is!("nap(ms) := ms; letters(name) := { n = 0; for c in chars(name) { n += 1 }; n }; c = go nap(5); letters(\"abc\") + await c", 8);
}

#[test]
fn a_loop_variable_leaves_the_main_variable_alone() {
	is!("letters(name) := { n = 0; for c in chars(name) { n += 1 }; n }; c = 7; letters(\"abc\") + c", 10);
	is!("f(k) := { n = 0; for i in 1..k { n += i }; n }; i = 7; f(3) + i", 10);
}
