// card where-it-type-leak: the element of one filter has no shape of another's, `numbers where it > 0` beside
// `members without years > 20` compares numbers (it was "Member is not Comparable" of the numbers' filter)
use crate::is;

const MEMBERS: &str = "class Member { name: text; years: int }; members: [Member] = [Member{name: \"Al\", years: 30}, Member{name: \"Bo\", years: 10}];";

#[test]
fn a_number_filter_beside_a_class_filter() {
	is!(&format!("numbers = [3, -1, 2]; kept = count(numbers where it > 0); {MEMBERS} kept + count(members without years > 20)"), 3);
}

#[test]
fn one_loop_variable_over_numbers_and_instances() {
	is!(
		&format!(
			"{MEMBERS} total = 0; for x in [1, 2] {{ total += x }}; for x in members {{ total += x.years }}; total"
		),
		43
	);
}
