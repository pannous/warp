//! A time unit word that starts the next entry is a key, no unit: `{year:1970 month:1 day:1}` (card key-unit)
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn a_unit_word_before_a_colon_is_a_key() {
	is!("d = {year:1970 month:1 day:1}; [d.year, d.month, d.day]", parse("[1970 1 1]"));
	is!("d = {a:1 hour:2}; d.hour", 2);
}

#[test]
fn a_unit_word_after_a_number_is_still_a_unit() {
	is!("x = 3 months; x == 3 months", 1);
}
