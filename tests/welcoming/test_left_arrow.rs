//! P145 (user, 2026-10-06): R's `x <- 3` assigns with a got-it note "write x = 3"; the cramped `x<-3` is a loud error
//! naming both readings, `x = 3` and `x < -3`; `x < -3` compares. `<-` stays reserved: no docs or examples use it
use crate::is;
use warp::diagnostic::take_assumptions;

#[test]
fn a_spaced_left_arrow_assigns_with_a_note() {
	is!("x <- 3; x + 1", 4);
	take_assumptions();
	warp::warp_parser::parse("x <- 3");
	let assumed = take_assumptions();
	assert!(assumed.iter().any(|note| note.message.contains("x = 3")), "{assumed:?}");
}

#[test]
fn a_cramped_left_arrow_is_an_error_naming_both_readings() {
	crate::common::fails_with("x = -5; x<-3", "x = 3");
	crate::common::fails_with("x = -5; x<-3", "x < -3");
}

#[test]
fn a_spaced_less_than_compares() {
	is!("x = -5; x < -3 ? 1 : 2", 1);
}
