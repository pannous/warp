//! Card random-should (user): a word of the host taking nothing, `random`, is called where its value is used, as a
//! function of the program without parameters is (test_bare_calls, decision reflection P82)
use crate::is;

#[test]
fn random_is_called_where_its_value_is_used() {
	is!("random as int", 0);
	is!("random < 1", true);
	is!("x = random; x < 1 and x >= 0", true);
	is!("floor(random * 6) < 6", true);
}

#[test]
fn a_program_naming_random_keeps_its_own() {
	is!("random = 4; random + 1", 5);
	is!("f(random) := random * 2; f(3)", 6);
}

#[test]
fn random_alone_or_printed_is_a_number() {
	is!("type random", "float");
	is!("x = random as text; count(x) > 2", true);
}
