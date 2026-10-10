// card float-global: a global started exact (`global g = 0.3`) that is later given a value of run-time kind (what an
// untyped function returns, here a float) holds a Node, as a local does, instead of refusing it with "not an int"
use crate::is;

#[test]
fn a_value_of_run_time_kind_widens_an_exact_global() {
	is!("f(l) := 10^(l/20.0); global g = 0.3; g = f(-9); round(g * 1000)", 355);
	is!("f(l) := 10^(l/20.0); global g = 0.3; h() := g*2; g = f(-9); round(h() * 1000)", 710);
	is!("f(l) := 10^(l/20.0); global g = 0.3; set() := { g = f(-9) }; set(); round(g * 1000)", 355);
}
