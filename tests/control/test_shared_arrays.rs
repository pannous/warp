// Shared arrays (user, P33 step 6, P44): `shared xs = int[n]` is n Ints every task of the run reaches, `go f(xs)`
// passes the same array; `xs#i += v` adds atomically
use warp::is;

#[test]
fn a_shared_array_reads_writes_and_adds() {
	is!("shared xs = int[4]; xs#1 += 2; xs#1 += 3; xs#1", 5);
	is!("shared xs = int[4]; xs[0] = 7; xs[0] + #xs", 11);
	crate::common::fails_with("shared xs = int[2]; xs#5", "index out of range");
}

#[test]
fn tasks_add_to_one_shared_array() {
	is!("shared hits = int[1]; bump(h, n) := { for i in 1 to n { h#1 += 1 }; n }; a = go bump(hits, 1000); b = go bump(hits, 1000); await a + await b; hits#1", 2000);
}

#[test]
fn a_shared_array_of_floats() {
	is!("shared xs = float[2]; xs#1 += 1.5; xs#1 += 2.25; xs#1", 3.75);
	is!("shared xs = float[2]; xs#2 = 0.5; xs#2 * 4", 2.0);
}

#[test]
fn tasks_add_floats_to_one_shared_array() {
	is!("shared total = float[1]; add(t, n) := { for i in 1 to n { t#1 += 0.5 }; n }; a = go add(total, 1000); b = go add(total, 1000); await a + await b; total#1", 1000.0);
}
