// Card task-char: a character a task gives comes back as that character, not its code point
use crate::is;

#[test]
fn a_task_gives_back_its_character() {
	is!("job = go { 'a' }; await job", 'a');
	is!("f() := { \"a\" }; job = go f(); await job", 'a');
	is!("f() := 'b'; jobs = [go f(), go f()]; results = await all jobs; results#2", 'b');
}
