// Hundreds of tasks at once: each runs in its own instance and store, whose GC heap reserves 1 GB of address space
// lazily (util.rs GC_HEAP_INITIAL_BYTES); 300 of them overlapping still finish together and agree on a shared sum
#![cfg(feature = "native")] // threads of the wasmtime runner
use std::time::{Duration, Instant};
use warp::is;

const TASKS: i64 = 300;
/// 300 tasks of 200 ms each: overlapping well under the minute they would take one after the other
const OVERLAPPING_LIMIT: Duration = Duration::from_secs(10);

#[test]
fn hundreds_of_tasks_run_at_once() {
	let program = format!("shared total = int[1]; f(i, xs) := {{ sleep(200); m = {{}}; m[\"k\"] = i; xs#1 += m[\"k\"]; 0 }}; for i in 0..{TASKS} {{ go f(i, total) }}; await go f(0, total); sleep(1000); total#1");
	let started = Instant::now();
	is!(&program, TASKS * (TASKS - 1) / 2);
	assert!(started.elapsed() < OVERLAPPING_LIMIT, "{TASKS} tasks of 200 ms took {:?}", started.elapsed());
}
