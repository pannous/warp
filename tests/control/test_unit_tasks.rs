// Card units-tasks: a task whose function or go block uses units runs its specialisation (static units), as a call does
use crate::is;

#[test]
fn a_go_block_with_units_runs() {
	is!("job = go { 3 m }; await job", 3);
	is!("go { print 440Hz }; 1", 1);
}

#[test]
fn a_started_function_with_units_runs() {
	is!("f() := { 2 m + 1 m }; job = go f(); await job", 3);
	is!("f() := { print 440Hz }; go f(); 1", 1);
}
