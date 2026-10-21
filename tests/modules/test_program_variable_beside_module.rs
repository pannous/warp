//! A program's variable leaves a used module's local of the same name alone, also where the module passes it to a method
//! (`out.has(k)` of list.warp's tally beside the program's `k = 3`; card program-var-leaks)
use crate::is;

#[test]
fn a_module_local_named_like_a_program_variable_stays_the_modules() {
	is!("k = 3; tally([1, 2, 1])[\"1\"] + k", 5);
	is!("k = 3; count(group_by([1, 2, 1], x => x)[\"1\"])", 2);
}
