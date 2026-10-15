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

/// decision random-forms (Interviewer, 2026-10-15): `random(a, b)` of whole numbers is a whole number from a to b,
/// both included; of a float a float in [a, b); `random(n)` stays 0…n-1
#[test]
fn random_between_two_numbers() {
	is!("all([random(1, 6) for i in 1 to 50], x => x >= 1 and x <= 6)", true);
	is!("type random(1, 6)", "int");
	is!("x = random(0.5, 2); x >= 0.5 and x < 2", true);
	is!("type random(0.5, 2)", "float");
	is!("random(3, 3)", 3);
}

/// decision random-forms: `random T` is a random value of the type T
#[test]
fn random_of_a_type() {
	is!("x = random int; type x", "int");
	is!("x = random bool; type x", "bool");
	is!("x = random codepoint; type x", "codepoint");
	is!("x = random float; type x", "float");
	crate::common::fails_with("random text", "random int");
}

/// decision random-forms: random of a range picks one of its elements, `..` without its end, `...` and `to` with it; the
/// type parameter takes the type itself, `random(int)` as `random int`
#[test]
fn random_of_a_range() {
	is!("all([random 1..4 for i in 1 to 50], x => x >= 1 and x < 4)", true);
	is!("any([random 1...4 for i in 1 to 80], x => x == 4)", true);
	is!("random 1..2", 1);
	is!("random 3...3", 3);
	is!("random 4 to 4", 4);
	is!("x = random(int); type x", "int");
	is!("x = random 0.5..2; x >= 0.5 and x < 2", true);
}

/// decision random-forms: the result follows the argument, and a declared float target makes the random a float
#[test]
fn random_into_a_declared_type() {
	is!("int z = random 1..2; z", 1);
	is!("int z = random 1..10; z >= 1 and z < 10", true);
	is!("float z = random 1..10; z >= 1 and z < 10", true);
	is!("float z = random 1..10; type z", "float");
	is!("z: float = random(1, 10); type z", "float");
}

/// The prelude words the random forms lower to, called by themselves
#[test]
fn the_random_prelude_words() {
	is!("use prelude; random_until(1, 2)", 1);
	is!("use prelude; random_between(3, 3)", 3);
	is!("use prelude; x = random_fraction(1, 2); x >= 1 and x < 2", true);
	is!("use prelude; type random_int()", "int");
	is!("use prelude; type random_bool()", "bool");
	is!("use prelude; type random_codepoint()", "codepoint");
}
