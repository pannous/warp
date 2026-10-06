// A raise inside a task goes to the starting program's `on` handlers (P110, notes/signals.md phase 6): they run there,
// with the program's variables, at its next await or loop start or at the end of the run
use crate::is;

#[test]
fn a_raise_in_a_task_runs_the_programs_handler() {
	is!("n = 0; check(x) := { raise found{value: x}; 0 }; on found { n = n + 1 }; job = go check(5); await job; n", 1);
	is!("seen = 0; check(x) := { raise found{value: x}; 0 }; on found { seen = event.value }; await go check(7); seen", 7);
}

#[test]
#[cfg(feature = "native")]
fn a_raise_nobody_awaits_is_handled_before_the_run_ends() {
	let printed = crate::common::printed("check() := { raise alarm; 0 }\non alarm { print \"alarm!\" }\ngo check()\nprint \"main\"");
	assert!(printed.starts_with("main\nalarm!\n"), "printed {printed:?}");
}

#[test]
fn a_raise_without_handler_fails_the_task() {
	crate::common::fails_with("check() := { raise \"boom\"; 0 }; job = go check(); await job", "boom");
}
