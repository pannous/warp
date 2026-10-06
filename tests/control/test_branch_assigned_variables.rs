// A variable holding a Node that is first assigned inside the branches of an `if` (or a loop) is read after them.
use crate::is;

#[test]
fn a_text_assigned_in_both_branches_is_read_after_them() {
	is!("c=2; if c > 1 { n = \"ab\" } else { n = \"cd\" }; n", "ab");
	is!("f(c) := { if c > 1 { n = \"ab\" } else { n = \"cd\" }; n }; f(0)", "cd");
	is!("samples/sorting_idiomatic.wasp", "-3,0,1,2,5,5,6,7,8,9");
}
