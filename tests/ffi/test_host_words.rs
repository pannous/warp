//! Host words of the running program's environment: `sleep(ms)` pauses, `random()` is a float in [0, 1),
//! `random(n)` an int in 0..n (n excluded), `clock()` the milliseconds since the Unix epoch
use crate::is;

#[test]
fn test_random_stays_in_range() {
	is!("r = random(); r >= 0 and r < 1", true);
	is!("ok = 1; for i in 1 to 100 { r = random(10); if r < 0 or r >= 10 { ok = 0 } }; ok", 1);
	is!("random(1)", 0);
}

#[test]
fn test_clock_and_sleep() {
	is!("clock() > 1700000000000", true);
	is!("start = clock(); sleep(20); clock() - start >= 20", true);
}

#[test] // each runner used to link one import family: printing and reading failed with `unknown import: host::fetch`
fn test_a_program_prints_and_uses_the_host() {
	is!("x = read(\"Cargo.toml\"); print \"read\"; count(x) > 3", true);
	is!("import rand from \"c\"; print \"called C\"; rand() >= 0", true);
}
