// Async cases of other systems in warp's task syntax (probes/async_ports.md): JS Promise.all, Python asyncio.gather,
// Kotlin async/await, Go goroutines with a WaitGroup
use crate::is;

// Promise.all([f(1), f(2)]): every task starts, then each is awaited, results in order (they were the task handles)
#[test]
fn await_all_of_started_tasks_gives_their_results() {
	is!("f(x) := { sleep(30 ms); x * 10 }; await all [go f(1), go f(2), go f(3)]", warp::ints(vec![10, 20, 30]));
	is!("await all [go { 1 + 1 }, go { 2 * 3 }]", warp::ints(vec![2, 6]));
	is!("f(x) := { sleep(300 ms); x }; started = clock(); xs = await all [go f(1), go f(2)]; clock() - started < 550", true);
}

#[test]
fn gather_nested_and_wait_group_forms() {
	is!("jobs = []; for i in 1 to 3 { jobs.add(go { i * i }) }; sum(await all jobs)", 14);
	is!("f(n) := { a = go { n + 1 }; b = go { n * 2 }; await a + await b }; await go f(5)", 16);
	is!("shared n = 0; jobs = []; for i in 1 to 4 { jobs.add(go { n += 1 }) }; await all jobs; n", 4);
	is!("bad() := [1]#5; try await go bad() else 0", 0);
}

// Promise.race, asyncio FIRST_COMPLETED: `await any [a, b]` is the result of the first task to finish
#[test]
fn await_any_gives_the_first_to_finish() {
	const RACERS: &str = "slow() := { sleep(300 ms); 10 }; quick() := { sleep(10 ms); 20 }; ";
	is!(&format!("{RACERS}await any [go slow(), go quick()]"), 20);
	is!(&format!("{RACERS}started = clock(); r = await any [go slow(), go quick()]; clock() - started < 250"), true);
}

// asyncio.wait_for, Kotlin withTimeout (P154): `await job within 100 ms or 0` stops a late job and gives the `or` value
#[test]
fn await_within_a_deadline() {
	is!("job = go { sleep(2000 ms); 1 }; await job within 100 ms or 0", 0);
	is!("job = go { sleep(10 ms); 7 }; await job within 1000 ms or 0", 7);
	is!("job = go { sleep(2000 ms); 1 }; x = await job within 100 ms or 5; x * 2", 10);
	is!("job = go { sleep(10 ms); 7 }; x = await job within 1000 ms or 5; x * 2", 14);
	is!("job = go { sleep(2000 ms); 1 }; try await job within 50 ms else 9", 9);
}

// P153: `await first [tasks]` is the first task's result, whichever ends first (a note names `await any`)
#[test]
fn await_first_is_the_first_tasks_result() {
	is!("slow() := { sleep(300 ms); 10 }; quick() := { sleep(10 ms); 20 }; await first [go slow(), go quick()]", 10);
}
