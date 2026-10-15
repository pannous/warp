// card xor-xor: a negated group is a number, so xor combines its bits instead of reading it as a truthy `or`
use crate::is;

#[test]
fn xor_of_a_negated_group_combines_bits() {
	is!("-(4) xor 1", -3);
	is!("-(4) xor 0", -4);
	is!("(-(4)) xor -1", 3);
	is!("-(2 * 3) xor 3", -7);
}
