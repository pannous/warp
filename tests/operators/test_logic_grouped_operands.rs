//! `and` / `or` take the truthiness of a grouped or negated left operand at runtime, not at compile time

use crate::is;

#[test]
fn a_grouped_comparison_is_the_left_operand_of_and() {
	is!("(1==2) and 1", 0);
	is!("(1==1) and 5", 5);
}

#[test]
fn a_negation_is_the_left_operand_of_and_and_or() {
	is!("(not true) and 1", 0);
	is!("not true and 1", 0);
	is!("(not 1) or 5", 5);
	is!("(not 0) and 7", 7);
}

#[test]
fn the_logic_table_of_negations() {
	is!("not true and !true", false);
	is!("not true and !false", false);
	is!("not false and !true", false);
	is!("not false and !false", true);
}
