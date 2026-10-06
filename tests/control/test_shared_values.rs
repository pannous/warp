// Shared values (user decision P106, 2026-10-06): `shared done = false` is one value every task of the run reaches, as
// P44's shared arrays; everything else stays a copy per task (P33)
use crate::is;

#[test]
fn a_shared_value_reads_writes_and_adds() {
	is!("shared n = 5; n += 2; n", 7);
	is!("shared n = 5; n = n * 3; n", 15);
	is!("shared x = 0.5; x += 1.25; x", 1.75);
	is!("shared done = false; done = true; done", true);
	is!("atomic n = 1; n -= 3; n", -2);
}

#[test]
fn a_go_block_writes_a_shared_value() {
	is!("shared done = false; go { sleep(50); done = true }; sleep(500); done", true);
	is!("shared hits = 0; jobs = []; for i in 1..5 { jobs.add(go { for j in 1 to 100 { hits += 1 }; 0 }) }; await all jobs; hits", 400);
}

#[test]
fn a_function_given_a_shared_value_shares_it() {
	is!("shared n = 0; bump(c) := { c += 1; 0 }; await go bump(n); await go bump(n); n", 2);
}
