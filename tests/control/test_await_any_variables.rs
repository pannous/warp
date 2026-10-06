// `await any [a, b]` over task variables: the parser pairs `await any`, the race reads the tasks themselves
use crate::is;

const RACERS: &str = "slow() := { sleep(300 ms); 10 }; quick() := { sleep(10 ms); 20 }; a = go slow(); b = go quick(); ";

#[test]
fn await_any_of_task_variables_gives_the_first_to_finish() {
	is!(&format!("{RACERS}await any [a, b]"), 20);
	is!(&format!("{RACERS}await any [b, a]"), 20);
	is!(&format!("{RACERS}r = await any [a, b]; r * 2"), 40);
	is!(&format!("{RACERS}started = clock(); r = await any [a, b]; clock() - started < 250"), true);
}
