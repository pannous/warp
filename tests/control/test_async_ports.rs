// Async cases of other systems in wasp's task syntax (probes/async_ports.md): JS Promise.all, Python asyncio.gather,
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
