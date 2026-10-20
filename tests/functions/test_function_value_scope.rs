// card function-value-names: a variable holding a function value is that function's own, not every variable of its name
use crate::is;

const HOLDERS: &str = "maker(n) := x => x + n
r(n) := { found = maker(n); found }
remember(called, key: text) := { found = 2; called[key] = found; called }
g(called, key: text) := { count = called[key]; count }
";

#[test]
fn a_local_holding_a_closure_leaves_other_functions_names_alone() {
	is!(&format!("{HOLDERS}count(x) := x + 1; count(4)"), 5);
	is!(&format!("{HOLDERS}\"abc\".remove_prefix(\"a\")"), "bc");
}
