//! card big-literals: a 100 KB data literal on one line took 2.4 GB and about a minute (debug); every word read the
//! rest of its line, every node kept a copy of it, and a pass copied the tree at every level
//! card literal-test-load: measured in CPU time of the compiling thread, which other load on the machine does not
//! stretch as it stretches wall-clock time (10 s alone, over 40 s in a busy gate)
use std::time::Duration;

const RECORDS: usize = 2200;
/// Generous for a debug build (about 8 s; the quadratic version took over a minute)
const PATIENCE: Duration = Duration::from_secs(40);
#[cfg(target_os = "macos")]
const CLOCK_THREAD_CPUTIME_ID: i32 = 16;
#[cfg(not(target_os = "macos"))]
const CLOCK_THREAD_CPUTIME_ID: i32 = 3;

#[repr(C)]
struct Timespec {
	seconds: i64,
	nanoseconds: i64,
}

unsafe extern "C" {
	fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
}

/// The CPU time this thread has used so far
pub(crate) fn thread_cpu_time() -> Duration {
	let mut time = Timespec { seconds: 0, nanoseconds: 0 };
	assert_eq!(unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut time) }, 0, "clock_gettime failed");
	Duration::new(time.seconds as u64, time.nanoseconds as u32)
}

#[test]
fn a_100_kb_literal_compiles_in_linear_time() {
	let records: Vec<String> =
		(0..RECORDS).map(|i| format!("{{name: \"item{i}\", n: {i}, tags: [\"a\", \"b\"]}}")).collect();
	let source = format!("xs = [{}]\ncount xs", records.join(", "));
	assert!(source.len() > 100_000);
	let started = thread_cpu_time();
	let count = warp::wasm_emitter::eval(&source);
	let spent = thread_cpu_time() - started;
	assert!(spent < PATIENCE, "{RECORDS} records took {spent:?} of CPU time");
	assert_eq!(count, warp::Node::int(RECORDS as i64));
}
