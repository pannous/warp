//! `exec sh "crontab -l"` (card read-chron): a sh template runs as its argument vector, without a shell; the value is
//! what the command printed, its trailing newlines dropped as `$(…)` does; a failing command is an error naming it
use crate::common::fails_with;
use crate::is;

#[test]
fn exec_gives_what_the_command_printed() {
	is!("exec sh \"echo hello world\"", "hello world");
	is!("word=\"a; rm -rf ~\";exec sh \"echo $word\"", "a; rm -rf ~"); // one argument, never a shell command
	is!("text=\"a\\nb\";split(exec sh \"echo $text\", \"\\n\")#2", "b");
}

#[test]
fn a_failing_command_is_an_error() {
	fails_with("exec sh \"ls /no/such/folder\"", "ls");
	fails_with("exec sh \"no_such_program_xyz\"", "no_such_program_xyz");
}
