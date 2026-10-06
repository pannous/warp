//! `rational` names the exact fractions; `rationals` is a list of them, like `ints`. `int` stays the type of whole numbers.
use crate::is;

#[test]
fn test_type_of_a_list_of_decimals_is_a_list_of_rational() {
	is!("type([1.5 2.5])", "list of rational");
	is!("type([0.5, 0.75])", "list of rational");
	is!("type([1/2, 3/4])", "list of rational");
}

#[test]
fn test_int_is_a_special_case_of_rational() {
	is!("type([1 2.5])", "list of rational");
	is!("type([1/2 1])", "list of rational");
	is!("type([1 2])", "list of int");
	is!("type([2.0 3.0])", "list of int");
}

#[test]
fn test_floats_are_not_rational() {
	is!("type([1.5f 2.5f])", "list of float");
	is!("type([1.5 2.5f])", "list of number");
	is!("type([1 2.5f])", "list of number");
}

#[test]
fn test_pi_is_real_whatever_its_representation() {
	is!("type(π)", "real");
	is!("type(pi)", "real");
	is!("type([π π])", "list of real");
	is!("type([π 1])", "list of number");
}

#[test]
fn test_other_mixes_stay_plain_lists() {
	is!("type([1.5 'a'])", "list");
}

#[test]
fn test_rational_and_rationals_are_type_words() {
	is!("x:rational=1/3; x + 1/6 == 1/2", 1);
	is!("type(1.5)", "rational");
	is!("type(2.0)", "int");
	is!("type(1.5f)", "float");
	is!("x:rationals=[1/2, 3/4]; count x", 2);
	is!("x:rationals=[1/2, 3/4]; type(x)", "list of rational");
	is!("x:list<rational>=[1/2, 3/4]; type(x)", "list of rational");
	is!("x:list of rational=[1/2, 3/4]; type(x)", "list of rational");
}
