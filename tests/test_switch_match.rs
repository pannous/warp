//! `switch subject {key: body …}` is map indexing that executes: the chosen body runs, `default:` catches the rest

use crate::common::fails_with;
use warp::is;

#[test]
fn a_switch_picks_the_case_of_its_subject() {
	is!("color=1; switch color {1: 10 2: 20}", 10);
	is!("color=2; switch color {1: 10 2: 20}", 20);
	is!("switch \"b\" {\"a\": 1 \"b\": 2}", 2);
	is!("switch \"abc\" {\"abc\": 1 \"b\": 2}", 1);
}

#[test]
fn the_default_case_catches_what_nothing_else_matches() {
	is!("switch 3 {1: 10 default: 0}", 0);
	is!("switch 1 {1: 10 default: 0}", 10);
	is!("n=7; switch n {1: 10 default: n+1}", 8);
}

#[test]
fn only_the_chosen_body_runs() {
	is!("x=0; switch 1 {1: x=5 2: x=7}; x", 5);
	is!("x=0; switch 2 {1: x=5 2: x=7}; x", 7);
	is!("x=0; switch 3 {1: x=5 default: x=9}; x", 9);
}

#[test]
fn match_is_another_word_for_switch() {
	is!("match 2 {1: 10 2: 20}", 20);
	is!("match 5 {1: 10 default: 0}", 0);
}

#[test]
fn no_case_and_no_default_is_a_loud_error() {
	fails_with("switch 3 {1: 10 2: 20}", "no case for 3");
	fails_with("n=4; switch n {1: 10}", "no case for n");
}

#[test]
fn a_switch_is_a_value() {
	is!("y=switch 2 {1: 10 2: 20}; y+1", 21);
}

#[test]
fn a_user_function_named_switch_wins() {
	is!("switch(a,b):=a*b; switch(2,3)", 6);
}
