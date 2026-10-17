//! The parallel let `a b = f(x) g(y)` (notes/bend.md, card lower-parallel): the calls run in parallel, each but the last
//! in a task while fewer tasks run than there are cores, else one after the other
use crate::is;

#[test]
fn a_parallel_let_binds_each_name_to_its_call() {
	is!("f(x) := x * 2; g(y) := y + 1; a b = f(3) g(4); a * 10 + b", 65);
	is!("f(x) := x * 2; a b c = f(1) f(2) f(3); a + b * 10 + c * 100", 642);
}

#[test]
fn a_recursive_parallel_let_forks_only_while_cores_are_free() {
	is!("sum(d) := { if d == 0 { return 1 }; a b = sum(d - 1) sum(d - 1); a + b }; sum(10)", 1024);
}
