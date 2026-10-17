// card exhaustive-assigned: a variable only ever assigned cases of one enum (`c = Dime`) is of that enum, so a match on
// it that misses a case assigned to it fails to compile; a case it never holds may be left out (test_sum_types)
use crate::common::fails_with;
use crate::is;

const COIN: &str = "enum Coin { Penny, Nickel, Dime }\n";

#[test]
fn a_variable_assigned_a_case_is_checked() {
	fails_with(&format!("{COIN}c = Dime\nmatch c {{ Penny: 1 }}"), "match on Coin misses Dime");
	fails_with(&format!("{COIN}c = Dime\nc = Penny\nmatch c {{ Penny: 1; Nickel: 5 }}"), "match on Coin misses Dime");
}

#[test]
fn a_variable_assigned_other_values_keeps_the_run_time_error() {
	fails_with(&format!("{COIN}c = Dime\nc = 5\nmatch c {{ Penny: 1 }}"), "no case");
}

#[test]
fn a_complete_match_on_an_assigned_case_runs() {
	is!(&format!("{COIN}c = Dime\nmatch c {{ Penny: 1; Nickel: 5; Dime: 10 }}"), 10);
}

#[test]
fn a_case_never_assigned_may_be_left_out() {
	is!(&format!("{COIN}c = Dime\nmatch c {{ Dime: 10; Penny: 1 }}"), 10);
}
