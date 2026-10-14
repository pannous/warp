//! card big-literals: a 100 KB data literal on one line took 2.4 GB and about a minute (debug); every word read the
//! rest of its line, every node kept a copy of it, and a pass copied the tree at every level
use std::time::{Duration, Instant};

const RECORDS: usize = 2200;
/// Generous for a debug build on a busy machine (about 8 s alone; the quadratic version took over a minute)
const PATIENCE: Duration = Duration::from_secs(40);

#[test]
fn a_100_kb_literal_compiles_in_linear_time() {
	let records: Vec<String> =
		(0..RECORDS).map(|i| format!("{{name: \"item{i}\", n: {i}, tags: [\"a\", \"b\"]}}")).collect();
	let source = format!("xs = [{}]\ncount xs", records.join(", "));
	assert!(source.len() > 100_000);
	let started = Instant::now();
	let count = warp::wasm_emitter::eval(&source);
	assert!(started.elapsed() < PATIENCE, "{RECORDS} records took {:?}", started.elapsed());
	assert_eq!(count, warp::Node::int(RECORDS as i64));
}
