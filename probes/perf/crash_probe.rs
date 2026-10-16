// A test binary that crashes, for probes/perf/crash_rerun_probe.sh (test.sh's rerun of crashed binaries)
#[test]
fn passes() {}

#[test]
fn fails_with_a_panic() {
	assert_eq!(1 + 1, 3, "arithmetic is broken");
}

#[test]
fn aborts_the_process() {
	std::thread::sleep(std::time::Duration::from_millis(300));
	std::process::abort();
}

#[test]
fn still_running_at_the_abort() {
	std::thread::sleep(std::time::Duration::from_secs(5));
}
