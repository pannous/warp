// Listeners on shared values (notes/signals.md phase 6): a task writes a `shared` value in its own instance, so the
// program cannot check after the write; its listener polls at the program's check points instead (loop starts, sleep,
// await, the start and end of main) and runs on the program's thread when the value changed
use crate::is;

/// What the program prints, without the CLI's last line `» value`
#[cfg(feature = "native")]
fn printed(code: &str) -> String {
	let output = crate::common::printed(code);
	output.lines().filter(|line| !line.starts_with('»')).map(|line| format!("{line}\n")).collect()
}

#[test]
fn whenever_a_task_sets_a_shared_value() {
	is!("shared done = false; seen = 0; whenever done { seen = 1 }; go { sleep(30); done = true }; sleep(400); seen", 1);
}

#[test]
#[cfg(feature = "native")]
fn the_listener_runs_while_the_program_sleeps() {
	assert_eq!(printed("shared done = false\nwhenever done { print \"done!\" }\ngo { sleep(30); done = true }\nsleep(400)\nprint \"end\""), "done!\nend\n");
}

#[test]
fn on_change_of_a_shared_value_sees_the_new_value() {
	is!("shared n = 0; seen = 0; on change n { seen = value }; go { n = 5 }; sleep(300); seen", 5);
}

#[test]
fn once_on_a_shared_value_runs_once() {
	is!("shared n = 0; hits = 0; once n > 2 { hits += 1 }; go { for i in 1 to 6 { n = i; sleep(20) } }; sleep(500); hits", 1);
}

#[test]
fn a_write_of_the_program_itself_is_seen_at_its_next_check_point() {
	is!("shared n = 0; seen = 0; on change n { seen = value }; n = 7; for i in 1 to 2 { i }; seen", 7);
}

#[test]
fn changes_before_the_listener_are_not_seen() {
	is!("shared n = 0; n = 3; hits = 0; on change n { hits += 1 }; for i in 1 to 2 { i }; hits", 0);
}
