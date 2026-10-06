// `go { … }` starts a block as a task, and the program goes on at once; it waits for its tasks before it ends
// (user, 2026-10-06: "def hi: {sleep(1000 ms);print('hi')} go { hi() } print('faster')")
use crate::is;

#[cfg(feature = "native")]
const USER_PROGRAM: &str = "def hi: {sleep(1000 ms);print('hi')}\n\ngo {\n  hi()\n  }\nprint('faster')";

#[test]
#[cfg(feature = "native")]
fn a_go_block_prints_after_the_program_goes_on() {
	let printed = crate::common::printed(USER_PROGRAM);
	assert!(printed.starts_with("faster\nhi\n"), "printed {printed:?}");
}

#[test]
fn a_go_block_does_not_hold_up_the_program() {
	is!("def hi: {sleep(1000);print('hi')}; started = clock(); go { hi() }; clock() - started < 500", true);
	is!("hi() := { sleep(1000); print(\"hi\") }; started = clock(); go hi(); clock() - started < 500", true);
}

#[test]
fn a_task_of_no_arguments_gives_its_value() {
	is!("hi() := { sleep(10); \"hi\" }; await go hi()", "hi");
	is!("def hi: { sleep(10); \"hi\" }; job = go hi(); await job", "hi");
}

#[test]
fn a_go_block_sees_the_values_of_its_variables() {
	is!("n = 20; job = go { n + 1 }; await job", 21);
}

/// sleep takes a constant duration in any time unit: its milliseconds
#[test]
fn sleep_takes_a_duration() {
	is!("started = clock(); sleep(300 ms); clock() - started >= 300", true);
	is!("started = clock(); sleep 1s; clock() - started >= 1000", true);
	is!("f() := { sleep(10 ms); \"hi\" }; f()", "hi");
}
