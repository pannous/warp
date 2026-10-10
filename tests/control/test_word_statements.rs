// Card error-beep: a word alone on a line of a block runs as at the program's top level, it is no call with the next
// statements as its arguments (`go{ beep⏎ play 440Hz … }` gave "beep takes 0 arguments, got 7"), and a glued `go{…}`
// starts a task as `go {…}` does
use crate::is;

#[test]
fn a_word_statement_in_a_block_is_no_call_of_the_next_lines() {
	is!("seven() := 7\nf() := {\nseven\n8\n}\nf()", 8);
	is!("seven() := 7\nif 1 {\nseven\n9\n}", 9);
	is!("seven() := 7\njob = go {\nseven\n10\n}\nawait job", 10);
}

#[test]
fn a_glued_go_block_is_a_task() {
	is!("job = go{ 6 * 7 }; await job", 42);
	is!("seven() := 7\njob = go{\nseven\n10\n}\nawait job", 10);
}
