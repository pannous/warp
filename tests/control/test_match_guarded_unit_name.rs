// card pattern-variable: a pattern variable named like a unit (l, the liter) is the variable in a guarded arm too;
// the guard hid the pattern, so `l` in the body was 1 liter
use crate::is;

#[test]
fn a_guarded_arm_binds_a_unit_named_variable() {
	is!("match [\"key\", \"=\", \"x\", 1] { [\"key\", op, l, r] if op == \"=\" => \"assign \" + l, _ => 0 }", "assign x");
	is!("match [\"key\", \"=\", \"x\", 1] { [\"key\", op, l, r] => \"assign \" + l, _ => 0 }", "assign x");
}
